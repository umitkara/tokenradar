//! The widget's window: a layered, topmost tool window on its own `widget` thread that sleeps
//! in `GetMessageW` (the ADR-030 pattern).
//!
//! Pixels go out through `UpdateLayeredWindow` with per-pixel alpha; there is no `WM_PAINT`.
//! The window never takes the focus: `WS_EX_NOACTIVATE` and `MA_NOACTIVATE` keep the user's
//! keyboard where it was. Theme, scale, high contrast and monitor changes arrive as broadcasts
//! to the window itself. The only timers are the next visible change (a countdown minute) and,
//! when the user wants it, the full-screen check every five seconds.

#![allow(unsafe_code)]

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};

use tauri::AppHandle;
use tokenradar_core::config::{HotkeyKey, ThemePref, WidgetMode, WidgetPosition, WidgetPrefs};
use tokenradar_core::domain::Timestamp;
use tokenradar_i18n::{Arg, Domain, Locale, Localizer};
use windows_sys::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, ERROR_SUCCESS, GetLastError, HWND, LPARAM, LRESULT, POINT, RECT,
    SIZE, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, COLOR_WINDOW,
    COLOR_WINDOWTEXT, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject,
    EnumDisplayMonitors, GetDC, GetMonitorInfoW, GetSysColor, HDC, HMONITOR,
    MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MONITORINFOEXW,
    MonitorFromPoint, MonitorFromWindow, ReleaseDC, SelectObject,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey, UnregisterHotKey, VK_F1,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CS_DBLCLKS, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, GWL_EXSTYLE, GetCursorPos, GetMessageW, GetWindowLongPtrW,
    GetWindowRect, HMENU, HTCAPTION, HWND_TOPMOST, KillTimer, MA_NOACTIVATE, MF_CHECKED, MF_POPUP,
    MF_SEPARATOR, MF_STRING, MSG, PostMessageW, PostQuitMessage, RegisterClassW,
    SPI_GETHIGHCONTRAST, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    SystemParametersInfoW, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx,
    TranslateMessage, ULW_ALPHA, UpdateLayeredWindow, WM_APP, WM_DESTROY, WM_DISPLAYCHANGE,
    WM_DPICHANGED, WM_EXITSIZEMOVE, WM_HOTKEY, WM_MOUSEACTIVATE, WM_NCHITTEST, WM_NCLBUTTONDBLCLK,
    WM_NCRBUTTONUP, WM_NULL, WM_SETTINGCHANGE, WM_SYSCOLORCHANGE, WM_THEMECHANGED, WM_TIMER,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_POPUP,
};

use super::fullscreen::{Gate, Shell};
use super::model::{WidgetModel, WidgetVisual};
use super::place::{self, Rect};
use super::render::{self, Style, Theme};
use super::text::DirectWrite;
use super::{Init, Msg};
use crate::tray::system::SystemClock;

const CLASS: &str = "TokenRadarWidget";
/// Posted after a message was queued on the channel.
const WM_WAKE: u32 = WM_APP + 1;
const TIMER_CHANGE: usize = 1;
const TIMER_FULLSCREEN: usize = 2;
const FULLSCREEN_EVERY_MS: u32 = 5_000;
const HOTKEY_ID: i32 = 1;

const CMD_HIDE: usize = 1;
const CMD_CLICK_THROUGH: usize = 2;
const CMD_SETTINGS: usize = 3;
const CMD_PANEL: usize = 4;
const CMD_OPACITY: usize = 100;
const OPACITY_STEPS: [u8; 5] = [100, 85, 70, 55, 40];

/// Sends to a running widget thread.
pub struct Handle {
    tx: Sender<Msg>,
    hwnd: Arc<AtomicIsize>,
}

impl Handle {
    /// Queues `msg` and wakes the window. Before the window exists the message waits on the
    /// channel; the thread drains it right after creating the window.
    pub fn send(&self, msg: Msg) {
        if self.tx.send(msg).is_err() {
            return;
        }
        let hwnd = self.hwnd.load(Ordering::Acquire);
        if hwnd != 0 {
            // SAFETY: posting to a window handle that may already be gone fails harmlessly.
            unsafe { PostMessageW(hwnd as HWND, WM_WAKE, 0, 0) };
        }
    }
}

