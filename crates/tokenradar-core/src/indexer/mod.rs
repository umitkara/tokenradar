//! Incremental JSONL indexer that extracts usage metadata only, never conversation content.
//!
//! Pipeline (architecture §5, ADR-007, ADR-024):
//!
//! 1. **Inventory:** stat every transcript under `<config>/projects` and compare `(size, mtime)`
//!    with `source_files`. Unchanged files are skipped; files that shrank are re-read from the
//!    start (deduplication makes that idempotent); vanished files are marked deleted.
//! 2. **Queue:** changed files, newest first, so the current window is ready within seconds.
//! 3. **Workers** (`min(4, cores/2)`) read each file from its offset and parse lines into
//!    owned records, sending them in chunks. Files from the last 7 days run at normal priority,
//!    older ones at background priority (ADR-026).
//! 4. **Coordinator** (the calling thread) resolves sessions, projects and models, prices the
//!    events and flushes batches to the store writer every 5,000 events or 250 ms. Each batch
//!    writes events and the matching file offsets in one transaction, so after a crash a file
//!    is never marked as read past events that were lost.

mod reader;
mod retention;

pub mod priority;

use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{self, Seek, SeekFrom};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

pub use reader::{CHUNK, LineReader, MAX_LINE, ReadStats};
pub use retention::{CHECK_EVERY, DEFAULT_KEEP_DAYS, RetentionScheduler};

use serde::Serialize;
use specta::Type;

use crate::domain::{
    AccountId, ModelId, MsgKey, ProjectId, ProjectSpec, SessionId, SessionKind, SessionSpec,
    SessionTitle, Timestamp, TitleSource, Usage, UsageEvent,
};
use crate::pricing::{Pricing, family_of};
use crate::providers::claude::{
    LineParser, MetaRecord, ParseOutcome, ProjectKey, SourceKind, UnknownReason, classify_path,
};
use crate::store::{IndexBatch, SourceFileState, Store, StoreError, UpsertStats};

/// Events a worker collects before handing a chunk to the coordinator.
const WORKER_CHUNK_EVENTS: usize = 2_000;

/// Chunks in flight between workers and the coordinator; bounds memory when the writer lags.
const CHANNEL_CHUNKS: usize = 16;

/// A file this large that yields no usage records hints at a format change.
const DRIFT_MIN_BYTES: u64 = 16 << 20;

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("could not list transcripts: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub workers: usize,
    /// Flush after this many events …
    pub batch_events: usize,
    /// … or this long after the last flush, whichever comes first.
    pub flush_interval: Duration,
    /// Files modified within this window make up [`IndexMilestone::RecentReady`].
    pub recent_window: Duration,
    /// Drop workers to background CPU and I/O priority once they reach files older than
    /// `recent_window`. Recent files always run at normal priority so the current window is
    /// ready within the 5 s budget (ADR-026); `false` keeps everything at normal priority.
    pub background: bool,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            workers: default_workers(),
            batch_events: 5_000,
            flush_interval: Duration::from_millis(250),
            recent_window: Duration::from_hours(7 * 24),
            background: true,
        }
    }
}

