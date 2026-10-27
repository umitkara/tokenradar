//! Retention: raw events are pruned after a period, the five-minute rollup is kept (ADR-020).

use rusqlite::{Connection, OptionalExtension, params};

use super::Result;
use crate::domain::{AccountId, Timestamp};

/// Rows deleted per transaction, so the writer never holds the lock for long.
const PRUNE_CHUNK: i64 = 10_000;

/// Free pages returned to the OS per `incremental_vacuum` call (4 KiB pages → 16 MiB).
const VACUUM_PAGES: u32 = 4096;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PruneStats {
    pub deleted: usize,
    pub vacuumed_pages: u64,
}

/// When retention last ran, in epoch milliseconds.
const LAST_RUN_KEY: &str = "retention.last_run";

/// How often retention runs.
pub const RETENTION_INTERVAL_MS: i64 = 24 * 60 * 60 * 1000;

fn watermark_key(account: AccountId) -> String {
    format!("retention.pruned_before.{account}")
}

/// Events older than this were pruned for the account. Re-indexing them must not insert them
/// again, because `usage_5m` already counts them.
pub(crate) fn watermark(conn: &Connection, account: AccountId) -> Result<Option<Timestamp>> {
    let value: Option<String> = conn
        .prepare_cached("SELECT value FROM kv WHERE key = ?1")?
        .query_row([watermark_key(account)], |r| r.get(0))
        .optional()?;
    Ok(value
        .and_then(|v| v.parse().ok())
        .map(Timestamp::from_millis))
}

/// Deletes `usage_events` rows with `ts < older_than`, records the watermark for every
/// account, then returns free pages with `incremental_vacuum`. Never touches `usage_5m`.
///
/// The watermark only moves forward: pruning with an older cutoff later keeps the newer one.
pub(crate) fn prune_raw_events(conn: &mut Connection, older_than: Timestamp) -> Result<PruneStats> {
    let mut stats = PruneStats::default();
    {
        let tx = conn.transaction()?;
        let accounts: Vec<AccountId> = tx
            .prepare("SELECT id FROM accounts")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        for account in accounts {
            let current = watermark(&tx, account)?;
            if current.is_none_or(|w| w < older_than) {
                tx.execute(
                    "INSERT INTO kv (key, value) VALUES (?1, ?2)
                     ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                    params![watermark_key(account), older_than.as_millis().to_string()],
                )?;
            }
        }
        tx.commit()?;
    }
    loop {
        let tx = conn.transaction()?;
        let deleted = tx.execute(
            "DELETE FROM usage_events WHERE id IN
               (SELECT id FROM usage_events WHERE ts < ?1 LIMIT ?2)",
            params![older_than, PRUNE_CHUNK],
        )?;
        tx.commit()?;
        stats.deleted += deleted;
        if i64::try_from(deleted).unwrap_or(i64::MAX) < PRUNE_CHUNK {
            break;
        }
    }
    if stats.deleted > 0 {
        let free_before: i64 = conn.query_row("PRAGMA freelist_count", [], |r| r.get(0))?;
        conn.execute_batch(&format!("PRAGMA incremental_vacuum({VACUUM_PAGES});"))?;
        let free_after: i64 = conn.query_row("PRAGMA freelist_count", [], |r| r.get(0))?;
        stats.vacuumed_pages = u64::try_from(free_before - free_after).unwrap_or(0);
    }
    tracing::info!(
        deleted = stats.deleted,
        vacuumed_pages = stats.vacuumed_pages,
        "pruned raw usage events"
    );
    Ok(stats)
}

/// Prunes raw events older than `keep_days` if retention has not run in the last
/// [`RETENTION_INTERVAL_MS`]. Returns `None` when it was not due.
pub(crate) fn prune_if_due(
    conn: &mut Connection,
    now: Timestamp,
    keep_days: u32,
) -> Result<Option<PruneStats>> {
    let last: Option<i64> = conn
        .prepare_cached("SELECT value FROM kv WHERE key = ?1")?
        .query_row([LAST_RUN_KEY], |r| r.get::<_, String>(0))
        .optional()?
        .and_then(|v| v.parse().ok());
    if last.is_some_and(|t| now.as_millis() - t < RETENTION_INTERVAL_MS && t <= now.as_millis()) {
        return Ok(None);
    }
    let cutoff = now.saturating_sub_millis(i64::from(keep_days) * 24 * 60 * 60 * 1000);
    let stats = prune_raw_events(conn, cutoff)?;
    conn.execute(
        "INSERT INTO kv (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![LAST_RUN_KEY, now.as_millis().to_string()],
    )?;
    Ok(Some(stats))
}
