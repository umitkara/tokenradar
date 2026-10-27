//! Tray icon and its context menu.

use tauri::{
    AppHandle, include_image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use tokenradar_core::domain::Timestamp;

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

/// Temporary Step 05 tooltip: live session count and the time of the last indexing pass.
// TODO(step-08): replace with the real tooltip, localized through `tokenradar-i18n`.
pub fn set_debug_status(app: &AppHandle, live: usize, last_index: Option<Timestamp>) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let indexed = last_index.map_or_else(|| "-".to_owned(), |t| local_hms(t.as_millis()));
    let text = format!("TokenRadar · {live} live · indexed {indexed}");
    if let Err(error) = tray.set_tooltip(Some(text)) {
        eprintln!("TokenRadar: could not update the tray tooltip: {error}");
    }
}

/// `HH:MM:SS` in the system's time zone.
#[cfg(windows)]
#[allow(unsafe_code)]
fn local_hms(unix_ms: i64) -> String {
    use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows_sys::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};

    const UNIX_EPOCH_FILETIME: i64 = 116_444_736_000_000_000;
    let ticks = unix_ms
        .saturating_mul(10_000)
        .saturating_add(UNIX_EPOCH_FILETIME);
    let ticks = u64::try_from(ticks).unwrap_or(0);
    let ft = FILETIME {
        // Splitting a u64 into its 32-bit halves.
        dwLowDateTime: (ticks & 0xFFFF_FFFF) as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    // SAFETY: both calls only read and write the local structs passed by pointer; a null time
    // zone means the current one.
    unsafe {
        let mut utc: SYSTEMTIME = std::mem::zeroed();
        let mut local: SYSTEMTIME = std::mem::zeroed();
        if FileTimeToSystemTime(&raw const ft, &raw mut utc) == 0
            || SystemTimeToTzSpecificLocalTime(std::ptr::null(), &raw const utc, &raw mut local)
                == 0
        {
            return "?".to_owned();
        }
        format!(
            "{:02}:{:02}:{:02}",
            local.wHour, local.wMinute, local.wSecond
        )
    }
}

#[cfg(not(windows))]
fn local_hms(unix_ms: i64) -> String {
    let secs = unix_ms.div_euclid(1000).rem_euclid(86_400);
    format!(
        "{:02}:{:02}:{:02} UTC",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}
