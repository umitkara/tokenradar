//! What every TokenRadar window shares: the system backdrop, the theme and the settings the
//! page needs before its first render.
//!
//! On Windows 11 the window is transparent and the system draws Mica (long-lived windows) or
//! Acrylic (the panel, a transient surface) behind the page; the page keeps `html` and `body`
//! transparent. Windows 10 gets an opaque window: its Acrylic stutters while dragging.

use tauri::utils::config::WindowEffectsConfig;
use tauri::window::{Color, Effect};
use tauri::{AppHandle, Manager, Theme, WebviewWindowBuilder, Wry};
use tokenradar_core::config::ThemePref;

use crate::prefs::{OsFlavor, Prefs, SettingsView};

/// How a window uses the backdrop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// Light-dismiss surfaces (the panel): Acrylic.
    Transient,
    /// Regular windows (settings, gallery, the main window): Mica.
    Window,
}

/// Opaque window colors for Windows 10: `--surface-0-solid` in `tokens.css` (checked by `tokens.test.ts`).
const SOLID_DARK: Color = Color(19, 22, 26, 255);
const SOLID_LIGHT: Color = Color(243, 245, 248, 255);

/// Applies the backdrop, the theme and the boot script to a window under construction.
pub fn dress<'a>(
    app: &AppHandle,
    builder: WebviewWindowBuilder<'a, Wry, AppHandle>,
    surface: Surface,
) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    let prefs = app.state::<Prefs>();
    let view = prefs.view();
    let platform = prefs.platform();
    let builder = builder
        .initialization_script(boot_script(&view))
        .theme(window_theme(view.theme_pref));
    match platform.os {
        OsFlavor::Win11 => {
            let effect = match surface {
                Surface::Transient if platform.has_system_acrylic() => Effect::Acrylic,
                _ => Effect::Mica,
            };
            builder.transparent(true).effects(WindowEffectsConfig {
                effects: vec![effect],
                ..WindowEffectsConfig::default()
            })
        }
        OsFlavor::Win10 => builder.background_color(match view.theme_pref {
            ThemePref::Light => SOLID_LIGHT,
            _ => SOLID_DARK,
        }),
    }
}

/// Sets every window's theme: the title bar, the backdrop tint and `prefers-color-scheme`.
pub fn apply_theme(app: &AppHandle, pref: ThemePref) {
    for window in app.webview_windows().values() {
        let _ = window.set_theme(window_theme(pref));
    }
}

/// `None` follows the system.
const fn window_theme(pref: ThemePref) -> Option<Theme> {
    match pref {
        ThemePref::System => None,
        ThemePref::Light => Some(Theme::Light),
        ThemePref::Dark => Some(Theme::Dark),
    }
}

/// Runs before the page's own scripts, so the first render already has the right language and
/// theme without waiting for a command round trip. The page reads it in `lib/settings`.
fn boot_script(view: &SettingsView) -> String {
    // Enums and booleans only: the JSON is a valid JavaScript literal.
    let json = serde_json::to_string(view).unwrap_or_else(|_| "null".to_owned());
    format!("window.__TOKENRADAR_BOOT__ = {json};")
}

#[cfg(test)]
mod tests {
    use tokenradar_core::config::LocalePref;

    use super::*;
    use crate::prefs::UiLocale;

    #[test]
    fn boot_script_is_a_plain_assignment() {
        let view = SettingsView {
            locale_pref: LocalePref::System,
            theme_pref: ThemePref::Dark,
            locale: UiLocale::Tr,
            os: OsFlavor::Win11,
            hour12: false,
        };
        assert_eq!(
            boot_script(&view),
            "window.__TOKENRADAR_BOOT__ = {\"locale_pref\":\"system\",\"theme_pref\":\"dark\",\
             \"locale\":\"tr\",\"os\":\"win11\",\"hour12\":false};"
        );
    }
}
