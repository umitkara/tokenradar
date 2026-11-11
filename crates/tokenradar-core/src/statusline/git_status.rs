//! The working tree's state for the statusline's `git_status` item (Step 12r, ADR-059).
//!
//! `git status` is a process start (≈ 20–80 ms on Windows), past the statusline's latency
//! budget, so the app runs it, not the CLI: in the background, cached per repository. A query
//! never waits for git. With nothing usable cached it starts a run and the item stays hidden
//! that round; the next draw shows it.
//!
//! ```text
//! git --no-optional-locks status --porcelain=v2 --branch -z [-uno]
//! ```
//!
//! `--no-optional-locks` keeps git from taking `index.lock` to refresh the index, which would
//! make the user's own `git commit` at that moment fail. Only counts are kept: file names go
//! by in the output but are skipped, never copied, logged or sent.

use std::collections::VecDeque;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

use super::git;

/// A run that takes longer is killed and the item stays hidden that round.
pub const TIMEOUT: Duration = Duration::from_secs(2);
/// How long a result counts as current while `HEAD` and `index` are unchanged. Untracked files
/// change without touching the index, so it is not forever.
pub const TTL: Duration = Duration::from_secs(5);
/// A result past [`TTL`] is still shown, while a new run goes on, until it is this old.
pub const SERVE_STALE: Duration = Duration::from_mins(1);
/// Repositories kept, least recently asked dropped first.
pub const MAX_REPOS: usize = 8;
/// Timeouts in a row after which a repository is scanned without untracked files.
pub const TIMEOUTS_TO_SKIP_UNTRACKED: u8 = 2;
/// The most output read; a tree with more changes than this is counted up to here.
const MAX_OUTPUT: u64 = 32 * 1024 * 1024;

/// What `git status` says, in numbers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatus {
    /// Changes in the index.
    pub staged: u32,
    /// Changes in the working tree not in the index.
    pub modified: u32,
    pub untracked: u32,
    /// Unmerged paths.
    #[serde(default)]
    pub conflicted: u32,
    /// Commits ahead of and behind the upstream; `None` without one.
    #[serde(default)]
    pub ahead: Option<u32>,
    #[serde(default)]
    pub behind: Option<u32>,
    /// Untracked files were not scanned (the item's choice, or two timeouts in a row): the
    /// count is unknown, not 0.
    #[serde(default)]
    pub untracked_skipped: bool,
}

impl GitStatus {
    /// Nothing to commit and nothing to push or pull.
    pub fn is_clean(&self) -> bool {
        self.staged == 0
            && self.modified == 0
            && self.untracked == 0
            && self.conflicted == 0
            && self.ahead.unwrap_or(0) == 0
            && self.behind.unwrap_or(0) == 0
    }

    /// Something in the working tree or the index differs from `HEAD`.
    pub fn is_dirty(&self) -> bool {
        self.staged > 0 || self.modified > 0 || self.untracked > 0 || self.conflicted > 0
    }
}

/// Counts `--porcelain=v2 --branch -z` output. Unknown records are skipped (ADR-042).
pub fn parse(out: &[u8]) -> GitStatus {
    let mut s = GitStatus::default();
    let mut fields = out.split(|&b| b == 0);
    while let Some(rec) = fields.next() {
        match rec.first() {
            Some(b'#') => {
                if let Some(ab) = rec.strip_prefix(b"# branch.ab ") {
                    let mut parts = ab.split(|&b| b == b' ');
                    s.ahead = parts.next().and_then(|p| number(p.strip_prefix(b"+")?));
                    s.behind = parts.next().and_then(|p| number(p.strip_prefix(b"-")?));
                }
            }
            Some(b'1' | b'2') => {
                let (x, y) = (rec.get(2).copied(), rec.get(3).copied());
                if x.is_some_and(|c| c != b'.') {
                    s.staged = s.staged.saturating_add(1);
                }
                if y.is_some_and(|c| c != b'.') {
                    s.modified = s.modified.saturating_add(1);
                }
                // A rename or copy carries the original path as the next field.
                if rec[0] == b'2' {
                    fields.next();
                }
            }
            Some(b'u') => s.conflicted = s.conflicted.saturating_add(1),
            Some(b'?') => s.untracked = s.untracked.saturating_add(1),
            _ => {}
        }
    }
    s
}

fn number(b: &[u8]) -> Option<u32> {
    std::str::from_utf8(b).ok()?.parse().ok()
}

/// The arguments of the run; a test checks `--no-optional-locks` is there.
pub fn args(untracked: bool) -> Vec<&'static str> {
    let mut a = vec![
        "--no-optional-locks",
        "status",
        "--porcelain=v2",
        "--branch",
        "-z",
    ];
    if !untracked {
        a.push("-uno");
    }
    a
}

/// Why a run gave no result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunError {
    /// No `git` on `PATH`.
    Missing,
    TimedOut,
    /// git ran and failed (not a repository after all, a broken one, …).
    Failed,
}

/// Runs git in `root`; replaced in tests.
pub type Runner = dyn Fn(&Path, bool) -> Result<GitStatus, RunError> + Send + Sync;

/// Runs the real `git` from `PATH`, without a console window, killed after [`TIMEOUT`].
pub fn run_git(root: &Path, untracked: bool) -> Result<GitStatus, RunError> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(root)
        .args(args(untracked))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => RunError::Missing,
        _ => RunError::Failed,
    })?;
    let Some(mut stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(RunError::Failed);
    };
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let ok = (&mut stdout).take(MAX_OUTPUT).read_to_end(&mut buf).is_ok();
        let _ = tx.send(ok.then_some(buf));
    });
    let Ok(out) = rx.recv_timeout(TIMEOUT) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(RunError::TimedOut);
    };
    let status = child.wait().map_err(|_| RunError::Failed)?;
    match out {
        Some(out) if status.success() => Ok(parse(&out)),
        _ => Err(RunError::Failed),
    }
}

