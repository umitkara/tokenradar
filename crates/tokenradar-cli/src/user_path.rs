//! `tokenradar path add|remove`: puts the CLI's own directory on the user's `PATH` or takes it
//! off. The installer calls it (Step 12) and portable users can run it by hand.
//!
//! NSIS cannot do this safely: its strings stop at 1024 characters, so reading a longer `PATH`
//! and writing it back would cut it. The value keeps its registry type, so `%USERPROFILE%`-style
//! entries in a `REG_EXPAND_SZ` value stay unexpanded.

/// Longest `PATH` we write. Older tools (`setx`, the classic environment dialog) cut values
/// beyond this, so going over it is refused instead.
pub const MAX_LEN: usize = 2047;

#[derive(Debug, PartialEq, Eq)]
pub enum Edit {
    /// The value to write.
    Changed(String),
    /// Already as wanted; nothing to write.
    Unchanged,
}

/// Compares entries the way Windows resolves them: expanded, without quotes or a trailing
/// separator, ignoring case.
fn normalize(entry: &str, expand: &dyn Fn(&str) -> String) -> String {
    expand(entry.trim())
        .trim()
        .trim_matches('"')
        .trim_end_matches(['\\', '/'])
        .to_lowercase()
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Appends `dir` to `current` unless an entry already points at it. `Err` carries the length
/// the value would have had when it exceeds [`MAX_LEN`].
pub fn add(current: &str, dir: &str, expand: &dyn Fn(&str) -> String) -> Result<Edit, usize> {
    let wanted = normalize(dir, expand);
    if current
        .split(';')
        .any(|entry| !entry.trim().is_empty() && normalize(entry, expand) == wanted)
    {
        return Ok(Edit::Unchanged);
    }
    let base = current.trim_end_matches(';');
    let value = if base.trim().is_empty() {
        dir.to_owned()
    } else {
        format!("{base};{dir}")
    };
    match utf16_len(&value) {
        len if len > MAX_LEN => Err(len),
        _ => Ok(Edit::Changed(value)),
    }
}

/// Drops every entry that points at `dir`; the others stay exactly as they were.
pub fn remove(current: &str, dir: &str, expand: &dyn Fn(&str) -> String) -> Edit {
    let wanted = normalize(dir, expand);
    let entries: Vec<&str> = current.split(';').collect();
    let kept: Vec<&str> = entries
        .iter()
        .copied()
        .filter(|entry| entry.trim().is_empty() || normalize(entry, expand) != wanted)
        .collect();
    if kept.len() == entries.len() {
        Edit::Unchanged
    } else {
        Edit::Changed(kept.join(";"))
    }
}

#[cfg(windows)]
pub use imp::{broadcast, expand, read, write};

#[cfg(windows)]
#[allow(unsafe_code)]
mod imp {
    use std::io;

    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_EXPAND_SZ, REG_SZ,
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    };

    const KEY: &str = "Environment";
    const NAME: &str = "Path";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn check(status: u32) -> io::Result<()> {
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            #[allow(clippy::cast_possible_wrap)]
            Err(io::Error::from_raw_os_error(status as i32))
        }
    }

    /// `HKCU\Environment`, closed on drop.
    struct Key(HKEY);

    impl Key {
        fn open() -> io::Result<Self> {
            let mut key: HKEY = std::ptr::null_mut();
            let name = wide(KEY);
            // SAFETY: `name` is NUL-terminated and `key` is valid for the handle write.
            check(unsafe {
                RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    name.as_ptr(),
                    0,
                    KEY_QUERY_VALUE | KEY_SET_VALUE,
                    &raw mut key,
                )
            })?;
            Ok(Self(key))
        }
    }

    impl Drop for Key {
        fn drop(&mut self) {
            // SAFETY: the handle came from `RegOpenKeyExW` and is closed once.
            unsafe { RegCloseKey(self.0) };
        }
    }

    /// The user's `PATH` as stored and whether it is `REG_EXPAND_SZ`. A missing value reads as
    /// an empty expandable one, which is what Windows creates.
    pub fn read() -> io::Result<(String, bool)> {
        let key = Key::open()?;
        let name = wide(NAME);
        let mut kind = 0u32;
        let mut size = 0u32;
        // SAFETY: a size query; no data pointer is passed.
        let status = unsafe {
            RegQueryValueExW(
                key.0,
                name.as_ptr(),
                std::ptr::null(),
                &raw mut kind,
                std::ptr::null_mut(),
                &raw mut size,
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok((String::new(), true));
        }
        check(status)?;
        if kind != REG_SZ && kind != REG_EXPAND_SZ {
            return Err(io::Error::other("the user PATH is not a string value"));
        }
        let mut buf = vec![0u16; (size as usize).div_ceil(2) + 1];
        #[allow(clippy::cast_possible_truncation)]
        let mut bytes = (buf.len() * 2) as u32;
        // SAFETY: `buf` holds `bytes` bytes and outlives the call.
        check(unsafe {
            RegQueryValueExW(
                key.0,
                name.as_ptr(),
                std::ptr::null(),
                &raw mut kind,
                buf.as_mut_ptr().cast::<u8>(),
                &raw mut bytes,
            )
        })?;
        buf.truncate((bytes as usize) / 2);
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok((String::from_utf16_lossy(&buf[..len]), kind == REG_EXPAND_SZ))
    }

    /// Writes `value` with the given type; an empty value deletes it.
    pub fn write(value: &str, expandable: bool) -> io::Result<()> {
        let key = Key::open()?;
        let name = wide(NAME);
        if value.is_empty() {
            // SAFETY: `name` is NUL-terminated.
            return check(unsafe { RegDeleteValueW(key.0, name.as_ptr()) });
        }
        let data = wide(value);
        let kind = if expandable { REG_EXPAND_SZ } else { REG_SZ };
        #[allow(clippy::cast_possible_truncation)]
        let bytes = (data.len() * 2) as u32;
        // SAFETY: `data` is NUL-terminated and `bytes` is its length in bytes.
        check(unsafe {
            RegSetValueExW(
                key.0,
                name.as_ptr(),
                0,
                kind,
                data.as_ptr().cast::<u8>(),
                bytes,
            )
        })
    }

    /// Tells Explorer and other running programs that the environment changed, so new
    /// terminals see the new `PATH`.
    pub fn broadcast() {
        let what = wide(KEY);
        let mut result = 0usize;
        // SAFETY: `what` is NUL-terminated and lives for the call; a hung window is skipped.
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                what.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                5000,
                &raw mut result,
            );
        }
    }

    /// `%VAR%` expanded with this process's environment; the input when that fails.
    pub fn expand(s: &str) -> String {
        let src = wide(s);
        // SAFETY: a size query with no output buffer.
        let needed = unsafe { ExpandEnvironmentStringsW(src.as_ptr(), std::ptr::null_mut(), 0) };
        if needed == 0 {
            return s.to_owned();
        }
        let mut buf = vec![0u16; needed as usize];
        // SAFETY: `buf` holds `needed` characters.
        let written = unsafe { ExpandEnvironmentStringsW(src.as_ptr(), buf.as_mut_ptr(), needed) };
        if written == 0 || written > needed {
            return s.to_owned();
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(s: &str) -> String {
        s.replace("%LOCALAPPDATA%", r"C:\Users\u\AppData\Local")
    }

    const DIR: &str = r"C:\Users\u\AppData\Local\Programs\TokenRadar\bin";

    #[test]
    fn appends_to_the_end_and_keeps_the_rest_verbatim() {
        let current = r"%USERPROFILE%\bin;C:\Tools";
        assert_eq!(
            add(current, DIR, &expand),
            Ok(Edit::Changed(format!(r"%USERPROFILE%\bin;C:\Tools;{DIR}")))
        );
    }

    #[test]
    fn an_empty_or_missing_path_gets_just_the_directory() {
        assert_eq!(add("", DIR, &expand), Ok(Edit::Changed(DIR.to_owned())));
        assert_eq!(add(";", DIR, &expand), Ok(Edit::Changed(DIR.to_owned())));
    }

    #[test]
    fn a_trailing_separator_is_not_doubled() {
        assert_eq!(
            add(r"C:\Tools;", DIR, &expand),
            Ok(Edit::Changed(format!(r"C:\Tools;{DIR}")))
        );
    }

    #[test]
    fn an_existing_entry_in_any_spelling_is_not_added_again() {
        for spelling in [
            DIR.to_owned(),
            DIR.to_uppercase(),
            format!(r"{DIR}\"),
            format!("\"{DIR}\""),
            r"%LOCALAPPDATA%\Programs\TokenRadar\bin".to_owned(),
        ] {
            let current = format!(r"C:\Tools;{spelling};C:\Other");
            assert_eq!(
                add(&current, DIR, &expand),
                Ok(Edit::Unchanged),
                "{spelling}"
            );
        }
    }

    #[test]
    fn refuses_to_grow_past_the_limit() {
        let current = "x".repeat(MAX_LEN - DIR.len());
        assert_eq!(add(&current, DIR, &expand), Err(MAX_LEN + 1));
        let fits = "x".repeat(MAX_LEN - DIR.len() - 1);
        assert!(matches!(add(&fits, DIR, &expand), Ok(Edit::Changed(v)) if v.len() == MAX_LEN));
    }

    #[test]
    fn removes_every_spelling_and_keeps_other_entries_verbatim() {
        let current =
            format!(r"%USERPROFILE%\bin;{DIR};C:\Tools;;%LOCALAPPDATA%\Programs\TokenRadar\bin\");
        assert_eq!(
            remove(&current, DIR, &expand),
            Edit::Changed(r"%USERPROFILE%\bin;C:\Tools;".to_owned())
        );
    }

    #[test]
    fn removing_an_absent_entry_changes_nothing() {
        assert_eq!(remove(r"C:\Tools;C:\Other", DIR, &expand), Edit::Unchanged);
        assert_eq!(remove("", DIR, &expand), Edit::Unchanged);
    }

    #[test]
    fn removing_the_only_entry_leaves_an_empty_value() {
        assert_eq!(remove(DIR, DIR, &expand), Edit::Changed(String::new()));
    }
}
