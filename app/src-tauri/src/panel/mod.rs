//! The tray panel: a frameless popover next to the tray icon (Step 10).
//!
//! Lifecycle (architecture.md §2):
//! - **Cold:** the window is built hidden. The page loads the snapshot, renders, measures its
//!   height and calls `panel_ready`; only then is the window sized, placed and shown, so the
//!   first visible frame is complete. A page that never reports is shown after
//!   [`READY_TIMEOUT`].
//! - **Warm:** a hidden panel is placed and shown at once; the page reloads the snapshot when it
//!   becomes visible.
//! - **Pre-warm:** hovering over the tray icon builds the window in the background, so a click
//!   that follows finds the page loaded (or loading) instead of starting from nothing. Unused, it
//!   is destroyed after [`WARM_TTL`] like a hidden panel.
//! - **Hide:** on blur, `Esc` or a tray click. WebView2 is asked to trim memory, and after
//!   [`WARM_TTL`] the webview is destroyed, which ends its browser processes (ADR-023).
//!
//! A tray click first takes the focus away from the panel, which hides it, and then arrives as
//! a click that would show it again. The press remembers that the panel was open (or closed by
//! blur within [`BLUR_GRACE`]), and the release then leaves it closed.

#[cfg(any(debug_assertions, feature = "spike"))]
pub mod fake;
mod place;
#[cfg(any(debug_assertions, feature = "spike"))]
pub mod scenario;
mod system;

use std::io::Write as _;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde::Serialize;
use specta::Type;
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Webview, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_specta::Event;
use tokenradar_core::domain::Timestamp;

use crate::chrome::{self, Surface};
use crate::{core_bridge, tray};
use place::{Edge, Rect, Screen};

pub const LABEL: &str = "panel";

/// Logical width (design-language.md §5).
const WIDTH: f64 = 360.0;
/// Content taller than this scrolls.
const MAX_HEIGHT: f64 = 600.0;
/// Used until the page reports its height.
const DEFAULT_HEIGHT: f64 = 420.0;
/// The default warm TTL; the user may change it (`general.panel_warm_ttl`, Step 11).
const WARM_TTL: Duration =
    Duration::from_secs(tokenradar_core::config::DEFAULT_PANEL_WARM_TTL_S as u64);
const BLUR_GRACE: Duration = Duration::from_millis(200);
const READY_TIMEOUT: Duration = Duration::from_millis(1500);

/// Sent to the panel each time it appears: it moves the focus inside and replays its entrance.
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct PanelShown;

/// Managed by Tauri.
#[derive(Default)]
pub struct PanelState(Mutex<Inner>);

#[derive(Default)]
struct Inner {
    /// Bumped on every show and hide; a pending timer only acts if it is unchanged.
    generation: u64,
    /// When blur, `Esc` or a click last hid the panel.
    hidden_at: Option<Instant>,
    /// The next tray release must not open the panel.
    swallow_release: bool,
    /// The tray icon, in physical pixels.
    anchor: Option<Rect>,
    /// The page's content height in logical pixels.
    height: Option<f64>,
    page: Page,
    /// When the current show was requested, for the latency log.
    requested: Option<(Instant, Kind)>,
    /// WebView2 runs at the low memory level.
    trimmed: bool,
}

/// Where the panel's page is.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Page {
    /// No window.
    #[default]
    Absent,
    /// Built, but the page has not reported `panel_ready` yet. `reveal`: show it when it does
    /// (a pre-warmed window waits hidden).
    Loading { reveal: bool },
    /// The page rendered at least once.
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Cold,
    /// Requested while a pre-warmed page was still loading.
    Prewarmed,
    Warm,
}

impl PanelState {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn state(app: &AppHandle) -> MutexGuard<'_, Inner> {
    app.state::<PanelState>().inner().lock()
}

pub fn init(app: &AppHandle) {
    app.manage(PanelState::default());
}

fn visible(app: &AppHandle) -> bool {
    app.get_webview_window(LABEL)
        .is_some_and(|w| w.is_visible().unwrap_or(false))
}

/// Left button pressed on the tray icon.
pub fn tray_pressed(app: &AppHandle) {
    let open = visible(app);
    let mut s = state(app);
    s.swallow_release = open || s.hidden_at.is_some_and(|t| t.elapsed() < BLUR_GRACE);
}

/// Left button released on the tray icon: toggles the panel.
pub fn tray_released(app: &AppHandle, icon: tauri::Rect) {
    let swallow = {
        let mut s = state(app);
        s.anchor = Some(to_rect(icon));
        std::mem::take(&mut s.swallow_release)
    };
    if swallow {
        if visible(app) {
            hide(app);
        }
    } else {
        report(show(app));
    }
}

/// The pointer entered the tray icon: builds the panel in the background if there is none.
pub fn tray_hovered(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_some() {
        return;
    }
    let generation = {
        let mut s = state(app);
        s.page = Page::Loading { reveal: false };
        s.generation += 1;
        s.generation
    };
    log("prewarm start");
    if let Err(error) = create(app) {
        state(app).page = Page::Absent;
        report(Err(error));
        return;
    }
    expire(app, generation);
}

