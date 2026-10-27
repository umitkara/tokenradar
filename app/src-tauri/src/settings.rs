//! The settings window. Step 08 only opens it from the tray menu; its content and "About"
//! arrive in Step 11. Its warm TTL is 0 s (architecture §2): closing it destroys the webview.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::chrome::{self, Surface};

pub const LABEL: &str = "settings";

/// Shows and focuses the settings window, creating it if needed.
pub fn show(app: &AppHandle) -> tauri::Result<()> {
    let window = if let Some(window) = app.get_webview_window(LABEL) {
        window
    } else {
        let builder =
            WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("settings.html".into()));
        chrome::dress(app, builder, Surface::Window)
            .title("TokenRadar")
            .inner_size(640.0, 480.0)
            .build()?
    };
    window.show()?;
    window.set_focus()
}
