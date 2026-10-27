//! Versions of the operating system and WebView2, read from the registry.

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SystemInfo {
    pub os: &'static str,
    pub os_build: Option<String>,
    /// e.g. `24H2`.
    pub os_display_version: Option<String>,
    /// Update build revision (`UBR`).
    pub os_revision: Option<u32>,
    /// The Evergreen WebView2 runtime; `None` when it is not installed.
    pub webview2: Option<String>,
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod imp {
    use std::ffi::c_void;

    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
    };

    use super::SystemInfo;

    const NT: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    /// The Evergreen WebView2 runtime's client id.
    const WEBVIEW2: &str = r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

    pub fn info() -> SystemInfo {
        let webview2 = [
            (
                HKEY_LOCAL_MACHINE,
                format!(r"SOFTWARE\WOW6432Node\{WEBVIEW2}"),
            ),
            (HKEY_LOCAL_MACHINE, format!(r"SOFTWARE\{WEBVIEW2}")),
            (HKEY_CURRENT_USER, format!(r"Software\{WEBVIEW2}")),
        ]
        .into_iter()
        .find_map(|(root, key)| string(root, &key, "pv"))
        .filter(|v| v != "0.0.0.0");
        SystemInfo {
            os: "windows",
            os_build: string(HKEY_LOCAL_MACHINE, NT, "CurrentBuildNumber"),
            os_display_version: string(HKEY_LOCAL_MACHINE, NT, "DisplayVersion"),
            os_revision: dword(HKEY_LOCAL_MACHINE, NT, "UBR"),
            webview2,
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn string(root: HKEY, key: &str, name: &str) -> Option<String> {
        let (key, name) = (wide(key), wide(name));
        let mut buf = [0u16; 128];
        #[allow(clippy::cast_possible_truncation)]
        let mut size = size_of_val(&buf) as u32;
        // SAFETY: `buf` and `size` are valid for writes and `size` is the buffer's byte length;
        // RRF_RT_REG_SZ makes the call fail instead of writing anything but a string.
        let status = unsafe {
            RegGetValueW(
                root,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buf.as_mut_ptr().cast::<c_void>(),
                &raw mut size,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]).trim().to_owned())
    }

    fn dword(root: HKEY, key: &str, name: &str) -> Option<u32> {
        let (key, name) = (wide(key), wide(name));
        let mut v = 0u32;
        #[allow(clippy::cast_possible_truncation)]
        let mut size = size_of::<u32>() as u32;
        // SAFETY: `v` is a valid 4-byte buffer and `size` says so; RRF_RT_REG_DWORD rejects
        // any other value type.
        let status = unsafe {
            RegGetValueW(
                root,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&raw mut v).cast::<c_void>(),
                &raw mut size,
            )
        };
        (status == ERROR_SUCCESS).then_some(v)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::SystemInfo;

    pub fn info() -> SystemInfo {
        SystemInfo {
            os: std::env::consts::OS,
            ..SystemInfo::default()
        }
    }
}

pub use imp::info;