pub fn spawn(app: AppHandle, init: Init) -> std::io::Result<Handle> {
    let (tx, rx) = mpsc::channel();
    let hwnd = Arc::new(AtomicIsize::new(0));
    let shared = Arc::clone(&hwnd);
    std::thread::Builder::new()
        .name("widget".into())
        .spawn(move || run(app, init, rx, &shared))?;
    Ok(Handle { tx, hwnd })
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

thread_local! {
    static WINDOW: RefCell<Option<Window>> = const { RefCell::new(None) };
}

/// The window's state, owned by the `widget` thread.
#[allow(clippy::struct_excessive_bools)] // independent window states
struct Window {
    app: AppHandle,
    hwnd: HWND,
    rx: Receiver<Msg>,
    model: WidgetModel,
    prefs: WidgetPrefs,
    position: Option<WidgetPosition>,
    l10n: Localizer,
    theme_pref: ThemePref,
    persist: bool,
    text: Option<DirectWrite>,
    gate: Gate,
    /// What is on screen: the redraw key.
    shown: Option<(WidgetVisual, Style, (i32, i32))>,
    /// The window's top-left corner and size, physical pixels.
    pos: (i32, i32),
    size: (i32, i32),
    visible: bool,
    hotkey: bool,
    quit: bool,
}

fn run(app: AppHandle, init: Init, rx: Receiver<Msg>, hwnd_out: &AtomicIsize) {
    let class = wide(CLASS);
    // SAFETY (whole block): every pointer is a valid local or null where the API allows it.
    let hwnd = unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSW {
            style: CS_DBLCLKS,
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        if RegisterClassW(&raw const wc) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
            tracing::warn!("widget: window class not registered");
            return;
        }
        let ex = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST;
        CreateWindowExW(
            ex,
            class.as_ptr(),
            wide("TokenRadar").as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        tracing::warn!("widget: window not created");
        return;
    }
    let text = match DirectWrite::new(bcp47(init.locale)) {
        Ok(text) => Some(text),
        Err(error) => {
            tracing::warn!(%error, "widget: DirectWrite unavailable, text is not drawn");
            None
        }
    };
    let window = Window {
        app,
        hwnd,
        rx,
        model: init.model,
        prefs: init.prefs,
        position: init.position,
        l10n: Localizer::new(init.locale, &[Domain::Tray]),
        theme_pref: init.theme,
        persist: init.persist,
        text,
        gate: Gate::default(),
        shown: None,
        pos: (0, 0),
        size: (0, 0),
        visible: false,
        hotkey: false,
        quit: false,
    };
    WINDOW.with(|w| *w.borrow_mut() = Some(window));
    with_window(|w| {
        w.apply_prefs(None);
        w.place_from_memory();
        w.refresh();
    });
    hwnd_out.store(hwnd as isize, Ordering::Release);
    // Messages sent before the handle was published.
    wake();
    tracing::info!("widget: on");

    // SAFETY: the loop dispatches this thread's own window messages until `WM_QUIT`.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&raw mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&raw const msg);
            DispatchMessageW(&raw const msg);
        }
    }
    hwnd_out.store(0, Ordering::Release);
    WINDOW.with(|w| w.borrow_mut().take());
    tracing::info!("widget: off");
}

/// Runs `f` on the window state. Re-entrant calls (a modal loop dispatching while the state
/// is borrowed) are skipped rather than panicking.
fn with_window<R>(f: impl FnOnce(&mut Window) -> R) -> Option<R> {
    WINDOW.with(|w| {
        let mut guard = w.try_borrow_mut().ok()?;
        guard.as_mut().map(f)
    })
}

fn wake() {
    let quit = with_window(|w| {
        w.drain();
        if w.quit {
            return true;
        }
        w.refresh();
        false
    });
    if quit == Some(true) {
        with_window(|w| w.hwnd).map(|hwnd| {
            // SAFETY: our own window, destroyed on its own thread.
            unsafe { DestroyWindow(hwnd) }
        });
    }
}

const fn bcp47(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "en-US",
        Locale::Tr => "tr-TR",
    }
}

