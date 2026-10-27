//! Update checks against GitHub Releases (ADR-012, ADR-013, ADR-036).
//!
//! - At most one automatic check a day, two minutes after the start at the earliest, and only
//!   while `[data].update_check` is on. "Check now" in About always works.
//! - Nothing installs on its own: a found update shows in the tray menu and in About, and the
//!   user starts "Install and restart". The minisign signature is verified before the installer
//!   runs; the installer then replaces the app in passive mode and starts it again.
//! - Portable copies (and Scoop) are updated by replacing the folder, so they only show the
//!   release and a link to it.

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt as _};
use tauri_specta::Event;
use tokenradar_core::domain::Timestamp;
use tokenradar_core::paths::Paths;
use tokio::sync::Notify;

use crate::{core_bridge, ipc, prefs, tray};

/// `kv` key of the last automatic or manual check, in Unix milliseconds.
const KV_CHECKED_AT: &str = "update_checked_at";
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
/// Keeps the network and TLS work away from the start (tray and indexing come first).
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpdateStatus {
    /// Not checked since the app started.
    Idle,
    Checking,
    /// The last check found nothing newer.
    UpToDate,
    /// The last check found [`UpdateView::available`].
    Found,
    /// Downloading; the installer takes over at 100 %.
    Installing {
        percent: Option<u8>,
    },
    /// The last check or install failed. A found update stays available.
    Failed {
        stage: FailedStage,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum FailedStage {
    Check,
    Install,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct AvailableUpdate {
    pub version: String,
    /// Release notes from `latest.json` (Markdown, shown as plain text).
    pub notes: Option<String>,
    /// Publication date as `YYYY-MM-DD`.
    pub date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct UpdateView {
    pub status: UpdateStatus,
    /// The newer release once a check found one; kept until a check finds none.
    pub available: Option<AvailableUpdate>,
    pub current_version: String,
    /// Last check, in Unix milliseconds.
    pub checked_at: Option<i64>,
    /// `false` for portable copies and development builds: no installer to run.
    pub can_install: bool,
}

/// Sent to visible windows whenever [`UpdateView`] changes.
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct UpdateChanged(pub UpdateView);

/// Managed by Tauri.
pub struct Updates {
    inner: Mutex<Inner>,
    /// Wakes the scheduler when the setting changes.
    wake: Notify,
    can_install: bool,
}

struct Inner {
    status: UpdateStatus,
    checked_at: Option<i64>,
    /// The update "Install and restart" applies, with what the UI shows of it.
    pending: Option<(Update, AvailableUpdate)>,
}

impl Updates {
    fn view(&self) -> UpdateView {
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        UpdateView {
            status: inner.status,
            available: inner.pending.as_ref().map(|(_, shown)| shown.clone()),
            current_version: env!("CARGO_PKG_VERSION").to_owned(),
            checked_at: inner.checked_at,
            can_install: self.can_install,
        }
    }
}

/// When the next automatic check is due, counted from `now`; `None` while checks are off.
/// Never more than a day, even when the stored time is ahead of a clock that went back.
fn next_check_in(enabled: bool, checked_at: Option<i64>, now: i64) -> Option<Duration> {
    if !enabled {
        return None;
    }
    let due = checked_at.map_or(now, |at| at.saturating_add(DAY_MS));
    let wait = due.saturating_sub(now).clamp(0, DAY_MS);
    Some(Duration::from_millis(u64::try_from(wait).unwrap_or(0)))
}

/// Only an installed copy can update itself: the NSIS installer leaves `uninstall.exe` next to
/// the app, and a portable marker means the folder is replaced by hand (or by Scoop).
fn installed() -> bool {
    let portable = Paths::resolve().is_ok_and(|p| p.is_portable());
    let has_uninstaller = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("uninstall.exe").is_file()))
        .unwrap_or(false);
    !portable && has_uninstaller
}

/// Registers the state and starts the daily schedule (release builds only: development builds
/// check when asked).
pub fn init(app: &AppHandle) {
    app.manage(Updates {
        inner: Mutex::new(Inner {
            status: UpdateStatus::Idle,
            checked_at: None,
            pending: None,
        }),
        wake: Notify::new(),
        can_install: installed(),
    });
    if cfg!(debug_assertions) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK_DELAY).await;
        loop {
            let enabled = app.state::<prefs::Prefs>().prefs_view().data.update_check;
            let checked_at = stored_checked_at(&app);
            let updates = app.state::<Updates>();
            match next_check_in(enabled, checked_at, Timestamp::now().as_millis()) {
                Some(wait) if wait.is_zero() => {
                    check(&app).await;
                }
                Some(wait) => {
                    tokio::select! {
                        () = tokio::time::sleep(wait) => {}
                        () = updates.wake.notified() => {}
                    }
                }
                None => updates.wake.notified().await,
            }
        }
    });
}

/// The automatic-check setting changed.
pub fn setting_changed(app: &AppHandle) {
    if let Some(updates) = app.try_state::<Updates>() {
        updates.wake.notify_one();
    }
}

pub fn view(app: &AppHandle) -> UpdateView {
    app.state::<Updates>().view()
}

/// The stored check time; the in-memory one while the core is not running.
fn stored_checked_at(app: &AppHandle) -> Option<i64> {
    let mut value = None;
    core_bridge::with_core(app, |core| {
        value = core.store().kv_get(KV_CHECKED_AT).ok().flatten();
    });
    value
        .and_then(|v| v.parse().ok())
        .or_else(|| view(app).checked_at)
}