/// `min(4, cores / 2)`, at least one (ADR-024: the disk is the bottleneck beyond four).
pub fn default_workers() -> usize {
    let cores = thread::available_parallelism().map_or(2, std::num::NonZero::get);
    (cores / 2).clamp(1, 4)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum IndexPhase {
    Inventory,
    Indexing,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct IndexProgress {
    pub files_done: usize,
    pub files_total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub phase: IndexPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum IndexMilestone {
    /// Every file modified within [`IndexerConfig::recent_window`] is indexed: the current
    /// 5-hour window and this week can be shown.
    RecentReady,
    /// The first full scan after start finished. Reported by the runtime, not the indexer.
    Complete,
}

/// Progress callback payload; the runtime forwards it to the event bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexEvent {
    Progress(IndexProgress),
    Milestone(IndexMilestone),
}

/// Line-level counters. Unknown and suspect counts are schema-drift signals (§15).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ParseStats {
    pub lines: u64,
    pub usage_records: u64,
    pub advisor_records: u64,
    pub title_records: u64,
    pub malformed: u64,
    pub bad_timestamps: u64,
    /// Usage lines that only the loose pre-filter caught (ADR-024).
    pub prefilter_misses: u64,
    /// Usage lines without `message.id`, keyed by a fallback hash.
    pub fallback_keys: u64,
    pub oversized_lines: u64,
    /// Oversized lines that might have carried usage.
    pub oversized_suspect: u64,
}

impl ParseStats {
    fn add(&mut self, o: &Self) {
        self.lines += o.lines;
        self.usage_records += o.usage_records;
        self.advisor_records += o.advisor_records;
        self.title_records += o.title_records;
        self.malformed += o.malformed;
        self.bad_timestamps += o.bad_timestamps;
        self.prefilter_misses += o.prefilter_misses;
        self.fallback_keys += o.fallback_keys;
        self.oversized_lines += o.oversized_lines;
        self.oversized_suspect += o.oversized_suspect;
    }

    pub fn unknown(&self) -> u64 {
        self.malformed + self.bad_timestamps
    }
}

/// What one indexing pass did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IndexReport {
    pub files_seen: usize,
    pub files_unchanged: usize,
    pub files_indexed: usize,
    pub files_truncated: usize,
    pub files_deleted: usize,
    pub files_failed: usize,
    pub bytes_read: u64,
    pub events: UpsertStats,
    pub parse: ParseStats,
    /// Sessions that received at least one event in this pass.
    pub sessions: BTreeSet<SessionId>,
    /// Earliest and latest timestamp among those events.
    pub ts_range: Option<(Timestamp, Timestamp)>,
    /// The pass stopped early because [`Indexer::set_cancel`]'s flag was raised. Everything
    /// reported was stored; the remaining files are picked up by the next pass.
    pub cancelled: bool,
}

fn add_upsert(a: &mut UpsertStats, b: UpsertStats) {
    a.inserted += b.inserted;
    a.updated += b.updated;
    a.kept_existing += b.kept_existing;
    a.skipped_pruned += b.skipped_pruned;
}

/// A transcript on disk.
#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    key: String,
    kind: SourceKind,
    size: u64,
    mtime: i64,
}

/// A file to read from `start` to `entry.size`.
#[derive(Debug)]
struct Job {
    entry: FileEntry,
    start: u64,
}

/// One response, owned, ready for the coordinator.
#[derive(Debug)]
struct RawEvent {
    key: MsgKey,
    ts: Timestamp,
    model: String,
    usage: Usage,
}

/// Part of a file's records, up to `offset`.
#[derive(Debug, Default)]
struct Chunk {
    /// Deduplicated within the chunk (see [`Chunk::push`]).
    events: Vec<RawEvent>,
    /// Position of each key in `events`.
    seen: HashMap<MsgKey, usize>,
    titles: Vec<(TitleSource, String)>,
    /// First cwd seen in this chunk.
    cwd: Option<String>,
    git_branch: Option<String>,
    version: Option<String>,
    entrypoint: Option<String>,
    first_ts: Option<Timestamp>,
    last_ts: Option<Timestamp>,
    offset: u64,
    stats: ParseStats,
}

impl Chunk {
    /// Adds an event, collapsing repeats of a key the way the store would: the larger
    /// `output` wins, ties go to the later line. All lines of a file belong to one session,
    /// so this is exactly the store's same-session rule. Claude Code writes one line per
    /// content block, so this removes more than half of the events before they reach the
    /// writer (Step 04 measurement).
    fn push(&mut self, event: RawEvent) {
        if let Some(&i) = self.seen.get(&event.key) {
            if event.usage.tokens.output >= self.events[i].usage.tokens.output {
                self.events[i] = event;
            }
        } else {
            self.seen.insert(event.key, self.events.len());
            self.events.push(event);
        }
    }
}

enum WorkerMsg {
    Chunk {
        job: usize,
        chunk: Box<Chunk>,
        last: bool,
    },
    Failed {
        job: usize,
        error: io::Error,
    },
}

/// Indexes one Claude Code config directory into the store. Not `Sync`: one pass at a time.
pub struct Indexer {
    store: Store,
    account: AccountId,
    projects_dir: PathBuf,
    config: IndexerConfig,
    cancel: Arc<AtomicBool>,
    sessions: HashMap<String, SessionId>,
    projects: HashMap<String, ProjectId>,
    models: HashMap<String, ModelId>,
}

