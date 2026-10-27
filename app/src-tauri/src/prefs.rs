//! The user's appearance settings (language, theme) and the platform facts every window needs
//! before its first paint.
//!
//! `config.toml` is read on a helper thread at startup: the main thread never waits on disk.
//! Until it has been read, the defaults (`system`) apply; a window opened in that instant gets
//! the corrected settings through [`SettingsChanged`].

use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokenradar_core::config::{self, AppConfig, LocalePref, ThemePref};
use tokenradar_core::paths::Paths;
use tokenradar_i18n::{HourCycle, Locale};

use crate::tray::system::{self, LocalClock, SystemClock};

/// First Windows 11 build: Mica exists from here on.
const WIN11_BUILD: u32 = 22_000;
/// First build with the system Acrylic backdrop (`DWMSBT_TRANSIENTWINDOW`). Older builds fall
/// back to an Acrylic that stutters while dragging, so they get Mica instead.
const SYSTEM_ACRYLIC_BUILD: u32 = 22_523;

/// The resolved UI language: `system` already applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum UiLocale {
    En,
    Tr,
}

impl From<Locale> for UiLocale {
    fn from(l: Locale) -> Self {
        match l {
            Locale::En => Self::En,
            Locale::Tr => Self::Tr,
        }
    }
}

/// Which window backdrop the system supports (`data-os` on `<html>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum OsFlavor {
    /// Mica and Acrylic backdrops; the page background stays transparent.
    Win11,
    /// No usable backdrop: opaque surfaces (design-language.md §4).
    Win10,
}

/// Facts about the running system, read once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    pub os: OsFlavor,
    pub build: u32,
}

impl Platform {
    pub fn detect() -> Self {
        let build = system::os_build().unwrap_or(0);
        Self {
            os: if build >= WIN11_BUILD {
                OsFlavor::Win11
            } else {
                OsFlavor::Win10
            },
            build,
        }
    }

    /// Whether the system Acrylic backdrop is available (transient surfaces such as the panel).
    pub const fn has_system_acrylic(self) -> bool {
        self.build >= SYSTEM_ACRYLIC_BUILD
    }
}

/// Everything a page needs to look right: sent by `get_settings` and [`SettingsChanged`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SettingsView {
    pub locale_pref: LocalePref,
    pub theme_pref: ThemePref,
    /// `locale_pref` with `system` resolved.
    pub locale: UiLocale,
    pub os: OsFlavor,
    /// English times follow the OS region's 12/24-hour choice; Turkish is always 24 hours.
    pub hour12: bool,
}

/// Sent to every open window when [`SettingsView`] changed.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SettingsChanged(pub SettingsView);

#[derive(Debug, Clone, Copy, Default)]
struct Stored {
    locale: LocalePref,
    theme: ThemePref,
}

/// Managed by Tauri.
pub struct Prefs {
    platform: Platform,
    stored: Mutex<Stored>,
}

impl Prefs {
    pub fn view(&self) -> SettingsView {
        let stored = *self.stored.lock().unwrap_or_else(PoisonError::into_inner);
        let locale = match stored.locale {
            LocalePref::System => Locale::detect(),
            LocalePref::En => Locale::En,
            LocalePref::Tr => Locale::Tr,
        };
        SettingsView {
            locale_pref: stored.locale,
            theme_pref: stored.theme,
            locale: locale.into(),
            os: self.platform.os,
            hour12: locale == Locale::En && SystemClock.hour_cycle() == HourCycle::H12,
        }
    }

    pub const fn platform(&self) -> Platform {
        self.platform
    }

    fn update(&self, f: impl FnOnce(&mut Stored)) {
        f(&mut self.stored.lock().unwrap_or_else(PoisonError::into_inner));
    }
}

/// Registers the settings with their defaults and reads `config.toml` in the background.
pub fn init(app: &AppHandle) {
    app.manage(Prefs {
        platform: Platform::detect(),
        stored: Mutex::default(),
    });
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("prefs-load".into())
        .spawn(move || {
            let general = Paths::resolve()
                .ok()
                .and_then(|p| AppConfig::load(p.config_path()).ok())
                .map(|c| c.general)
                .unwrap_or_default();
            let prefs = app.state::<Prefs>();
            let before = prefs.view();
            prefs.update(|s| {
                s.locale = general.locale_pref();
                s.theme = general.theme_pref();
            });
            let after = prefs.view();
            if after != before {
                crate::chrome::apply_theme(&app, after.theme_pref);
                let _ = SettingsChanged(after).emit(&app);
            }
        });
    if let Err(error) = spawned {
        eprintln!("TokenRadar: could not read the settings: {error}");
    }
}

/// Stores the language, tells the tray and every open window.
pub fn set_locale(app: &AppHandle, pref: LocalePref) -> Result<SettingsView, String> {
    save("locale", pref.as_str())?;
    let prefs = app.state::<Prefs>();
    prefs.update(|s| s.locale = pref);
    let view = prefs.view();
    let locale = match view.locale {
        UiLocale::En => Locale::En,
        UiLocale::Tr => Locale::Tr,
    };
    crate::tray::send(app, crate::tray::TrayMsg::Locale(locale));
    let _ = SettingsChanged(view.clone()).emit(app);
    Ok(view)
}

/// Stores the theme, applies it to every window's backdrop and tells every open window.
pub fn set_theme(app: &AppHandle, pref: ThemePref) -> Result<SettingsView, String> {
    save("theme", pref.as_str())?;
    let prefs = app.state::<Prefs>();
    prefs.update(|s| s.theme = pref);
    crate::chrome::apply_theme(app, pref);
    let view = prefs.view();
    let _ = SettingsChanged(view.clone()).emit(app);
    Ok(view)
}

fn save(key: &str, value: &str) -> Result<(), String> {
    let paths = Paths::resolve().map_err(|e| e.to_string())?;
    config::save_general(paths.config_path(), key, value).map_err(|e| e.to_string())
}
