//! Starts and stops the living core: watcher, indexers, live sessions and retention.
//!
//! The core runs on its own named threads rather than on an async runtime (ADR-027); the
//! indexer and the store are blocking code, and plain threads make shutdown easy to verify.
//!
//! ```text
//!  notify ──► core-watch ──(dirty transcripts)──► core-index-<account> ──► store-writer
//!               │  ▲                                    │
//!               │  └──────────(indexed)─────────────────┘
//!               ├─ live-session trackers, credentials mtime
//!               └─ timers: debounce, hot files, liveness, safety scan
//!               └─ live sessions, credentials ──► core-limits ◄── core-api-<account> (S2)
//!  every thread ──► EventBus ──► app
//! ```
//!
//! `core-limits` owns the [`LimitEngine`]: it fuses observations, detects resets and publishes
//! [`CoreEvent::LimitsUpdated`] when the panel would look different. Each account has a
//! `core-api-<id>` poller that only sends requests with the user's consent (ADR-006).
//!
//! Start-up order (architecture §2): the watcher starts at once; the first full scan waits
//! [`Timings::startup_delay`] so the tray and IPC come up first.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;
use specta::Type;

use crate::clock::SystemClock;
use crate::domain::{AccountId, AccountSpec, LimitObservation, Provider, Timestamp};
use crate::events::{CoreEvent, DriftSource, EventBus, Subscription};
use crate::indexer::{
    self, IndexEvent, IndexMilestone, IndexReport, Indexer, IndexerConfig, RetentionScheduler,
};
use crate::limits::{AccountMeta, IndexState, LimitEngine, LimitEvent, PanelSnapshot, VisibleKey};
use crate::live_sessions::{self, LiveSession, LiveSessionTracker};
use crate::providers::claude::credentials::{self, CREDENTIALS_FILE};
use crate::providers::claude::usage_api::{
    HttpTransport, PollInputs, PollUpdate, PollerMsg, UsageApiPoller,
};
use crate::providers::claude::{AccountDescriptor, PROJECTS_DIR, classify_path};
use crate::store::{Store, StoreError};
use crate::watcher::{self, Debouncer, FsSignal, FsWatcher, HotFiles, Stamp};

/// How long shutdown waits for the threads by default (Step 05 acceptance: exit within 2 s).
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// How often liveness is re-checked while sessions are listed. Exits that remove the file are
/// seen at once through the watcher; this only catches crashes that leave the file behind.
pub const LIVENESS_INTERVAL: Duration = Duration::from_secs(15);