/// Shows the panel, creating it if needed (tray menu, second launch, tray click).
pub fn show(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(LABEL) {
        let mut s = state(app);
        match s.page {
            Page::Ready => {
                s.requested = Some((Instant::now(), Kind::Warm));
                drop(s);
                return reveal(app, &window);
            }
            // Still loading: it shows itself when ready.
            Page::Loading { reveal: true } => return Ok(()),
            // Pre-warmed and still loading: show it as soon as the page is ready.
            Page::Loading { reveal: false } | Page::Absent => {
                s.page = Page::Loading { reveal: true };
                s.requested = Some((Instant::now(), Kind::Prewarmed));
                s.generation += 1;
                let generation = s.generation;
                drop(s);
                ready_timeout(app, generation);
                return Ok(());
            }
        }
    }
    let generation = {
        let mut s = state(app);
        s.page = Page::Loading { reveal: true };
        s.requested = Some((Instant::now(), Kind::Cold));
        s.generation += 1;
        s.generation
    };
    if let Err(error) = create(app) {
        state(app).page = Page::Absent;
        return Err(error);
    }
    ready_timeout(app, generation);
    Ok(())
}

/// Builds the hidden window; its page reports `panel_ready` once it rendered.
fn create(app: &AppHandle) -> tauri::Result<()> {
    let started = Instant::now();
    let window = builder(app).build()?;
    log(&format!(
        "phase built {:.1}",
        started.elapsed().as_secs_f64() * 1000.0
    ));
    system::round_corners(&window);
    let handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Focused(false) => hide(&handle),
        WindowEvent::Destroyed => {
            let mut s = state(&handle);
            s.page = Page::Absent;
            s.trimmed = false;
        }
        _ => {}
    });
    Ok(())
}

/// Shows a window whose page has not reported after [`READY_TIMEOUT`] anyway.
fn ready_timeout(app: &AppHandle, generation: u64) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(READY_TIMEOUT).await;
        let late = {
            let s = state(&handle);
            s.page == Page::Loading { reveal: true } && s.generation == generation
        };
        if late {
            log("ready timeout; showing anyway");
            ready(&handle, None);
        }
    });
}

/// Destroys the hidden window after the warm TTL (`general.panel_warm_ttl`, default
/// [`WARM_TTL`]) unless it was shown or hidden again since.
fn expire(app: &AppHandle, generation: u64) {
    let handle = app.clone();
    let ttl = app
        .try_state::<crate::prefs::Prefs>()
        .map_or(WARM_TTL, |p| p.panel_warm_ttl());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(ttl).await;
        if state(&handle).generation != generation || visible(&handle) {
            return;
        }
        if let Some(window) = handle.get_webview_window(LABEL) {
            log("warm ttl over; destroying");
            report(window.destroy());
        }
    });
}

/// The page rendered its first snapshot (`panel_ready`). `height` is its content height.
pub fn ready(app: &AppHandle, height: Option<f64>) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let previous = {
        let mut s = state(app);
        let previous = std::mem::replace(&mut s.page, Page::Ready);
        if height.is_some() && previous != Page::Ready {
            s.height = height;
        }
        previous
    };
    match previous {
        Page::Loading { reveal: true } => {
            phase(app, "ready");
            report(reveal(app, &window));
        }
        Page::Loading { reveal: false } => {
            // A pre-warmed page waits like a hidden panel: once shown it becomes visible and
            // reloads its snapshot.
            log("prewarm ready");
            report(AsRef::<Webview>::as_ref(&window).hide());
            state(app).trimmed = true;
            system::set_memory_target_low(&window, true);
        }
        // A page that reloaded in its window (renderer recovery): only its height is new.
        Page::Ready | Page::Absent => {
            if let Some(height) = height {
                resize(app, height);
            }
        }
    }
}

/// The page's content height changed (`panel_resize`).
pub fn resize(app: &AppHandle, height: f64) {
    let changed = {
        let mut s = state(app);
        let changed = s.height != Some(height);
        s.height = Some(height);
        changed && s.page == Page::Ready
    };
    if changed && let Some(window) = app.get_webview_window(LABEL) {
        report(layout(app, &window));
    }
}

/// Hides the panel and starts the warm TTL.
pub fn hide(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    if !window.is_visible().unwrap_or(false) {
        return;
    }
    report(window.hide());
    // Tauri hides only the window. Hiding the webview too makes the page `hidden`: its clock
    // stops, it drops its event listener and reloads when shown again.
    report(AsRef::<Webview>::as_ref(&window).hide());
    let generation = {
        let mut s = state(app);
        s.hidden_at = Some(Instant::now());
        s.generation += 1;
        s.trimmed = true;
        s.generation
    };
    system::set_memory_target_low(&window, true);
    expire(app, generation);
}

