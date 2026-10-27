//! Step 02 measurement scenario for the WebView2 lifecycle (the ADR-004 gate).
//!
//! Runs when `TOKENRADAR_SPIKE=lifecycle` is set, in debug builds or with the `spike` feature
//! (release builds measure the real frontend bundle instead of the Vite dev server). It drives the
//! panel through cold open, warm show, destroy, a second window and the hidden-window memory
//! levers, writing timestamped markers that `scripts/spike-lifecycle.ps1` joins with the samples
//! of `scripts/measure-memory.ps1`. The app exits when the scenario ends.
//!
//! Environment:
//! - `TOKENRADAR_SPIKE_LOG`: marker file; markers also go to stderr.
//! - `TOKENRADAR_SPIKE_BROWSER_ARGS`: extra WebView2 browser arguments for every spike window.

// COM calls into WebView2 for the visibility and memory-target experiments.
#![allow(unsafe_code)]

use std::error::Error;
use std::fs::File;
use std::io::Write as _;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Wry};

use crate::panel;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const WIDGET_LABEL: &str = "spike-widget";
const READY_TIMEOUT: Duration = Duration::from_secs(15);

/// Reports `DOMContentLoaded` and the second animation frame after it (the first painted frame).
const READY_SCRIPT: &str = r"
document.addEventListener('DOMContentLoaded', () => {
  const ready = (event) => window.__TAURI_INTERNALS__.invoke('spike_ready', { event });
  ready('dcl');
  requestAnimationFrame(() => requestAnimationFrame(() => ready('frame')));
});
";

/// Reports the second animation frame from now: the first frame painted after a show.
const FRAME_SCRIPT: &str = "requestAnimationFrame(() => requestAnimationFrame(() => \
    window.__TAURI_INTERNALS__.invoke('spike_ready', { event: 'frame' })));";

/// `(window label, event)` with the moment the command ran.
type Signal = (String, String, Instant);

struct Ready(Mutex<Sender<Signal>>);

// Command arguments are deserialized into owned values.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
fn spike_ready(window: WebviewWindow, ready: tauri::State<'_, Ready>, event: String) {
    let at = Instant::now();
    if let Ok(tx) = ready.0.lock() {
        let _ = tx.send((window.label().to_owned(), event, at));
    }
}

/// Registers the readiness command next to the app's `typed` commands. Tauri keeps only the
/// last `invoke_handler`, so both go through one dispatcher.
pub fn register(
    builder: tauri::Builder<Wry>,
    typed: impl Fn(tauri::ipc::Invoke<Wry>) -> bool + Send + Sync + 'static,
) -> tauri::Builder<Wry> {
    let spike = spike_handler();
    builder.invoke_handler(move |invoke| {
        if invoke.message.command() == "spike_ready" {
            spike(invoke)
        } else {
            typed(invoke)
        }
    })
}

fn spike_handler() -> impl Fn(tauri::ipc::Invoke<Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![spike_ready]
}

/// Starts the scenario named by `TOKENRADAR_SPIKE`, if any.
pub fn start(app: &AppHandle) {
    let Ok(name) = std::env::var("TOKENRADAR_SPIKE") else {
        return;
    };
    if name != "lifecycle" {
        eprintln!("spike: unknown scenario '{name}'");
        return;
    }
    let (tx, rx) = mpsc::channel();
    app.manage(Ready(Mutex::new(tx)));
    let app = app.clone();
    thread::spawn(move || {
        let mut scenario = Scenario {
            app: app.clone(),
            rx,
            log: Log::open(),
            browser_args: std::env::var("TOKENRADAR_SPIKE_BROWSER_ARGS").ok(),
        };
        if let Err(err) = scenario.lifecycle() {
            scenario.log.write("error", &err.to_string());
        }
        app.exit(0);
    });
}

struct Scenario {
    app: AppHandle,
    rx: Receiver<Signal>,
    log: Log,
    browser_args: Option<String>,
}

impl Scenario {
    fn lifecycle(&mut self) -> Result<()> {
        self.log.write("mark", "start");
        if let Some(args) = &self.browser_args {
            self.log.write("info", &format!("browser_args={args}"));
        }
        self.idle(30, "baseline");

        // 1. First cold open: includes creating the WebView2 environment.
        let panel = self.open_panel("cold1")?;
        self.idle(10, "panel_open");
        panel.hide()?;
        self.idle(5, "panel_hidden");
        self.show_warm(&panel, "warm1")?;
        thread::sleep(Duration::from_secs(2));
        panel.hide()?;
        thread::sleep(Duration::from_secs(1));

        // 2. Destroy the only window: do the WebView2 processes exit?
        self.log.write("mark", "destroy1");
        panel.destroy()?;
        self.idle(60, "after_destroy1");

        // 3. A later cold open, then a widget-sized second window next to it.
        let panel = self.open_panel("cold2")?;
        self.idle(10, "one_window");
        let widget = self.open_widget()?;
        self.idle(10, "two_windows");
        self.log.write("mark", "destroy2");
        widget.destroy()?;
        panel.destroy()?;
        self.idle(60, "after_destroy2");

        // 4. A warm but hidden panel, and the levers that might shrink it.
        let panel = self.open_panel("cold3")?;
        thread::sleep(Duration::from_secs(5));
        panel.hide()?;
        self.idle(15, "hidden_window");
        set_controller_visible(&panel, false)?;
        self.idle(15, "hidden_controller");
        set_memory_target_low(&panel, true)?;
        self.idle(15, "hidden_low");
        set_memory_target_low(&panel, false)?;
        set_controller_visible(&panel, true)?;
        self.show_warm(&panel, "warm_after_low")?;
        thread::sleep(Duration::from_secs(2));
        self.log.write("mark", "destroy3");
        panel.destroy()?;
        self.idle(30, "end");
        Ok(())
    }

