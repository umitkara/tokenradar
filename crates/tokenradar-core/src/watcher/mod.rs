//! File-system watching: `notify` events, debounced into batches of dirty paths.
//!
//! - **Debounce:** the first event opens a window of [`DEBOUNCE`]; every path seen until it
//!   closes lands in one batch. The window is fixed, not extended by later events: Claude Code
//!   writes several times a second while streaming, and a sliding window would never close.
//! - **Hot files:** `notify` 8 on Windows drops a `ReadDirectoryChangesW` buffer overflow
//!   silently instead of reporting a rescan (ADR-027). Transcripts that changed recently are
//!   therefore re-checked with a cheap `stat` every [`HOT_INTERVAL`] while there are any; with
//!   no recent activity there is no timer at all.
//! - **Safety net:** the runtime also runs a full stat scan every [`SAFETY_SCAN`].
//!
//! The loop that routes batches to the indexer, the live-session tracker and the credentials
//! check lives in [`crate::runtime`].

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

/// Debounce window (architecture §5: at least 750 ms).
pub const DEBOUNCE: Duration = Duration::from_millis(750);

/// How often hot files are re-checked while there are any.
pub const HOT_INTERVAL: Duration = Duration::from_secs(30);

/// A file stays hot this long after its last change.
pub const HOT_TTL: Duration = Duration::from_mins(15);

/// Full stat scan interval: catches whatever the watcher missed.
pub const SAFETY_SCAN: Duration = Duration::from_mins(10);

/// Collects paths for one fixed window.
#[derive(Debug)]
pub struct Debouncer {
    window: Duration,
    pending: HashSet<PathBuf>,
    deadline: Option<Instant>,
}

impl Debouncer {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            pending: HashSet::new(),
            deadline: None,
        }
    }

    /// Records a path; the first one after a flush starts the window.
    pub fn add(&mut self, path: PathBuf, now: Instant) {
        self.pending.insert(path);
        self.deadline.get_or_insert(now + self.window);
    }

    /// When the current window closes, if one is open.
    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// The batch, once its window has closed.
    pub fn take_due(&mut self, now: Instant) -> Option<HashSet<PathBuf>> {
        match self.deadline {
            Some(deadline) if now >= deadline => {
                self.deadline = None;
                Some(std::mem::take(&mut self.pending))
            }
            _ => None,
        }
    }
}

/// Size and modification time; enough to tell whether a file changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub size: u64,
    pub mtime: Option<std::time::SystemTime>,
}

impl Stamp {
    pub fn of(path: &Path) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        Some(Self {
            size: meta.len(),
            mtime: meta.modified().ok(),
        })
    }
}

/// Recently changed transcripts and their last known [`Stamp`].
#[derive(Debug)]
pub struct HotFiles {
    ttl: Duration,
    files: HashMap<PathBuf, (Option<Stamp>, Instant)>,
}

impl HotFiles {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            files: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Notes that `path` changed now and what it looks like.
    pub fn touch(&mut self, path: PathBuf, stamp: Option<Stamp>, now: Instant) {
        self.files.insert(path, (stamp, now));
    }

    /// Stats every hot file and returns those that changed since they were last seen. Files
    /// quiet for longer than the TTL are dropped.
    pub fn changed(&mut self, now: Instant, stat: impl Fn(&Path) -> Option<Stamp>) -> Vec<PathBuf> {
        let mut out = Vec::new();
        self.files.retain(|path, (stamp, last)| {
            let current = stat(path);
            if current != *stamp {
                *stamp = current;
                *last = now;
                out.push(path.clone());
            }
            // A vanished file is reported once, then forgotten.
            current.is_some() && now.duration_since(*last) < self.ttl
        });
        out
    }
}

/// What the `notify` callback forwards to the watch loop.
#[derive(Debug)]
pub enum FsSignal {
    Path(PathBuf),
    /// Events may have been lost; rescan everything.
    Rescan,
}

/// A `notify` watcher whose events become [`FsSignal`]s via `wrap`.
pub struct FsWatcher {
    inner: RecommendedWatcher,
    watched: HashSet<PathBuf>,
}

impl FsWatcher {
    pub fn new<M: Send + 'static>(
        tx: mpsc::Sender<M>,
        wrap: fn(FsSignal) -> M,
    ) -> notify::Result<Self> {
        let inner =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
                Ok(event) => {
                    if event.need_rescan() {
                        let _ = tx.send(wrap(FsSignal::Rescan));
                    }
                    for path in event.paths {
                        let _ = tx.send(wrap(FsSignal::Path(path)));
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "file watcher error; rescanning");
                    let _ = tx.send(wrap(FsSignal::Rescan));
                }
            })?;
        Ok(Self {
            inner,
            watched: HashSet::new(),
        })
    }

    /// Watches `dir` unless it already is or does not exist. Returns whether it is watched.
    pub fn ensure(&mut self, dir: &Path, recursive: bool) -> bool {
        if self.watched.contains(dir) {
            return true;
        }
        if !dir.is_dir() {
            return false;
        }
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        match self.inner.watch(dir, mode) {
            Ok(()) => {
                tracing::debug!(dir = %dir.display(), recursive, "watching");
                self.watched.insert(dir.to_path_buf());
                true
            }
            Err(error) => {
                tracing::warn!(%error, dir = %dir.display(), "could not watch directory");
                false
            }
        }
    }

    pub fn is_watched(&self, dir: &Path) -> bool {
        self.watched.contains(dir)
    }

    /// Forgets a directory that disappeared, so [`FsWatcher::ensure`] re-adds it later.
    pub fn forget(&mut self, dir: &Path) {
        if self.watched.remove(dir) {
            let _ = self.inner.unwatch(dir);
        }
    }
}

#[cfg(test)]
mod tests;