/// The page painted after becoming visible (`panel_painted`): logs the open latency.
pub fn painted(app: &AppHandle) {
    let Some((at, kind)) = state(app).requested.take() else {
        return;
    };
    let ms = at.elapsed().as_secs_f64() * 1000.0;
    let kind = match kind {
        Kind::Cold => "cold",
        Kind::Prewarmed => "prewarmed",
        Kind::Warm => "warm",
    };
    log(&format!("painted {kind} {ms:.1}"));
}

fn reveal(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    let trimmed = {
        let mut s = state(app);
        s.generation += 1;
        s.hidden_at = None;
        std::mem::take(&mut s.trimmed)
    };
    if trimmed {
        system::set_memory_target_low(window, false);
    }
    AsRef::<Webview>::as_ref(window).show()?;
    phase(app, "webview-shown");
    layout(app, window)?;
    phase(app, "laid-out");
    window.show()?;
    phase(app, "window-shown");
    // A tray click is user input, so Windows lets the panel take the foreground.
    window.set_focus()?;
    phase(app, "focused");
    let _ = PanelShown.emit_to(app, LABEL);
    phase(app, "emitted");
    core_bridge::panel_opened(app);
    Ok(())
}

/// Logs the time since the open request (`phase <name> <ms>`), for the latency breakdown.
fn phase(app: &AppHandle, name: &str) {
    let requested = state(app).requested;
    if let Some((at, _)) = requested {
        log(&format!(
            "phase {name} {:.1}",
            at.elapsed().as_secs_f64() * 1000.0
        ));
    }
}

/// Sizes the window to its content and places it next to the tray icon.
fn layout(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    let (anchor, height) = {
        let s = state(app);
        (s.anchor, s.height)
    };
    let taskbar = system::taskbar();
    let anchor = anchor
        .or_else(|| tray::icon_rect(app).map(to_rect))
        .or_else(|| taskbar.map(|t| tray_corner(t.edge, t.rect)));
    let Some(anchor) = anchor else {
        return window.set_size(tauri::LogicalSize::new(WIDTH, panel_height(height)));
    };
    let (cx, cy) = (
        anchor.left + (anchor.right - anchor.left) / 2,
        anchor.top + (anchor.bottom - anchor.top) / 2,
    );
    let screen = system::screen_at(cx, cy).unwrap_or(Screen {
        monitor: anchor,
        work: anchor,
        scale: window.scale_factor().unwrap_or(1.0),
    });
    #[allow(clippy::cast_possible_truncation)]
    let size = (
        (WIDTH * screen.scale).round() as i32,
        (panel_height(height) * screen.scale).round() as i32,
    );
    let (x, y) = place::place(anchor, &screen, taskbar, size);
    // Moving to a monitor with another scale makes Windows resize the window, so the size is
    // set after the first move and the position once more after it.
    let position = PhysicalPosition::new(x, y);
    window.set_position(position)?;
    #[allow(clippy::cast_sign_loss)]
    window.set_size(PhysicalSize::new(size.0 as u32, size.1 as u32))?;
    window.set_position(position)
}

fn panel_height(content: Option<f64>) -> f64 {
    content.unwrap_or(DEFAULT_HEIGHT).clamp(1.0, MAX_HEIGHT)
}

/// Where the tray sits on a taskbar: its far end.
const fn tray_corner(edge: Edge, bar: Rect) -> Rect {
    match edge {
        Edge::Bottom | Edge::Top => Rect::new(bar.right - 1, bar.top, bar.right, bar.bottom),
        Edge::Left | Edge::Right => Rect::new(bar.left, bar.bottom - 1, bar.right, bar.bottom),
    }
}

fn to_rect(r: tauri::Rect) -> Rect {
    let p = r.position.to_physical::<i32>(1.0);
    let s = r.size.to_physical::<i32>(1.0);
    Rect::new(p.x, p.y, p.x + s.width, p.y + s.height)
}

/// The panel's window configuration, shared with the Step 02 measurement scenario.
pub fn builder(app: &AppHandle) -> WebviewWindowBuilder<'_, tauri::Wry, AppHandle> {
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("panel.html".into()));
    chrome::dress(app, builder, Surface::Transient)
        .title("TokenRadar")
        .inner_size(WIDTH, DEFAULT_HEIGHT)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .decorations(false)
        .shadow(true)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .focused(false)
}

/// Reports a failed window operation. Event handlers have nowhere to propagate errors to.
// TODO(step-11): route through `tracing` once logging exists.
pub fn report(result: tauri::Result<()>) {
    if let Err(err) = result {
        tracing::warn!(error = %err, "panel");
    }
}

/// `TOKENRADAR_PANEL_LOG=<file>`: timestamped lifecycle lines for `scripts/measure-panel.ps1`
/// (`<unix ms> <kind> <text>`, the Step 02 marker format). Also on stderr in debug builds.
pub fn log(line: &str) {
    let now = Timestamp::now().as_millis();
    let (kind, text) = line
        .split_once(' ')
        .map_or(("event", line), |(k, t)| (k, t));
    #[cfg(debug_assertions)]
    eprintln!("[panel] {line}");
    if let Some(path) = std::env::var_os("TOKENRADAR_PANEL_LOG")
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        let _ = writeln!(file, "{now} {kind} {text}");
    }
}