impl Window {
    fn drain(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Model(model) => self.model = model,
                Msg::Prefs(prefs) => self.apply_prefs(Some(prefs)),
                Msg::Locale(locale) => {
                    if locale != self.l10n.locale() {
                        self.l10n = Localizer::new(locale, &[Domain::Tray]);
                        if let Some(text) = &mut self.text {
                            text.set_locale(bcp47(locale));
                        }
                    }
                }
                Msg::Theme(theme) => self.theme_pref = theme,
                Msg::Quit => self.quit = true,
            }
        }
    }

    /// Applies new settings (or re-applies the current ones with `None`): click-through,
    /// the shortcut and the full-screen timer.
    fn apply_prefs(&mut self, prefs: Option<WidgetPrefs>) {
        let old = prefs.as_ref().map(|_| self.prefs.clone());
        if let Some(prefs) = prefs {
            self.prefs = prefs;
        }
        // SAFETY: style and timer calls on our own window.
        unsafe {
            let ex = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE);
            let through = WS_EX_TRANSPARENT.cast_signed() as isize;
            let want = if self.prefs.click_through {
                ex | through
            } else {
                ex & !through
            };
            if want != ex {
                SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, want);
            }
            if self.prefs.hide_in_fullscreen {
                SetTimer(self.hwnd, TIMER_FULLSCREEN, FULLSCREEN_EVERY_MS, None);
            } else {
                KillTimer(self.hwnd, TIMER_FULLSCREEN);
            }
        }
        if !self.prefs.hide_in_fullscreen {
            self.poll_fullscreen();
        }
        let hotkey_changed = old.as_ref().is_none_or(|o| o.hotkey != self.prefs.hotkey);
        if hotkey_changed {
            self.register_hotkey();
        }
    }

    fn register_hotkey(&mut self) {
        let hk = self.prefs.hotkey();
        let mut mods = MOD_NOREPEAT;
        for (on, m) in [
            (hk.ctrl, MOD_CONTROL),
            (hk.alt, MOD_ALT),
            (hk.shift, MOD_SHIFT),
            (hk.win, MOD_WIN),
        ] {
            if on {
                mods |= m;
            }
        }
        let vk = match hk.key {
            HotkeyKey::Letter(c) => u32::from(c),
            HotkeyKey::Digit(d) => 0x30 + u32::from(d),
            HotkeyKey::F(n) => u32::from(VK_F1) + u32::from(n) - 1,
        };
        // SAFETY: (un)registering a hotkey on our own window.
        unsafe {
            if self.hotkey {
                UnregisterHotKey(self.hwnd, HOTKEY_ID);
            }
            self.hotkey = RegisterHotKey(self.hwnd, HOTKEY_ID, mods, vk) != 0;
        }
        if !self.hotkey {
            tracing::warn!(hotkey = %hk, "widget: shortcut is taken by another app");
        }
    }

    fn poll_fullscreen(&mut self) {
        if self
            .gate
            .poll(&Shell, self.prefs.hide_in_fullscreen)
            .is_some()
        {
            self.update_visibility();
        }
    }

    fn update_visibility(&mut self) {
        let want = !self.gate.hidden() && self.shown.is_some();
        if want == self.visible {
            return;
        }
        // SAFETY: showing or hiding our own window without activating it.
        unsafe {
            if want {
                ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            } else {
                ShowWindow(self.hwnd, SW_HIDE);
            }
        }
        self.visible = want;
    }

    fn style(&self) -> Style {
        // SAFETY: a query on our own window.
        let dpi = unsafe { GetDpiForWindow(self.hwnd) }.max(96);
        Style {
            theme: theme(self.theme_pref, self.prefs.mode),
            scale_pct: dpi * 100 / 96,
            opacity: self.prefs.opacity,
        }
    }

    /// Recomputes the frame and draws it if anything visible changed; schedules the next
    /// change.
    fn refresh(&mut self) {
        let now = Timestamp::now();
        let visual = self
            .model
            .visual(&self.l10n, &SystemClock, now, self.prefs.pace_mark);
        let style = self.style();
        let (w, h) = render::size(visual.rows.len(), style);
        let size = (
            i32::try_from(w).unwrap_or(i32::MAX),
            i32::try_from(h).unwrap_or(i32::MAX),
        );
        if size != self.size {
            // One row more or less: the remembered place decides where it grows.
            self.size = size;
            self.place_from_memory();
        }
        let key = (visual, style, self.pos);
        if self.shown.as_ref() != Some(&key) {
            self.draw(&key.0, style);
            self.shown = Some(key);
        }
        self.update_visibility();
        self.schedule(now);
    }

    fn schedule(&self, now: Timestamp) {
        // SAFETY: timer calls on our own window.
        unsafe {
            match self.model.next_change(now, self.prefs.pace_mark) {
                Some(at) => {
                    let ms = (at.as_millis() - now.as_millis()).clamp(1, i64::from(u32::MAX));
                    SetTimer(
                        self.hwnd,
                        TIMER_CHANGE,
                        u32::try_from(ms).unwrap_or(u32::MAX),
                        None,
                    );
                }
                None => {
                    KillTimer(self.hwnd, TIMER_CHANGE);
                }
            }
        }
    }

    fn draw(&mut self, visual: &WidgetVisual, style: Style) {
        let pixmap = match &mut self.text {
            Some(text) => render::render(visual, style, text),
            None => render::render(visual, style, &mut NoText),
        };
        let (w, h) = (pixmap.width(), pixmap.height());
        // SAFETY: the DIB section is sized for the pixmap; every GDI object is released.
        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            let mem = CreateCompatibleDC(screen);
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).unwrap_or(40),
                biWidth: i32::try_from(w).unwrap_or(1),
                // Negative: top-down rows, as the pixmap.
                biHeight: -i32::try_from(h).unwrap_or(1),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let dib = CreateDIBSection(
                mem,
                &raw const info,
                DIB_RGB_COLORS,
                &raw mut bits,
                std::ptr::null_mut(),
                0,
            );
            if !dib.is_null() && !bits.is_null() {
                let out = std::slice::from_raw_parts_mut(bits.cast::<u8>(), (w * h * 4) as usize);
                render::to_bgra(&pixmap, out);
                let old = SelectObject(mem, dib);
                let dst = POINT {
                    x: self.pos.0,
                    y: self.pos.1,
                };
                let size = SIZE {
                    cx: i32::try_from(w).unwrap_or(1),
                    cy: i32::try_from(h).unwrap_or(1),
                };
                let src = POINT { x: 0, y: 0 };
                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER.try_into().unwrap_or_default(),
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA.try_into().unwrap_or_default(),
                };
                if UpdateLayeredWindow(
                    self.hwnd,
                    screen,
                    &raw const dst,
                    &raw const size,
                    mem,
                    &raw const src,
                    0,
                    &raw const blend,
                    ULW_ALPHA,
                ) == 0
                {
                    tracing::debug!("widget: UpdateLayeredWindow failed");
                }
                SelectObject(mem, old);
                DeleteObject(dib);
            }
            DeleteDC(mem);
            ReleaseDC(std::ptr::null_mut(), screen);
        }
    }

    /// Puts the window where the user left it: the remembered monitor (or the primary one)
    /// and the remembered share of its free space (or the bottom-right corner).
    fn place_from_memory(&mut self) {
        let screen = self
            .position
            .as_ref()
            .and_then(|p| monitor_named(&p.monitor))
            .or_else(primary);
        let Some(screen) = screen else { return };
        let size = if self.size == (0, 0) {
            // Before the first frame: the size this monitor's scale gives.
            let style = Style {
                scale_pct: screen.dpi * 100 / 96,
                ..self.style()
            };
            let (w, h) = render::size(1 + usize::from(self.model.five_hour.is_some()), style);
            (i32::try_from(w).unwrap_or(1), i32::try_from(h).unwrap_or(1))
        } else {
            self.size
        };
        let scale = f64::from(screen.dpi) / 96.0;
        self.pos = match &self.position {
            Some(p) if screen.name == p.monitor => {
                place::from_relative((p.x, p.y), size, screen.work, scale)
            }
            _ => place::default_position(size, screen.work, scale),
        };
    }

    /// After a drag: snap to the nearest edge or corner and remember the place.
    fn dropped(&mut self) {
        let mut r: RECT = unsafe { std::mem::zeroed() };
        // SAFETY: a writable local rect for our own window.
        if unsafe { GetWindowRect(self.hwnd, &raw mut r) } == 0 {
            return;
        }
        let Some(screen) = monitor_of(self.hwnd) else {
            return;
        };
        let scale = f64::from(screen.dpi) / 96.0;
        let win = Rect::new(r.left, r.top, r.right, r.bottom);
        let (x, y) = place::snap(win, screen.work, scale);
        self.pos = (x, y);
        // SAFETY: moving our own window without activating it.
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        let placed = Rect::new(x, y, x + (r.right - r.left), y + (r.bottom - r.top));
        let (rx, ry) = place::relative(placed, screen.work, scale);
        let position = WidgetPosition {
            monitor: screen.name,
            x: rx,
            y: ry,
        };
        self.position = Some(position.clone());
        if let Some(shown) = &mut self.shown {
            shown.2 = self.pos;
        }
        super::remember_position(&self.app, position.clone());
        if self.persist {
            // Off this thread: the window should not wait on the disk.
            let _ = std::thread::Builder::new()
                .name("widget-save".into())
                .spawn(move || crate::prefs::save_widget_position(&position));
        }
    }

    /// The user changed a setting from the widget itself (menu or shortcut).
    fn change(&mut self, f: impl FnOnce(&mut WidgetPrefs)) {
        let mut prefs = self.prefs.clone();
        f(&mut prefs);
        if self.persist {
            // Saves, tells every window and the tray, and sends the prefs back here.
            let app = self.app.clone();
            let _ = std::thread::Builder::new()
                .name("widget-save".into())
                .spawn(move || {
                    if let Err(error) = crate::prefs::set_widget(&app, prefs) {
                        tracing::warn!(%error, "widget: setting not saved");
                    }
                });
        } else if prefs.enabled {
            self.apply_prefs(Some(prefs));
            self.refresh();
        } else {
            self.quit = true;
        }
    }
}

