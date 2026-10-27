//! Runs the core with the app: started after the tray is up, stopped on exit.
//!
//! Starting opens the database and runs migrations, so it happens on a helper thread: the main
//! thread owns the event loop and must never wait on disk.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

use tauri::{AppHandle, Manager};
use tokenradar_core::config::{self, AppConfig};
use tokenradar_core::domain::Timestamp;
use tokenradar_core::events::{CoreEvent, Delivery, Subscription};
use tokenradar_core::limits::{LimitEvent, PanelSnapshot};
use tokenradar_core::live_sessions::LiveStatus;
use tokenradar_core::paths::Paths;
use tokenradar_core::providers::claude;
use tokenradar_core::runtime::{
    CoreConfig, CoreHandle, CoreRuntime, CoreStatus, IpcConfig, NotifySetup, SHUTDOWN_TIMEOUT,
};

use crate::tray::system::SystemClock;
use crate::tray::{self, TrayMsg};
use crate::{ipc, notify};

/// The running core, once it has started.
#[derive(Default)]
struct CoreState(Arc<Mutex<Option<CoreHandle>>>);

/// Starts the core in the background.
pub fn start(app: &AppHandle) {
    let state = Arc::default();
    app.manage(CoreState(Arc::clone(&state)));
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("core-start".into())
        .spawn(move || match open() {
            Ok(handle) => {
                let sub = handle.subscribe();
                tray::send(&app, TrayMsg::CoreReady(handle.store().clone()));
                if let Some(limits) = handle.status().limits {
                    tray::send(&app, TrayMsg::Snapshot(Box::new(limits)));
                }
                *state.lock().unwrap_or_else(PoisonError::into_inner) = Some(handle);
                tauri::async_runtime::spawn(forward(app, sub, state));
            }
            Err(error) => {
                eprintln!("TokenRadar: could not start the core: {error}");
                tray::send(&app, TrayMsg::CoreFailed);
            }
        });
    if let Err(error) = spawned {
        eprintln!("TokenRadar: could not start the core thread: {error}");
    }
}

/// Stops the core, waiting at most [`SHUTDOWN_TIMEOUT`].
pub fn shutdown(app: &AppHandle) {
    let Some(state) = app.try_state::<CoreState>() else {
        return;
    };
    let handle = state
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    if let Some(handle) = handle {
        let report = handle.shutdown(SHUTDOWN_TIMEOUT);
        if !report.timed_out.is_empty() {
            eprintln!(
                "TokenRadar: core threads still running at exit: {:?}",
                report.timed_out
            );
        }
    }
}

fn open() -> Result<CoreHandle, Box<dyn std::error::Error>> {
    let paths = Paths::resolve()?;
    let accounts = claude::discover(&[]);
    // The first discovered account is the primary one: a v1 consent moves to it (ADR-033).
    let primary = accounts.first().map(|a| a.config_dir.clone());
    match config::migrate(paths.config_path(), primary.as_deref()) {
        Ok(true) => tracing::info!("config.toml migrated to schema v2"),
        Ok(false) => {}
        Err(error) => tracing::warn!(%error, "config.toml not migrated"),
    }
    // An unreadable config must not stop the app; it only means the defaults and no consent.
    let app_config = AppConfig::load(paths.config_path()).unwrap_or_else(|error| {
        tracing::warn!(%error, "config.toml ignored");
        AppConfig::default()
    });
    let mut config = CoreConfig::new(paths.db_path(), accounts);
    // One consent for all pollers until multi-account settings exist (Step 19).
    config.api_consent = primary
        .as_deref()
        .is_some_and(|dir| app_config.api_consent_granted(dir));
    config.retention_days = Some(app_config.data.clone().normalized().raw_retention_days);
    config.notify = Some(NotifySetup {
        prefs: app_config.notifications.clone(),
        locale: app_config.general.locale(),
        clock: Arc::new(SystemClock),
    });
    // The statusline bridge (Step 07): serve the CLI and import what it spooled meanwhile.
    match tokenradar_core::ipc::pipe_name() {
        Ok(pipe_name) => {
            config.ipc = Some(IpcConfig {
                pipe_name,
                server_id: format!("tokenradar-app/{}", env!("CARGO_PKG_VERSION")),
            });
        }
        Err(error) => eprintln!("TokenRadar: no IPC pipe: {error}"),
    }
    config.spool_dir = Some(paths.spool_dir());
    Ok(CoreRuntime::start(&config)?)
}

/// The panel's view model, or `None` while the core is not running (starting or failed).
pub fn panel_snapshot(app: &AppHandle) -> Option<PanelSnapshot> {
    #[cfg(any(debug_assertions, feature = "spike"))]
    if let Some(snapshot) = crate::panel::fake::panel_snapshot(app) {
        return snapshot;
    }
    let state = app.try_state::<CoreState>()?;
    status(&state.0).and_then(|s| s.panel_snapshot())
}

/// The panel opened or "Refresh" was clicked: with API consent, data older than 60 s is
/// fetched again (the poller keeps its minimum interval and backoff).
pub fn panel_opened(app: &AppHandle) {
    with_core(app, CoreHandle::panel_opened);
}

