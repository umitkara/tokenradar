//! "Always show the icon" (Step 11c, ADR-035).
//!
//! Windows 11 keeps one key per notification icon under
//! `HKCU\Control Panel\NotifyIconSettings\<id>`; `IsPromoted = 1` moves the icon out of the
//! overflow flyout onto the taskbar, which is what the taskbar settings page writes. The format
//! is not documented, so the helper only writes when the user clicks the button, finds our
//! keys by `ExecutablePath`, and reads the value back. Anything else falls back to opening the
//! taskbar settings with written instructions.
//!
//! `ExecutablePath` is either a plain path or a known-folder GUID followed by the rest of the
//! path (`{6D809377-…}\Vendor\app.exe` for Program Files); both are compared to our exe.

use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt as _;

/// The taskbar settings page; its "Other system tray icons" list has a switch per icon.
const TASKBAR_SETTINGS: &str = "ms-settings:taskbar";

/// Where the tray icon is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TrayPinState {
    /// Always on the taskbar.
    Shown,
    /// In the overflow flyout (the default for a new icon).
    Hidden,
    /// Windows has no entry for this exe yet, or it could not be read.
    Unknown,
    /// Not Windows 11: only the settings page and the instructions are offered.
    Unsupported,
}

/// What the icon's entries say right now.
pub fn state() -> TrayPinState {
    let Some(exe) = current_exe() else {
        return TrayPinState::Unknown;
    };
    sys::state(&exe)
}

/// Promotes every entry of this exe, then reads them back.
pub fn pin() -> TrayPinState {
    let Some(exe) = current_exe() else {
        return TrayPinState::Unknown;
    };
    let state = sys::pin(&exe);
    tracing::info!(?state, "tray icon promotion requested");
    state
}

/// The fallback: the taskbar settings page.
pub fn open_taskbar_settings(app: &AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(TASKBAR_SETTINGS, None::<&str>)
        .map_err(|e| e.to_string())
}

fn current_exe() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

/// A known-folder GUID such as `{6D809377-6AF0-444B-8957-A3773F02200E}` as a `u128` in the
/// field order `GUID::from_u128` expects.
fn parse_guid(text: &str) -> Option<u128> {
    let inner = text.strip_prefix('{')?.strip_suffix('}')?;
    let groups: Vec<&str> = inner.split('-').collect();
    let lengths = [8, 4, 4, 4, 12];
    if groups.len() != lengths.len()
        || groups.iter().zip(lengths).any(|(g, n)| g.len() != n)
        || !groups
            .iter()
            .all(|g| g.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return None;
    }
    u128::from_str_radix(&groups.concat(), 16).ok()
}

/// `ExecutablePath` as a plain path; `known` resolves a known-folder GUID.
fn expand(stored: &str, known: impl Fn(u128) -> Option<PathBuf>) -> Option<PathBuf> {
    if !stored.starts_with('{') {
        return Some(PathBuf::from(stored));
    }
    let end = stored.find('}')?;
    let folder = known(parse_guid(&stored[..=end])?)?;
    let rest = stored[end + 1..].trim_start_matches(['\\', '/']);
    Some(folder.join(rest))
}