/// Without DirectWrite: shapes only.
struct NoText;

impl render::Text for NoText {
    fn width(&mut self, _text: &str, _font: render::Font) -> f32 {
        0.0
    }

    fn draw(
        &mut self,
        _pixmap: &mut tiny_skia::Pixmap,
        _text: &str,
        _font: render::Font,
        _x: f32,
        _baseline: f32,
        _color: tiny_skia::Color,
    ) {
    }
}

/// A monitor as placement needs it.
struct Screen {
    name: String,
    work: Rect,
    dpi: u32,
}

fn screen_of(monitor: HMONITOR) -> Option<Screen> {
    if monitor.is_null() {
        return None;
    }
    // SAFETY: `info` is a correctly sized MONITORINFOEXW; DPI out parameters are locals.
    unsafe {
        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = u32::try_from(size_of::<MONITORINFOEXW>()).ok()?;
        if GetMonitorInfoW(monitor, (&raw mut info).cast::<MONITORINFO>()) == 0 {
            return None;
        }
        let len = info
            .szDevice
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(info.szDevice.len());
        let (mut dx, mut dy) = (96, 96);
        if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &raw mut dx, &raw mut dy) != 0 {
            dx = 96;
        }
        let w = info.monitorInfo.rcWork;
        Some(Screen {
            name: String::from_utf16_lossy(&info.szDevice[..len]),
            work: Rect::new(w.left, w.top, w.right, w.bottom),
            dpi: dx,
        })
    }
}

