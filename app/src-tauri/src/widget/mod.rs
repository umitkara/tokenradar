//! The optional compact strip (Step 18, ADR-058): a native layered window on its own thread,
//! drawn with `tiny-skia` and DirectWrite. No `WebView`.
//!
//! The widget is off by default (ADR-003). While it is off there is no thread and no window;
//! turning it on spawns the `widget` thread, turning it off destroys both. The tray feeds it
//! the same model it draws its icon from, so the two never disagree.

mod fullscreen;
pub(crate) mod model;
mod place;
mod render;
#[cfg(windows)]
mod system;
#[cfg(windows)]
mod text;

#[cfg(any(debug_assertions, feature = "spike"))]
pub mod fake;

use std::sync::{Mutex, PoisonError};

use tauri::{AppHandle, Manager};
use tokenradar_core::config::{ThemePref, WidgetMode, WidgetPosition, WidgetPrefs};
use tokenradar_i18n::Locale;

pub use model::WidgetModel;

/// What the `widget` thread reacts to.
#[derive(Debug)]
pub enum Msg {
    Model(WidgetModel),
    Prefs(WidgetPrefs),
    Locale(Locale),
    Theme(ThemePref),
    Quit,
}

/// Everything a new widget thread starts from.
#[derive(Debug, Clone)]
pub struct Init {
    pub model: WidgetModel,
    pub prefs: WidgetPrefs,
    pub position: Option<WidgetPosition>,
    pub locale: Locale,
    pub theme: ThemePref,
    /// Write drags and menu choices to `config.toml`. Off in fake mode, so a look at a scene
    /// never changes the user's settings.
    pub persist: bool,
}

#[derive(Default)]
struct Shared {
    model: WidgetModel,
    prefs: Option<WidgetPrefs>,
    position: Option<WidgetPosition>,
    locale: Option<Locale>,
    theme: ThemePref,
    /// Fake mode: shown whatever the settings say, in this mode, nothing persisted.
    forced: Option<WidgetMode>,
    #[cfg(windows)]
    running: Option<system::Handle>,
}

/// Managed by Tauri.
#[derive(Default)]
pub struct Widget(Mutex<Shared>);

pub fn init(app: &AppHandle) {
    app.manage(Widget::default());
}

fn with<R>(app: &AppHandle, f: impl FnOnce(&mut Shared) -> R) -> Option<R> {
    let widget = app.try_state::<Widget>()?;
    let mut shared = widget.0.lock().unwrap_or_else(PoisonError::into_inner);
    Some(f(&mut shared))
}

/// The tray's model changed.
pub fn update(app: &AppHandle, model: WidgetModel) {
    with(app, |s| {
        if s.model == model {
            return;
        }
        s.model = model.clone();
        #[cfg(windows)]
        if let Some(handle) = &s.running {
            handle.send(Msg::Model(model));
        }
    });
}

/// The settings were read or changed: starts, updates or stops the thread.
pub fn apply(app: &AppHandle, prefs: &WidgetPrefs, position: Option<WidgetPosition>) {
    with(app, |s| {
        s.prefs = Some(prefs.clone());
        if position.is_some() {
            s.position = position;
        }
        reconcile(app, s);
    });
}

/// Where the user left the widget (from the widget thread, after a drag).
pub fn remember_position(app: &AppHandle, position: WidgetPosition) {
    with(app, |s| s.position = Some(position));
}

pub fn set_locale(app: &AppHandle, locale: Locale) {
    with(app, |s| {
        s.locale = Some(locale);
        #[cfg(windows)]
        if let Some(handle) = &s.running {
            handle.send(Msg::Locale(locale));
        }
    });
}

pub fn set_theme(app: &AppHandle, theme: ThemePref) {
    with(app, |s| {
        s.theme = theme;
        #[cfg(windows)]
        if let Some(handle) = &s.running {
            handle.send(Msg::Theme(theme));
        }
    });
}

/// Fake mode (`TOKENRADAR_FAKE_LIMITS=widget:<scene>[:taskbar]`): shows the widget in `mode`
/// without touching the settings.
#[cfg(any(debug_assertions, feature = "spike"))]
pub fn force(app: &AppHandle, mode: WidgetMode) {
    with(app, |s| {
        s.forced = Some(mode);
        reconcile(app, s);
    });
}

/// Whether the widget is on and whether it lets clicks through, for the tray menu.
pub fn menu_state(prefs: &WidgetPrefs) -> (bool, bool) {
    (prefs.enabled, prefs.enabled && prefs.click_through)
}

/// Whether the widget is drawn light, by what it stands on (ADR-058).
///
/// Floating, it sits over app windows and the desktop: the app's theme setting decides, and
/// "system" means the apps theme (`apps_light`), as the panel follows it. Docked on the taskbar
/// (Step 18b), it is part of the taskbar: the taskbar's theme (`taskbar_light`) decides, as for
/// the tray icon, whatever the app's setting says.
pub fn is_light(
    pref: ThemePref,
    mode: WidgetMode,
    apps_light: impl FnOnce() -> bool,
    taskbar_light: impl FnOnce() -> bool,
) -> bool {
    match (mode, pref) {
        (WidgetMode::Taskbar, _) => taskbar_light(),
        (WidgetMode::Floating, ThemePref::Light) => true,
        (WidgetMode::Floating, ThemePref::Dark) => false,
        (WidgetMode::Floating, ThemePref::System) => apps_light(),
    }
}

#[cfg(windows)]
fn reconcile(app: &AppHandle, s: &mut Shared) {
    // Nothing happens before `config.toml` was read: the defaults would say "off" anyway.
    let Some(prefs) = s.prefs.clone() else { return };
    let mut prefs = prefs;
    if let Some(mode) = s.forced {
        prefs.enabled = true;
        prefs.mode = mode;
    }
    match (&s.running, prefs.enabled) {
        (Some(handle), true) => handle.send(Msg::Prefs(prefs)),
        (Some(handle), false) => {
            handle.send(Msg::Quit);
            s.running = None;
        }
        (None, true) => {
            let init = Init {
                model: s.model.clone(),
                prefs,
                position: s.position.clone(),
                locale: s
                    .locale
                    .unwrap_or_else(|| app.state::<crate::prefs::Prefs>().locale()),
                theme: s.theme,
                persist: s.forced.is_none(),
            };
            match system::spawn(app.clone(), init) {
                Ok(handle) => s.running = Some(handle),
                Err(error) => tracing::warn!(%error, "widget: could not start"),
            }
        }
        (None, false) => {}
    }
}

/// Other platforms (Step 21): no widget yet.
#[cfg(not(windows))]
fn reconcile(_app: &AppHandle, _s: &mut Shared) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floating_follows_the_apps_and_docked_follows_the_taskbar() {
        // Dark taskbar, light apps: the mix where the two answers differ.
        let light = |pref, mode| is_light(pref, mode, || true, || false);
        assert!(light(ThemePref::System, WidgetMode::Floating));
        assert!(!light(ThemePref::Dark, WidgetMode::Floating));
        assert!(!light(ThemePref::System, WidgetMode::Taskbar));
        assert!(!light(ThemePref::Light, WidgetMode::Taskbar));
    }
}