/// Runs `f` on the core once it is running; before that, or after a failed start, nothing
/// happens (the core reads the stored settings when it starts).
pub fn with_core(app: &AppHandle, f: impl FnOnce(&CoreHandle)) {
    let Some(state) = app.try_state::<CoreState>() else {
        return;
    };
    if let Some(handle) = state
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
    {
        f(handle);
    }
}

/// Hands the core's limit snapshots to the tray and its events to the visible windows.
async fn forward(app: AppHandle, mut sub: Subscription, state: Arc<Mutex<Option<CoreHandle>>>) {
    let log = std::env::var_os("TOKENRADAR_DEBUG_EVENTS").is_some();
    while let Some(delivery) = sub.recv().await {
        if log {
            log_delivery(&delivery);
        }
        match delivery {
            Delivery::Event(event) => {
                match &event {
                    CoreEvent::LimitsUpdated(snapshot) => {
                        tray::send(&app, TrayMsg::Snapshot(snapshot.clone()));
                    }
                    CoreEvent::Notify(notice) => notify::show(&app, notice),
                    _ => {}
                }
                ipc::emit_core_event(&app, event);
            }
            // Missed events: rebuild from the full state instead of patching.
            Delivery::Resync { .. } => {
                if let Some(limits) = status(&state).and_then(|s| s.panel_snapshot()) {
                    let limits = Box::new(limits);
                    tray::send(&app, TrayMsg::Snapshot(limits.clone()));
                    ipc::emit_core_event(&app, CoreEvent::LimitsUpdated(limits));
                }
            }
        }
    }
}

fn status(state: &Mutex<Option<CoreHandle>>) -> Option<CoreStatus> {
    state
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map(CoreHandle::status)
}

/// One timestamped stderr line per event, for the Step 05 latency checks. Counts and times
/// only: no paths, names or session ids.
fn log_delivery(delivery: &Delivery) {
    let now = Timestamp::now();
    let line = match delivery {
        Delivery::Event(CoreEvent::UsageAppended {
            sessions, to_ts, ..
        }) => format!(
            "usage_appended sessions={} lag_from_last_record_ms={}",
            sessions.len(),
            now.as_millis() - to_ts.as_millis()
        ),
        Delivery::Event(CoreEvent::LiveSessionsChanged(list)) => {
            let busy = list.iter().filter(|s| s.status == LiveStatus::Busy).count();
            format!("live_sessions total={} busy={busy}", list.len())
        }
        Delivery::Event(CoreEvent::IndexProgress { progress, .. }) => format!(
            "index_progress {:?} files={}/{}",
            progress.phase, progress.files_done, progress.files_total
        ),
        Delivery::Event(CoreEvent::IndexMilestone { milestone, .. }) => {
            format!("index_milestone {milestone:?}")
        }
        Delivery::Event(CoreEvent::CredentialsChanged { .. }) => "credentials_changed".to_owned(),
        Delivery::Event(CoreEvent::SchemaDrift { unknown_count, .. }) => {
            format!("schema_drift unknown={unknown_count}")
        }
        Delivery::Event(CoreEvent::LimitsUpdated(snapshot)) => limits_line(snapshot),
        Delivery::Event(CoreEvent::Limit(event)) => limit_event_line(event),
        Delivery::Event(_) => "other".to_owned(),
        Delivery::Resync { missed } => format!("resync missed={missed}"),
    };
    eprintln!("[{now}] {line}");
}

/// Percentages, sources and API state for the Step 06 comparison with `/usage`. No token,
/// path or session content can appear here: the snapshot holds none of them.
fn limits_line(snapshot: &PanelSnapshot) -> String {
    let mut line = format!(
        "limits_updated api={:?} requests={} errors={} last_success={}",
        snapshot.api.state,
        snapshot.api.diagnostics.requests,
        snapshot.api.diagnostics.errors,
        snapshot
            .api
            .diagnostics
            .last_success
            .map_or_else(|| "-".to_owned(), |t| t.to_string()),
    );
    for l in &snapshot.limits {
        let scope = l
            .scope
            .as_ref()
            .map_or_else(String::new, |s| format!("[{s:?}]"));
        let resets = l
            .resets_at
            .map_or_else(|| "-".to_owned(), |t| t.to_string());
        let _ = write!(
            line,
            " | {:?}{scope} {:.1}% src={:?} {:?} resets={resets}",
            l.kind, l.percent, l.source, l.freshness
        );
    }
    line
}

fn limit_event_line(event: &LimitEvent) -> String {
    match event {
        LimitEvent::ThresholdCrossed {
            kind,
            threshold,
            percent,
            ..
        } => format!("limit_threshold {kind:?} >= {threshold} ({percent:.1}%)"),
        LimitEvent::ProjectionChanged {
            kind,
            will_hit_before_reset,
            ..
        } => format!("limit_projection {kind:?} will_hit={will_hit_before_reset}"),
        LimitEvent::Reset {
            kind,
            previous_percent,
            ..
        } => format!("limit_reset {kind:?} previous={previous_percent:.1}%"),
    }
}