fn primary() -> Option<Screen> {
    // SAFETY: a plain query.
    screen_of(unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) })
}

fn monitor_of(hwnd: HWND) -> Option<Screen> {
    // SAFETY: a plain query on our own window.
    screen_of(unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) })
}

/// The connected monitor with device name `name`, if any.
fn monitor_named(name: &str) -> Option<Screen> {
    unsafe extern "system" fn each(m: HMONITOR, _dc: HDC, _r: *mut RECT, data: LPARAM) -> i32 {
        // SAFETY: `data` is the `Vec` passed below, alive for the whole enumeration.
        let list = unsafe { &mut *(data as *mut Vec<HMONITOR>) };
        list.push(m);
        1
    }
    let mut monitors: Vec<HMONITOR> = Vec::new();
    // SAFETY: the callback only pushes into `monitors`, which outlives the call.
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(each),
            (&raw mut monitors) as LPARAM,
        );
    }
    monitors
        .into_iter()
        .filter_map(screen_of)
        .find(|s| s.name == name)
}

/// The widget's colors: high contrast first, else what it stands on ([`super::is_light`]).
fn theme(pref: ThemePref, mode: WidgetMode) -> Theme {
    if let Some((ink, back)) = high_contrast() {
        return Theme::HighContrast { ink, back };
    }
    let light = super::is_light(
        pref,
        mode,
        || light_flag("AppsUseLightTheme", true),
        || light_flag("SystemUsesLightTheme", false),
    );
    if light { Theme::Light } else { Theme::Dark }
}

