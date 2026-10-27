//! Statusline readings written while the app is not running (ADR-009, architecture §11).
//!
//! The CLI appends one [`StatuslinePush`] per line to `spool/statusline-YYYYMMDD.jsonl` (UTC
//! date) under an exclusive file lock. The app imports the files at start and every five
//! minutes, then deletes them. A file is renamed to `*.importing` before it is read, so a CLI
//! that starts writing during an import creates a fresh file instead of losing its line.
//! Import is idempotent: a crash between ingesting and deleting re-reads the file, and the
//! caller skips readings already stored.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::payload::StatuslinePush;
use crate::domain::Timestamp;

/// A day's file stops growing at this size; later readings that day are dropped.
pub const MAX_DAILY_BYTES: u64 = 5 * 1024 * 1024;
/// Files the app never imported are deleted after this many days.
pub const RETENTION_DAYS: i64 = 14;
/// How often the running app looks for new files.
pub const IMPORT_INTERVAL: Duration = Duration::from_mins(5);

const PREFIX: &str = "statusline-";
const SUFFIX: &str = ".jsonl";
const IMPORTING: &str = ".importing";

/// What [`append`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appended {
    Written,
    /// The day's file reached [`MAX_DAILY_BYTES`].
    Full,
}

/// Appends `push` to the file of its UTC day.
pub fn append(dir: &Path, push: &StatuslinePush) -> std::io::Result<Appended> {
    let day = day_of(push.observed_at);
    let path = dir.join(format!("{PREFIX}{day}{SUFFIX}"));
    let new_day = !path.exists();
    if new_day {
        std::fs::create_dir_all(dir)?;
        prune(dir, push.observed_at);
    }
    let mut line = serde_json::to_vec(push)?;
    line.push(b'\n');
    // `read` too: LockFileEx refuses a handle that only has append access.
    let mut file = OpenOptions::new()
        .read(true)
        .create(true)
        .append(true)
        .open(&path)?;
    file.lock()?;
    if file.metadata()?.len() + line.len() as u64 > MAX_DAILY_BYTES {
        return Ok(Appended::Full);
    }
    // One write call under the lock: readers never see half a line from us.
    file.write_all(&line)?;
    Ok(Appended::Written)
}

/// One claimed spool file and its readings, oldest line first.
#[derive(Debug)]
pub struct Claimed {
    path: PathBuf,
    pub pushes: Vec<StatuslinePush>,
    /// Lines that did not parse (counted, never fatal).
    pub malformed: usize,
}

impl Claimed {
    /// Deletes the file once its readings are stored.
    pub fn finish(self) -> std::io::Result<()> {
        std::fs::remove_file(&self.path)
    }
}

/// Claims every spool file in `dir`, including ones left by an interrupted import.
pub fn claim(dir: &Path) -> Vec<Claimed> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with(PREFIX))
        .collect();
    names.sort();
    let mut out = Vec::new();
    for name in names {
        let path = dir.join(&name);
        let claimed_path = if name.ends_with(IMPORTING) {
            path
        } else if name.ends_with(SUFFIX) {
            let to = dir.join(format!("{name}{IMPORTING}"));
            // A file that cannot be renamed right now is left for the next round.
            if std::fs::rename(&path, &to).is_err() {
                continue;
            }
            to
        } else {
            continue;
        };
        match read_claimed(&claimed_path) {
            Ok(c) => out.push(c),
            Err(error) => {
                tracing::warn!(%error, file = %name, "could not read a spool file");
            }
        }
    }
    out
}

fn read_claimed(path: &Path) -> std::io::Result<Claimed> {
    let file = File::open(path)?;
    // Waits for a writer that opened the file before the rename and still holds the lock.
    file.lock_shared()?;
    let mut pushes = Vec::new();
    let mut malformed = 0;
    for line in BufReader::new(&file).split(b'\n') {
        let line = line?;
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        match serde_json::from_slice::<StatuslinePush>(&line) {
            Ok(p) => pushes.push(p),
            Err(_) => malformed += 1,
        }
    }
    Ok(Claimed {
        path: path.to_path_buf(),
        pushes,
        malformed,
    })
}

