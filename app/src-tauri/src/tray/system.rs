//! What the operating system tells the tray: icon size, taskbar theme, high contrast, local
//! time, and when any of these change.
//!
//! Changes arrive on one hidden top-level window that sleeps in `GetMessageW` (ADR-030):
//! `WM_SETTINGCHANGE` covers the taskbar theme (`ImmersiveColorSet`), high contrast and the
//! display scale; `WM_DISPLAYCHANGE` and `WM_DPICHANGED` cover monitor changes. Nothing polls.

#![cfg_attr(windows, allow(unsafe_code))]

use tokenradar_core::domain::Timestamp;
use tokenradar_i18n::{HourCycle, Weekday};

use super::render::Theme;

/// Everything about the system that changes the icon's pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Environment {
    /// Icon edge in physical pixels: 16 at 100 %, 20 at 125 %, 24 at 150 %, 32 at 200 %.
    pub px: u32,
    pub theme: Theme,
}

/// A wall-clock reading in the system's time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    /// Year, month, day: only compared, to tell "today" from other days.
    pub date: (u16, u16, u16),
    pub weekday: Weekday,
    pub hour: u8,
    pub minute: u8,
}

/// Local time conversion, swappable in tests.
pub trait LocalClock {
    fn local(&self, t: Timestamp) -> Option<LocalTime>;
    /// The user's 12/24-hour preference (English only; Turkish is always 24 hours).
    fn hour_cycle(&self) -> HourCycle;
}

/// The operating system's clock and regional settings.
pub struct SystemClock;

