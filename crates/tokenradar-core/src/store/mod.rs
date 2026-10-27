//! SQLite store (WAL, single writer): schema migrations, rollups and queries.
//!
//! - **Writes** go through one dedicated thread that owns the only read-write connection
//!   ([`Writer`]). Concurrent SQLite writers would only contend for the lock.
//! - **Reads** borrow a read-only connection from a small pool ([`Store::read`]); WAL lets
//!   them run while the writer commits.
//!
//! Repository functions take a `&Connection` / `&Transaction`, so tests and the writer thread
//! share one implementation.

mod catalog;
mod events;
mod index;
mod kv;
mod migrate;
mod notifications;
mod retention;
mod snapshots;
mod sql;
mod windows;

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use rusqlite::{Connection, OpenFlags};

pub use events::UpsertStats;
pub use index::{IndexBatch, SourceFileState};
pub use retention::{PruneStats, RETENTION_INTERVAL_MS};
pub use windows::{SessionLabel, WINDOW_MATCH_TOLERANCE_MS};

use crate::domain::{
    Account, AccountId, AccountSpec, LimitObservation, LimitWindow, MicroUsd, ModelFamily, ModelId,
    ProjectId, ProjectSpec, Session, SessionId, SessionSpec, Timestamp, UsageEvent,
};
use crate::limits::RootUsage;

/// Read connections kept open between reads. More may exist briefly under load. Each idle
/// connection keeps its page cache, which counts against the 25 MB tray budget.
const MAX_IDLE_READERS: usize = 1;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("could not create the data directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("database schema version {found} is newer than this build supports ({supported})")]
    SchemaTooNew { found: i64, supported: i64 },
    #[error("estimates are kept in memory and never written to limit_snapshots")]
    EstimateNotPersisted,
    #[error("the store writer thread has stopped")]
    WriterGone,
    #[error("corrupt value in column {column}: {detail}")]
    Corrupt {
        column: &'static str,
        detail: String,
    },
}

pub type Result<T, E = StoreError> = std::result::Result<T, E>;

type Job = Box<dyn FnOnce(&mut Connection) + Send>;

/// Handle to the database. Cheap to clone; the writer thread stops when the last handle drops.
#[derive(Clone)]
pub struct Store {
    inner: Arc<Inner>,
}

struct Inner {
    path: PathBuf,
    writer: Writer,
    readers: Mutex<Vec<Connection>>,
}

