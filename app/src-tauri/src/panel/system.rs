//! What Windows tells the panel: the icon's monitor, the taskbar, rounded corners and the
//! WebView2 memory level of a hidden panel.

#![cfg_attr(windows, allow(unsafe_code))]

#[cfg(not(windows))]
use tauri::WebviewWindow;

#[cfg(not(windows))]
use super::place::{Screen, Taskbar};

#[cfg(windows)]
mod imp {
    use tauri::WebviewWindow;
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
    };
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    };
    use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows_sys::Win32::UI::Shell::{
        ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETTASKBARPOS, APPBARDATA, SHAppBarMessage,
    };

    use super::super::place::{Edge, Rect, Screen, Taskbar};

    const fn rect(r: RECT) -> Rect {
        Rect::new(r.left, r.top, r.right, r.bottom)
    }

    pub fn screen_at(x: i32, y: i32) -> Option<Screen> {
        // SAFETY: plain calls with a stack struct whose `cbSize` is set as required.
        unsafe {
            let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
            if monitor.is_null() {
                return None;
            }
            let mut info = MONITORINFO {
                cbSize: u32::try_from(size_of::<MONITORINFO>()).ok()?,
                ..std::mem::zeroed()
            };
            if GetMonitorInfoW(monitor, &raw mut info) == 0 {
                return None;
            }
            let (mut dx, mut dy) = (96, 96);
            let scale =
                if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &raw mut dx, &raw mut dy) == 0 {
                    f64::from(dx) / 96.0
                } else {
                    1.0
                };
            Some(Screen {
                monitor: rect(info.rcMonitor),
                work: rect(info.rcWork),
                scale,
            })
        }
    }

    pub fn taskbar() -> Option<Taskbar> {
        // SAFETY: `ABM_GETTASKBARPOS` only fills the struct it is given.
        unsafe {
            let mut data = APPBARDATA {
                cbSize: u32::try_from(size_of::<APPBARDATA>()).ok()?,
                ..std::mem::zeroed()
            };
            if SHAppBarMessage(ABM_GETTASKBARPOS, &raw mut data) == 0 {
                return None;
            }
            let edge = match data.uEdge {
                ABE_LEFT => Edge::Left,
                ABE_TOP => Edge::Top,
                ABE_RIGHT => Edge::Right,
                ABE_BOTTOM => Edge::Bottom,
                _ => return None,
            };
            Some(Taskbar {
                edge,
                rect: rect(data.rc),
            })
        }
    }

    /// Windows 11 rounds framed windows only; a frameless panel asks for it. Windows 10 ignores it.
    pub fn round_corners(window: &WebviewWindow) {
        let Ok(hwnd) = window.hwnd() else { return };
        let preference = DWMWCP_ROUND;
        // SAFETY: a valid window handle and a 4-byte attribute value that outlives the call.
        unsafe {
            DwmSetWindowAttribute(
                hwnd.0 as HWND,
                u32::try_from(DWMWA_WINDOW_CORNER_PREFERENCE).unwrap_or_default(),
                (&raw const preference).cast(),
                u32::try_from(size_of_val(&preference)).unwrap_or_default(),
            );
        }
    }

    /// `ICoreWebView2_19::SetMemoryUsageTargetLevel`. Low trims about 14 MB of the hidden
    /// panel's private working set and costs ~11 ms on the next show (Step 02, ADR-023).
    /// Fire and forget: the call runs on the UI thread, which may be the caller's.
    pub fn set_memory_target_low(window: &WebviewWindow, low: bool) {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL, ICoreWebView2_19,
        };
        use windows_core::Interface as _;

        let level = if low {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
        } else {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
        };
        let _ = window.with_webview(move |webview| {
            // SAFETY: `with_webview` runs this on the UI thread that owns the live controller.
            let result = unsafe {
                webview
                    .controller()
                    .CoreWebView2()
                    .and_then(|core| core.cast::<ICoreWebView2_19>())
                    .and_then(|core| core.SetMemoryUsageTargetLevel(level))
            };
            if let Err(error) = result {
                tracing::warn!(%error, "panel: memory target level");
            }
        });
    }
}

#[cfg(windows)]
pub use imp::{round_corners, screen_at, set_memory_target_low, taskbar};

#[cfg(not(windows))]
pub fn screen_at(_x: i32, _y: i32) -> Option<Screen> {
    None
}

#[cfg(not(windows))]
pub fn taskbar() -> Option<Taskbar> {
    None
}

#[cfg(not(windows))]
pub fn round_corners(_window: &WebviewWindow) {}

#[cfg(not(windows))]
pub fn set_memory_target_low(_window: &WebviewWindow, _low: bool) {}