#[cfg(windows)]
mod imp {
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicU32, Ordering};

    use windows_sys::Win32::Foundation::{
        ERROR_SUCCESS, FILETIME, HWND, LPARAM, LRESULT, SYSTEMTIME, WPARAM,
    };
    use windows_sys::Win32::Globalization::{GetLocaleInfoEx, LOCALE_STIMEFORMAT};
    use windows_sys::Win32::Graphics::Gdi::{COLOR_WINDOWTEXT, GetSysColor};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows_sys::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
    use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
    use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow, GetSystemMetricsForDpi};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetMessageW, MSG,
        RegisterClassW, RegisterWindowMessageW, SM_CXSMICON, SPI_GETHIGHCONTRAST,
        SystemParametersInfoW, TranslateMessage, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_SETTINGCHANGE,
        WM_SYSCOLORCHANGE, WM_THEMECHANGED, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
    };

    use super::{Environment, HourCycle, LocalClock, LocalTime, SystemClock, Theme, Timestamp};
    use tokenradar_i18n::Weekday;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn environment() -> Environment {
        Environment {
            px: icon_px(),
            theme: theme(),
        }
    }

    /// The small-icon metric at the taskbar's DPI. The taskbar's window gives the DPI of the
    /// monitor the notification area is on; the system DPI is the fallback.
    fn icon_px() -> u32 {
        let class = wide("Shell_TrayWnd");
        // SAFETY: plain Win32 queries with a valid, NUL-terminated class name; a null window
        // is checked before use.
        let px = unsafe {
            let taskbar = FindWindowW(class.as_ptr(), std::ptr::null());
            let dpi = if taskbar.is_null() {
                0
            } else {
                GetDpiForWindow(taskbar)
            };
            let dpi = if dpi == 0 { GetDpiForSystem() } else { dpi };
            GetSystemMetricsForDpi(SM_CXSMICON, dpi)
        };
        u32::try_from(px).unwrap_or(16).clamp(16, 64)
    }

    fn theme() -> Theme {
        if let Some(ink) = high_contrast_ink() {
            return Theme::HighContrast { ink };
        }
        if taskbar_uses_light_theme() {
            Theme::Light
        } else {
            Theme::Dark
        }
    }

    fn high_contrast_ink() -> Option<[u8; 3]> {
        #[allow(clippy::cast_possible_truncation)]
        let size = size_of::<HIGHCONTRASTW>() as u32;
        // SAFETY: `hc` is a correctly sized, writable HIGHCONTRASTW with `cbSize` set.
        let on = unsafe {
            let mut hc: HIGHCONTRASTW = std::mem::zeroed();
            hc.cbSize = size;
            SystemParametersInfoW(SPI_GETHIGHCONTRAST, size, (&raw mut hc).cast::<c_void>(), 0) != 0
                && hc.dwFlags & HCF_HIGHCONTRASTON != 0
        };
        if !on {
            return None;
        }
        // SAFETY: no pointers involved.
        let c = unsafe { GetSysColor(COLOR_WINDOWTEXT) };
        // COLORREF is 0x00BBGGRR.
        Some(c.to_le_bytes()[..3].try_into().unwrap_or([255, 255, 255]))
    }

    /// `SystemUsesLightTheme` is the taskbar's theme; missing means dark (the default).
    fn taskbar_uses_light_theme() -> bool {
        let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
        let name = wide("SystemUsesLightTheme");
        let mut value: u32 = 0;
        #[allow(clippy::cast_possible_truncation)]
        let mut size = size_of::<u32>() as u32;
        // SAFETY: `value` and `size` are valid for writes; RRF_RT_REG_DWORD limits the data to
        // four bytes.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&raw mut value).cast::<c_void>(),
                &raw mut size,
            )
        };
        status == ERROR_SUCCESS && value != 0
    }

    impl LocalClock for SystemClock {
        fn local(&self, t: Timestamp) -> Option<LocalTime> {
            const UNIX_EPOCH_FILETIME: i64 = 116_444_736_000_000_000;
            let ticks = t
                .as_millis()
                .checked_mul(10_000)?
                .checked_add(UNIX_EPOCH_FILETIME)?;
            let ticks = u64::try_from(ticks).ok()?;
            let ft = FILETIME {
                // Splitting a u64 into its 32-bit halves.
                dwLowDateTime: (ticks & 0xFFFF_FFFF) as u32,
                dwHighDateTime: (ticks >> 32) as u32,
            };
            // SAFETY: both calls only read and write the local structs passed by pointer; a
            // null time zone means the current one.
            let local = unsafe {
                let mut utc: SYSTEMTIME = std::mem::zeroed();
                let mut local: SYSTEMTIME = std::mem::zeroed();
                if FileTimeToSystemTime(&raw const ft, &raw mut utc) == 0
                    || SystemTimeToTzSpecificLocalTime(
                        std::ptr::null(),
                        &raw const utc,
                        &raw mut local,
                    ) == 0
                {
                    return None;
                }
                local
            };
            Some(LocalTime {
                date: (local.wYear, local.wMonth, local.wDay),
                weekday: Weekday::from_sunday0(local.wDayOfWeek),
                hour: u8::try_from(local.wHour).ok()?,
                minute: u8::try_from(local.wMinute).ok()?,
            })
        }

        /// The user's short time format: `HH:mm` is 24 hours, `h:mm tt` is 12.
        fn hour_cycle(&self) -> HourCycle {
            let mut buf = [0u16; 80];
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            // SAFETY: `buf` is writable for its full length; a null name is the user default.
            let n = unsafe {
                GetLocaleInfoEx(
                    std::ptr::null(),
                    LOCALE_STIMEFORMAT,
                    buf.as_mut_ptr(),
                    buf.len() as i32,
                )
            };
            let format = String::from_utf16_lossy(&buf[..usize::try_from(n).unwrap_or(0)]);
            if format.contains('H') || !format.contains('h') {
                HourCycle::H23
            } else {
                HourCycle::H12
            }
        }
    }

    thread_local! {
        static ON_CHANGE: RefCell<Option<Box<dyn Fn()>>> = RefCell::new(None);
    }
    static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

    /// Starts the `tray-env` thread. It lives as long as the process.
    pub fn watch(on_change: Box<dyn Fn() + Send>) -> std::io::Result<()> {
        std::thread::Builder::new()
            .name("tray-env".into())
            .spawn(move || {
                ON_CHANGE.with(|f| *f.borrow_mut() = Some(on_change));
                // SAFETY: the window and its class belong to this thread, which runs their
                // message loop until the process ends.
                unsafe { run_window() };
            })
            .map(drop)
    }

    unsafe fn run_window() {
        let class = wide("TokenRadarTrayEnv");
        // SAFETY (whole function): every pointer passed is a valid local or null where the API
        // allows it; the class name outlives the window because the loop never returns early.
        unsafe {
            TASKBAR_CREATED.store(
                RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
                Ordering::Relaxed,
            );
            let instance = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: instance,
                lpszClassName: class.as_ptr(),
                ..std::mem::zeroed()
            };
            if RegisterClassW(&raw const wc) == 0 {
                eprintln!("TokenRadar: tray environment watcher unavailable (class)");
                return;
            }
            // A hidden top-level window: broadcasts reach it, message-only windows miss them.
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class.as_ptr(),
                class.as_ptr(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if hwnd.is_null() {
                eprintln!("TokenRadar: tray environment watcher unavailable (window)");
                return;
            }
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&raw mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&raw const msg);
                DispatchMessageW(&raw const msg);
            }
        }
    }

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        let relevant = matches!(
            msg,
            WM_SETTINGCHANGE
                | WM_DISPLAYCHANGE
                | WM_DPICHANGED
                | WM_THEMECHANGED
                | WM_SYSCOLORCHANGE
        ) || (msg != 0 && msg == TASKBAR_CREATED.load(Ordering::Relaxed));
        if relevant {
            ON_CHANGE.with(|f| {
                if let Some(f) = f.borrow().as_ref() {
                    f();
                }
            });
        }
        // SAFETY: forwarding the unchanged arguments of this window's own message.
        unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
    }

    /// GDI and USER handles of this process (the step 08 HICON leak check).
    #[cfg(any(debug_assertions, feature = "spike"))]
    pub fn gui_objects() -> (u32, u32) {
        use windows_sys::Win32::System::Threading::{
            GR_GDIOBJECTS, GR_USEROBJECTS, GetCurrentProcess, GetGuiResources,
        };
        // SAFETY: the pseudo handle of the current process is always valid.
        unsafe {
            let p = GetCurrentProcess();
            (
                GetGuiResources(p, GR_GDIOBJECTS),
                GetGuiResources(p, GR_USEROBJECTS),
            )
        }
    }
}

