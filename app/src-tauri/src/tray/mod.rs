//! The tray icon: a live dual ring drawn in Rust (ADR-003), its tooltip and its menu.
//!
//! One `tray` thread owns the state. It sleeps on a channel and wakes for core snapshots, menu
//! actions, system changes (theme, DPI, high contrast) and the next minute of the 5-hour
//! countdown. Each wake recomputes the frame; [`Presenter`] only touches the icon, tooltip or
//! menu when what the user sees actually changed.

mod render;
pub(crate) mod system;
mod tooltip;
mod visual;

#[cfg(any(debug_assertions, feature = "spike"))]
pub mod fake;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tokenradar_core::config::AppConfig;
use tokenradar_core::domain::Timestamp;
use tokenradar_core::limits::PanelSnapshot;
use tokenradar_core::paths::Paths;
use tokenradar_core::store::Store;
use tokenradar_i18n::{Domain, Locale, Localizer};

use crate::ipc::MuteChanged;
use crate::{panel, settings};
#[cfg(test)]
use render::Theme;
use system::{Environment, SystemClock};
use visual::{TrayModel, TrayVisual};

const TRAY_ID: &str = "main";
const MENU_OPEN_PANEL: &str = "open-panel";
const MENU_MUTE: &str = "mute";
const MENU_SETTINGS: &str = "settings";
const MENU_ABOUT: &str = "about";
const MENU_QUIT: &str = "quit";
#[cfg(debug_assertions)]
const MENU_GALLERY: &str = "gallery";

/// `kv` key holding the end of the notification mute, in Unix milliseconds. Survives restarts
/// (step 08); the notifier (Step 11) reads the same key.
pub const KV_NOTIFY_SNOOZED_UNTIL: &str = "notify_snoozed_until";
const MUTE_FOR_MS: i64 = 60 * 60 * 1000;

/// What the `tray` thread reacts to.
pub enum TrayMsg {
    /// The core is running; the tray reads and writes the mute through its store.
    CoreReady(Store),
    CoreFailed,
    Snapshot(Box<PanelSnapshot>),
    /// "Mute for 1 hour" / "Unmute" was clicked.
    ToggleMute,
    /// Sets the mute directly (debug scenarios).
    #[cfg(any(debug_assertions, test, feature = "spike"))]
    MutedUntil(Option<Timestamp>),
    /// The UI language changed. Sent by the settings window (Step 11) and the `cycle` scenario.
    #[cfg_attr(not(any(debug_assertions, feature = "spike")), allow(dead_code))]
    Locale(Locale),
    /// Theme, scale or high contrast may have changed.
    Environment,
}

/// Sends to the `tray` thread; managed by Tauri.
pub struct Tray(Sender<TrayMsg>);

/// Whether notifications are muted, as the tray last showed it; managed by Tauri for the panel.
#[derive(Default)]
pub struct Muted(AtomicBool);

/// Whether notifications are muted right now.
pub fn is_muted(app: &AppHandle) -> bool {
    app.try_state::<Muted>()
        .is_some_and(|m| m.0.load(Ordering::Relaxed))
}

/// The tray icon's screen rectangle, in physical pixels.
pub fn icon_rect(app: &AppHandle) -> Option<tauri::Rect> {
    app.tray_by_id(TRAY_ID)?.rect().ok().flatten()
}

/// Sends `msg` to the tray thread. Before the tray exists, or after it stopped, it is dropped.
pub fn send(app: &AppHandle, msg: TrayMsg) {
    if let Some(tray) = app.try_state::<Tray>() {
        let _ = tray.0.send(msg);
    }
}

/// Shows the tray icon with an empty ring right away, then hands it to the `tray` thread.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    // The OS language for now; the thread switches to `config.toml`'s choice once it read it
    // (the main thread never waits on disk).
    let l10n = Localizer::new(Locale::detect(), &[Domain::Tray]);
    let env = system::environment();
    let visual = TrayVisual::default();
    let now = Timestamp::now();
    let model = TrayModel::default();
    let tip = tooltip::tooltip(&model, &l10n, &SystemClock, now);
    let (menu, mute_item) = build_menu(app, &l10n, false)?;

    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon_image(visual, env))
        .tooltip(&tip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN_PANEL => panel::report(panel::show(app)),
            MENU_MUTE => send(app, TrayMsg::ToggleMute),
            MENU_SETTINGS | MENU_ABOUT => panel::report(settings::show(app, None)),
            MENU_QUIT => app.exit(0),
            #[cfg(debug_assertions)]
            MENU_GALLERY => panel::report(crate::gallery::show(app)),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            match event {
                // The press only remembers whether the panel was open; the release toggles it.
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state,
                    rect,
                    ..
                } => match button_state {
                    MouseButtonState::Down => panel::tray_pressed(tray.app_handle()),
                    MouseButtonState::Up => panel::tray_released(tray.app_handle(), rect),
                },
                TrayIconEvent::Enter { .. } => panel::tray_hovered(tray.app_handle()),
                _ => {}
            }
        })
        .build(app)?;

    let (tx, rx) = mpsc::channel();
    let watcher_tx = tx.clone();
    if let Err(error) = system::watch(Box::new(move || {
        let _ = watcher_tx.send(TrayMsg::Environment);
    })) {
        eprintln!("TokenRadar: tray will not follow theme or scale changes: {error}");
    }
    app.manage(Tray(tx));
    app.manage(Muted::default());

    let presenter = Presenter {
        icon: Some((visual, env)),
        tooltip: Some(tip),
        menu: Some((l10n.locale(), false)),
    };
    let sink = TauriSink {
        app: app.clone(),
        tray,
        mute_item,
    };
    // `Localizer` is not `Send`; the thread builds its own.
    let locale = l10n.locale();
    std::thread::Builder::new()
        .name("tray".into())
        .spawn(move || {
            let driver = Driver {
                rx,
                model,
                l10n: Localizer::new(locale, &[Domain::Tray]),
                env,
                store: None,
                presenter,
                sink,
            };
            driver.run();
        })?;
    Ok(())
}