    fn idle(&mut self, secs: u64, mark: &str) {
        thread::sleep(Duration::from_secs(secs));
        self.log.write("mark", mark);
    }

    fn with_browser_args<'a>(
        &self,
        builder: WebviewWindowBuilder<'a, Wry, AppHandle>,
    ) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
        match &self.browser_args {
            Some(args) => builder.additional_browser_args(args),
            None => builder,
        }
    }

    /// Creates and shows the panel, timing build, show, `DOMContentLoaded` and first frame.
    fn open_panel(&mut self, name: &str) -> Result<WebviewWindow> {
        self.drain();
        let started = Instant::now();
        let builder = panel::builder(&self.app).initialization_script(READY_SCRIPT);
        let window = self.with_browser_args(builder).build()?;
        self.log
            .metric(&format!("{name}_build_ms"), started, Instant::now());
        window.show()?;
        self.log
            .metric(&format!("{name}_show_ms"), started, Instant::now());
        let dcl = self.wait(panel::LABEL, "dcl")?;
        self.log.metric(&format!("{name}_dcl_ms"), started, dcl);
        let frame = self.wait(panel::LABEL, "frame")?;
        self.log.metric(&format!("{name}_frame_ms"), started, frame);
        Ok(window)
    }

    /// Shows a hidden window and times the first frame painted afterwards.
    fn show_warm(&mut self, window: &WebviewWindow, name: &str) -> Result<()> {
        self.drain();
        let started = Instant::now();
        window.show()?;
        self.log
            .metric(&format!("{name}_show_ms"), started, Instant::now());
        window.eval(FRAME_SCRIPT)?;
        let frame = self.wait(window.label(), "frame")?;
        self.log.metric(&format!("{name}_frame_ms"), started, frame);
        Ok(())
    }

    /// A stand-in for the v0.3 widget: same page, 220×44 logical pixels.
    fn open_widget(&mut self) -> Result<WebviewWindow> {
        self.drain();
        let started = Instant::now();
        let builder = WebviewWindowBuilder::new(
            &self.app,
            WIDGET_LABEL,
            WebviewUrl::App("panel.html".into()),
        )
        .title("TokenRadar widget spike")
        .inner_size(220.0, 44.0)
        .resizable(false)
        .decorations(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .initialization_script(READY_SCRIPT);
        let window = self.with_browser_args(builder).build()?;
        let frame = self.wait(WIDGET_LABEL, "frame")?;
        self.log.metric("widget_frame_ms", started, frame);
        Ok(window)
    }

    fn drain(&self) {
        while self.rx.try_recv().is_ok() {}
    }

    fn wait(&self, label: &str, event: &str) -> Result<Instant> {
        let deadline = Instant::now() + READY_TIMEOUT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let (from, what, at) = self
                .rx
                .recv_timeout(left)
                .map_err(|_| format!("no '{event}' from '{label}' within {READY_TIMEOUT:?}"))?;
            if from == label && what == event {
                return Ok(at);
            }
        }
    }
}

/// Timestamped marker lines: `<unix ms> <kind> <text>`.
struct Log(Option<File>);

impl Log {
    fn open() -> Self {
        Self(std::env::var_os("TOKENRADAR_SPIKE_LOG").and_then(|path| File::create(path).ok()))
    }

    fn write(&mut self, kind: &str, text: &str) {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let line = format!("{ms} {kind} {text}");
        eprintln!("spike: {line}");
        if let Some(file) = &mut self.0 {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }
    }

    fn metric(&mut self, name: &str, from: Instant, to: Instant) {
        let ms = to.saturating_duration_since(from).as_secs_f64() * 1000.0;
        self.write("metric", &format!("{name}={ms:.1}"));
    }
}

/// Runs `f` with the window's WebView2 controller on the UI thread and returns its result.
#[cfg(windows)]
fn with_controller(
    window: &WebviewWindow,
    f: impl FnOnce(
        &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller,
    ) -> windows_core::Result<()>
    + Send
    + 'static,
) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    window.with_webview(move |webview| {
        let _ = tx.send(f(&webview.controller()));
    })?;
    rx.recv_timeout(Duration::from_secs(5))??;
    Ok(())
}

/// Tells WebView2 whether its host is visible. Tauri's `hide()` only hides the window, so
/// without this WebView2 keeps rendering as if the panel were on screen.
#[cfg(windows)]
fn set_controller_visible(window: &WebviewWindow, visible: bool) -> Result<()> {
    // SAFETY: `with_webview` runs the closure on the UI thread that owns the live controller.
    with_controller(window, move |controller| unsafe {
        controller.SetIsVisible(visible)
    })
}

/// `ICoreWebView2_19::SetMemoryUsageTargetLevel`: Low asks WebView2 to trim memory.
#[cfg(windows)]
fn set_memory_target_low(window: &WebviewWindow, low: bool) -> Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
        ICoreWebView2_19,
    };
    use windows_core::Interface as _;

    let level = if low {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
    } else {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
    };
    // SAFETY: `with_webview` runs the closure on the UI thread that owns the live controller.
    with_controller(window, move |controller| unsafe {
        controller
            .CoreWebView2()?
            .cast::<ICoreWebView2_19>()?
            .SetMemoryUsageTargetLevel(level)
    })
}

#[cfg(not(windows))]
fn set_controller_visible(_window: &WebviewWindow, _visible: bool) -> Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn set_memory_target_low(_window: &WebviewWindow, _low: bool) -> Result<()> {
    Ok(())
}
