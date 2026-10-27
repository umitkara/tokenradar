//! Fake limits for looking at and measuring the tray (debug builds or the `spike` feature).
//!
//! `TOKENRADAR_FAKE_LIMITS` replaces the core: the app does not open the database, index or
//! call the API, so these runs never touch real data.
//!
//! - `cycle`: every 2 s the next state (no data, ok, warn, crit, stale, error, muted, partial);
//!   each round switches between English and Turkish.
//! - `sweep`: one `LimitsUpdated`-like snapshot per second with a moving percentage, for the
//!   10-minute CPU measurement.
//! - `stress`: 10,000 `set_icon` calls in a row; logs GDI and USER handle counts.
//!
//! `TOKENRADAR_FAKE_LOG=<file>` also writes the log lines to a file (release builds have no
//! console).

use std::io::Write as _;
use std::time::{Duration, Instant};

use tauri::AppHandle;
use tauri::image::Image;
use tokenradar_core::domain::{LimitKind, Source, Timestamp};
use tokenradar_core::limits::{
    ApiStatusView, Freshness, IndexState, LimitView, PanelSnapshot, Severity, StatuslineState,
};
use tokenradar_core::providers::claude::usage_api::{ApiState, PollDiagnostics};
use tokenradar_i18n::Locale;

use super::visual::{Ring, TrayVisual};
use super::{TRAY_ID, TrayMsg, render, send, system};

const STRESS_UPDATES: u32 = 10_000;

/// The requested scenario, if any.
pub fn mode() -> Option<String> {
    std::env::var("TOKENRADAR_FAKE_LIMITS").ok()
}

pub fn start(app: &AppHandle, mode: &str) {
    let app = app.clone();
    let mode = mode.to_owned();
    let spawned = std::thread::Builder::new()
        .name("tray-fake".into())
        .spawn(move || match mode.as_str() {
            "cycle" => cycle(&app),
            "sweep" => sweep(&app),
            "stress" => stress(&app),
            other => log(&format!("unknown TOKENRADAR_FAKE_LIMITS '{other}'")),
        });
    if let Err(error) = spawned {
        eprintln!("TokenRadar: fake limits not started: {error}");
    }
}

fn log(line: &str) {
    let line = format!("[{}] tray-fake: {line}", Timestamp::now());
    eprintln!("{line}");
    if let Some(path) = std::env::var_os("TOKENRADAR_FAKE_LOG")
        && let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        let _ = writeln!(f, "{line}");
    }
}

fn limit(kind: LimitKind, percent: f64, freshness: Freshness, resets_in_min: i64) -> LimitView {
    let now = Timestamp::now().as_millis();
    let age = if freshness == Freshness::Stale {
        45 * 60_000
    } else {
        0
    };
    LimitView {
        kind,
        scope: None,
        percent,
        resets_at: Some(Timestamp::from_millis(now + resets_in_min * 60_000)),
        severity: Severity::of(percent),
        provider_severity: None,
        source: Source::Statusline,
        estimate_reason: None,
        observed_at: Timestamp::from_millis(now - age),
        freshness,
        projection: None,
    }
}

fn snapshot(limits: Vec<LimitView>, api: ApiState) -> Box<PanelSnapshot> {
    Box::new(PanelSnapshot {
        account: None,
        limits,
        live_sessions: Vec::new(),
        top_sessions: Vec::new(),
        index_state: IndexState::default(),
        api: ApiStatusView {
            state: api,
            diagnostics: PollDiagnostics::default(),
        },
        statusline: StatuslineState::default(),
        generated_at: Timestamp::now(),
    })
}

fn both(five_hour: f64, weekly: f64, freshness: Freshness, api: ApiState) -> Box<PanelSnapshot> {
    snapshot(
        vec![
            limit(LimitKind::Session, five_hour, freshness, 108),
            limit(LimitKind::WeeklyAll, weekly, freshness, 3 * 24 * 60 + 17),
        ],
        api,
    )
}

fn cycle(app: &AppHandle) {
    let fresh = Freshness::Fresh;
    let active = || ApiState::Active;
    let mut locale = Locale::En;
    loop {
        log(&format!("locale {}", locale.tag()));
        send(app, TrayMsg::Locale(locale));
        locale = match locale {
            Locale::En => Locale::Tr,
            Locale::Tr => Locale::En,
        };
        let scenes: Vec<(&str, Box<PanelSnapshot>, bool)> = vec![
            ("no-data", snapshot(Vec::new(), ApiState::Off), false),
            ("ok", both(42.0, 34.0, fresh, active()), false),
            ("warn", both(80.0, 60.0, fresh, active()), false),
            ("crit", both(93.0, 91.0, fresh, active()), false),
            ("stale", both(42.0, 34.0, Freshness::Stale, active()), false),
            (
                "error",
                both(42.0, 34.0, Freshness::Stale, ApiState::TokenStale),
                false,
            ),
            ("muted", both(42.0, 34.0, fresh, active()), true),
            (
                "five-hour-only",
                snapshot(vec![limit(LimitKind::Session, 55.0, fresh, 30)], active()),
                false,
            ),
        ];
        for (name, snap, muted) in scenes {
            log(&format!("state {name}"));
            let until =
                muted.then(|| Timestamp::from_millis(Timestamp::now().as_millis() + 3_600_000));
            send(app, TrayMsg::MutedUntil(until));
            send(app, TrayMsg::Snapshot(snap));
            std::thread::sleep(Duration::from_secs(2));
        }
    }
}

fn sweep(app: &AppHandle) {
    let started = Instant::now();
    let mut i: u32 = 0;
    loop {
        let p = f64::from(i % 101);
        send(
            app,
            TrayMsg::Snapshot(both(p, p / 2.0, Freshness::Fresh, ApiState::Active)),
        );
        i += 1;
        if i.is_multiple_of(60) {
            log(&format!("sweep {i} updates in {:?}", started.elapsed()));
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn stress(app: &AppHandle) {
    // Let the tray settle first.
    std::thread::sleep(Duration::from_secs(2));
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        log("no tray icon");
        return;
    };
    let env = system::environment();
    let (gdi, user) = system::gui_objects();
    log(&format!("stress start px={} gdi={gdi} user={user}", env.px));
    let started = Instant::now();
    for i in 1..=STRESS_UPDATES {
        #[allow(clippy::cast_possible_truncation)]
        let percent = (i % 101) as u8;
        let visual = TrayVisual {
            five_hour: Some(Ring {
                percent,
                severity: Severity::of(f64::from(percent)),
            }),
            weekly: Some(Ring {
                percent: 100 - percent,
                severity: Severity::of(f64::from(100 - percent)),
            }),
            ..TrayVisual::default()
        };
        let pixmap = render::render(visual, env.px, env.theme);
        let image = Image::new_owned(render::to_rgba(&pixmap), pixmap.width(), pixmap.height());
        if let Err(error) = tray.set_icon(Some(image)) {
            log(&format!("set_icon failed at {i}: {error}"));
            return;
        }
        if i.is_multiple_of(1000) {
            let (gdi, user) = system::gui_objects();
            log(&format!(
                "stress {i} gdi={gdi} user={user} elapsed={:?}",
                started.elapsed()
            ));
        }
    }
    std::thread::sleep(Duration::from_secs(2));
    let (gdi, user) = system::gui_objects();
    log(&format!(
        "stress done gdi={gdi} user={user} total={:?}",
        started.elapsed()
    ));
}
