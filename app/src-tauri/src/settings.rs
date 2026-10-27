//! The settings window. Step 08 only opens it from the tray menu; its content and "About"
//! arrive in Step 11. Its warm TTL is 0 s (architecture §2): closing it destroys the webview.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "settings";

/// Shows and focuses the settings window, creating it if needed.
pub fn show(app: &AppHandle) -> tauri::Result<()> {
    let window = match app.get_webview_window(LABEL) {
        Some(window) => window,
        None => WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("settings.html".into()))
            .title("TokenRadar")
            .inner_size(640.0, 480.0)
            .build()?,
    };
    window.show()?;
    window.set_focus()
}