/// The panel snapshot is rebuilt at most this often; statusline calls can arrive every second.
pub const LIMITS_REBUILD_GAP: Duration = Duration::from_secs(2);

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("could not start a core thread: {0}")]
    Thread(#[from] std::io::Error),
    #[error("could not start the file watcher: {0}")]
    Watch(#[from] notify::Error),
}

/// Timers, all overridable for tests.
#[derive(Debug, Clone)]
pub struct Timings {
    pub debounce: Duration,
    pub startup_delay: Duration,
    pub hot_interval: Duration,
    pub hot_ttl: Duration,
    pub liveness_interval: Duration,
    pub safety_scan: Duration,
}

impl Default for Timings {
    fn default() -> Self {
        Self {
            debounce: watcher::DEBOUNCE,
            startup_delay: Duration::from_secs(2),
            hot_interval: watcher::HOT_INTERVAL,
            hot_ttl: watcher::HOT_TTL,
            liveness_interval: LIVENESS_INTERVAL,
            safety_scan: watcher::SAFETY_SCAN,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CoreConfig {
    pub db_path: PathBuf,
    /// Claude Code config directories to track, e.g. from [`crate::providers::claude::discover`].
    pub accounts: Vec<AccountDescriptor>,
    pub timings: Timings,
    pub indexer: IndexerConfig,
    /// Raw-event retention in days; `None` disables the daily job.
    pub retention_days: Option<u32>,
    /// The user allowed the OAuth usage API (see [`crate::config::AppConfig::api_consent_granted`]).
    /// Without it no request is ever sent and the token is never read.
    pub api_consent: bool,
}

impl CoreConfig {
    pub fn new(db_path: PathBuf, accounts: Vec<AccountDescriptor>) -> Self {
        Self {
            db_path,
            accounts,
            timings: Timings::default(),
            indexer: IndexerConfig::default(),
            retention_days: Some(indexer::DEFAULT_KEEP_DAYS),
            api_consent: false,
        }
    }
}

/// A snapshot of the runtime's state; what a subscriber reloads after [`crate::events::Delivery::Resync`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Type)]
pub struct CoreStatus {
    pub live_sessions: Vec<LiveSession>,
    /// End of the last indexing pass that stored something, or of the first full scan.
    pub last_indexed_at: Option<Timestamp>,
    /// The first full scan of every account finished.
    pub initial_scan_done: bool,
    /// Incremental passes since start (diagnostics).
    pub incremental_passes: u64,
    /// The latest panel view model; `None` until the limit engine has built one.
    pub limits: Option<PanelSnapshot>,
}

/// Entry point.
pub struct CoreRuntime;

impl CoreRuntime {
    /// Opens the store and starts every core thread. Returns quickly; indexing starts after
    /// [`Timings::startup_delay`].
    pub fn start(config: &CoreConfig) -> Result<CoreHandle, RuntimeError> {
        let store = Store::open(&config.db_path)?;
        let bus = EventBus::default();
        let status = Arc::new(Mutex::new(CoreStatus::default()));
        let cancel = Arc::new(AtomicBool::new(false));
        let (watch_tx, watch_rx) = mpsc::channel::<WatchMsg>();
        let (limits_tx, limits_rx) = mpsc::channel::<LimitsMsg>();
        let mut threads = Vec::new();
        let mut metas = Vec::new();
        let mut pollers = Vec::new();

        let mut watched = Vec::new();
        let mut index_txs = Vec::new();
        let pending_initial = Arc::new(std::sync::atomic::AtomicUsize::new(config.accounts.len()));
        if config.accounts.is_empty() {
            lock(&status).initial_scan_done = true;
        }
        for desc in &config.accounts {
            let meta = register_account(&store, desc)?;
            let account = meta.id;
            metas.push(meta);
            let (thread, link) = spawn_poller(
                account,
                desc.config_dir.clone(),
                config.api_consent,
                &cancel,
                limits_tx.clone(),
            )?;
            threads.push(thread);
            pollers.push(link);
            let mut indexer = Indexer::new(
                store.clone(),
                account,
                &desc.config_dir,
                config.indexer.clone(),
            );
            indexer.set_cancel(cancel.clone());
            let (tx, rx) = mpsc::channel::<IndexCmd>();
            let ctx = IndexCtx {
                account,
                indexer,
                bus: bus.clone(),
                status: status.clone(),
                watch_tx: watch_tx.clone(),
                limits_tx: limits_tx.clone(),
                startup_delay: config.timings.startup_delay,
                pending_initial: pending_initial.clone(),
            };
            threads.push(spawn(&format!("core-index-{account}"), move || {
                index_loop(ctx, &rx);
            })?);
            index_txs.push(tx.clone());
            watched.push(AccountWatch::new(
                account,
                &desc.config_dir,
                store.clone(),
                tx,
            ));
        }

        let engine = LimitEngine::new(store.clone(), metas, config.api_consent, Timestamp::now())?;
        let ctx = LimitsCtx {
            engine,
            pollers,
            bus: bus.clone(),
            status: status.clone(),
        };
        threads.push(spawn("core-limits", move || limits_loop(ctx, &limits_rx))?);

        let fs = FsWatcher::new(watch_tx.clone(), WatchMsg::Fs)?;
        let ctx = WatchCtx {
            accounts: watched,
            fs,
            limits_tx: limits_tx.clone(),
            bus: bus.clone(),
            status: status.clone(),
            timings: config.timings.clone(),
        };
        threads.push(spawn("core-watch", move || watch_loop(ctx, &watch_rx))?);

        let retention = match config.retention_days {
            Some(days) => Some(RetentionScheduler::spawn(
                store.clone(),
                days,
                indexer::CHECK_EVERY,
            )?),
            None => None,
        };

        Ok(CoreHandle {
            store,
            bus,
            status,
            cancel,
            watch_tx,
            limits_tx,
            index_txs,
            threads,
            retention,
        })
    }
}

/// Handle to a running core. Dropping it without [`CoreHandle::shutdown`] also stops the
/// threads, but without a time limit.
pub struct CoreHandle {
    store: Store,
    bus: EventBus,
    status: Arc<Mutex<CoreStatus>>,
    cancel: Arc<AtomicBool>,
    watch_tx: mpsc::Sender<WatchMsg>,
    limits_tx: mpsc::Sender<LimitsMsg>,
    index_txs: Vec<mpsc::Sender<IndexCmd>>,
    threads: Vec<JoinHandle<()>>,
    retention: Option<RetentionScheduler>,
}

/// What [`CoreHandle::shutdown`] managed to stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutdownReport {
    pub joined: usize,
    /// Names of threads still running when the timeout passed. They are detached.
    pub timed_out: Vec<String>,
}

impl CoreHandle {
    pub fn subscribe(&self) -> Subscription {
        self.bus.subscribe()
    }

    /// Read access for queries. Writes go through the core's own threads.
    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn status(&self) -> CoreStatus {
        lock(&self.status).clone()
    }

    /// Feeds a limit reading into the engine, e.g. from the statusline bridge (Step 07).
    pub fn ingest_limit(&self, obs: LimitObservation) {
        let _ = self.limits_tx.send(LimitsMsg::Observe(obs));
    }

    /// The panel opened: the usage API refreshes data older than 60 s (with consent).
    pub fn panel_opened(&self) {
        let _ = self.limits_tx.send(LimitsMsg::PanelOpened);
    }

    /// Grants or withdraws consent for the usage API at runtime (settings, Step 11).
    pub fn set_api_consent(&self, granted: bool) {
        let _ = self.limits_tx.send(LimitsMsg::Consent(granted));
    }

    /// Stops every core thread and waits up to `timeout`. An indexing pass in progress stops
    /// after its current file; what it stored stays consistent.
    pub fn shutdown(mut self, timeout: Duration) -> ShutdownReport {
        self.stop_all(timeout)
    }

    fn stop_all(&mut self, timeout: Duration) -> ShutdownReport {
        // `None`: wait as long as it takes (drop without an explicit shutdown).
        let deadline = Instant::now().checked_add(timeout);
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.watch_tx.send(WatchMsg::Stop);
        let _ = self.limits_tx.send(LimitsMsg::Stop);
        for tx in self.index_txs.drain(..) {
            let _ = tx.send(IndexCmd::Stop);
        }
        while deadline.is_none_or(|d| Instant::now() < d)
            && self.threads.iter().any(|t| !t.is_finished())
        {
            thread::sleep(Duration::from_millis(5));
        }
        let mut report = ShutdownReport {
            joined: 0,
            timed_out: Vec::new(),
        };
        for t in self.threads.drain(..) {
            if t.is_finished() {
                let _ = t.join();
                report.joined += 1;
            } else {
                report
                    .timed_out
                    .push(t.thread().name().unwrap_or("?").to_owned());
            }
        }
        // Joins the retention thread; its wait wakes as soon as its channel closes.
        if self.retention.take().is_some() {
            report.joined += 1;
        }
        if !report.timed_out.is_empty() {
            tracing::warn!(threads = ?report.timed_out, "core threads did not stop in time");
        }
        report
    }
}

impl Drop for CoreHandle {
    fn drop(&mut self) {
        if !self.threads.is_empty() {
            self.stop_all(Duration::MAX);
        }
    }
}

/// Upserts a discovered account with its plan. Plan metadata needs no consent: the token
/// fields are never deserialized.
fn register_account(store: &Store, desc: &AccountDescriptor) -> Result<AccountMeta, StoreError> {
    let plan = credentials::read_plan(&desc.config_dir).unwrap_or_default();
    let id = store.writer().upsert_account(AccountSpec {
        provider: Provider::Claude,
        label: desc.label.clone(),
        config_dir: desc.config_dir.to_string_lossy().into_owned(),
        plan: plan.subscription_type.clone(),
        tier: plan.rate_limit_tier.clone(),
        environment: "native".into(),
    })?;
    Ok(AccountMeta {
        id,
        label: desc.label.clone(),
        plan: plan.subscription_type,
        tier: plan.rate_limit_tier,
    })
}

/// Starts `core-api-<account>`. It idles on its channel until consent and a due request.
fn spawn_poller(
    account: AccountId,
    config_dir: PathBuf,
    consent: bool,
    cancel: &Arc<AtomicBool>,
    out: mpsc::Sender<LimitsMsg>,
) -> std::io::Result<(JoinHandle<()>, PollerLink)> {
    let (tx, rx) = mpsc::channel::<PollerMsg>();
    let poller = UsageApiPoller::new(
        account,
        config_dir.clone(),
        consent,
        HttpTransport::with_cancel(cancel.clone()),
        SystemClock,
    );
    let thread = spawn(&format!("core-api-{account}"), move || {
        poller.run(&rx, |update| {
            let _ = out.send(LimitsMsg::Poll(Box::new(update)));
        });
    })?;
    let link = PollerLink {
        account,
        config_dir,
        tx,
        sent: None,
    };
    Ok((thread, link))
}

fn spawn(name: &str, f: impl FnOnce() + Send + 'static) -> std::io::Result<JoinHandle<()>> {
    thread::Builder::new().name(name.to_owned()).spawn(f)
}

fn lock(status: &Mutex<CoreStatus>) -> std::sync::MutexGuard<'_, CoreStatus> {
    status.lock().unwrap_or_else(PoisonError::into_inner)
}

// ---------------------------------------------------------------------------------------------
// Index thread
// ---------------------------------------------------------------------------------------------

enum IndexCmd {
    Files(Vec<PathBuf>),
    FullScan,
    Stop,
}

/// Work queued while a pass runs; merged so a burst becomes one pass.
#[derive(Default)]
struct Pending {
    full: bool,
    files: HashSet<PathBuf>,
}

impl Pending {
    /// Returns `false` for [`IndexCmd::Stop`].
    fn merge(&mut self, cmd: IndexCmd) -> bool {
        match cmd {
            IndexCmd::Files(files) => self.files.extend(files),
            IndexCmd::FullScan => self.full = true,
            IndexCmd::Stop => return false,
        }
        true
    }

    fn is_empty(&self) -> bool {
        !self.full && self.files.is_empty()
    }
}

struct IndexCtx {
    account: AccountId,
    indexer: Indexer,
    bus: EventBus,
    status: Arc<Mutex<CoreStatus>>,
    watch_tx: mpsc::Sender<WatchMsg>,
    limits_tx: mpsc::Sender<LimitsMsg>,
    startup_delay: Duration,
    pending_initial: Arc<std::sync::atomic::AtomicUsize>,
}

fn index_loop(mut ctx: IndexCtx, rx: &mpsc::Receiver<IndexCmd>) {
    let mut pending = Pending {
        full: true, // the initial scan
        files: HashSet::new(),
    };
    // Start-up delay: collect work, but run nothing yet.
    let start = Instant::now() + ctx.startup_delay;
    loop {
        let left = start.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        match rx.recv_timeout(left) {
            Ok(cmd) => {
                if !pending.merge(cmd) {
                    return;
                }
            }
            Err(RecvTimeoutError::Timeout) => break,
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }

    let mut initial = true;
    loop {
        // Drain whatever else arrived so a burst is one pass.
        loop {
            match rx.try_recv() {
                Ok(cmd) => {
                    if !pending.merge(cmd) {
                        return;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }
        if !pending.is_empty() {
            let work = std::mem::take(&mut pending);
            run_pass(&mut ctx, work, initial);
            initial = false;
        }
        match rx.recv() {
            Ok(cmd) => {
                if !pending.merge(cmd) {
                    return;
                }
            }
            Err(_) => return,
        }
    }
}

fn run_pass(ctx: &mut IndexCtx, work: Pending, initial: bool) {
    let account = ctx.account;
    let bus = ctx.bus.clone();
    let full = work.full;
    let mut on_event = |e: IndexEvent| match e {
        // Progress is for long scans; incremental passes would only add noise.
        IndexEvent::Progress(progress) if full => {
            bus.publish(CoreEvent::IndexProgress { account, progress });
        }
        IndexEvent::Milestone(milestone) if initial => {
            bus.publish(CoreEvent::IndexMilestone { account, milestone });
        }
        _ => {}
    };
    let result = if full {
        // A full scan also covers every queued file.
        ctx.indexer.full_scan(&mut on_event)
    } else {
        let files: Vec<PathBuf> = work.files.into_iter().collect();
        ctx.indexer.index_files(&files, &mut on_event)
    };
    let report = match result {
        Ok(report) => report,
        Err(error) => {
            tracing::warn!(%error, account = %account, "indexing pass failed; the next one retries");
            if initial {
                finish_initial(ctx);
            }
            return;
        }
    };
    tracing::debug!(
        account = %account,
        full,
        files = report.files_indexed,
        inserted = report.events.inserted,
        updated = report.events.updated,
        "indexing pass"
    );
    publish_report(ctx, &report, initial);
    {
        let mut status = lock(&ctx.status);
        if !full {
            status.incremental_passes += 1;
        }
        if initial || !report.sessions.is_empty() {
            status.last_indexed_at = Some(Timestamp::now());
        }
    }
    if initial && !report.cancelled {
        finish_initial(ctx);
    }
    if !report.sessions.is_empty() {
        let _ = ctx.watch_tx.send(WatchMsg::Indexed);
        let _ = ctx.limits_tx.send(LimitsMsg::Indexed);
    }
}

fn publish_report(ctx: &IndexCtx, report: &IndexReport, initial: bool) {
    let p = &report.parse;
    let unknown = p.unknown() + p.prefilter_misses + p.oversized_suspect;
    if unknown > 0 {
        ctx.bus.publish(CoreEvent::SchemaDrift {
            source: DriftSource::ClaudeJsonl,
            unknown_count: unknown,
        });
    }
    // After the first scan every view reloads anyway (`Complete`); listing thousands of
    // sessions here would only cost memory.
    if !initial && let Some((from_ts, to_ts)) = report.ts_range {
        ctx.bus.publish(CoreEvent::UsageAppended {
            account: ctx.account,
            sessions: report.sessions.iter().copied().collect(),
            from_ts,
            to_ts,
        });
    }
}

fn finish_initial(ctx: &IndexCtx) {
    ctx.bus.publish(CoreEvent::IndexMilestone {
        account: ctx.account,
        milestone: IndexMilestone::Complete,
    });
    if ctx.pending_initial.fetch_sub(1, Ordering::AcqRel) == 1 {
        lock(&ctx.status).initial_scan_done = true;
        let _ = ctx.limits_tx.send(LimitsMsg::Indexed);
    }
}

// ---------------------------------------------------------------------------------------------
// Watch thread
// ---------------------------------------------------------------------------------------------

enum WatchMsg {
    Fs(FsSignal),
    /// An index pass stored events; live sessions may now map to indexed sessions.
    Indexed,
    Stop,
}

struct AccountWatch {
    account: AccountId,
    config_dir: PathBuf,
    projects_dir: PathBuf,
    tracker: LiveSessionTracker,
    credentials_mtime: Option<SystemTime>,
    index_tx: mpsc::Sender<IndexCmd>,
}

impl AccountWatch {
    fn new(
        account: AccountId,
        config_dir: &Path,
        store: Store,
        index_tx: mpsc::Sender<IndexCmd>,
    ) -> Self {
        Self {
            account,
            config_dir: config_dir.to_path_buf(),
            projects_dir: config_dir.join(PROJECTS_DIR),
            tracker: LiveSessionTracker::new(account, config_dir, store),
            credentials_mtime: credentials_mtime(config_dir),
            index_tx,
        }
    }

    fn sessions_dir(&self) -> &Path {
        self.tracker.sessions_dir()
    }

    fn ensure_watches(&self, fs: &mut FsWatcher) {
        // Non-recursive on the config dir: the credentials file and the creation of
        // `projects/` or `sessions/` when Claude Code runs for the first time.
        fs.ensure(&self.config_dir, false);
        fs.ensure(&self.projects_dir, true);
        fs.ensure(self.sessions_dir(), false);
    }

    fn is_credentials(&self, path: &Path) -> bool {
        path.parent() == Some(self.config_dir.as_path())
            && path.file_name().is_some_and(|n| {
                if cfg!(windows) {
                    n.eq_ignore_ascii_case(CREDENTIALS_FILE)
                } else {
                    n == CREDENTIALS_FILE
                }
            })
    }

    /// Publishes [`CoreEvent::CredentialsChanged`] when the modification time moved, and
    /// tells the limit thread: a stale API token may have been renewed.
    fn check_credentials(&mut self, bus: &EventBus, limits_tx: &mpsc::Sender<LimitsMsg>) {
        let mtime = credentials_mtime(&self.config_dir);
        if mtime != self.credentials_mtime {
            self.credentials_mtime = mtime;
            bus.publish(CoreEvent::CredentialsChanged {
                account: self.account,
            });
            let _ = limits_tx.send(LimitsMsg::CredentialsChanged(self.account));
        }
    }
}

/// Metadata only: the file is never opened.
fn credentials_mtime(config_dir: &Path) -> Option<SystemTime> {
    std::fs::metadata(config_dir.join(CREDENTIALS_FILE))
        .and_then(|m| m.modified())
        .ok()
}

struct WatchCtx {
    accounts: Vec<AccountWatch>,
    fs: FsWatcher,
    limits_tx: mpsc::Sender<LimitsMsg>,
    bus: EventBus,
    status: Arc<Mutex<CoreStatus>>,
    timings: Timings,
}

impl WatchCtx {
    /// Refreshes every tracker and publishes the combined list if anything changed.
    fn refresh_live(&mut self) {
        let mut changed = false;
        for a in &mut self.accounts {
            changed |= a.tracker.refresh();
        }
        if changed {
            let all: Vec<LiveSession> = self
                .accounts
                .iter()
                .flat_map(|a| a.tracker.sessions().iter().cloned())
                .collect();
            lock(&self.status).live_sessions.clone_from(&all);
            let _ = self.limits_tx.send(LimitsMsg::Live(all.clone()));
            self.bus.publish(CoreEvent::LiveSessionsChanged(all));
        }
    }

    fn has_live(&self) -> bool {
        self.accounts
            .iter()
            .any(|a| !a.tracker.sessions().is_empty())
    }

    /// Everything a lost event could have hidden.
    fn rescan(&mut self) {
        for a in &mut self.accounts {
            a.ensure_watches(&mut self.fs);
            a.check_credentials(&self.bus, &self.limits_tx);
            let _ = a.index_tx.send(IndexCmd::FullScan);
        }
        self.refresh_live();
    }

    /// Routes one debounced batch.
    fn flush(&mut self, batch: &HashSet<PathBuf>, hot: &mut HotFiles, now: Instant) {
        let mut live = false;
        let mut rewatch = false;
        for a in &mut self.accounts {
            let mut files = Vec::new();
            for path in batch {
                if classify_path(&a.projects_dir, path).is_some() {
                    let stamp = Stamp::of(path);
                    if stamp.is_some() {
                        hot.touch(path.clone(), stamp, now);
                    }
                    files.push(path.clone());
                } else if live_sessions::is_session_file(a.tracker.sessions_dir(), path) {
                    live = true;
                } else if a.is_credentials(path) {
                    a.check_credentials(&self.bus, &self.limits_tx);
                } else if (path == &a.projects_dir || path == a.tracker.sessions_dir())
                    && (!self.fs.is_watched(path) || !path.is_dir())
                {
                    // Created, deleted or renamed: (re)attach the watch. Plain "modified"
                    // events on a watched directory are noise and ignored.
                    rewatch = true;
                    self.fs.forget(path);
                }
            }
            if !files.is_empty() {
                let _ = a.index_tx.send(IndexCmd::Files(files));
            }
        }
        if rewatch {
            for a in &self.accounts {
                a.ensure_watches(&mut self.fs);
            }
            // Anything written before the watch existed.
            self.rescan();
        } else if live {
            self.refresh_live();
        }
    }

    /// Re-checks hot transcripts and queues those that changed unseen.
    fn check_hot(&mut self, hot: &mut HotFiles, now: Instant) {
        let changed = hot.changed(now, Stamp::of);
        if changed.is_empty() {
            return;
        }
        tracing::debug!(files = changed.len(), "hot files changed without an event");
        for a in &self.accounts {
            let files: Vec<PathBuf> = changed
                .iter()
                .filter(|p| p.starts_with(&a.projects_dir))
                .cloned()
                .collect();
            if !files.is_empty() {
                let _ = a.index_tx.send(IndexCmd::Files(files));
            }
        }
    }
}

fn watch_loop(mut ctx: WatchCtx, rx: &mpsc::Receiver<WatchMsg>) {
    let t = ctx.timings.clone();
    for a in &ctx.accounts {
        a.ensure_watches(&mut ctx.fs);
    }
    ctx.refresh_live();

    let mut debouncer = Debouncer::new(t.debounce);
    let mut hot = HotFiles::new(t.hot_ttl);
    let now = Instant::now();
    let mut next_safety = now + t.safety_scan;
    let mut next_hot = now + t.hot_interval;
    let mut next_liveness = now + t.liveness_interval;

    loop {
        // Sleep until the earliest timer that applies; with nothing live and nothing hot that
        // is the safety scan, minutes away.
        let mut wake = next_safety;
        if let Some(d) = debouncer.deadline() {
            wake = wake.min(d);
        }
        if !hot.is_empty() {
            wake = wake.min(next_hot);
        }
        if ctx.has_live() {
            wake = wake.min(next_liveness);
        }
        let msg = match rx.recv_timeout(wake.saturating_duration_since(Instant::now())) {
            Ok(msg) => Some(msg),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let now = Instant::now();
        match msg {
            Some(WatchMsg::Stop) => return,
            Some(WatchMsg::Fs(FsSignal::Path(path))) => debouncer.add(path, now),
            Some(WatchMsg::Fs(FsSignal::Rescan)) => {
                ctx.rescan();
                next_safety = now + t.safety_scan;
            }
            Some(WatchMsg::Indexed) if ctx.accounts.iter().any(|a| a.tracker.has_unresolved()) => {
                ctx.refresh_live();
            }
            Some(WatchMsg::Indexed) | None => {}
        }

        if let Some(batch) = debouncer.take_due(now) {
            ctx.flush(&batch, &mut hot, now);
        }
        if hot.is_empty() {
            next_hot = now + t.hot_interval;
        } else if now >= next_hot {
            ctx.check_hot(&mut hot, now);
            next_hot = now + t.hot_interval;
        }
        if !ctx.has_live() {
            next_liveness = now + t.liveness_interval;
        } else if now >= next_liveness {
            ctx.refresh_live();
            next_liveness = now + t.liveness_interval;
        }
        if now >= next_safety {
            ctx.rescan();
            next_safety = now + t.safety_scan;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Limits thread
// ---------------------------------------------------------------------------------------------

enum LimitsMsg {
    Observe(LimitObservation),
    Poll(Box<PollUpdate>),
    Live(Vec<LiveSession>),
    /// An indexing pass stored events, or the first scan finished.
    Indexed,
    CredentialsChanged(AccountId),
    PanelOpened,
    Consent(bool),
    Stop,
}

struct PollerLink {
    account: AccountId,
    config_dir: PathBuf,
    tx: mpsc::Sender<PollerMsg>,
    /// Last inputs sent, so unchanged ones are not sent again.
    sent: Option<PollInputs>,
}

struct LimitsCtx {
    engine: LimitEngine,
    pollers: Vec<PollerLink>,
    bus: EventBus,
    status: Arc<Mutex<CoreStatus>>,
}

impl LimitsCtx {
    /// Publishes the engine's signals; `true` if there were any.
    fn publish_all(&self, events: Vec<LimitEvent>) -> bool {
        let any = !events.is_empty();
        for e in events {
            self.bus.publish(CoreEvent::Limit(e));
        }
        any
    }

    fn sync_poll_inputs(&mut self) {
        for p in &mut self.pollers {
            let inputs = self.engine.poll_inputs(p.account);
            if p.sent.as_ref() != Some(&inputs) {
                let _ = p.tx.send(PollerMsg::Inputs(inputs.clone()));
                p.sent = Some(inputs);
            }
        }
    }

    fn broadcast(&self, msg: &PollerMsg) {
        for p in &self.pollers {
            let _ = p.tx.send(msg.clone());
        }
    }

    /// Applies one message. Returns `false` on [`LimitsMsg::Stop`].
    fn handle(&mut self, msg: LimitsMsg, now: Timestamp) -> bool {
        match msg {
            LimitsMsg::Stop => {
                self.broadcast(&PollerMsg::Stop);
                return false;
            }
            LimitsMsg::Observe(obs) => {
                let events = self.engine.ingest(&obs, now);
                self.publish_all(events);
            }
            LimitsMsg::Poll(update) => {
                let events = self.engine.apply_poll(*update, now);
                self.publish_all(events);
            }
            LimitsMsg::Live(list) => self.engine.set_live_sessions(list),
            LimitsMsg::Indexed => {}
            LimitsMsg::CredentialsChanged(account) => {
                if let Some(p) = self.pollers.iter().find(|p| p.account == account) {
                    let _ = p.tx.send(PollerMsg::CredentialsChanged);
                    let plan = credentials::read_plan(&p.config_dir).unwrap_or_default();
                    self.engine
                        .set_plan(account, plan.subscription_type, plan.rate_limit_tier);
                }
            }
            LimitsMsg::PanelOpened => self.broadcast(&PollerMsg::PanelOpened),
            LimitsMsg::Consent(granted) => self.broadcast(&PollerMsg::Consent(granted)),
        }
        true
    }

    /// Rebuilds the snapshot, keeps it in [`CoreStatus`] and publishes it if it looks different.
    fn rebuild(&mut self, now: Timestamp, last: &mut Option<VisibleKey>) {
        {
            let status = lock(&self.status);
            self.engine.set_index_state(IndexState {
                initial_scan_done: status.initial_scan_done,
                last_indexed_at: status.last_indexed_at,
            });
        }
        let (snapshot, events) = self.engine.snapshot(now);
        self.publish_all(events);
        let key = snapshot.visible_key();
        lock(&self.status).limits = Some(snapshot.clone());
        if last.as_ref() != Some(&key) {
            *last = Some(key);
            self.bus
                .publish(CoreEvent::LimitsUpdated(Box::new(snapshot)));
        }
    }
}

fn limits_loop(mut ctx: LimitsCtx, rx: &mpsc::Receiver<LimitsMsg>) {
    let mut last_key = None;
    let mut dirty = true;
    let mut next_build = Instant::now();
    loop {
        let now = Timestamp::now();
        let resets = ctx.engine.tick(now);
        dirty |= ctx.publish_all(resets);
        ctx.sync_poll_inputs();
        if dirty && Instant::now() >= next_build {
            ctx.rebuild(now, &mut last_key);
            dirty = false;
            next_build = Instant::now() + LIMITS_REBUILD_GAP;
        }

        // Sleep until the engine's next reset or freshness step, or the pending rebuild.
        let mut wait: Option<Duration> = ctx.engine.next_wake(now).map(|w| {
            Duration::from_millis(u64::try_from(w.as_millis() - now.as_millis()).unwrap_or(0))
        });
        if dirty {
            let until_build = next_build.saturating_duration_since(Instant::now());
            wait = Some(wait.map_or(until_build, |w| w.min(until_build)));
        }
        let msg = match wait {
            Some(wait) => match rx.recv_timeout(wait) {
                Ok(msg) => Some(msg),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            },
            None => match rx.recv() {
                Ok(msg) => Some(msg),
                Err(_) => return,
            },
        };
        // A timeout means time itself changed something (freshness, reset).
        dirty = true;
        if let Some(msg) = msg
            && !ctx.handle(msg, Timestamp::now())
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