impl Store {
    /// Opens or creates the database, applies pending migrations and starts the writer thread.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(dir) = path.parent()
            && !dir.as_os_str().is_empty()
        {
            std::fs::create_dir_all(dir)?;
        }
        let mut conn = Connection::open(&path)?;
        configure_writer(&conn)?;
        migrate::run(&mut conn)?;
        let writer = Writer::spawn(conn)?;
        Ok(Self {
            inner: Arc::new(Inner {
                path,
                writer,
                readers: Mutex::new(Vec::new()),
            }),
        })
    }

    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    /// Handle to the writer thread's command channel.
    pub fn writer(&self) -> &Writer {
        &self.inner.writer
    }

    /// Runs a read-only query on a pooled connection.
    pub fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let pooled = self
            .inner
            .readers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop();
        let conn = match pooled {
            Some(conn) => conn,
            None => open_reader(&self.inner.path)?,
        };
        let result = f(&conn);
        let mut idle = self
            .inner
            .readers
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if idle.len() < MAX_IDLE_READERS {
            idle.push(conn);
        }
        result
    }

    /// Returns cached pages to the allocator: shrinks the writer's page cache and closes idle
    /// readers. Call when indexing settles so the tray-only footprint stays small.
    pub fn release_memory(&self) -> Result<()> {
        self.inner
            .readers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        self.inner
            .writer
            .call(|c| Ok(c.execute_batch("PRAGMA shrink_memory;")?))
    }

    // --- Read helpers ---------------------------------------------------------------------

    pub fn account(&self, id: AccountId) -> Result<Option<Account>> {
        self.read(|c| catalog::account(c, id))
    }

    pub fn session(&self, id: SessionId) -> Result<Option<Session>> {
        self.read(|c| catalog::session(c, id))
    }

    pub fn latest_snapshots(&self, account: AccountId) -> Result<Vec<LimitObservation>> {
        self.read(|c| snapshots::latest(c, account))
    }

    /// Whether this exact reading is already stored (spool import idempotency).
    pub fn snapshot_exists(&self, obs: &LimitObservation) -> Result<bool> {
        self.read(|c| snapshots::exists(c, obs))
    }

    pub fn snapshots_between(
        &self,
        account: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<Vec<LimitObservation>> {
        self.read(|c| snapshots::between(c, account, from, to))
    }

    pub fn limit_windows(&self, account: AccountId) -> Result<Vec<LimitWindow>> {
        self.read(|c| windows::windows(c, account))
    }

    /// Cost and tokens per session tree with `from <= ts < to`, largest cost first.
    pub fn root_usage_between(
        &self,
        account: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<Vec<RootUsage>> {
        self.read(|c| windows::root_usage_between(c, account, from, to))
    }

    pub fn cost_between(
        &self,
        account: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<MicroUsd> {
        self.read(|c| windows::cost_between(c, account, from, to))
    }

    /// Titles and project names of the given sessions; unknown ids are left out.
    pub fn session_labels(&self, ids: &[SessionId]) -> Result<Vec<SessionLabel>> {
        self.read(|c| windows::session_labels(c, ids))
    }

    pub fn kv_get(&self, key: &str) -> Result<Option<String>> {
        self.read(|c| kv::get(c, key))
    }

    /// `notification_log` rows `(rule, dedupe_key, fired_at)` fired at or after `since`.
    pub fn notifications_since(
        &self,
        since: Timestamp,
    ) -> Result<Vec<(String, String, Timestamp)>> {
        self.read(|c| notifications::since(c, since))
    }

    /// Read positions of all transcript files of an account, keyed by path.
    pub fn source_files(
        &self,
        account: AccountId,
    ) -> Result<std::collections::HashMap<String, SourceFileState>> {
        self.read(|c| index::files(c, account))
    }

    /// Read positions of the given files (by [`SourceFileState::path`]) that are known.
    pub fn source_files_at(
        &self,
        account: AccountId,
        paths: &[String],
    ) -> Result<std::collections::HashMap<String, SourceFileState>> {
        self.read(|c| index::files_at(c, account, paths))
    }

    /// Id of a session by uuid, if it has been indexed.
    pub fn session_id_by_uuid(&self, account: AccountId, uuid: &str) -> Result<Option<SessionId>> {
        self.read(|c| catalog::session_id_by_uuid(c, account, uuid))
    }
}

/// Sends work to the writer thread and waits for the result.
pub struct Writer {
    tx: Mutex<Option<mpsc::Sender<Job>>>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
}

impl Writer {
    fn spawn(mut conn: Connection) -> Result<Self> {
        let (tx, rx) = mpsc::channel::<Job>();
        let thread = thread::Builder::new()
            .name("store-writer".into())
            .spawn(move || {
                for job in rx {
                    job(&mut conn);
                }
            })?;
        Ok(Self {
            tx: Mutex::new(Some(tx)),
            thread: Mutex::new(Some(thread)),
        })
    }

    /// Runs `f` on the writer connection. Use a transaction inside `f` for multi-statement
    /// work; the typed helpers below already do.
    pub fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        let job: Job = Box::new(move |conn| {
            // The caller may have given up; a closed reply channel is fine.
            let _ = reply_tx.send(f(conn));
        });
        self.tx
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .ok_or(StoreError::WriterGone)?
            .send(job)
            .map_err(|_| StoreError::WriterGone)?;
        reply_rx.recv().map_err(|_| StoreError::WriterGone)?
    }

    /// Upserts a batch of events in one transaction ("max output wins") and keeps
    /// `usage_5m` in step.
    pub fn upsert_events(&self, batch: Vec<UsageEvent>) -> Result<UpsertStats> {
        self.call(move |conn| {
            let tx = conn.transaction()?;
            let stats = events::upsert(&tx, &batch)?;
            tx.commit()?;
            Ok(stats)
        })
    }

    /// Writes one indexer flush (sessions, events, file offsets) in a single transaction.
    pub fn commit_index_batch(&self, batch: IndexBatch) -> Result<UpsertStats> {
        self.call(move |conn| index::commit(conn, &batch))
    }

    /// Marks transcript files as deleted; their data stays.
    pub fn mark_files_deleted(&self, paths: Vec<String>, at: Timestamp) -> Result<()> {
        self.call(move |conn| index::mark_deleted(conn, &paths, at))
    }

    pub fn upsert_account(&self, spec: AccountSpec) -> Result<AccountId> {
        self.call(move |conn| catalog::upsert_account(conn, &spec, Timestamp::now()))
    }

    pub fn upsert_project(&self, spec: ProjectSpec) -> Result<ProjectId> {
        self.call(move |conn| catalog::upsert_project(conn, &spec))
    }

    pub fn upsert_session(&self, spec: SessionSpec) -> Result<SessionId> {
        self.call(move |conn| catalog::upsert_session(conn, &spec))
    }

    pub fn upsert_model(&self, name: String, family: ModelFamily) -> Result<ModelId> {
        self.call(move |conn| catalog::upsert_model(conn, &name, family))
    }

    /// Stores an observation. Estimates are rejected (architecture §7).
    pub fn insert_snapshot(&self, obs: LimitObservation) -> Result<i64> {
        self.call(move |conn| snapshots::insert(conn, &obs))
    }

    /// Stores an observation and records it in its window (`window_length_ms`, when the
    /// limit has windows), in one transaction.
    pub fn record_snapshot(
        &self,
        obs: LimitObservation,
        window_length_ms: Option<i64>,
    ) -> Result<i64> {
        self.call(move |conn| {
            let tx = conn.transaction()?;
            let id = snapshots::insert(&tx, &obs)?;
            if let Some(length) = window_length_ms {
                windows::upsert_window(&tx, &obs, length)?;
            }
            tx.commit()?;
            Ok(id)
        })
    }

    /// Stores the API's `seven_day_breakdown`.
    pub fn insert_breakdown(
        &self,
        account: AccountId,
        as_of: Timestamp,
        window_started_at: Option<Timestamp>,
        rows: Vec<(String, Option<f64>)>,
    ) -> Result<()> {
        self.call(move |conn| {
            windows::insert_breakdown(conn, account, as_of, window_started_at, &rows)
        })
    }

    pub fn kv_set(&self, key: String, value: String) -> Result<()> {
        self.call(move |conn| kv::set(conn, &key, &value))
    }

    pub fn kv_delete(&self, key: String) -> Result<()> {
        self.call(move |conn| kv::delete(conn, &key))
    }

    /// Remembers fired notification rules `(rule, dedupe_key, fired_at)`; also forgets rows
    /// older than 30 days.
    pub fn record_notifications(
        &self,
        rows: Vec<(String, String, Timestamp)>,
        now: Timestamp,
    ) -> Result<()> {
        self.call(move |conn| notifications::record(conn, &rows, now))
    }

    /// Deletes raw events older than `older_than`; `usage_5m` is untouched (ADR-020).
    pub fn prune_raw_events(&self, older_than: Timestamp) -> Result<PruneStats> {
        self.call(move |conn| retention::prune_raw_events(conn, older_than))
    }

    /// Runs [`Writer::prune_raw_events`] with a `keep_days` cutoff unless it already ran within
    /// [`RETENTION_INTERVAL_MS`]. Returns `None` when it was not due.
    pub fn prune_if_due(&self, now: Timestamp, keep_days: u32) -> Result<Option<PruneStats>> {
        self.call(move |conn| retention::prune_if_due(conn, now, keep_days))
    }

    /// Stops the writer thread after queued jobs finish. Further calls return `WriterGone`.
    pub fn shutdown(&self) {
        self.tx
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(thread) = self
            .thread
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            let _ = thread.join();
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// PRAGMAs for the read-write connection. `auto_vacuum` only takes effect on a new database,
/// before the first table exists, so it runs before migrations.
fn configure_writer(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA auto_vacuum = INCREMENTAL;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;
         PRAGMA temp_store = MEMORY;
         PRAGMA cache_size = -8000;",
    )?;
    Ok(())
}

/// Read-only access from another process, e.g. the CLI while the app is closed or busy. It
/// never creates, migrates or writes the database; WAL lets it read beside the app's writer.
pub struct ReadOnlyStore {
    conn: Connection,
}

impl ReadOnlyStore {
    /// `None` when the database does not exist or was never initialized.
    pub fn open(path: impl AsRef<Path>) -> Result<Option<Self>> {
        let path = path.as_ref();
        if !path.is_file() {
            return Ok(None);
        }
        let conn = open_reader(path)?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version == 0 {
            return Ok(None);
        }
        if version > migrate::LATEST {
            return Err(StoreError::SchemaTooNew {
                found: version,
                supported: migrate::LATEST,
            });
        }
        Ok(Some(Self { conn }))
    }

    pub fn accounts(&self) -> Result<Vec<Account>> {
        catalog::accounts(&self.conn)
    }

    pub fn latest_snapshots(&self, account: AccountId) -> Result<Vec<LimitObservation>> {
        snapshots::latest(&self.conn, account)
    }
}

fn open_reader(path: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_URI,
    )?;
    // Reads are short and narrow; the writer keeps the 8 MB cache for bulk upserts.
    conn.execute_batch(
        "PRAGMA busy_timeout = 5000;
         PRAGMA temp_store = MEMORY;
         PRAGMA cache_size = -2000;",
    )?;
    Ok(conn)
}

#[cfg(test)]
pub(crate) mod testutil;

#[cfg(test)]
mod tests;
