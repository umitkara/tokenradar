//! Where TokenRadar keeps its database, config, logs and spool files (architecture §14).

use std::path::{Path, PathBuf};

/// Name of the marker file that switches on portable mode when placed next to the executable.
pub const PORTABLE_MARKER: &str = "portable";

#[derive(Debug, thiserror::Error)]
pub enum PathsError {
    #[error("could not determine the user's home directory")]
    NoHome,
    #[error("could not locate the running executable: {0}")]
    CurrentExe(#[source] std::io::Error),
}

/// Resolved file locations. Resolve once at startup and pass around.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    data_dir: PathBuf,
    config_path: PathBuf,
    portable: bool,
}

impl Paths {
    /// Resolves locations for the running executable: portable if a `portable` file sits
    /// next to it, the platform's per-user directories otherwise.
    pub fn resolve() -> Result<Self, PathsError> {
        let exe = std::env::current_exe().map_err(PathsError::CurrentExe)?;
        if let Some(dir) = exe.parent().and_then(portable_root) {
            return Ok(Self::portable(&dir));
        }
        Self::platform()
    }

    /// Portable layout: everything under `<exe_dir>/data`.
    pub fn portable(exe_dir: &Path) -> Self {
        let data_dir = exe_dir.join("data");
        Self {
            config_path: data_dir.join("config.toml"),
            data_dir,
            portable: true,
        }
    }

    /// Per-user platform layout.
    pub fn platform() -> Result<Self, PathsError> {
        let base = directories::BaseDirs::new().ok_or(PathsError::NoHome)?;
        Ok(Self::from_base_dirs(&base))
    }

    fn from_base_dirs(base: &directories::BaseDirs) -> Self {
        if cfg!(target_os = "linux") {
            // XDG keeps data and config apart.
            Self {
                data_dir: base.data_dir().join("tokenradar"),
                config_path: base.config_dir().join("tokenradar").join("config.toml"),
                portable: false,
            }
        } else {
            // Windows: %LOCALAPPDATA%\TokenRadar; macOS: ~/Library/Application Support/TokenRadar.
            let data_dir = base.data_local_dir().join("TokenRadar");
            Self {
                config_path: data_dir.join("config.toml"),
                data_dir,
                portable: false,
            }
        }
    }

    pub fn is_portable(&self) -> bool {
        self.portable
    }

    /// Root for the database, spool and caches.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("tokenradar.db")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    /// Statusline payloads written by the CLI while the app is not running.
    pub fn spool_dir(&self) -> PathBuf {
        self.data_dir.join("spool")
    }
}

/// The portable root for an executable in `exe_dir`: the directory itself when the marker is
/// there, or its parent for the CLI in the portable zip's `bin` (the marker sits next to the
/// app, one level up).
fn portable_root(exe_dir: &Path) -> Option<PathBuf> {
    if exe_dir.join(PORTABLE_MARKER).is_file() {
        return Some(exe_dir.to_path_buf());
    }
    let is_bin = exe_dir
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("bin"));
    exe_dir
        .parent()
        .filter(|root| is_bin && root.join(PORTABLE_MARKER).is_file())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_mode_keeps_everything_next_to_the_exe() {
        let exe_dir = Path::new("portable-root");
        let paths = Paths::portable(exe_dir);
        assert!(paths.is_portable());
        assert_eq!(paths.data_dir(), exe_dir.join("data"));
        assert_eq!(
            paths.config_path(),
            exe_dir.join("data").join("config.toml")
        );
        assert_eq!(paths.db_path(), exe_dir.join("data").join("tokenradar.db"));
        assert_eq!(paths.logs_dir(), exe_dir.join("data").join("logs"));
        assert_eq!(paths.spool_dir(), exe_dir.join("data").join("spool"));
    }

    #[test]
    fn the_marker_is_found_next_to_the_exe_or_above_bin() {
        let root = tempfile::tempdir().expect("temp dir");
        let bin = root.path().join("bin");
        std::fs::create_dir(&bin).expect("bin");
        assert_eq!(portable_root(root.path()), None);
        assert_eq!(portable_root(&bin), None);

        std::fs::write(root.path().join(PORTABLE_MARKER), "").expect("marker");
        assert_eq!(portable_root(root.path()), Some(root.path().to_path_buf()));
        assert_eq!(portable_root(&bin), Some(root.path().to_path_buf()));

        // Only `bin` looks one level up.
        let other = root.path().join("tools");
        std::fs::create_dir(&other).expect("tools");
        assert_eq!(portable_root(&other), None);
    }

    #[test]
    fn platform_layout_uses_per_user_dirs() {
        let base = directories::BaseDirs::new().expect("home directory");
        let paths = Paths::from_base_dirs(&base);
        assert!(!paths.is_portable());
        #[cfg(windows)]
        assert_eq!(paths.data_dir(), base.data_local_dir().join("TokenRadar"));
        assert!(paths.config_path().ends_with("config.toml"));
    }
}