/// Other platforms (Step 21): a fixed 16 px dark icon, no change notifications, UTC time.
#[cfg(not(windows))]
mod imp {
    use super::{Environment, HourCycle, LocalClock, LocalTime, SystemClock, Theme, Timestamp};
    use tokenradar_i18n::Weekday;

    pub fn environment() -> Environment {
        Environment {
            px: 16,
            theme: Theme::Dark,
        }
    }

    pub fn watch(_on_change: Box<dyn Fn() + Send>) -> std::io::Result<()> {
        Ok(())
    }

    impl LocalClock for SystemClock {
        fn local(&self, t: Timestamp) -> Option<LocalTime> {
            let secs = t.as_millis().div_euclid(1000);
            let days = secs.div_euclid(86_400);
            let in_day = secs.rem_euclid(86_400);
            Some(LocalTime {
                date: (0, 0, u16::try_from(days.rem_euclid(65_536)).ok()?),
                // 1970-01-01 was a Thursday.
                weekday: Weekday::from_sunday0(u16::try_from((days + 4).rem_euclid(7)).ok()?),
                hour: u8::try_from(in_day / 3600).ok()?,
                minute: u8::try_from(in_day / 60 % 60).ok()?,
            })
        }

        fn hour_cycle(&self) -> HourCycle {
            HourCycle::H23
        }
    }

    #[cfg(any(debug_assertions, feature = "spike"))]
    pub fn gui_objects() -> (u32, u32) {
        (0, 0)
    }
}

#[cfg(any(debug_assertions, feature = "spike"))]
pub use imp::gui_objects;
pub use imp::{environment, watch};
