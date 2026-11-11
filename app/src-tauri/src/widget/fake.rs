//! Fixed widget scenes for looking at and screenshotting the strip (debug builds or the
//! `spike` feature): `TOKENRADAR_FAKE_LIMITS=widget:<scene>`.
//!
//! - `weekly`: one row, an account without a 5-hour window.
//! - `both`: both rows.
//! - `stale`: both rows, readings older than the stale threshold.
//! - `estimate`: both rows from the estimate (dashed, `≈`).
//!
//! A `:taskbar` suffix (`widget:both:taskbar`) docks it on the taskbar (Step 18b). The widget
//! is forced on and nothing is written to `config.toml`.

use std::time::Duration;

use tauri::AppHandle;
use tokenradar_core::config::WidgetMode;
use tokenradar_core::domain::{LimitKind, Source, Timestamp};
use tokenradar_core::limits::{Freshness, PanelSnapshot};
use tokenradar_core::providers::claude::usage_api::ApiState;

use crate::tray::fake::{both, limit, log, snapshot, without_five_hour};
use crate::tray::{TrayMsg, send};

fn scene(name: &str) -> Option<Box<PanelSnapshot>> {
    Some(match name {
        "weekly" => without_five_hour(34.0),
        "both" => both(72.0, 34.0, Freshness::Fresh, ApiState::Active),
        "stale" => both(72.0, 34.0, Freshness::Stale, ApiState::Active),
        "estimate" => {
            let mut s = snapshot(
                vec![
                    limit(LimitKind::Session, 48.0, Freshness::Fresh, 108),
                    limit(
                        LimitKind::WeeklyAll,
                        61.0,
                        Freshness::Fresh,
                        3 * 24 * 60 + 17,
                    ),
                ],
                ApiState::Off,
            );
            for l in &mut s.limits {
                l.source = Source::Estimate;
            }
            s
        }
        _ => return None,
    })
}

/// Window lengths, so the time mark (ADR-044) has a start to measure from.
fn with_window_starts(mut s: Box<PanelSnapshot>) -> Box<PanelSnapshot> {
    for l in &mut s.limits {
        let len_ms = match l.kind {
            LimitKind::Session => 5 * 3_600_000,
            _ => 7 * 24 * 3_600_000,
        };
        l.window_start = l
            .resets_at
            .map(|r| Timestamp::from_millis(r.as_millis() - len_ms));
    }
    s
}

pub fn start(app: &AppHandle, name: &str) {
    let (name, mode) = match name.strip_suffix(":taskbar") {
        Some(name) => (name, WidgetMode::Taskbar),
        None => (name, WidgetMode::Floating),
    };
    let Some(snap) = scene(name).map(with_window_starts) else {
        log(&format!("unknown widget scene '{name}'"));
        return;
    };
    // Let the tray and the settings settle first, so the forced widget starts from this scene.
    std::thread::sleep(Duration::from_secs(1));
    log(&format!("widget scene {name} ({mode:?})"));
    send(app, TrayMsg::Snapshot(snap));
    super::force(app, mode);
}
