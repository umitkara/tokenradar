//! Running Claude Code sessions, from `<config>/sessions/<pid>.json` (data-sources.md S4).
//!
//! Claude Code writes one small JSON file per running process and rewrites it when the session
//! turns busy or idle. A crashed process leaves its file behind, and Windows reuses pids, so
//! a file only counts when its pid is alive *and* the process creation time matches the
//! file's `procStart`. Files are never modified or deleted: they belong to Claude Code. The
//! `<pid>.<hash>.key` files next to them and `messagingSocketPath` are Claude Code's own IPC
//! and are never touched.

mod process;

use std::collections::HashMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

pub use process::{Liveness, filetime_to_unix_ms, probe};

use crate::domain::{AccountId, SessionId, Timestamp};
use crate::store::Store;

/// Name of the live-session directory inside a config directory.
pub const SESSIONS_DIR: &str = "sessions";

/// Session files are a few hundred bytes; anything far larger is not one.
const MAX_FILE_BYTES: u64 = 64 << 10;

/// `procStart` and the OS creation time agree to the tick in practice; allow one millisecond
/// in case a future version rounds.
const PROC_START_TOLERANCE: u64 = 10_000;

/// Without `procStart`, the process must have started at most this long before `startedAt` …
const STARTED_AT_LEAD_MS: i64 = 10 * 60 * 1000;
/// … and not after it (plus clock jitter).
const STARTED_AT_LAG_MS: i64 = 1000;

/// Prefix of `pidDomain` for pids this machine can check. Other domains (e.g. a WSL distro
/// writing into a shared directory) are skipped rather than probed against the wrong pids.
#[cfg(windows)]
const LOCAL_PID_DOMAIN: Option<&str> = Some("win32");
#[cfg(not(windows))]
const LOCAL_PID_DOMAIN: Option<&str> = None;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum LiveStatus {
    Busy,
    Idle,
    /// Missing or a value this build does not know.
    Unknown,
}

impl LiveStatus {
    fn parse(s: Option<&str>) -> Self {
        match s {
            Some("busy") => Self::Busy,
            Some("idle") => Self::Idle,
            _ => Self::Unknown,
        }
    }
}

/// A Claude Code process that is running right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct LiveSession {
    pub account: AccountId,
    pub session_uuid: String,
    /// The indexed session, once its transcript has been seen.
    pub session_id: Option<SessionId>,
    pub pid: u32,
    pub cwd: String,
    /// Display name Claude Code chose or the user set with `/rename`.
    pub name: Option<String>,
    pub status: LiveStatus,
    pub started_at: Option<Timestamp>,
    pub status_updated_at: Option<Timestamp>,
}

impl LiveSession {
    /// Equal for change detection: a rewrite that only bumps timestamps is not a change.
    fn same_state(&self, other: &Self) -> bool {
        self.pid == other.pid
            && self.session_uuid == other.session_uuid
            && self.session_id == other.session_id
            && self.status == other.status
            && self.cwd == other.cwd
            && self.name == other.name
    }
}

/// The fields read from a session file. Unknown fields, including the socket path, are
/// skipped without being stored. Numbers and strings are both accepted where versions differ.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFile {
    pid: Option<serde_json::Value>,
    session_id: Option<String>,
    cwd: Option<String>,
    name: Option<String>,
    status: Option<String>,
    started_at: Option<i64>,
    status_updated_at: Option<i64>,
    proc_start: Option<serde_json::Value>,
    pid_domain: Option<String>,
}

/// A parsed session file, before the liveness check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFile {
    pub pid: u32,
    pub session_uuid: String,
    pub cwd: String,
    pub name: Option<String>,
    pub status: LiveStatus,
    pub started_at: Option<Timestamp>,
    pub status_updated_at: Option<Timestamp>,
    /// Process creation time as a FILETIME.
    pub proc_start: Option<u64>,
    pub pid_domain: Option<String>,
}

