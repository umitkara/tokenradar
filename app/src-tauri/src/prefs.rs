//! The user's settings (appearance, notifications, data, log level) and the platform facts
//! every window needs before its first paint.
//!
//! `config.toml` is read on a helper thread at startup: the main thread never waits on disk.
//! Until it has been read, the defaults (`system`) apply; a window opened in that instant gets
//! the corrected settings through [`SettingsChanged`].

use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokenradar_core::config::{
    self, AdvancedConfig, AppConfig, DataConfig, LocalePref, LogLevel, PanelView, ThemePref,
};
use tokenradar_core::domain::{LimitKind, Timestamp, WindowPresence};
use tokenradar_core::notifier::NotifyPrefs;
use tokenradar_core::paths::Paths;
use tokenradar_core::statusline::line::{
    self, Format, Group, Item, Line, Preset, SampleState, SegmentKind, Span, StatuslinePrefs, Style,
};
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

impl From<UiLocale> for Locale {
    fn from(l: UiLocale) -> Self {
        match l {
            UiLocale::En => Self::En,
            UiLocale::Tr => Self::Tr,
        }
    }
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

/// The rest of the settings, for the settings window only: sent by `get_prefs` and
/// [`PrefsChanged`]. Kept apart from [`SettingsView`], which every page gets before its first
/// paint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PrefsView {
    pub notifications: NotifyPrefs,
    pub data: DataConfig,
    pub log_level: LogLevel,
    /// How long a hidden panel stays ready, in seconds (`0`: destroyed at once).
    pub panel_warm_ttl_s: u32,
    /// What `tokenradar statusline` shows (`[statusline]`).
    pub statusline: StatuslinePrefs,
}

/// Sent to every open window when [`PrefsView`] changed.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct PrefsChanged(pub PrefsView);

#[derive(Debug, Clone)]
struct Stored {
    locale: LocalePref,
    theme: ThemePref,
    panel_view: PanelView,
    prefs: PrefsView,
}

impl Default for Stored {
    fn default() -> Self {
        Self::from_config(&AppConfig::default())
    }
}

impl Stored {
    fn from_config(c: &AppConfig) -> Self {
        Self {
            locale: c.general.locale_pref(),
            theme: c.general.theme_pref(),
            panel_view: c.general.panel_view(),
            prefs: PrefsView {
                notifications: c.notifications.clone().normalized(),
                data: c.data.clone().normalized(),
                log_level: c.advanced.log_level,
                panel_warm_ttl_s: c.general.panel_warm_ttl_s(),
                statusline: c.statusline.clone().normalized(),
            },
        }
    }
}

/// Managed by Tauri.
pub struct Prefs {
    platform: Platform,
    stored: Mutex<Stored>,
}

impl Prefs {
    pub fn view(&self) -> SettingsView {
        let (locale_pref, theme_pref) = {
            let s = self.stored.lock().unwrap_or_else(PoisonError::into_inner);
            (s.locale, s.theme)
        };
        let locale = resolve(locale_pref);
        SettingsView {
            locale_pref,
            theme_pref,
            locale: locale.into(),
            os: self.platform.os,
            hour12: locale == Locale::En && SystemClock.hour_cycle() == HourCycle::H12,
        }
    }

    pub fn prefs_view(&self) -> PrefsView {
        self.stored
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .prefs
            .clone()
    }

    /// The resolved UI language, also the notifications' language.
    pub fn locale(&self) -> Locale {
        resolve(
            self.stored
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .locale,
        )
    }

    /// The panel view last chosen.
    pub fn panel_view(&self) -> PanelView {
        self.stored
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .panel_view
    }

    /// How long a hidden panel stays ready.
    pub fn panel_warm_ttl(&self) -> std::time::Duration {
        let s = self.stored.lock().unwrap_or_else(PoisonError::into_inner);
        std::time::Duration::from_secs(u64::from(s.prefs.panel_warm_ttl_s))
    }

    pub const fn platform(&self) -> Platform {
        self.platform
    }

    fn update(&self, f: impl FnOnce(&mut Stored)) {
        f(&mut self.stored.lock().unwrap_or_else(PoisonError::into_inner));
    }
}