/// Windows paths compare case-insensitively and with either separator.
fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| p.to_string_lossy().replace('/', "\\").to_lowercase();
    norm(a) == norm(b)
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod sys {
    use std::ffi::c_void;
    use std::path::{Path, PathBuf};

    use windows_sys::Win32::Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_DWORD, RRF_RT_REG_DWORD,
        RRF_RT_REG_SZ, RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, RegSetKeyValueW,
    };
    use windows_sys::Win32::UI::Shell::SHGetKnownFolderPath;
    use windows_sys::core::{GUID, PWSTR};

    use super::{TrayPinState, expand, same_path};

    const KEY: &str = r"Control Panel\NotifyIconSettings";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// An open registry key, closed on drop.
    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful RegOpenKeyExW and is closed once.
            unsafe { RegCloseKey(self.0) };
        }
    }

    fn open(access: u32) -> Option<Key> {
        let path = wide(KEY);
        let mut key: HKEY = std::ptr::null_mut();
        // SAFETY: `path` is NUL-terminated and `key` is valid for writes.
        let status =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, access, &raw mut key) };
        (status == ERROR_SUCCESS).then_some(Key(key))
    }

    /// The subkeys whose `ExecutablePath` is `exe`, as NUL-terminated names.
    fn entries(key: &Key, exe: &Path) -> Vec<Vec<u16>> {
        let mut found = Vec::new();
        for index in 0.. {
            // Subkey names are decimal ids; 256 UTF-16 units is the registry's maximum.
            let mut name = [0u16; 256];
            #[allow(clippy::cast_possible_truncation)]
            let mut len = name.len() as u32;
            // SAFETY: `name` and `len` are valid for writes and `len` is the buffer length in
            // characters; the optional class and time outputs are null.
            let status = unsafe {
                RegEnumKeyExW(
                    key.0,
                    index,
                    name.as_mut_ptr(),
                    &raw mut len,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if status == ERROR_NO_MORE_ITEMS {
                break;
            }
            if status != ERROR_SUCCESS {
                continue;
            }
            let mut subkey = name[..len as usize].to_vec();
            subkey.push(0);
            let matches = string(key, &subkey, "ExecutablePath")
                .and_then(|stored| expand(&stored, known_folder))
                .is_some_and(|path| same_path(&path, exe));
            if matches {
                found.push(subkey);
            }
        }
        found
    }

    fn string(key: &Key, subkey: &[u16], name: &str) -> Option<String> {
        let name = wide(name);
        let mut buf = vec![0u16; 1024];
        #[allow(clippy::cast_possible_truncation)]
        let mut size = (buf.len() * 2) as u32;
        // SAFETY: `buf` and `size` are valid for writes and `size` is the buffer's byte length;
        // RRF_RT_REG_SZ refuses anything but a string. Longer values fail and are skipped.
        let status = unsafe {
            RegGetValueW(
                key.0,
                subkey.as_ptr(),
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
        Some(String::from_utf16_lossy(&buf[..len]))
    }

    /// `IsPromoted`; a missing value means the icon was never moved (hidden).
    fn promoted(key: &Key, subkey: &[u16]) -> bool {
        let name = wide("IsPromoted");
        let mut value: u32 = 0;
        #[allow(clippy::cast_possible_truncation)]
        let mut size = size_of::<u32>() as u32;
        // SAFETY: `value` and `size` are valid for writes; RRF_RT_REG_DWORD limits the data to
        // four bytes.
        let status = unsafe {
            RegGetValueW(
                key.0,
                subkey.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&raw mut value).cast::<c_void>(),
                &raw mut size,
            )
        };
        status == ERROR_SUCCESS && value == 1
    }

    fn summarize(key: &Key, subkeys: &[Vec<u16>]) -> TrayPinState {
        if subkeys.is_empty() {
            TrayPinState::Unknown
        } else if subkeys.iter().all(|s| promoted(key, s)) {
            TrayPinState::Shown
        } else {
            TrayPinState::Hidden
        }
    }

    pub fn state(exe: &Path) -> TrayPinState {
        let Some(key) = open(KEY_READ) else {
            return TrayPinState::Unsupported;
        };
        summarize(&key, &entries(&key, exe))
    }

    pub fn pin(exe: &Path) -> TrayPinState {
        let Some(key) = open(KEY_READ | KEY_SET_VALUE) else {
            return TrayPinState::Unsupported;
        };
        let subkeys = entries(&key, exe);
        let name = wide("IsPromoted");
        let one: u32 = 1;
        for subkey in &subkeys {
            // SAFETY: `subkey` and `name` are NUL-terminated; the data is a four-byte DWORD.
            let status = unsafe {
                RegSetKeyValueW(
                    key.0,
                    subkey.as_ptr(),
                    name.as_ptr(),
                    REG_DWORD,
                    (&raw const one).cast::<c_void>(),
                    4,
                )
            };
            if status != ERROR_SUCCESS {
                tracing::warn!(status, "could not promote the tray icon");
            }
        }
        summarize(&key, &subkeys)
    }

    fn known_folder(id: u128) -> Option<PathBuf> {
        let guid = GUID::from_u128(id);
        let mut out: PWSTR = std::ptr::null_mut();
        // SAFETY: `guid` lives across the call and `out` is valid for writes. The returned
        // string is freed with CoTaskMemFree as documented, also when the call failed.
        unsafe {
            let status =
                SHGetKnownFolderPath(&raw const guid, 0, std::ptr::null_mut(), &raw mut out);
            let path = if status == 0 && !out.is_null() {
                // The string is NUL-terminated.
                let mut len = 0;
                while *out.add(len) != 0 {
                    len += 1;
                }
                Some(PathBuf::from(String::from_utf16_lossy(
                    std::slice::from_raw_parts(out, len),
                )))
            } else {
                None
            };
            CoTaskMemFree(out.cast::<c_void>().cast_const());
            path
        }
    }
}

#[cfg(not(windows))]
mod sys {
    use std::path::Path;

    use super::TrayPinState;

    pub fn state(_exe: &Path) -> TrayPinState {
        TrayPinState::Unsupported
    }

    pub fn pin(_exe: &Path) -> TrayPinState {
        TrayPinState::Unsupported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROGRAM_FILES: u128 = 0x6D80_9377_6AF0_444B_8957_A377_3F02_200E;

    #[test]
    fn known_folder_guids_parse() {
        assert_eq!(
            parse_guid("{6D809377-6AF0-444B-8957-A3773F02200E}"),
            Some(PROGRAM_FILES)
        );
        assert_eq!(parse_guid("{6D809377-6AF0-444B-8957-A3773F02200}"), None);
        assert_eq!(parse_guid("6D809377-6AF0-444B-8957-A3773F02200E"), None);
        assert_eq!(parse_guid("{6D809377-6AF0-444B-8957-A3773F02200G}"), None);
    }

    #[test]
    fn stored_paths_expand() {
        let known = |id| (id == PROGRAM_FILES).then(|| PathBuf::from(r"C:\Program Files"));
        assert_eq!(
            expand(r"D:\Apps\tokenradar-app.exe", known),
            Some(PathBuf::from(r"D:\Apps\tokenradar-app.exe"))
        );
        assert_eq!(
            expand(
                r"{6D809377-6AF0-444B-8957-A3773F02200E}\TokenRadar\tokenradar-app.exe",
                known
            ),
            Some(PathBuf::from(
                r"C:\Program Files\TokenRadar\tokenradar-app.exe"
            ))
        );
        // An unknown folder cannot be ours.
        assert_eq!(
            expand(r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\Taskmgr.exe", known),
            None
        );
    }

    #[test]
    fn paths_compare_like_windows() {
        assert!(same_path(
            Path::new(r"C:\Users\A\AppData\Local\TokenRadar\tokenradar-app.exe"),
            Path::new("c:/users/a/appdata/local/tokenradar/TOKENRADAR-APP.EXE"),
        ));
        assert!(!same_path(
            Path::new(r"C:\target\debug\tokenradar-app.exe"),
            Path::new(r"C:\target\release\tokenradar-app.exe"),
        ));
    }
}