/// Whether `git` can be started from `PATH` (for doctor).
pub fn git_available() -> bool {
    let mut cmd = Command::new("git");
    cmd.arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.status().is_ok_and(|s| s.success())
}

/// `HEAD` and `index` modification times.
type Stamp = (Option<SystemTime>, Option<SystemTime>);

/// What the cache knows about one repository.
struct Entry {
    root: PathBuf,
    /// `HEAD` and `index` modification times when the result was taken.
    stamp: Stamp,
    result: Option<(GitStatus, Instant)>,
    running: bool,
    timeouts: u8,
    /// Untracked files are skipped since [`TIMEOUTS_TO_SKIP_UNTRACKED`] timeouts.
    untracked_off: bool,
}

#[derive(Default)]
struct State {
    /// Most recently asked first.
    entries: VecDeque<Entry>,
    git_missing: bool,
}

/// Results per repository, computed in the background.
#[derive(Clone)]
pub struct GitStatusCache {
    state: Arc<Mutex<State>>,
    runner: Arc<Runner>,
    ttl: Duration,
}

impl Default for GitStatusCache {
    fn default() -> Self {
        Self::new(Arc::new(run_git))
    }
}

impl GitStatusCache {
    pub fn new(runner: Arc<Runner>) -> Self {
        Self {
            state: Arc::default(),
            runner,
            ttl: TTL,
        }
    }

    #[cfg(test)]
    fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The status of the working tree holding `cwd`, if one is cached and usable; starts a
    /// background run when the cached one is missing or old. `untracked`: the item wants
    /// untracked files counted. Never waits for git.
    pub fn get(&self, cwd: &Path, untracked: bool) -> Option<GitStatus> {
        let (root, gitdir) = git::locate(cwd)?;
        let stamp = stamp_of(&gitdir);
        let mut state = self.lock();
        if state.git_missing {
            return None;
        }
        let idx = state.entries.iter().position(|e| e.root == root);
        let mut entry = match idx.and_then(|i| state.entries.remove(i)) {
            Some(e) => e,
            None => Entry {
                root: root.clone(),
                stamp,
                result: None,
                running: false,
                timeouts: 0,
                untracked_off: false,
            },
        };
        let skip_untracked = !untracked || entry.untracked_off;
        let shown = entry.result.and_then(|(status, at)| {
            let same_tree = entry.stamp == stamp;
            let same_scan = status.untracked_skipped == skip_untracked;
            let age = at.elapsed();
            let current = same_tree && same_scan && age < self.ttl;
            let servable = same_tree && same_scan && age < SERVE_STALE;
            servable.then_some((status, current))
        });
        let start = !entry.running && !shown.is_some_and(|(_, current)| current);
        if start {
            entry.running = true;
        }
        state.entries.push_front(entry);
        state.entries.truncate(MAX_REPOS);
        drop(state);
        if start {
            self.spawn(&root, stamp, !skip_untracked);
        }
        shown.map(|(s, _)| s)
    }

    fn spawn(&self, root: &Path, stamp: Stamp, untracked: bool) {
        let cache = self.clone();
        let theirs = root.to_path_buf();
        let spawned = std::thread::Builder::new()
            .name("git-status".into())
            .spawn(move || {
                let result = (cache.runner)(&theirs, untracked);
                cache.finish(&theirs, stamp, untracked, result);
            });
        if spawned.is_err() {
            let mut state = self.lock();
            if let Some(e) = state.entries.iter_mut().find(|e| e.root == root) {
                e.running = false;
            }
        }
    }

    fn finish(
        &self,
        root: &Path,
        stamp: Stamp,
        untracked: bool,
        result: Result<GitStatus, RunError>,
    ) {
        let mut state = self.lock();
        if result == Err(RunError::Missing) {
            state.git_missing = true;
        }
        let Some(e) = state.entries.iter_mut().find(|e| e.root == root) else {
            return;
        };
        e.running = false;
        match result {
            Ok(mut status) => {
                status.untracked_skipped = !untracked;
                if !untracked {
                    status.untracked = 0;
                }
                e.timeouts = 0;
                e.stamp = stamp;
                e.result = Some((status, Instant::now()));
            }
            Err(RunError::TimedOut) => {
                e.timeouts = e.timeouts.saturating_add(1);
                if untracked && e.timeouts >= TIMEOUTS_TO_SKIP_UNTRACKED {
                    tracing::info!("git status timed out twice; untracked files are skipped");
                    e.untracked_off = true;
                    e.timeouts = 0;
                }
                e.result = None;
            }
            Err(_) => e.result = None,
        }
    }

    /// Whether a run found no `git` on `PATH`.
    pub fn git_missing(&self) -> bool {
        self.lock().git_missing
    }

    /// Whether the repository holding `cwd` is scanned without untracked files since its
    /// runs timed out.
    pub fn untracked_off(&self, cwd: &Path) -> bool {
        let Some((root, _)) = git::locate(cwd) else {
            return false;
        };
        self.lock()
            .entries
            .iter()
            .any(|e| e.root == root && e.untracked_off)
    }
}

fn stamp_of(gitdir: &Path) -> Stamp {
    let mtime = |name: &str| std::fs::metadata(gitdir.join(name)).ok()?.modified().ok();
    (mtime("HEAD"), mtime("index"))
}

#[cfg(test)]
mod tests;