fn icon_image(visual: TrayVisual, env: Environment) -> Image<'static> {
    let pixmap = render::render(visual, env.px, env.theme);
    Image::new_owned(render::to_rgba(&pixmap), pixmap.width(), pixmap.height())
}

fn build_menu(
    app: &AppHandle,
    l10n: &Localizer,
    muted: bool,
) -> tauri::Result<(Menu<Wry>, MenuItem<Wry>)> {
    let item = |id: &str, key: &str| MenuItem::with_id(app, id, l10n.msg(key), true, None::<&str>);
    let mute = item(MENU_MUTE, mute_key(muted))?;
    let menu = Menu::with_items(
        app,
        &[
            &item(MENU_OPEN_PANEL, "tray-menu-open-panel")?,
            &mute,
            &PredefinedMenuItem::separator(app)?,
            &item(MENU_SETTINGS, "tray-menu-settings")?,
            &item(MENU_ABOUT, "tray-menu-about")?,
            &PredefinedMenuItem::separator(app)?,
            &item(MENU_QUIT, "tray-menu-quit")?,
        ],
    )?;
    // Development builds only: the component gallery (Step 09).
    #[cfg(debug_assertions)]
    menu.insert(&item(MENU_GALLERY, "tray-menu-gallery")?, 5)?;
    Ok((menu, mute))
}

const fn mute_key(muted: bool) -> &'static str {
    if muted {
        "tray-menu-unmute"
    } else {
        "tray-menu-mute"
    }
}

/// What the tray shows at one moment.
struct Frame {
    visual: TrayVisual,
    env: Environment,
    tooltip: String,
    muted: bool,
}

/// Where frames go: Tauri in the app, a recorder in tests.
trait Sink {
    fn set_icon(&mut self, visual: TrayVisual, env: Environment);
    fn set_tooltip(&mut self, text: &str);
    fn rebuild_menu(&mut self, l10n: &Localizer, muted: bool);
    fn set_mute_label(&mut self, l10n: &Localizer, muted: bool);
}

/// The update throttle: remembers what is on screen and skips calls that would not change it.
/// Each `set_icon` creates a new HICON, so an unchanged icon is never set again.
struct Presenter {
    icon: Option<(TrayVisual, Environment)>,
    tooltip: Option<String>,
    menu: Option<(Locale, bool)>,
}

impl Presenter {
    fn present(&mut self, frame: Frame, l10n: &Localizer, sink: &mut impl Sink) {
        let icon = (frame.visual, frame.env);
        if self.icon != Some(icon) {
            sink.set_icon(frame.visual, frame.env);
            self.icon = Some(icon);
        }
        if self.tooltip.as_deref() != Some(frame.tooltip.as_str()) {
            sink.set_tooltip(&frame.tooltip);
            self.tooltip = Some(frame.tooltip);
        }
        match self.menu {
            Some((locale, muted)) if locale == l10n.locale() => {
                if muted != frame.muted {
                    sink.set_mute_label(l10n, frame.muted);
                }
            }
            _ => sink.rebuild_menu(l10n, frame.muted),
        }
        self.menu = Some((l10n.locale(), frame.muted));
    }
}

struct TauriSink {
    app: AppHandle,
    tray: TrayIcon,
    mute_item: MenuItem<Wry>,
}

impl TauriSink {
    /// Tells the panel when the mute changed (the tray thread owns it).
    fn publish_mute(&self, muted: bool) {
        let Some(state) = self.app.try_state::<Muted>() else {
            return;
        };
        if state.0.swap(muted, Ordering::Relaxed) != muted {
            crate::ipc::emit_to_visible(&self.app, &MuteChanged { muted });
        }
    }
}

impl Sink for TauriSink {
    fn set_icon(&mut self, visual: TrayVisual, env: Environment) {
        if let Err(error) = self.tray.set_icon(Some(icon_image(visual, env))) {
            eprintln!("TokenRadar: could not update the tray icon: {error}");
        }
    }