impl Indexer {
    /// `config_dir` is the account's Claude Code directory, e.g. `~/.claude`.
    pub fn new(store: Store, account: AccountId, config_dir: &Path, config: IndexerConfig) -> Self {
        Self {
            store,
            account,
            projects_dir: config_dir.join(crate::providers::claude::PROJECTS_DIR),
            config,
            cancel: Arc::new(AtomicBool::new(false)),
            sessions: HashMap::new(),
            projects: HashMap::new(),
            models: HashMap::new(),
        }
    }

    /// Shares a cancellation flag. Once it is set, workers finish the file they are reading and
    /// take no new one; the pass stores what it has and returns with `cancelled` set.
    pub fn set_cancel(&mut self, flag: Arc<AtomicBool>) {
        self.cancel = flag;
    }

    /// Inventories every transcript and indexes what changed, newest first.
    pub fn full_scan(
        &mut self,
        on_event: &mut dyn FnMut(IndexEvent),
    ) -> Result<IndexReport, IndexError> {
        on_event(IndexEvent::Progress(IndexProgress {
            files_done: 0,
            files_total: 0,
            bytes_done: 0,
            bytes_total: 0,
            phase: IndexPhase::Inventory,
        }));
        let entries = inventory(&self.projects_dir)?;
        let stored = self.store.source_files(self.account)?;
        let mut report = IndexReport {
            files_seen: entries.len(),
            ..IndexReport::default()
        };

        let present: std::collections::HashSet<&str> =
            entries.iter().map(|e| e.key.as_str()).collect();
        let vanished: Vec<String> = stored
            .values()
            .filter(|f| f.deleted_at.is_none() && !present.contains(f.path.as_str()))
            .map(|f| f.path.clone())
            .collect();
        report.files_deleted = vanished.len();
        if !vanished.is_empty() {
            self.store
                .writer()
                .mark_files_deleted(vanished, Timestamp::now())?;
        }

        let jobs = plan_jobs(entries, &stored, &mut report);
        self.run(&jobs, report, on_event)
    }

