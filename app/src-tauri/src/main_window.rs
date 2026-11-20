//! The main window (Step 15): navigation and the history pages.
//!
//! - **Lifecycle** (architecture.md §2): built hidden, shown once its page reports `main_ready`
//!   (or after [`READY_TIMEOUT`]). Closing hides it; after [`WARM_TTL`] hidden the webview is
//!   destroyed, which ends its browser processes (ADR-023). A show inside the TTL is instant.
//! - **Frame:** the native title bar over Mica (`chrome::dress`), like the settings window. The
//!   page sets the title per route.
//! - **Geometry:** position, size and maximized state are kept in `kv`. A saved position whose
//!   title bar would not be reachable on any monitor's work area (a monitor was unplugged) is
//!   dropped and the window opens centered.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_specta::Event;
use tokenradar_core::deeplink::MainRoute;

use crate::chrome::{self, Surface};
use crate::core_bridge;

pub const LABEL: &str = "main";

/// Hidden this long, the webview is destroyed (architecture.md §2).
const WARM_TTL: Duration = Duration::from_secs(30);
const READY_TIMEOUT: Duration = Duration::from_millis(1500);
/// design-language.md §5.
const MIN_SIZE: (f64, f64) = (900.0, 600.0);
const DEFAULT_SIZE: (f64, f64) = (1180.0, 760.0);
const KV_GEOMETRY: &str = "main_window_geometry";

/// Sent to an open main window that should switch to `0`. A new window reads the route from
/// its URL fragment (`main.html#sessions`).
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct MainRouteRequested(pub MainRoute);

/// Managed by Tauri.
#[derive(Default)]
pub struct MainState(Mutex<Inner>);

#[derive(Default)]
struct Inner {
    /// Bumped on every show and hide; a pending timer only acts if it is unchanged.
    generation: u64,
    page: Page,
    /// A route asked for while the page was loading: sent once it is ready.
    pending: Option<MainRoute>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Page {
    #[default]
    Absent,
    /// Built hidden; shown when the page reports `main_ready`.
    Loading {
        since: Instant,
    },
    Ready,
}

fn state(app: &AppHandle) -> MutexGuard<'_, Inner> {
    app.state::<MainState>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

pub fn init(app: &AppHandle) {
    app.manage(MainState::default());
}

/// Shows and focuses the main window, creating it if needed; `route` switches the page.
pub fn show(app: &AppHandle, route: Option<MainRoute>) -> tauri::Result<()> {
    let page = {
        let mut s = state(app);
        s.generation += 1;
        s.page
    };
    match (page, app.get_webview_window(LABEL)) {
        (Page::Ready, Some(window)) => {
            if let Some(route) = route {
                let _ = MainRouteRequested(route).emit_to(app, LABEL);
            }
            reveal(&window)
        }
        (Page::Loading { .. }, Some(_)) => {
            if route.is_some() {
                state(app).pending = route;
            }
            Ok(())
        }
        _ => create(app, route),
    }
}

fn create(app: &AppHandle, route: Option<MainRoute>) -> tauri::Result<()> {
    let url = match route {
        Some(route) => format!("main.html#{}", route.as_str()),
        None => "main.html".to_owned(),
    };
    let saved = load_geometry(app);
    let (width, height) = saved.map_or(DEFAULT_SIZE, |g| (g.width, g.height));
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(url.into()));
    let window = chrome::dress(app, builder, Surface::Window)
        .title("TokenRadar")
        .visible(false)
        .inner_size(width.max(MIN_SIZE.0), height.max(MIN_SIZE.1))
        .min_inner_size(MIN_SIZE.0, MIN_SIZE.1)
        .center()
        // The page names the route in `document.title`; the title bar follows it.
        .on_document_title_changed(|window, title| {
            let _ = window.set_title(&title);
        })
        .build()?;
    if let Some(g) = saved {
        let areas = work_areas(&window);
        let size = window.outer_size()?;
        if title_bar_reachable(g.x, g.y, size.width, &areas) {
            window.set_position(PhysicalPosition::new(g.x, g.y))?;
        }
        if g.maximized {
            window.maximize()?;
        }
    }
    {
        let mut s = state(app);
        s.page = Page::Loading {
            since: Instant::now(),
        };
        s.pending = None;
    }
    let handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide(&handle);
        }
        WindowEvent::Destroyed => {
            let mut s = state(&handle);
            s.page = Page::Absent;
            s.pending = None;
        }
        _ => {}
    });
    ready_timeout(app);
    Ok(())
}

/// The page rendered (`main_ready`): show the window and send a route asked for meanwhile.
pub fn ready(app: &AppHandle) {
    let (since, pending) = {
        let mut s = state(app);
        let since = match s.page {
            Page::Loading { since } => Some(since),
            _ => None,
        };
        s.page = Page::Ready;
        (since, s.pending.take())
    };
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    if let Some(since) = since {
        tracing::info!(
            target: "main_window",
            ms = format!("{:.1}", since.elapsed().as_secs_f64() * 1000.0),
            "cold open"
        );
        report(reveal(&window));
    }
    if let Some(route) = pending {
        let _ = MainRouteRequested(route).emit_to(app, LABEL);
    }
}

fn ready_timeout(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(READY_TIMEOUT).await;
        if matches!(state(&handle).page, Page::Loading { .. }) {
            tracing::warn!(target: "main_window", "page did not report; showing anyway");
            ready(&handle);
        }
    });
}

