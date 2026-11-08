//! Installing fonts for the current user only, without administrator rights (Step 12o).
//!
//! Windows 10 1809 and later: the file goes into `%LOCALAPPDATA%\Microsoft\Windows\Fonts`, a
//! value under `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts` makes it permanent,
//! and `AddFontResourceW` plus a `WM_FONTCHANGE` broadcast make running programs see it now.
//! macOS (`~/Library/Fonts`) and Linux (`~/.local/share/fonts` + `fc-cache`) come in Step 21.

use std::path::{Path, PathBuf};

use super::download::FontFile;

/// `%LOCALAPPDATA%\Microsoft\Windows\Fonts`.
pub fn user_font_dir(local_app_data: &Path) -> PathBuf {
    local_app_data
        .join("Microsoft")
        .join("Windows")
        .join("Fonts")
}

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("installing fonts is not supported on this platform yet")]
    Unsupported,
    #[error("{0} is not a readable font")]
    BadFont(&'static str),
    #[error("could not write {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not register the font in the registry")]
    Registry,
}

/// Writes `files` into `dir` and registers them for the current user. A file already there
/// with the same bytes is kept as is (it may be in use). Returns the installed paths.
pub fn install(files: &[FontFile], dir: &Path) -> Result<Vec<PathBuf>, InstallError> {
    // Every file is checked before the first one is written.
    let names = files
        .iter()
        .map(|f| full_name(&f.bytes).ok_or(InstallError::BadFont(f.name)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut installed = Vec::with_capacity(files.len());
    for (file, full_name) in files.iter().zip(names) {
        let path = dir.join(file.name);
        let same = std::fs::read(&path).is_ok_and(|old| old == file.bytes);
        if !same {
            crate::fsutil::atomic_write(&path, &file.bytes).map_err(|source| InstallError::Io {
                path: path.clone(),
                source,
            })?;
        }
        sys::register(&full_name, &path)?;
        installed.push(path);
    }
    sys::broadcast();
    Ok(installed)
}

/// The full font name (name ID 4), as Windows' own installer names the registry value.
fn full_name(bytes: &[u8]) -> Option<String> {
    let face = ttf_parser::Face::parse(bytes, 0).ok()?;
    face.names()
        .into_iter()
        .filter(|n| n.name_id == ttf_parser::name_id::FULL_NAME)
        .filter_map(|n| {
            let rank = u8::from(
                !(n.platform_id == ttf_parser::PlatformId::Windows && n.language_id == 0x0409),
            );
            n.to_string().map(|s| (rank, s))
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, s)| s)
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod sys {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::Graphics::Gdi::AddFontResourceW;
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegSetKeyValueW};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, PostMessageW, WM_FONTCHANGE,
    };

    use super::InstallError;

    const FONTS_KEY: &str = r"Software\Microsoft\Windows NT\CurrentVersion\Fonts";

    fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    pub(super) fn register(full_name: &str, path: &Path) -> Result<(), InstallError> {
        let key = wide(FONTS_KEY.as_ref());
        let value = wide(format!("{full_name} (TrueType)").as_ref());
        let data = wide(path.as_os_str());
        let bytes = u32::try_from(data.len() * 2).map_err(|_| InstallError::Registry)?;
        // SAFETY: every string is NUL-terminated and `bytes` is the length of `data` in bytes,
        // terminator included, as REG_SZ requires. The key is created if missing.
        let status = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                REG_SZ,
                data.as_ptr().cast(),
                bytes,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(InstallError::Registry);
        }
        // SAFETY: `data` is a NUL-terminated path. A zero result only means this session does
        // not see the font until the next sign-in; the registry value is already written.
        unsafe { AddFontResourceW(data.as_ptr()) };
        Ok(())
    }

    pub(super) fn broadcast() {
        // SAFETY: a posted broadcast with no pointers; it does not wait for hung windows.
        unsafe { PostMessageW(HWND_BROADCAST, WM_FONTCHANGE, 0, 0) };
    }
}

#[cfg(not(windows))]
mod sys {
    use std::path::Path;

    use super::InstallError;

    pub(super) fn register(_full_name: &str, _path: &Path) -> Result<(), InstallError> {
        Err(InstallError::Unsupported)
    }

    pub(super) fn broadcast() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_font_folder_is_under_local_app_data() {
        let dir = user_font_dir(Path::new(r"C:\Users\x\AppData\Local"));
        assert!(dir.ends_with(Path::new("Microsoft").join("Windows").join("Fonts")));
    }

    #[test]
    fn a_file_that_is_not_a_font_is_refused_before_anything_is_written() {
        let dir = tempfile::tempdir().expect("tempdir");
        let files = [FontFile {
            name: "x.ttf",
            bytes: b"not a font".to_vec(),
        }];
        assert!(matches!(
            install(&files, dir.path()),
            Err(InstallError::BadFont("x.ttf"))
        ));
        assert_eq!(std::fs::read_dir(dir.path()).expect("ls").count(), 0);
    }
}