fn high_contrast() -> Option<([u8; 3], [u8; 3])> {
    let size = u32::try_from(size_of::<HIGHCONTRASTW>()).ok()?;
    // SAFETY: a correctly sized, writable HIGHCONTRASTW with `cbSize` set.
    let on = unsafe {
        let mut hc: HIGHCONTRASTW = std::mem::zeroed();
        hc.cbSize = size;
        SystemParametersInfoW(SPI_GETHIGHCONTRAST, size, (&raw mut hc).cast(), 0) != 0
            && hc.dwFlags & HCF_HIGHCONTRASTON != 0
    };
    if !on {
        return None;
    }
    // COLORREF is 0x00BBGGRR.
    let rgb = |c: u32| -> [u8; 3] {
        let b = c.to_le_bytes();
        [b[0], b[1], b[2]]
    };
    // SAFETY: no pointers involved.
    unsafe {
        Some((
            rgb(GetSysColor(COLOR_WINDOWTEXT)),
            rgb(GetSysColor(COLOR_WINDOW)),
        ))
    }
}

/// A light-theme flag under `Themes\Personalize`: `AppsUseLightTheme` (apps, missing means
/// light) or `SystemUsesLightTheme` (taskbar and Start, missing means dark).
fn light_flag(name: &str, missing: bool) -> bool {
    let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let name = wide(name);
    let mut value: u32 = u32::from(missing);
    let mut size = u32::try_from(size_of::<u32>()).unwrap_or(4);
    // SAFETY: `value` and `size` are valid for writes; RRF_RT_REG_DWORD limits the data.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&raw mut value).cast(),
            &raw mut size,
        )
    };
    if status == ERROR_SUCCESS {
        value != 0
    } else {
        missing
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_WAKE => {
            wake();
            return 0;
        }
        WM_TIMER => {
            match wp {
                TIMER_CHANGE => with_window(Window::refresh),
                TIMER_FULLSCREEN => with_window(Window::poll_fullscreen),
                _ => None,
            };
            return 0;
        }
        // The whole strip is a handle: dragging it moves the window.
        WM_NCHITTEST => return HTCAPTION.cast_signed() as LRESULT,
        WM_MOUSEACTIVATE => return MA_NOACTIVATE.cast_signed() as LRESULT,
        WM_NCLBUTTONDBLCLK => {
            if let Some(app) = with_window(|w| w.app.clone()) {
                open_panel(&app);
            }
            return 0;
        }
        WM_NCRBUTTONUP => {
            context_menu(hwnd);
            return 0;
        }
        WM_EXITSIZEMOVE => {
            with_window(Window::dropped);
        }
        WM_DPICHANGED => {
            // The suggested rectangle keeps the window under the cursor while dragging.
            // SAFETY: for this message `lp` points to a RECT owned by the system.
            let suggested = unsafe { *(lp as *const RECT) };
            with_window(|w| {
                w.pos = (suggested.left, suggested.top);
                w.size = (0, 0);
                w.shown = None;
                w.refresh();
            });
            return 0;
        }
        WM_DISPLAYCHANGE => {
            with_window(|w| {
                w.place_from_memory();
                w.refresh();
            });
        }
        WM_SETTINGCHANGE | WM_THEMECHANGED | WM_SYSCOLORCHANGE => {
            // Theme, high contrast or scale: re-read; an unchanged look draws nothing.
            with_window(Window::refresh);
        }
        WM_HOTKEY if i32::try_from(wp).ok() == Some(HOTKEY_ID) => {
            with_window(|w| w.change(|p| p.click_through = !p.click_through));
            wake();
            return 0;
        }
        WM_DESTROY => {
            with_window(|w| {
                // SAFETY: our own window's timers and hotkey.
                unsafe {
                    KillTimer(w.hwnd, TIMER_CHANGE);
                    KillTimer(w.hwnd, TIMER_FULLSCREEN);
                    if w.hotkey {
                        UnregisterHotKey(w.hwnd, HOTKEY_ID);
                    }
                }
            });
            // SAFETY: ends this thread's message loop.
            unsafe { PostQuitMessage(0) };
            return 0;
        }
        _ => {}
    }
    // SAFETY: forwarding the unchanged arguments of this window's own message.
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