fn reveal(window: &WebviewWindow) -> tauri::Result<()> {
    if window.is_minimized()? {
        window.unminimize()?;
    }
    window.show()?;
    window.set_focus()
}

/// Hides the window, keeps its geometry and starts the warm TTL.
fn hide(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    save_geometry(app, &window);
    report(window.hide());
    let generation = {
        let mut s = state(app);
        s.generation += 1;
        s.generation
    };
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(WARM_TTL).await;
        if state(&handle).generation != generation {
            return;
        }
        if let Some(window) = handle.get_webview_window(LABEL)
            && !window.is_visible().unwrap_or(false)
        {
            tracing::debug!(target: "main_window", "warm ttl over; destroying");
            report(window.destroy());
        }
    });
}

/// Where the window was: outer position in physical pixels (monitors can differ in scale),
/// inner size in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct Geometry {
    x: i32,
    y: i32,
    width: f64,
    height: f64,
    maximized: bool,
}

fn load_geometry(app: &AppHandle) -> Option<Geometry> {
    let text = core_bridge::store(app)?.kv_get(KV_GEOMETRY).ok()??;
    serde_json::from_str(&text).ok()
}

fn save_geometry(app: &AppHandle, window: &WebviewWindow) {
    if window.is_minimized().unwrap_or(true) {
        return;
    }
    let Some(store) = core_bridge::store(app) else {
        return;
    };
    let maximized = window.is_maximized().unwrap_or(false);
    // A maximized window's rect is the monitor's: keep the last normal one.
    let geometry = if maximized {
        load_geometry(app).map_or(
            Geometry {
                x: 0,
                y: 0,
                width: DEFAULT_SIZE.0,
                height: DEFAULT_SIZE.1,
                maximized: true,
            },
            |g| Geometry {
                maximized: true,
                ..g
            },
        )
    } else {
        let (Ok(pos), Ok(size), Ok(scale)) = (
            window.outer_position(),
            window.inner_size(),
            window.scale_factor(),
        ) else {
            return;
        };
        let size: LogicalSize<f64> = size.to_logical(scale);
        Geometry {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
            maximized: false,
        }
    };
    if let Ok(text) = serde_json::to_string(&geometry)
        && let Err(error) = store.writer().kv_set(KV_GEOMETRY.to_owned(), text)
    {
        tracing::warn!(%error, "main window geometry not saved");
    }
}

/// A monitor's work area in physical pixels: `(x, y, width, height)`.
type Area = (i32, i32, u32, u32);

fn work_areas(window: &WebviewWindow) -> Vec<Area> {
    window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let a = m.work_area();
            (a.position.x, a.position.y, a.size.width, a.size.height)
        })
        .collect()
}

/// Whether enough of the title bar of a window at `(x, y)`, `width` wide, lies on a work area
/// to grab it: 120 × 24 physical pixels.
fn title_bar_reachable(x: i32, y: i32, width: u32, areas: &[Area]) -> bool {
    const GRAB_W: i64 = 120;
    const GRAB_H: i64 = 24;
    let (x, y, w) = (i64::from(x), i64::from(y), i64::from(width));
    areas.iter().any(|&(ax, ay, aw, ah)| {
        let (ax, ay, aw, ah) = (i64::from(ax), i64::from(ay), i64::from(aw), i64::from(ah));
        let overlap_w = (x + w).min(ax + aw) - x.max(ax);
        let overlap_h = (y + GRAB_H).min(ay + ah) - y.max(ay);
        overlap_w >= GRAB_W && overlap_h >= GRAB_H
    })
}

fn report(result: tauri::Result<()>) {
    if let Err(error) = result {
        tracing::warn!(target: "main_window", %error, "window operation failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIMARY: Area = (0, 0, 1920, 1032);
    const RIGHT_150: Area = (1920, 0, 3840, 2100);

    #[test]
    fn title_bar_on_a_monitor_is_reachable() {
        assert!(title_bar_reachable(100, 100, 1200, &[PRIMARY]));
        // Mostly off the left edge, 200 px still on screen.
        assert!(title_bar_reachable(-1000, 10, 1200, &[PRIMARY]));
        // On the second monitor.
        assert!(title_bar_reachable(2500, 300, 1800, &[PRIMARY, RIGHT_150]));
    }

    #[test]
    fn unplugged_monitor_or_hidden_title_bar_is_not() {
        // The second monitor is gone.
        assert!(!title_bar_reachable(2500, 300, 1800, &[PRIMARY]));
        // Title bar under the taskbar / below the work area.
        assert!(!title_bar_reachable(100, 1020, 1200, &[PRIMARY]));
        // Above the top edge.
        assert!(!title_bar_reachable(100, -40, 1200, &[PRIMARY]));
        // Only a sliver on screen.
        assert!(!title_bar_reachable(1860, 100, 1200, &[PRIMARY]));
        assert!(!title_bar_reachable(100, 100, 1200, &[]));
    }

    #[test]
    fn geometry_round_trips_as_json() {
        let g = Geometry {
            x: -7,
            y: 12,
            width: 1180.0,
            height: 760.5,
            maximized: true,
        };
        let text = serde_json::to_string(&g).expect("json");
        assert_eq!(serde_json::from_str::<Geometry>(&text).expect("parse"), g);
    }
}
