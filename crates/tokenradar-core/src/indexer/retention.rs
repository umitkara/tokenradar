//! Runs raw-event retention once a day (ADR-020) on a small background thread.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use crate::domain::Timestamp;
use crate::store::Store;

/// Default raw-event retention (ADR-020).
pub const DEFAULT_KEEP_DAYS: u32 = 90;

/// How often the thread asks the store whether retention is due. The store remembers the last
/// run, so restarts and sleep do not cause extra runs; this only bounds the delay.
pub const CHECK_EVERY: Duration = Duration::from_hours(1);

/// Handle to the retention thread; dropping it stops the thread.
pub struct RetentionScheduler {
    keep_days: Arc<AtomicU32>,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl RetentionScheduler {
    /// Checks immediately, then every `check_every`.
    pub fn spawn(store: Store, keep_days: u32, check_every: Duration) -> std::io::Result<Self> {
        let (stop, rx) = mpsc::channel::<()>();
        let keep_days = Arc::new(AtomicU32::new(keep_days));
        let days = Arc::clone(&keep_days);
        let thread = thread::Builder::new()
            .name("retention".into())
            .spawn(move || {
                loop {
                    let keep_days = days.load(Ordering::Relaxed);
                    match store.writer().prune_if_due(Timestamp::now(), keep_days) {
                        Ok(Some(stats)) => tracing::debug!(?stats, "retention ran"),
                        Ok(None) => {}
                        Err(error) => tracing::warn!(%error, "retention failed; will retry"),
                    }
                    match rx.recv_timeout(check_every) {
                        Err(RecvTimeoutError::Timeout) => {}
                        Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            })?;
        Ok(Self {
            keep_days,
            stop: Some(stop),
            thread: Some(thread),
        })
    }

    /// Changes the retention for the next check (at most `check_every` away; the store still
    /// prunes at most once a day).
    pub fn set_keep_days(&self, days: u32) {
        self.keep_days.store(days, Ordering::Relaxed);
    }
}

impl Drop for RetentionScheduler {
    fn drop(&mut self) {
        self.stop.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