fn resolve(pref: LocalePref) -> Locale {
    match pref {
        LocalePref::System => Locale::detect(),
        LocalePref::En => Locale::En,
        LocalePref::Tr => Locale::Tr,
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
        .spawn(move || reload(&app));
    if let Err(error) = spawned {
        tracing::warn!(%error, "could not read the settings");
    }
}

/// Reads `config.toml` again and applies what changed: log level, theme, the tray's language
/// and every open window. Used at start and after "Delete all data".
pub fn reload(app: &AppHandle) {
    let loaded = Paths::resolve()
        .ok()
        .and_then(|p| AppConfig::load(p.config_path()).ok())
        .unwrap_or_default();
    crate::logging::set_level(loaded.advanced.log_level);
    let prefs = app.state::<Prefs>();
    let (before, before_prefs) = (prefs.view(), prefs.prefs_view());
    prefs.update(|s| *s = Stored::from_config(&loaded));
    let after = prefs.view();
    if after != before {
        crate::chrome::apply_theme(app, after.theme_pref);
        crate::tray::send(app, crate::tray::TrayMsg::Locale(after.locale.into()));
        let _ = SettingsChanged(after).emit(app);
    }
    let after_prefs = prefs.prefs_view();
    if after_prefs != before_prefs {
        let _ = PrefsChanged(after_prefs).emit(app);
    }
}

/// Stores the language, tells the tray and every open window.
pub fn set_locale(app: &AppHandle, pref: LocalePref) -> Result<SettingsView, String> {
    save("locale", pref.as_str())?;
    let prefs = app.state::<Prefs>();
    prefs.update(|s| s.locale = pref);
    let view = prefs.view();
    let locale: Locale = view.locale.into();
    crate::tray::send(app, crate::tray::TrayMsg::Locale(locale));
    crate::core_bridge::with_core(app, |core| core.set_notify_locale(locale));
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

/// Stores the panel view; the panel opens on it next time.
pub fn set_panel_view(app: &AppHandle, view: PanelView) -> Result<(), String> {
    let prefs = app.state::<Prefs>();
    if prefs.panel_view() == view {
        return Ok(());
    }
    save("panel_view", view.as_str())?;
    prefs.update(|s| s.panel_view = view);
    Ok(())
}

/// Stores the notification rules and hands them to the notifier.
pub fn set_notifications(app: &AppHandle, prefs: NotifyPrefs) -> Result<PrefsView, String> {
    let prefs = prefs.normalized();
    with_config(|path| config::save_notifications(path, &prefs))?;
    crate::core_bridge::with_core(app, |core| core.set_notify_prefs(prefs.clone()));
    Ok(changed(app, |p| p.notifications = prefs))
}

/// Stores the data settings; the new retention applies from the next daily run.
pub fn set_data(app: &AppHandle, data: DataConfig) -> Result<PrefsView, String> {
    let data = data.normalized();
    with_config(|path| config::save_data(path, &data))?;
    let days = data.raw_retention_days;
    crate::core_bridge::with_core(app, |core| core.set_retention_days(days));
    let view = changed(app, |p| p.data = data);
    crate::updater::setting_changed(app);
    Ok(view)
}

pub fn set_log_level(app: &AppHandle, level: LogLevel) -> Result<PrefsView, String> {
    with_config(|path| config::save_advanced(path, &AdvancedConfig { log_level: level }))?;
    crate::logging::set_level(level);
    Ok(changed(app, |p| p.log_level = level))
}

/// Stores the panel's warm TTL; it applies the next time the panel hides.
pub fn set_panel_warm_ttl(app: &AppHandle, seconds: u32) -> Result<PrefsView, String> {
    let seconds = seconds.min(config::MAX_PANEL_WARM_TTL_S);
    with_config(|path| config::save_panel_warm_ttl(path, seconds))?;
    Ok(changed(app, |p| p.panel_warm_ttl_s = seconds))
}

/// Stores `[statusline]`; the CLI reads it on its next call.
pub fn set_statusline(app: &AppHandle, prefs: StatuslinePrefs) -> Result<PrefsView, String> {
    let prefs = prefs.normalized();
    with_config(|path| config::save_statusline_prefs(path, &prefs))?;
    Ok(changed(app, |p| p.statusline = prefs))
}

/// The statusline preview in the settings window.
#[derive(Debug, Clone, Serialize, Type)]
pub struct StatuslinePreview {
    /// Painted spans per output line.
    pub lines: Vec<Vec<Span>>,
    /// Segments the account is known not to have (ADR-041): left out of the line.
    pub absent: Vec<SegmentKind>,
}

/// Whether the account has the 5-hour window, as far as the app knows (ADR-041).
fn has_five_hour(app: &AppHandle) -> bool {
    crate::core_bridge::panel_snapshot(app)
        .is_none_or(|s| s.presence(&LimitKind::Session, None) != WindowPresence::Absent)
}

fn cli_l10n(app: &AppHandle) -> tokenradar_i18n::Localizer {
    tokenradar_i18n::Localizer::new(
        app.state::<Prefs>().locale(),
        &[tokenradar_i18n::Domain::Cli],
    )
}

/// The statusline as `prefs` would draw it, on sample values in `state`, in the UI language,
/// `width` columns wide (`None`: unlimited). The 5-hour window is left out when the account
/// is known not to have it (ADR-041). Colors follow `prefs.color`, assuming a terminal
/// without `NO_COLOR` (what Claude Code gives the command, Step 12i).
pub fn statusline_preview(
    app: &AppHandle,
    prefs: &StatuslinePrefs,
    state: SampleState,
    width: Option<u16>,
) -> StatuslinePreview {
    let five_hour = has_five_hour(app);
    let prefs = prefs.clone().normalized();
    let sample = Line::sample_in(Timestamp::now(), five_hour, state);
    let style = Style {
        color: prefs.color.resolve(false),
        width: width.filter(|w| *w > 0).map(usize::from),
    };
    StatuslinePreview {
        lines: line::spans(&cli_l10n(app), &sample, &prefs, style),
        absent: if five_hour {
            Vec::new()
        } else {
            vec![SegmentKind::FiveHour]
        },
    }
}

/// One kind of item, for the add-item palette and the item editor.
#[derive(Debug, Clone, Serialize, Type)]
pub struct StatuslineKind {
    pub kind: SegmentKind,
    pub group: Group,
    /// The formats it accepts; empty when it has one look.
    pub formats: Vec<Format>,
    pub default_format: Option<Format>,
    /// Whether a label can be set, and the one shown when it is empty.
    pub labeled: bool,
    pub default_label: String,
    pub colored: bool,
    /// The item alone on the sample, without color; empty for layout items.
    pub sample: String,
    /// The account is known not to have it (ADR-041).
    pub absent: bool,
}

/// Every kind of item, in palette order.
pub fn statusline_catalog(app: &AppHandle) -> Vec<StatuslineKind> {
    let five_hour = has_five_hour(app);
    let l10n = cli_l10n(app);
    let sample = Line::sample(Timestamp::now(), true);
    let plain = Style {
        color: false,
        width: None,
    };
    SegmentKind::ALL
        .iter()
        .map(|&kind| StatuslineKind {
            kind,
            group: kind.group(),
            formats: kind.formats().to_vec(),
            default_format: kind.default_format(),
            labeled: kind.labeled(),
            default_label: line::default_label(&l10n, kind),
            colored: kind.colored(),
            sample: if kind.is_layout() {
                String::new()
            } else {
                let alone = StatuslinePrefs {
                    items: vec![Item::new(kind)],
                    ..StatuslinePrefs::default()
                };
                line::render(&l10n, &sample, &alone, plain)
            },
            absent: kind == SegmentKind::FiveHour && !five_hour,
        })
        .collect()
}

/// A built-in starting point.
#[derive(Debug, Clone, Serialize, Type)]
pub struct StatuslinePreset {
    pub preset: Preset,
    pub prefs: StatuslinePrefs,
}

pub fn statusline_presets() -> Vec<StatuslinePreset> {
    Preset::ALL
        .iter()
        .map(|&preset| StatuslinePreset {
            preset,
            prefs: preset.prefs(),
        })
        .collect()
}

/// Applies `f`, then tells every open window.
fn changed(app: &AppHandle, f: impl FnOnce(&mut PrefsView)) -> PrefsView {
    let prefs = app.state::<Prefs>();
    prefs.update(|s| f(&mut s.prefs));
    let view = prefs.prefs_view();
    let _ = PrefsChanged(view.clone()).emit(app);
    view
}

fn save(key: &str, value: &str) -> Result<(), String> {
    with_config(|path| config::save_general(path, key, value))
}

fn with_config(
    f: impl FnOnce(&std::path::Path) -> Result<(), config::ConfigError>,
) -> Result<(), String> {
    let paths = Paths::resolve().map_err(|e| e.to_string())?;
    f(paths.config_path()).map_err(|e| {
        tracing::warn!(error = %e, "could not save the settings");
        e.to_string()
    })
}
