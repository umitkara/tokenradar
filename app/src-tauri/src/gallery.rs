//! The component gallery (Step 09): every component in every state, for visual checks on the
//! real backdrop. Development builds only; `gallery.html` is not part of a release bundle.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::chrome::{self, Surface};

pub const LABEL: &str = "gallery";

/// `TOKENRADAR_GALLERY=1` opens the gallery at start, for screenshots without the tray menu.
pub fn open_on_start(app: &AppHandle) {
    if std::env::var_os("TOKENRADAR_GALLERY").is_some_and(|v| v == "1") {
        crate::panel::report(show(app));
    }
}

pub fn show(app: &AppHandle) -> tauri::Result<()> {
    let window = if let Some(window) = app.get_webview_window(LABEL) {
        window
    } else {
        let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("gallery.html".into()));
        chrome::dress(app, builder, Surface::Window)
            .title("TokenRadar · Gallery")
            .inner_size(1180.0, 860.0)
            .build()?
    };
    window.show()?;
    window.set_focus()
}
