//! Small file helpers shared by the config and settings writers.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Writes `bytes` to `path` atomically: a sibling temporary file is written, flushed to disk
/// and renamed over the target, so a reader sees the old or the new content, never half.
/// A symlinked target is followed, so the link itself survives.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let target = resolve_link(path);
    let dir = target
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let name = target
        .file_name()
        .map_or_else(|| "file".into(), |n| n.to_string_lossy().into_owned());
    let tmp = dir.join(format!(".{name}.tokenradar-{}.tmp", std::process::id()));
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        // On Windows this is MoveFileExW(MOVEFILE_REPLACE_EXISTING): atomic on one volume.
        std::fs::rename(&tmp, &target)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn resolve_link(path: &Path) -> PathBuf {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => std::fs::read_link(path).map_or_else(
            |_| path.to_path_buf(),
            |t| match path.parent() {
                Some(dir) if t.is_relative() => dir.join(t),
                _ => t,
            },
        ),
        _ => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_content_and_leaves_no_temporary_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a.json");
        std::fs::write(&path, "old").expect("seed");
        atomic_write(&path, b"new").expect("write");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "new");
        assert_eq!(std::fs::read_dir(dir.path()).expect("ls").count(), 1);
    }
}
