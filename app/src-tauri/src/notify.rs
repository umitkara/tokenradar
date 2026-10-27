//! Desktop notifications: shows what the core's notifier decided (ADR-033). Rules, mute and
//! quiet hours live in the core; this module only puts the text on screen.
//!
//! A click on a toast opens the panel while the toast is on screen. Windows does not report a
//! click from the Action Center to an app that registered no COM activator; that comes with
//! the installer (Step 12, ADR-033).
//!
//! Windows names the sender by the `AppUserModelID`. Builds run from `target\debug` or
//! `target\release` show as PowerShell; any other copy uses the app identifier, which only the
//! installer registers. Until then a portable copy may show no toast at all, without an error.

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt as _;
use tokenradar_core::notifier::Notice;

use crate::panel;

/// Registers the click handler. Toasts shown before this report nothing.
pub fn init(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.notification().on_action(move |performed| {
        if performed.action_id() == "tap" {
            panel::report(panel::show(&handle));
        }
    });
}

/// Shows `notice` as a toast.
pub fn show(app: &AppHandle, notice: &Notice) {
    let result = app
        .notification()
        .builder()
        .title(&notice.title)
        .body(&notice.body)
        .show();
    if let Err(error) = result {
        tracing::warn!(%error, "could not show a notification");
    }
}