fn as_u64(v: &serde_json::Value) -> Option<u64> {
    match v {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Parses a session file. `file_pid` comes from the file name and is used when the content
/// has no usable pid. `None` for anything that is not a session file.
pub fn parse_session_file(bytes: &[u8], file_pid: u32) -> Option<SessionFile> {
    let raw: RawFile = serde_json::from_slice(bytes).ok()?;
    let pid = match raw.pid.as_ref().and_then(as_u64) {
        Some(p) => u32::try_from(p).ok()?,
        None => file_pid,
    };
    let session_uuid = raw.session_id.filter(|s| !s.is_empty())?;
    Some(SessionFile {
        pid,
        session_uuid,
        cwd: raw.cwd.unwrap_or_default(),
        name: raw.name.filter(|n| !n.is_empty()),
        status: LiveStatus::parse(raw.status.as_deref()),
        started_at: raw.started_at.map(Timestamp::from_millis),
        status_updated_at: raw.status_updated_at.map(Timestamp::from_millis),
        proc_start: raw.proc_start.as_ref().and_then(as_u64),
        pid_domain: raw.pid_domain,
    })
}

/// Does the probed process belong to this file? See the module docs.
pub fn is_live(file: &SessionFile, liveness: Liveness) -> bool {
    match liveness {
        Liveness::Dead => false,
        // Cannot tell: keep it rather than hide a session that may well be running.
        Liveness::Unknown => true,
        Liveness::Alive { created } => match (file.proc_start, file.started_at) {
            (Some(start), _) => start.abs_diff(created) <= PROC_START_TOLERANCE,
            (None, Some(started)) => {
                let created = filetime_to_unix_ms(created);
                let started = started.as_millis();
                (started - STARTED_AT_LEAD_MS..=started + STARTED_AT_LAG_MS).contains(&created)
            }
            (None, None) => true,
        },
    }
}

/// `<pid>.json`; everything else in the directory (`.key` files, temp files) is ignored.
fn file_pid(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    let stem = name.strip_suffix(".json")?;
    if stem.is_empty() || !stem.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    stem.parse().ok()
}

/// Is `path` a session file of `sessions_dir`? The watcher routes events with this.
pub fn is_session_file(sessions_dir: &Path, path: &Path) -> bool {
    path.parent() == Some(sessions_dir) && file_pid(path).is_some()
}

fn read_small(path: &Path) -> std::io::Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    let mut buf = Vec::with_capacity(1024);
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > MAX_FILE_BYTES {
        return Err(std::io::Error::other("session file too large"));
    }
    Ok(buf)
}

/// The running sessions of one Claude Code directory, without a store (the CLI while the app
/// is closed). A missing or unreadable directory has none.
pub fn running_sessions(config_dir: &Path) -> Vec<SessionFile> {
    scan(&config_dir.join(SESSIONS_DIR), probe).unwrap_or_default()
}

/// The session files in `sessions_dir` whose process is alive; a missing directory is empty.
fn scan(sessions_dir: &Path, probe: fn(u32) -> Liveness) -> std::io::Result<Vec<SessionFile>> {
    let entries = match std::fs::read_dir(sessions_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(pid) = file_pid(&path) else { continue };
        // Claude Code may be rewriting the file; a failed read just skips it this round.
        let Ok(bytes) = read_small(&path) else {
            continue;
        };
        let Some(file) = parse_session_file(&bytes, pid) else {
            tracing::debug!(path = %path.display(), "not a session file");
            continue;
        };
        if let (Some(domain), Some(local)) = (&file.pid_domain, LOCAL_PID_DOMAIN)
            && !domain.starts_with(local)
        {
            continue;
        }
        if is_live(&file, probe(file.pid)) {
            files.push(file);
        }
    }
    Ok(files)
}

/// Tracks the live sessions of one account.
pub struct LiveSessionTracker {
    account: AccountId,
    sessions_dir: PathBuf,
    store: Store,
    probe: fn(u32) -> Liveness,
    sessions: Vec<LiveSession>,
    resolved: HashMap<String, SessionId>,
}

impl LiveSessionTracker {
    /// `config_dir` is the account's Claude Code directory.
    pub fn new(account: AccountId, config_dir: &Path, store: Store) -> Self {
        Self::with_probe(account, config_dir, store, probe)
    }

    /// [`LiveSessionTracker::new`] with a custom liveness check, for tests.
    pub fn with_probe(
        account: AccountId,
        config_dir: &Path,
        store: Store,
        probe: fn(u32) -> Liveness,
    ) -> Self {
        Self {
            account,
            sessions_dir: config_dir.join(SESSIONS_DIR),
            store,
            probe,
            sessions: Vec::new(),
            resolved: HashMap::new(),
        }
    }

    pub fn sessions_dir(&self) -> &Path {
        &self.sessions_dir
    }

    /// Current list, ordered by pid.
    pub fn sessions(&self) -> &[LiveSession] {
        &self.sessions
    }

    /// Whether some live session has no indexed session yet.
    pub fn has_unresolved(&self) -> bool {
        self.sessions.iter().any(|s| s.session_id.is_none())
    }

    /// Re-reads the directory and checks every pid. Returns whether the list or any status
    /// changed (timestamps alone do not count).
    pub fn refresh(&mut self) -> bool {
        let files = match scan(&self.sessions_dir, self.probe) {
            Ok(files) => files,
            Err(error) => {
                tracing::warn!(%error, dir = %self.sessions_dir.display(), "could not list live sessions");
                return false;
            }
        };
        let mut next = Vec::with_capacity(files.len());
        for file in files {
            let session_id = self.resolve(&file.session_uuid);
            next.push(LiveSession {
                account: self.account,
                session_uuid: file.session_uuid,
                session_id,
                pid: file.pid,
                cwd: file.cwd,
                name: file.name,
                status: file.status,
                started_at: file.started_at,
                status_updated_at: file.status_updated_at,
            });
        }
        next.sort_by_key(|s| s.pid);
        self.replace(next)
    }

    fn replace(&mut self, next: Vec<LiveSession>) -> bool {
        let changed = next.len() != self.sessions.len()
            || next
                .iter()
                .zip(&self.sessions)
                .any(|(a, b)| !a.same_state(b));
        self.sessions = next;
        changed
    }

    fn resolve(&mut self, uuid: &str) -> Option<SessionId> {
        if let Some(id) = self.resolved.get(uuid) {
            return Some(*id);
        }
        match self.store.session_id_by_uuid(self.account, uuid) {
            Ok(Some(id)) => {
                self.resolved.insert(uuid.to_owned(), id);
                Some(id)
            }
            Ok(None) => None,
            Err(error) => {
                tracing::warn!(%error, "could not look up a live session");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests;
