//! Panel measurement scenario (Step 10), in debug builds or with the `spike` feature.
//!
//! `TOKENRADAR_PANEL_SCENARIO=lifecycle` drives the real panel through three rounds of cold
//! open, hide, warm open and the warm TTL, then quits. `TOKENRADAR_PANEL_LOG` receives the
//! markers and the open latencies (`painted cold|warm <ms>`); `scripts/measure-panel.ps1` joins
//! them with the memory samples. Combine with `TOKENRADAR_FAKE_LIMITS=panel:<scene>` so the run
//! never touches real data.
//!
//! `TOKENRADAR_PANEL_SCENARIO=prewarm` hovers over the tray icon, then opens the panel 150, 300
//! and 600 ms later (`painted prewarmed|warm <ms>`), once per round, with the warm TTL between.
//!
//! `TOKENRADAR_PANEL_SCENARIO=open` opens the panel and reopens it within a second whenever it
//! hides, for screenshots and CDP checks without clicking the tray.

use std::thread::sleep;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use super::{WARM_TTL, hide, log, report, show, tray_hovered};

const ROUNDS: u32 = 3;

pub fn start(app: &AppHandle) {
    let run: fn(&AppHandle) = match std::env::var("TOKENRADAR_PANEL_SCENARIO").as_deref() {
        Ok("lifecycle") => lifecycle,
        Ok("prewarm") => prewarm,
        Ok("open") => keep_open,
        _ => return,
    };
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("panel-scenario".into())
        .spawn(move || run(&app));
    if let Err(error) = spawned {
        eprintln!("TokenRadar: panel scenario not started: {error}");
    }
}

fn keep_open(app: &AppHandle) {
    sleep(Duration::from_secs(2));
    loop {
        let visible = app
            .get_webview_window(super::LABEL)
            .and_then(|window| window.is_visible().ok())
            .unwrap_or(false);
        if !visible {
            report(show(app));
        }
        sleep(Duration::from_secs(1));
    }
}

fn prewarm(app: &AppHandle) {
    let secs = Duration::from_secs;
    sleep(secs(20));
    log("mark baseline");
    for (round, delay) in [150, 300, 600].into_iter().enumerate() {
        tray_hovered(app);
        sleep(Duration::from_millis(delay));
        report(show(app));
        sleep(secs(10));
        log(&format!("mark open{}", round + 1));
        hide(app);
        sleep(WARM_TTL + secs(10));
        log(&format!("mark ttl{}", round + 1));
    }
    log("mark end");
    app.exit(0);
}

fn lifecycle(app: &AppHandle) {
    let secs = Duration::from_secs;
    sleep(secs(20));
    log("mark baseline");
    for round in 1..=ROUNDS {
        report(show(app));
        sleep(secs(10));
        log(&format!("mark open{round}"));
        hide(app);
        sleep(secs(5));
        log(&format!("mark hidden{round}"));
        report(show(app));
        sleep(secs(3));
        hide(app);
        log(&format!("mark hide{round}"));
        sleep(WARM_TTL + secs(10));
        log(&format!("mark ttl{round}"));
    }
    log("mark end");
    app.exit(0);
}
