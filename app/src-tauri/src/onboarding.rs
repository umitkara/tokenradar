//! The first-run window (Step 11c): language, the Claude Code folder, limit data and the tray
//! icon. It opens by itself while `kv.onboarding_done` is missing, so a fresh database (first
//! start, "Delete all data") shows it again; the tray menu reopens it any time.
//!
//! Closing the window counts as done (ADR-035): only what the user clicked was applied, and
//! the app must not ask again on every start. Like the settings, its warm TTL is 0 s.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokenradar_core::domain::Timestamp;
use tokenradar_core::store::Store;

use crate::chrome::{self, Surface};
use crate::core_bridge;

pub const LABEL: &str = "onboarding";

/// Set (to the time in ms) once the window was closed or finished.
const KV_DONE: &str = "onboarding_done";

/// Skips opening the window by itself: measurement scripts start the app on empty data.
const SKIP_ENV: &str = "TOKENRADAR_SKIP_ONBOARDING";

/// Shows and focuses the window, creating it if needed.
pub fn show(app: &AppHandle) -> tauri::Result<()> {
    let window = if let Some(window) = app.get_webview_window(LABEL) {
        window
    } else {
        let builder =
            WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("onboarding.html".into()));
        let window = chrome::dress(app, builder, Surface::Window)
            .title("TokenRadar")
            .inner_size(720.0, 580.0)
            .min_inner_size(600.0, 520.0)
            .build()?;
        let handle = app.clone();
        window.on_window_event(move |event| {
            if matches!(event, WindowEvent::Destroyed) {
                mark_done(&handle);
            }
        });
        window
    };
    window.show()?;
    window.set_focus()
}

/// Opens the window when the core started on a database that has not seen it yet.
pub fn on_core_started(app: &AppHandle, store: &Store) {
    if std::env::var_os(SKIP_ENV).is_some() {
        return;
    }
    match store.kv_get(KV_DONE) {
        Ok(Some(_)) => {}
        Ok(None) => {
            tracing::info!("first run: opening onboarding");
            if let Err(error) = show(app) {
                tracing::warn!(%error, "could not open onboarding");
            }
        }
        Err(error) => tracing::warn!(%error, "could not read the onboarding state"),
    }
}

/// "Finish": closes the window, which records it as done.
pub fn finish(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.close();
    }
}

fn mark_done(app: &AppHandle) {
    let mut stored = false;
    core_bridge::with_core(app, |core| {
        match core
            .store()
            .writer()
            .kv_set(KV_DONE.to_owned(), Timestamp::now().as_millis().to_string())
        {
            Ok(()) => stored = true,
            Err(error) => tracing::warn!(%error, "could not record onboarding as done"),
        }
    });
    if !stored {
        tracing::warn!(
            "onboarding closed while the core was not running; it opens again next start"
        );
    }
}
