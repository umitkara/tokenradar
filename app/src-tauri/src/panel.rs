//! The tray panel: a small frameless window created on first use.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::chrome::{self, Surface};

pub const LABEL: &str = "panel";

const WIDTH: f64 = 360.0;
const HEIGHT: f64 = 480.0;

/// Hides the panel if it is visible, otherwise shows it (creating it if needed).
pub fn toggle(app: &AppHandle) -> tauri::Result<()> {
    match app.get_webview_window(LABEL) {
        Some(window) if window.is_visible()? => window.hide(),
        _ => show(app),
    }
}

/// Shows and focuses the panel, creating it if needed.
pub fn show(app: &AppHandle) -> tauri::Result<()> {
    let window = match app.get_webview_window(LABEL) {
        Some(window) => window,
        None => build(app)?,
    };
    window.show()?;
    window.set_focus()
}

fn build(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    builder(app).build()
}

/// The panel's window configuration, shared with the Step 02 measurement scenario.
pub fn builder(app: &AppHandle) -> WebviewWindowBuilder<'_, tauri::Wry, AppHandle> {
    // Positioning next to the tray and hide-on-blur arrive in Step 10.
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("panel.html".into()));
    chrome::dress(app, builder, Surface::Transient)
        .title("TokenRadar")
        .inner_size(WIDTH, HEIGHT)
        .resizable(false)
        .decorations(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
}

/// Reports a failed window operation. Event handlers have nowhere to propagate errors to.
// TODO(step-05): route through `tracing` once logging exists.
pub fn report(result: tauri::Result<()>) {
    if let Err(err) = result {
        eprintln!("panel: {err}");
    }
}