fn store_checked_at(app: &AppHandle, at: i64) {
    core_bridge::with_core(app, |core| {
        if let Err(error) = core
            .store()
            .writer()
            .kv_set(KV_CHECKED_AT.to_owned(), at.to_string())
        {
            tracing::warn!(%error, "could not store the update check time");
        }
    });
}

/// Changes the state, then tells the open windows and, when the offered version changed, the
/// tray.
fn publish(app: &AppHandle, f: impl FnOnce(&mut Inner)) {
    let updates = app.state::<Updates>();
    let before = updates.view();
    f(&mut updates.inner.lock().unwrap_or_else(PoisonError::into_inner));
    let after = updates.view();
    if after == before {
        return;
    }
    let version = |v: &UpdateView| v.available.as_ref().map(|a| a.version.clone());
    if version(&after) != version(&before) {
        tray::send(app, tray::TrayMsg::Update(version(&after)));
    }
    ipc::emit_to_visible(app, &UpdateChanged(after));
}

/// Looks for a newer release; the result is published and returned.
pub async fn check(app: &AppHandle) -> UpdateView {
    let busy = matches!(
        view(app).status,
        UpdateStatus::Checking | UpdateStatus::Installing { .. }
    );
    if busy {
        return view(app);
    }
    publish(app, |i| i.status = UpdateStatus::Checking);
    let now = Timestamp::now().as_millis();
    store_checked_at(app, now);

    let exit_app = app.clone();
    let result = match app
        .updater_builder()
        // The installer starts and the process exits without `RunEvent::Exit`: close the store.
        .on_before_exit(move || core_bridge::shutdown(&exit_app))
        .build()
    {
        Ok(updater) => updater.check().await,
        Err(error) => Err(error),
    };
    match result {
        Ok(Some(update)) => {
            tracing::info!(version = %update.version, "update available");
            let shown = AvailableUpdate {
                version: update.version.clone(),
                notes: update.body.clone().filter(|b| !b.trim().is_empty()),
                date: update.date.map(|d| d.date().to_string()),
            };
            publish(app, |i| {
                i.status = UpdateStatus::Found;
                i.checked_at = Some(now);
                i.pending = Some((update, shown));
            });
        }
        Ok(None) => {
            tracing::info!("no update available");
            publish(app, |i| {
                i.status = UpdateStatus::UpToDate;
                i.checked_at = Some(now);
                i.pending = None;
            });
        }
        Err(error) => {
            tracing::warn!(%error, "update check failed");
            publish(app, |i| {
                i.status = UpdateStatus::Failed {
                    stage: FailedStage::Check,
                };
                i.checked_at = Some(now);
            });
        }
    }
    view(app)
}

/// Downloads the pending update, verifies its signature and runs the installer. On success the
/// process exits inside this call (Windows); after an error the update stays available.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let updates = app.state::<Updates>();
    if !updates.can_install {
        return Err("this copy cannot install updates".to_owned());
    }
    let pending = {
        let inner = updates.inner.lock().unwrap_or_else(PoisonError::into_inner);
        if matches!(inner.status, UpdateStatus::Installing { .. }) {
            return Err("an update is already being installed".to_owned());
        }
        inner.pending.as_ref().map(|(update, _)| update.clone())
    };
    let Some(update) = pending else {
        return Err("no update to install".to_owned());
    };
    publish(app, |i| {
        i.status = UpdateStatus::Installing { percent: None }
    });

    let progress_app = app.clone();
    let mut received: u64 = 0;
    let mut shown: Option<u8> = None;
    let result = update
        .download_and_install(
            move |chunk, total| {
                received += chunk as u64;
                let percent = total
                    .filter(|t| *t > 0)
                    .map(|t| u8::try_from((received * 100 / t).min(100)).unwrap_or(100));
                if percent != shown {
                    shown = percent;
                    publish(&progress_app, |i| {
                        i.status = UpdateStatus::Installing { percent };
                    });
                }
            },
            || tracing::info!("update downloaded and verified; starting the installer"),
        )
        .await;
    // Reached only when something went wrong: on success the installer replaces the process.
    if let Err(error) = &result {
        tracing::warn!(%error, "update install failed");
        publish(app, |i| {
            i.status = UpdateStatus::Failed {
                stage: FailedStage::Install,
            };
        });
    }
    result.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;

    fn ms(v: i64) -> Duration {
        Duration::from_millis(u64::try_from(v).unwrap_or(0))
    }

    #[test]
    fn off_means_no_automatic_check() {
        assert_eq!(next_check_in(false, None, NOW), None);
        assert_eq!(next_check_in(false, Some(NOW - 2 * DAY_MS), NOW), None);
    }

    #[test]
    fn never_checked_is_due_now() {
        assert_eq!(next_check_in(true, None, NOW), Some(Duration::ZERO));
    }

    #[test]
    fn at_most_one_check_a_day() {
        assert_eq!(
            next_check_in(true, Some(NOW - 1000), NOW),
            Some(ms(DAY_MS - 1000))
        );
        assert_eq!(
            next_check_in(true, Some(NOW - DAY_MS), NOW),
            Some(Duration::ZERO)
        );
        assert_eq!(
            next_check_in(true, Some(NOW - 3 * DAY_MS), NOW),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn a_check_time_in_the_future_waits_at_most_a_day() {
        // The clock went back: the stored time is ahead of now.
        assert_eq!(next_check_in(true, Some(NOW + 1000), NOW), Some(ms(DAY_MS)));
        assert_eq!(
            next_check_in(true, Some(NOW + 300 * DAY_MS), NOW),
            Some(ms(DAY_MS))
        );
    }
}