    /// Indexes the given files incrementally, as [`Indexer::full_scan`] does for changed
    /// files. Paths outside the transcript layout are ignored; missing files are marked
    /// deleted.
    pub fn index_files(
        &mut self,
        paths: &[PathBuf],
        on_event: &mut dyn FnMut(IndexEvent),
    ) -> Result<IndexReport, IndexError> {
        // Look up only these files: the watcher calls this every few seconds with a handful of
        // paths, and loading every row of the account each time would dominate the pass.
        let keys: Vec<String> = paths.iter().map(|p| path_key(p)).collect();
        let stored = self.store.source_files_at(self.account, &keys)?;
        let mut entries = Vec::new();
        let mut vanished = Vec::new();
        for path in paths {
            let Some(kind) = classify_path(&self.projects_dir, path) else {
                continue;
            };
            match std::fs::metadata(path) {
                Ok(meta) if meta.is_file() => entries.push(FileEntry {
                    key: path_key(path),
                    path: path.clone(),
                    kind,
                    size: meta.len(),
                    mtime: mtime_ms(&meta),
                }),
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    let key = path_key(path);
                    if stored.get(&key).is_some_and(|f| f.deleted_at.is_none()) {
                        vanished.push(key);
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        let mut report = IndexReport {
            files_seen: entries.len(),
            files_deleted: vanished.len(),
            ..IndexReport::default()
        };
        if !vanished.is_empty() {
            self.store
                .writer()
                .mark_files_deleted(vanished, Timestamp::now())?;
        }
        let jobs = plan_jobs(entries, &stored, &mut report);
        self.run(&jobs, report, on_event)
    }

    fn run(
        &mut self,
        jobs: &[Job],
        mut report: IndexReport,
        on_event: &mut dyn FnMut(IndexEvent),
    ) -> Result<IndexReport, IndexError> {
        let mut progress = IndexProgress {
            files_done: 0,
            files_total: jobs.len(),
            bytes_done: 0,
            bytes_total: jobs.iter().map(|j| j.entry.size - j.start).sum(),
            phase: IndexPhase::Indexing,
        };
        on_event(IndexEvent::Progress(progress));

        let recent_cutoff = recent_cutoff(self.config.recent_window);
        let mut recent_left = jobs
            .iter()
            .filter(|j| j.entry.mtime >= recent_cutoff)
            .count();
        if recent_left == 0 {
            on_event(IndexEvent::Milestone(IndexMilestone::RecentReady));
        }

        let workers = self.config.workers.max(1).min(jobs.len().max(1));
        let next = AtomicUsize::new(0);
        let (tx, rx) = mpsc::sync_channel::<WorkerMsg>(CHANNEL_CHUNKS);
        let background = self.config.background;
        let cancel = Arc::clone(&self.cancel);
        let mut state = Coordinator::new(jobs.len());

        let outcome = thread::scope(|scope| -> Result<(), IndexError> {
            for i in 0..workers {
                let tx = tx.clone();
                let next = &next;
                let cancel = &*cancel;
                thread::Builder::new()
                    .name(format!("indexer-{i}"))
                    .spawn_scoped(scope, move || {
                        worker(jobs, next, &tx, background.then_some(recent_cutoff), cancel);
                    })?;
            }
            drop(tx);

            let mut last_flush = Instant::now();
            loop {
                let msg = match rx.recv_timeout(self.config.flush_interval) {
                    Ok(msg) => Some(msg),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => break,
                };
                if let Some(msg) = msg {
                    self.handle(msg, jobs, &mut state, &mut report)?;
                }
                if state.batch.events.len() >= self.config.batch_events
                    || last_flush.elapsed() >= self.config.flush_interval
                {
                    let done = self.flush(&mut state, &mut report)?;
                    last_flush = Instant::now();
                    Self::finish_jobs(
                        jobs,
                        done,
                        &mut progress,
                        &mut recent_left,
                        recent_cutoff,
                        on_event,
                    );
                }
            }
            let done = self.flush(&mut state, &mut report)?;
            Self::finish_jobs(
                jobs,
                done,
                &mut progress,
                &mut recent_left,
                recent_cutoff,
                on_event,
            );
            Ok(())
        });
        // On error the workers still finish their current job: dropping `rx` makes their
        // sends fail and they stop.
        outcome?;

        progress.phase = IndexPhase::Done;
        on_event(IndexEvent::Progress(progress));
        report.files_indexed = progress.files_done;
        report.cancelled = progress.files_done + report.files_failed < jobs.len();
        warn_on_drift(&report);
        self.store.release_memory()?;
        Ok(report)
    }

    /// Applies one worker message to the pending batch.
    fn handle(
        &mut self,
        msg: WorkerMsg,
        jobs: &[Job],
        state: &mut Coordinator,
        report: &mut IndexReport,
    ) -> Result<(), IndexError> {
        let (job, chunk, last) = match msg {
            WorkerMsg::Chunk { job, chunk, last } => (job, chunk, last),
            WorkerMsg::Failed { job, error } => {
                tracing::warn!(path = %jobs[job].entry.path.display(), %error, "could not read transcript");
                report.files_failed += 1;
                state.done.push(job);
                return Ok(());
            }
        };
        let entry = &jobs[job].entry;
        report.parse.add(&chunk.stats);
        report.bytes_read += chunk.offset - state.offsets[job].unwrap_or(jobs[job].start);
        state.offsets[job] = Some(chunk.offset);

        let session = self.session_for(&entry.kind)?;
        let mut spec = state
            .sessions
            .remove(&session)
            .unwrap_or_else(|| self.session_spec(&entry.kind));

        if !state.project_known[job] {
            let key = match &chunk.cwd {
                Some(cwd) => ProjectKey::from_cwd(cwd),
                // A file that produced events but never a cwd: fall back to the directory name.
                None if last && (state.had_events[job] || !chunk.events.is_empty()) => {
                    Some(ProjectKey::from_dir_name(entry.kind.project_dir()))
                }
                None => None,
            };
            if let Some(key) = key {
                spec.project_id = Some(self.project_for(key)?);
                state.project_known[job] = true;
            }
        }
        merge_chunk_meta(&mut spec, &chunk);
        state.had_events[job] |= !chunk.events.is_empty();
        if let (Some(first), Some(last)) = (chunk.first_ts, chunk.last_ts) {
            report.sessions.insert(session);
            report.ts_range = Some(match report.ts_range {
                Some((a, b)) => (a.min(first), b.max(last)),
                None => (first, last),
            });
        }

        for raw in chunk.events {
            let model_id = self.model_for(&raw.model)?;
            let cost = Pricing::claude().cost(&raw.model, &raw.usage).cost;
            state.batch.events.push(UsageEvent {
                account_id: self.account,
                session_id: session,
                model_id,
                ts: raw.ts,
                msg_key: raw.key,
                usage: raw.usage,
                cost,
            });
        }
        state.sessions.insert(session, spec);
        state.files.insert(
            job,
            SourceFileState {
                account_id: self.account,
                path: entry.key.clone(),
                kind: match entry.kind {
                    SourceKind::Main { .. } => SessionKind::Main,
                    SourceKind::Subagent { .. } => SessionKind::Subagent,
                },
                size: entry.size,
                mtime: entry.mtime,
                offset: chunk.offset,
                session_id: Some(session),
                deleted_at: None,
            },
        );
        if last {
            state.done.push(job);
        }
        Ok(())
    }

    /// Commits the pending batch. Returns the jobs that are now completely stored.
    fn flush(
        &mut self,
        state: &mut Coordinator,
        report: &mut IndexReport,
    ) -> Result<Vec<usize>, IndexError> {
        if state.batch.events.is_empty() && state.sessions.is_empty() && state.files.is_empty() {
            return Ok(std::mem::take(&mut state.done));
        }
        let mut batch = std::mem::take(&mut state.batch);
        batch.sessions = state.sessions.drain().map(|(_, s)| s).collect();
        batch.files = state.files.drain().map(|(_, f)| f).collect();
        let upserted = self.store.writer().commit_index_batch(batch)?;
        add_upsert(&mut report.events, upserted);
        Ok(std::mem::take(&mut state.done))
    }

    fn finish_jobs(
        jobs: &[Job],
        done: Vec<usize>,
        progress: &mut IndexProgress,
        recent_left: &mut usize,
        recent_cutoff: i64,
        on_event: &mut dyn FnMut(IndexEvent),
    ) {
        if done.is_empty() {
            return;
        }
        let mut recent_ready = false;
        for job in done {
            let entry = &jobs[job].entry;
            progress.files_done += 1;
            progress.bytes_done += entry.size - jobs[job].start;
            if entry.mtime >= recent_cutoff && *recent_left > 0 {
                *recent_left -= 1;
                recent_ready = *recent_left == 0;
            }
        }
        on_event(IndexEvent::Progress(*progress));
        if recent_ready {
            on_event(IndexEvent::Milestone(IndexMilestone::RecentReady));
        }
    }

    /// Id of the file's session, creating it (and a placeholder parent for a subagent whose
    /// main file has not been indexed yet) on first sight.
    fn session_for(&mut self, kind: &SourceKind) -> Result<SessionId, IndexError> {
        let uuid = match kind {
            SourceKind::Main { session, .. } => session,
            SourceKind::Subagent { agent, .. } => agent,
        };
        if let Some(id) = self.sessions.get(uuid) {
            return Ok(*id);
        }
        let spec = match kind {
            SourceKind::Main { session, .. } => {
                SessionSpec::new(self.account, session.clone(), SessionKind::Main)
            }
            SourceKind::Subagent { parent, agent, .. } => {
                let parent_id = self.main_session(parent)?;
                let mut spec = SessionSpec::new(self.account, agent.clone(), SessionKind::Subagent);
                spec.parent_id = Some(parent_id);
                spec.root_id = Some(parent_id);
                spec
            }
        };
        let id = self.store.writer().upsert_session(spec)?;
        self.sessions.insert(uuid.clone(), id);
        Ok(id)
    }

    fn main_session(&mut self, uuid: &str) -> Result<SessionId, IndexError> {
        if let Some(id) = self.sessions.get(uuid) {
            return Ok(*id);
        }
        let id = self.store.writer().upsert_session(SessionSpec::new(
            self.account,
            uuid,
            SessionKind::Main,
        ))?;
        self.sessions.insert(uuid.to_owned(), id);
        Ok(id)
    }

    /// The identity part of a session spec; metadata is merged in per chunk.
    /// Call after [`Indexer::session_for`], which caches the parent.
    fn session_spec(&self, kind: &SourceKind) -> SessionSpec {
        match kind {
            SourceKind::Main { session, .. } => {
                SessionSpec::new(self.account, session.clone(), SessionKind::Main)
            }
            SourceKind::Subagent { parent, agent, .. } => {
                let parent_id = self.sessions.get(parent).copied();
                let mut spec = SessionSpec::new(self.account, agent.clone(), SessionKind::Subagent);
                spec.parent_id = parent_id;
                spec.root_id = parent_id;
                spec
            }
        }
    }

    fn project_for(&mut self, key: ProjectKey) -> Result<ProjectId, IndexError> {
        if let Some(id) = self.projects.get(&key.path_norm) {
            return Ok(*id);
        }
        let id = self.store.writer().upsert_project(ProjectSpec {
            account_id: self.account,
            path_norm: key.path_norm.clone(),
            display_name: key.display_name,
        })?;
        self.projects.insert(key.path_norm, id);
        Ok(id)
    }

    fn model_for(&mut self, name: &str) -> Result<ModelId, IndexError> {
        if let Some(id) = self.models.get(name) {
            return Ok(*id);
        }
        let id = self
            .store
            .writer()
            .upsert_model(name.to_owned(), family_of(name))?;
        self.models.insert(name.to_owned(), id);
        Ok(id)
    }
}

/// Coordinator state between flushes.
struct Coordinator {
    batch: IndexBatch,
    sessions: HashMap<SessionId, SessionSpec>,
    files: HashMap<usize, SourceFileState>,
    /// Jobs whose last chunk is in the pending batch.
    done: Vec<usize>,
    offsets: Vec<Option<u64>>,
    project_known: Vec<bool>,
    had_events: Vec<bool>,
}

impl Coordinator {
    fn new(jobs: usize) -> Self {
        Self {
            batch: IndexBatch::default(),
            sessions: HashMap::new(),
            files: HashMap::new(),
            done: Vec::new(),
            offsets: vec![None; jobs],
            project_known: vec![false; jobs],
            had_events: vec![false; jobs],
        }
    }
}

/// Folds a chunk's metadata into the pending session spec. Later values win, except titles,
/// which follow [`TitleSource::rank`].
fn merge_chunk_meta(spec: &mut SessionSpec, chunk: &Chunk) {
    for (source, text) in &chunk.titles {
        if spec
            .title
            .as_ref()
            .is_none_or(|t| source.rank() >= t.source.rank())
        {
            spec.title = Some(SessionTitle {
                text: text.clone(),
                source: *source,
            });
        }
    }
    if chunk.git_branch.is_some() {
        spec.git_branch.clone_from(&chunk.git_branch);
    }
    if chunk.version.is_some() {
        spec.cc_version.clone_from(&chunk.version);
    }
    if chunk.entrypoint.is_some() {
        spec.entrypoint.clone_from(&chunk.entrypoint);
    }
    spec.first_ts = min_opt(spec.first_ts, chunk.first_ts);
    spec.last_ts = max_opt(spec.last_ts, chunk.last_ts);
}

fn min_opt(a: Option<Timestamp>, b: Option<Timestamp>) -> Option<Timestamp> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

fn max_opt(a: Option<Timestamp>, b: Option<Timestamp>) -> Option<Timestamp> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

/// Decides what to read for each file and orders the jobs newest first.
fn plan_jobs(
    entries: Vec<FileEntry>,
    stored: &HashMap<String, SourceFileState>,
    report: &mut IndexReport,
) -> Vec<Job> {
    let mut jobs = Vec::new();
    for entry in entries {
        let start = match stored.get(&entry.key) {
            None => 0,
            // Reappeared after being marked deleted: read it again; dedup keeps it idempotent.
            Some(prev) if prev.deleted_at.is_some() => 0,
            Some(prev) if prev.size == entry.size && prev.mtime == entry.mtime => {
                report.files_unchanged += 1;
                continue;
            }
            Some(prev) if entry.size < prev.offset => {
                report.files_truncated += 1;
                0
            }
            // Same size, new mtime: continue from the offset (pitfall: never restart).
            Some(prev) => prev.offset,
        };
        jobs.push(Job { entry, start });
    }
    jobs.sort_by(|a, b| {
        b.entry
            .mtime
            .cmp(&a.entry.mtime)
            .then_with(|| a.entry.key.cmp(&b.entry.key))
    });
    jobs
}

/// Lists transcripts: `<dir>/*.jsonl` and `<dir>/<session>/subagents/agent-*.jsonl`. Nothing
/// else is descended into, which prunes `tool-results` and friends.
fn inventory(projects_dir: &Path) -> io::Result<Vec<FileEntry>> {
    let mut out = Vec::new();
    let projects = match std::fs::read_dir(projects_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e),
    };
    let mut push = |path: PathBuf, meta: &std::fs::Metadata| {
        if let Some(kind) = classify_path(projects_dir, &path) {
            out.push(FileEntry {
                key: path_key(&path),
                path,
                kind,
                size: meta.len(),
                mtime: mtime_ms(meta),
            });
        }
    };
    for project in projects.flatten() {
        if !project.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let Ok(children) = std::fs::read_dir(project.path()) else {
            continue;
        };
        for child in children.flatten() {
            let Ok(ft) = child.file_type() else { continue };
            if ft.is_file() {
                // `DirEntry::metadata` comes from the directory listing on Windows: no open.
                if let Ok(meta) = child.metadata() {
                    push(child.path(), &meta);
                }
            } else if ft.is_dir() {
                let Ok(subagents) = std::fs::read_dir(child.path().join("subagents")) else {
                    continue;
                };
                for file in subagents.flatten() {
                    if file.file_type().is_ok_and(|t| t.is_file())
                        && let Ok(meta) = file.metadata()
                    {
                        push(file.path(), &meta);
                    }
                }
            }
        }
    }
    Ok(out)
}

/// The `source_files.path` key: the path with the platform's separator.
fn path_key(path: &Path) -> String {
    let s = path.to_string_lossy();
    if cfg!(windows) {
        s.replace('/', "\\")
    } else {
        s.into_owned()
    }
}

fn mtime_ms(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

fn recent_cutoff(window: Duration) -> i64 {
    let window = i64::try_from(window.as_millis()).unwrap_or(i64::MAX);
    Timestamp::now().saturating_sub_millis(window).as_millis()
}

fn warn_on_drift(report: &IndexReport) {
    let p = &report.parse;
    if p.prefilter_misses > 0 {
        tracing::warn!(
            lines = p.prefilter_misses,
            "usage lines matched only the loose pre-filter; the Claude Code JSONL layout may have changed"
        );
    }
    if p.unknown() > 0 || p.oversized_suspect > 0 {
        tracing::warn!(
            malformed = p.malformed,
            bad_timestamps = p.bad_timestamps,
            oversized_suspect = p.oversized_suspect,
            "some transcript lines could not be used"
        );
    }
    if report.bytes_read >= DRIFT_MIN_BYTES && p.usage_records == 0 {
        tracing::warn!(
            bytes = report.bytes_read,
            "read transcripts but found no usage records; the Claude Code format may have changed"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Workers
// ---------------------------------------------------------------------------------------------

/// `background_before`: enter background mode at the first job whose mtime is older. Jobs are
/// sorted newest first, so every later job is older too.
fn worker(
    jobs: &[Job],
    next: &AtomicUsize,
    tx: &mpsc::SyncSender<WorkerMsg>,
    background_before: Option<i64>,
    cancel: &AtomicBool,
) {
    let parser = LineParser::default();
    let mut reader = LineReader::default();
    let mut in_background = false;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let job = next.fetch_add(1, Ordering::Relaxed);
        let Some(j) = jobs.get(job) else { return };
        if !in_background && background_before.is_some_and(|cutoff| j.entry.mtime < cutoff) {
            priority::enter_background_mode();
            in_background = true;
        }
        let sent = match read_job(j, job, &parser, &mut reader, tx) {
            Ok(sent) => sent,
            Err(error) => tx.send(WorkerMsg::Failed { job, error }).is_ok(),
        };
        if !sent {
            return; // the coordinator stopped
        }
    }
}

/// Reads one file and sends its chunks. `Ok(false)` means the coordinator is gone.
fn read_job(
    job: &Job,
    index: usize,
    parser: &LineParser,
    reader: &mut LineReader,
    tx: &mpsc::SyncSender<WorkerMsg>,
) -> io::Result<bool> {
    // `File::open` shares read, write and delete access on Windows: Claude Code can keep
    // appending, and the file is never locked.
    let mut file = File::open(&job.entry.path)?;
    file.seek(SeekFrom::Start(job.start))?;
    let mut chunk = Chunk::default();
    let mut gone = false;
    let (offset, read) =
        reader.read_range(&mut file, job.start, job.entry.size, |line, next| {
            collect_line(parser, line, &mut chunk);
            if chunk.events.len() >= WORKER_CHUNK_EVENTS {
                chunk.offset = next;
                let full = std::mem::take(&mut chunk);
                if tx
                    .send(WorkerMsg::Chunk {
                        job: index,
                        chunk: Box::new(full),
                        last: false,
                    })
                    .is_err()
                {
                    gone = true;
                    return ControlFlow::Break(());
                }
            }
            ControlFlow::Continue(())
        })?;
    if gone {
        return Ok(false);
    }
    chunk.offset = offset;
    chunk.stats.oversized_lines += read.oversized;
    chunk.stats.oversized_suspect += read.oversized_suspect;
    Ok(tx
        .send(WorkerMsg::Chunk {
            job: index,
            chunk: Box::new(chunk),
            last: true,
        })
        .is_ok())
}

fn collect_line(parser: &LineParser, line: &[u8], chunk: &mut Chunk) {
    chunk.stats.lines += 1;
    match parser.parse(line) {
        ParseOutcome::Usage(rec) => {
            let s = &mut chunk.stats;
            s.usage_records += 1;
            s.advisor_records += rec.advisors.len() as u64;
            s.prefilter_misses += u64::from(rec.loose_match);
            s.fallback_keys += u64::from(rec.key_is_fallback);
            if chunk.cwd.is_none() {
                chunk.cwd = rec
                    .cwd
                    .map(std::borrow::Cow::into_owned)
                    .filter(|c| !c.is_empty());
            }
            set_latest(&mut chunk.git_branch, rec.git_branch.as_deref());
            set_latest(&mut chunk.version, rec.version.as_deref());
            set_latest(&mut chunk.entrypoint, rec.entrypoint.as_deref());
            chunk.first_ts = min_opt(chunk.first_ts, Some(rec.ts));
            chunk.last_ts = max_opt(chunk.last_ts, Some(rec.ts));
            for adv in rec.advisors {
                chunk.push(RawEvent {
                    key: adv.key,
                    ts: rec.ts,
                    model: model_name(&adv.model),
                    usage: adv.usage,
                });
            }
            chunk.push(RawEvent {
                key: rec.key,
                ts: rec.ts,
                model: model_name(&rec.model),
                usage: rec.usage,
            });
        }
        ParseOutcome::Meta(MetaRecord::Title { source, text }) => {
            chunk.stats.title_records += 1;
            chunk.titles.push((source, text.into_owned()));
        }
        ParseOutcome::Skip => {}
        ParseOutcome::Unknown(UnknownReason::Malformed) => chunk.stats.malformed += 1,
        ParseOutcome::Unknown(UnknownReason::BadTimestamp) => chunk.stats.bad_timestamps += 1,
    }
}

fn set_latest(slot: &mut Option<String>, value: Option<&str>) {
    if let Some(v) = value.filter(|v| !v.is_empty())
        && slot.as_deref() != Some(v)
    {
        *slot = Some(v.to_owned());
    }
}

fn model_name(model: &str) -> String {
    if model.is_empty() {
        "unknown".to_owned()
    } else {
        model.to_owned()
    }
}

#[cfg(test)]
mod tests;
