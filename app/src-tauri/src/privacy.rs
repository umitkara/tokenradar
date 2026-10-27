//! "Privacy & data" and "About" in the settings: storage, deleting TokenRadar's data, the
//! diagnostics bundle and the few links the app may open.

use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt as _;
use tauri_plugin_opener::OpenerExt as _;
use tokenradar_core::config;
use tokenradar_core::doctor::{self, Input};
use tokenradar_core::domain::Timestamp;
use tokenradar_core::paths::Paths;

use crate::core_bridge;

const REPO: &str = "https://github.com/umitkara/TokenRadar";

#[derive(Debug, Clone, Serialize, Type)]
pub struct StorageView {
    pub data_dir: String,
    /// Database with its WAL, in bytes.
    pub db_bytes: u64,
    pub logs_bytes: u64,
    pub portable: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct AboutView {
    pub version: String,
    pub repo_url: String,
}

/// The only pages the app opens (ADR-013: no other destination).
#[derive(Debug, Clone, Copy, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Link {
    Repo,
    Releases,
    Issues,
    License,
}

impl Link {
    fn url(self) -> String {
        match self {
            Self::Repo => REPO.to_owned(),
            Self::Releases => format!("{REPO}/releases"),
            Self::Issues => format!("{REPO}/issues"),
            Self::License => format!("{REPO}/blob/main/LICENSE"),
        }
    }
}

fn paths() -> Result<Paths, String> {
    Paths::resolve().map_err(|e| e.to_string())
}

fn size(path: &Path) -> u64 {
    std::fs::metadata(path).map_or(0, |m| m.len())
}

fn dir_size(dir: &Path) -> u64 {
    std::fs::read_dir(dir).map_or(0, |entries| {
        entries
            .flatten()
            .filter_map(|e| e.metadata().ok())
            .filter(std::fs::Metadata::is_file)
            .map(|m| m.len())
            .sum()
    })
}

pub fn storage() -> Result<StorageView, String> {
    let paths = paths()?;
    let db = paths.db_path();
    let sidecar = |suffix: &str| {
        let mut p = db.clone().into_os_string();
        p.push(suffix);
        size(Path::new(&p))
    };
    Ok(StorageView {
        data_dir: paths.data_dir().display().to_string(),
        db_bytes: size(&db) + sidecar("-wal") + sidecar("-shm"),
        logs_bytes: dir_size(&paths.logs_dir()),
        portable: paths.is_portable(),
    })
}

pub fn about() -> AboutView {
    AboutView {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        repo_url: REPO.to_owned(),
    }
}

pub fn open_link(app: &AppHandle, link: Link) -> Result<(), String> {
    app.opener()
        .open_url(link.url(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Deletes the database, spool and logs and resets `config.toml`, keeping only the
/// statusline install state (`uninstall` needs it to restore the user's command). Claude
/// Code's files are never touched. The core is stopped first and started again on an empty
/// database.
pub fn delete_all_data(app: &AppHandle) -> Result<(), String> {
    let paths = paths()?;
    tracing::info!("deleting all TokenRadar data");
    core_bridge::stop(app);
    let result = delete_files(&paths);
    // Back to defaults everywhere, then a fresh core, whatever happened above.
    crate::prefs::reload(app);
    core_bridge::start(app);
    result
}

fn delete_files(paths: &Paths) -> Result<(), String> {
    let mut failed = Vec::new();
    let db = paths.db_path();
    for suffix in ["", "-wal", "-shm"] {
        let mut p = db.clone().into_os_string();
        p.push(suffix);
        remove_file(Path::new(&p), &mut failed);
    }
    if let Ok(entries) = std::fs::read_dir(paths.spool_dir()) {
        for e in entries.flatten() {
            remove_file(&e.path(), &mut failed);
        }
    }
    clear_logs(&paths.logs_dir(), &mut failed);
    if let Err(e) = config::reset_keeping_statusline(paths.config_path()) {
        failed.push(format!("config.toml: {e}"));
    }
    if failed.is_empty() {
        Ok(())
    } else {
        tracing::warn!(
            count = failed.len(),
            "some TokenRadar files could not be deleted"
        );
        Err(failed.join("\n"))
    }
}

/// Removes every log file but today's, which is emptied instead: the log writer keeps it open
/// in append mode, so it goes on writing from the start. A deleted open file would swallow
/// everything logged until the next daily rotation.
fn clear_logs(dir: &Path, failed: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    // `tokenradar.YYYY-MM-DD.log`: the newest sorts last.
    files.sort();
    let current = files.pop();
    for path in &files {
        remove_file(path, failed);
    }
    if let Some(path) = current {
        let truncated = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .and_then(|f| f.set_len(0));
        if let Err(e) = truncated {
            failed.push(format!("{}: {e}", path.display()));
        }
    }
}

fn remove_file(path: &Path, failed: &mut Vec<String>) {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            failed.push(format!("{}: {e}", path.display()));
        }
        _ => {}
    }
}

/// Asks where to save the diagnostics bundle, writes it and shows it in Explorer. `Ok(None)`:
/// the user cancelled.
pub fn create_diagnostics(app: &AppHandle) -> Result<Option<String>, String> {
    let name = doctor::default_file_name(Timestamp::now());
    let Some(target) = app
        .dialog()
        .file()
        .set_file_name(&name)
        .add_filter("Zip", &["zip"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let out = target.into_path().map_err(|e| e.to_string())?;
    let paths = paths()?;
    let producer = format!("tokenradar-app/{}", env!("CARGO_PKG_VERSION"));
    let live = core_bridge::panel_snapshot(app).map(|s| doctor::live_from_snapshot(&s));
    let input = Input {
        producer: &producer,
        paths: &paths,
        live,
    };
    let written = doctor::write(&out, &input).map_err(|e| e.to_string())?;
    if let Err(error) = app.opener().reveal_item_in_dir(&written) {
        tracing::warn!(%error, "could not show the diagnostics bundle");
    }
    Ok(Some(written.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clearing_logs_empties_the_current_file_and_removes_the_rest() {
        let dir = tempfile::tempdir().expect("tempdir");
        let old = dir.path().join("tokenradar.2026-10-26.log");
        let current = dir.path().join("tokenradar.2026-10-27.log");
        std::fs::write(&old, "old\n").expect("old");
        std::fs::write(&current, "today\n").expect("current");
        // Held open in append mode, as the log writer does.
        let mut writer = std::fs::OpenOptions::new()
            .append(true)
            .open(&current)
            .expect("open");
        let mut failed = Vec::new();
        clear_logs(dir.path(), &mut failed);
        assert!(failed.is_empty(), "{failed:?}");
        assert!(!old.exists());
        assert_eq!(std::fs::read_to_string(&current).expect("read"), "");
        std::io::Write::write_all(&mut writer, b"after\n").expect("write");
        assert_eq!(std::fs::read_to_string(&current).expect("read"), "after\n");
    }
}