/// Deletes day files older than [`RETENTION_DAYS`].
fn prune(dir: &Path, now: Timestamp) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let cutoff = day_of(now.saturating_sub_millis(RETENTION_DAYS * 24 * 60 * 60 * 1000));
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(day) = name
            .strip_prefix(PREFIX)
            .and_then(|r| r.get(..8))
            .filter(|d| d.bytes().all(|b| b.is_ascii_digit()))
        else {
            continue;
        };
        if day < cutoff.as_str() {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// `YYYYMMDD` of the UTC day.
fn day_of(t: Timestamp) -> String {
    jiff::Timestamp::from_millisecond(t.as_millis()).map_or_else(
        |_| "00000000".to_owned(),
        |ts| ts.strftime("%Y%m%d").to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statusline::payload::StatuslinePayload;

    const DAY_MS: i64 = 24 * 60 * 60 * 1000;
    /// 2026-10-27T00:00:00Z.
    const T0: i64 = 1_793_059_200_000;

    fn push(ms: i64, pct: f64) -> StatuslinePush {
        let payload = StatuslinePayload::parse(
            format!(
                r#"{{"session_id":"s1","rate_limits":{{"five_hour":{{"used_percentage":{pct},"resets_at":1793070000}}}}}}"#
            )
            .as_bytes(),
        )
        .expect("payload");
        StatuslinePush {
            observed_at: Timestamp::from_millis(ms),
            payload,
        }
    }

    #[test]
    fn appended_lines_are_claimed_in_order_and_finished() {
        let dir = tempfile::tempdir().expect("tempdir");
        for i in 0..3 {
            let p = push(T0 + i * 1000, f64::from(i32::try_from(i).expect("small")));
            assert_eq!(append(dir.path(), &p).expect("append"), Appended::Written);
        }
        assert!(dir.path().join("statusline-20261027.jsonl").is_file());
        let claimed = claim(dir.path());
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].pushes.len(), 3);
        assert_eq!(claimed[0].pushes[2], push(T0 + 2000, 2.0));
        // A new reading during the import goes to a fresh file.
        append(dir.path(), &push(T0 + 5000, 9.0)).expect("append");
        for c in claimed {
            c.finish().expect("finish");
        }
        let again = claim(dir.path());
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].pushes, vec![push(T0 + 5000, 9.0)]);
    }

    #[test]
    fn an_interrupted_import_is_claimed_again() {
        let dir = tempfile::tempdir().expect("tempdir");
        append(dir.path(), &push(T0, 1.0)).expect("append");
        drop(claim(dir.path()));
        let again = claim(dir.path());
        assert_eq!(again.len(), 1, "the .importing file is picked up again");
        assert_eq!(again[0].pushes.len(), 1);
    }

    #[test]
    fn malformed_lines_are_counted_and_skipped() {
        let dir = tempfile::tempdir().expect("tempdir");
        append(dir.path(), &push(T0, 1.0)).expect("append");
        let path = dir.path().join("statusline-20261027.jsonl");
        let mut f = OpenOptions::new().append(true).open(&path).expect("open");
        f.write_all(b"{not json\n\n").expect("write");
        drop(f);
        append(dir.path(), &push(T0 + 1, 2.0)).expect("append");
        let claimed = claim(dir.path());
        assert_eq!(claimed[0].pushes.len(), 2);
        assert_eq!(claimed[0].malformed, 1);
    }

    #[test]
    fn a_full_day_drops_further_readings() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("statusline-20261027.jsonl");
        std::fs::create_dir_all(dir.path()).expect("dir");
        let filler = vec![b' '; usize::try_from(MAX_DAILY_BYTES).expect("fits") - 10];
        std::fs::write(&path, filler).expect("fill");
        assert_eq!(
            append(dir.path(), &push(T0, 1.0)).expect("append"),
            Appended::Full
        );
    }

    #[test]
    fn old_files_are_pruned_when_a_new_day_starts() {
        let dir = tempfile::tempdir().expect("tempdir");
        append(dir.path(), &push(T0 - 20 * DAY_MS, 1.0)).expect("old");
        append(dir.path(), &push(T0 - 3 * DAY_MS, 1.0)).expect("recent");
        append(dir.path(), &push(T0, 1.0)).expect("today");
        let mut names: Vec<_> = std::fs::read_dir(dir.path())
            .expect("ls")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            ["statusline-20261024.jsonl", "statusline-20261027.jsonl"]
        );
    }
}