fn open_panel(app: &AppHandle) {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || crate::panel::report(crate::panel::show(&app2)));
}

/// The right-click menu: open the panel, click-through, opacity, settings and hide. The state
/// is not borrowed while the menu's modal loop runs.
fn context_menu(hwnd: HWND) {
    let Some((l10n_locale, prefs, app)) =
        with_window(|w| (w.l10n.locale(), w.prefs.clone(), w.app.clone()))
    else {
        return;
    };
    let l10n = Localizer::new(l10n_locale, &[Domain::Tray]);
    let label = |key: &str| wide(&l10n.msg(key));
    // SAFETY (whole block): menus are created, shown and destroyed here; strings outlive the
    // `AppendMenuW` calls, which copy them.
    let cmd = unsafe {
        let menu: HMENU = CreatePopupMenu();
        let opacity: HMENU = CreatePopupMenu();
        for step in OPACITY_STEPS {
            let text = wide(&l10n.fmt(
                "widget-menu-opacity-option",
                &[("pct", Arg::from(i64::from(step)))],
            ));
            let flags = MF_STRING | if prefs.opacity == step { MF_CHECKED } else { 0 };
            AppendMenuW(
                opacity,
                flags,
                CMD_OPACITY + usize::from(step),
                text.as_ptr(),
            );
        }
        let through = wide(&l10n.fmt(
            "widget-menu-click-through",
            &[("hotkey", Arg::Str(&prefs.hotkey))],
        ));
        AppendMenuW(
            menu,
            MF_STRING,
            CMD_PANEL,
            label("widget-menu-open-panel").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            menu,
            MF_STRING | if prefs.click_through { MF_CHECKED } else { 0 },
            CMD_CLICK_THROUGH,
            through.as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_POPUP,
            opacity as usize,
            label("widget-menu-opacity").as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING,
            CMD_SETTINGS,
            label("widget-menu-settings").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            menu,
            MF_STRING,
            CMD_HIDE,
            label("widget-menu-hide").as_ptr(),
        );
        let mut at = POINT { x: 0, y: 0 };
        GetCursorPos(&raw mut at);
        // A popup menu only closes on an outside click when its owner is in the foreground:
        // the one moment the widget takes the focus, as every context menu does.
        SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenuEx(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
            at.x,
            at.y,
            hwnd,
            std::ptr::null(),
        );
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);
        usize::try_from(cmd).unwrap_or(0)
    };
    match cmd {
        0 => {}
        CMD_PANEL => open_panel(&app),
        CMD_SETTINGS => {
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                crate::panel::report(crate::settings::show(
                    &app2,
                    Some(crate::settings::SettingsSection::Widget),
                ));
            });
        }
        CMD_CLICK_THROUGH => {
            with_window(|w| w.change(|p| p.click_through = !p.click_through));
        }
        CMD_HIDE => {
            with_window(|w| w.change(|p| p.enabled = false));
        }
        c if c > CMD_OPACITY => {
            let step = u8::try_from(c - CMD_OPACITY).unwrap_or(100);
            with_window(|w| w.change(|p| p.opacity = step));
        }
        _ => {}
    }
    wake();
}
