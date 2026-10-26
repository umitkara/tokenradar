//! Tray icon and its context menu.

use tauri::{
    AppHandle, include_image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::panel;

const TRAY_ID: &str = "main";
const MENU_OPEN_PANEL: &str = "open-panel";
const MENU_QUIT: &str = "quit";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    // TODO(step-08): load these labels from `locales/*.ftl` through `tokenradar-i18n`.
    let open_panel = MenuItem::with_id(app, MENU_OPEN_PANEL, "Open panel", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_panel, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        // Decoded at compile time, so no PNG decoder ships in the binary.
        .icon(include_image!("icons/tray.png"))
        .tooltip("TokenRadar")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN_PANEL => panel::report(panel::show(app)),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Windows reports both the press and the release; act on the release only.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                panel::report(panel::toggle(tray.app_handle()));
            }
        })
        .build(app)?;
    Ok(())
}