    fn set_tooltip(&mut self, text: &str) {
        if let Err(error) = self.tray.set_tooltip(Some(text)) {
            eprintln!("TokenRadar: could not update the tray tooltip: {error}");
        }
    }

    fn rebuild_menu(&mut self, l10n: &Localizer, muted: bool) {
        match build_menu(&self.app, l10n, muted) {
            Ok((menu, mute_item)) => {
                if let Err(error) = self.tray.set_menu(Some(menu)) {
                    eprintln!("TokenRadar: could not replace the tray menu: {error}");
                }
                self.mute_item = mute_item;
            }
            Err(error) => eprintln!("TokenRadar: could not build the tray menu: {error}"),
        }
        self.publish_mute(muted);
    }

    fn set_mute_label(&mut self, l10n: &Localizer, muted: bool) {
        if let Err(error) = self.mute_item.set_text(l10n.msg(mute_key(muted))) {
            eprintln!("TokenRadar: could not update the tray menu: {error}");
        }
        self.publish_mute(muted);
    }
}

struct Driver<S: Sink> {
    rx: Receiver<TrayMsg>,
    model: TrayModel,
    l10n: Localizer,
    env: Environment,
    store: Option<Store>,
    presenter: Presenter,
    sink: S,
}

impl<S: Sink> Driver<S> {
    fn run(mut self) {
        self.load_locale();
        loop {
            let now = Timestamp::now();
            let msg = match tooltip::next_change(&self.model, now) {
                Some(at) => {
                    let wait = u64::try_from(at.as_millis() - now.as_millis()).unwrap_or(0);
                    match self.rx.recv_timeout(Duration::from_millis(wait)) {
                        Ok(msg) => Some(msg),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                None => match self.rx.recv() {
                    Ok(msg) => Some(msg),
                    Err(_) => return,
                },
            };
            if let Some(msg) = msg {
                self.handle(msg);
            }
            // Coalesce a burst (several snapshots, a settings storm) into one frame.
            while let Ok(msg) = self.rx.try_recv() {
                self.handle(msg);
            }
            self.present(Timestamp::now());
        }
    }

    /// `general.locale` from `config.toml`, else the OS language.
    fn load_locale(&mut self) {
        let locale = Paths::resolve()
            .ok()
            .and_then(|p| AppConfig::load(p.config_path()).ok())
            .map_or_else(Locale::detect, |c| c.general.locale());
        self.set_locale(locale);
    }

    fn set_locale(&mut self, locale: Locale) {
        if locale != self.l10n.locale() {
            self.l10n = Localizer::new(locale, &[Domain::Tray]);
        }
    }

    fn handle(&mut self, msg: TrayMsg) {
        match msg {
            TrayMsg::CoreReady(store) => {
                self.model.muted_until = read_mute(&store);
                self.store = Some(store);
            }
            TrayMsg::CoreFailed => self.model.core_failed = true,
            TrayMsg::Snapshot(snapshot) => self.model.snapshot = Some(*snapshot),
            TrayMsg::ToggleMute => {
                let now = Timestamp::now();
                let until = if self.model.is_muted(now) {
                    None
                } else {
                    Some(Timestamp::from_millis(now.as_millis() + MUTE_FOR_MS))
                };
                self.model.muted_until = until;
                if let Some(store) = &self.store {
                    write_mute(store, until);
                }
            }
            #[cfg(any(debug_assertions, test, feature = "spike"))]
            TrayMsg::MutedUntil(until) => self.model.muted_until = until,
            TrayMsg::Locale(locale) => self.set_locale(locale),
            TrayMsg::Environment => self.env = system::environment(),
        }
    }

    fn present(&mut self, now: Timestamp) {
        let frame = Frame {
            visual: self.model.visual(now),
            env: self.env,
            tooltip: tooltip::tooltip(&self.model, &self.l10n, &SystemClock, now),
            muted: self.model.is_muted(now),
        };
        self.presenter.present(frame, &self.l10n, &mut self.sink);
    }
}

fn read_mute(store: &Store) -> Option<Timestamp> {
    match store.kv_get(KV_NOTIFY_SNOOZED_UNTIL) {
        Ok(value) => value
            .and_then(|v| v.parse().ok())
            .map(Timestamp::from_millis),
        Err(error) => {
            eprintln!("TokenRadar: could not read the notification mute: {error}");
            None
        }
    }
}

fn write_mute(store: &Store, until: Option<Timestamp>) {
    let key = KV_NOTIFY_SNOOZED_UNTIL.to_owned();
    let result = match until {
        Some(t) => store.writer().kv_set(key, t.as_millis().to_string()),
        None => store.writer().kv_delete(key),
    };
    if let Err(error) = result {
        eprintln!("TokenRadar: could not store the notification mute: {error}");
    }
}

#[cfg(test)]
mod tests;
