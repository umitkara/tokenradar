//! `notification_log`: which notification rule fired for which window (the notifier's dedupe
//! memory, kept across restarts).

use rusqlite::{Connection, params};

use super::Result;
use crate::domain::Timestamp;

/// Rows older than this are deleted on the next write; the notifier only looks back
/// [`crate::notifier::HISTORY_MS`].
pub const KEEP_MS: i64 = 30 * 24 * 60 * 60 * 1000;

/// `(rule, dedupe_key, fired_at)` rows fired at or after `since`, oldest first.
pub(crate) fn since(
    conn: &Connection,
    since: Timestamp,
) -> Result<Vec<(String, String, Timestamp)>> {
    let mut stmt = conn.prepare_cached(
        "SELECT rule, dedupe_key, fired_at FROM notification_log
         WHERE fired_at >= ?1 ORDER BY fired_at, id",
    )?;
    let rows = stmt
        .query_map([since.as_millis()], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                Timestamp::from_millis(r.get::<_, Option<i64>>(2)?.unwrap_or(0)),
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Inserts `rows` (an existing key is kept as it was) and drops rows older than [`KEEP_MS`].
pub(crate) fn record(
    conn: &mut Connection,
    rows: &[(String, String, Timestamp)],
    now: Timestamp,
) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut insert = tx.prepare_cached(
            "INSERT INTO notification_log (rule, dedupe_key, fired_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (dedupe_key) DO NOTHING",
        )?;
        for (rule, key, at) in rows {
            insert.execute(params![rule, key, at.as_millis()])?;
        }
        tx.prepare_cached("DELETE FROM notification_log WHERE fired_at < ?1")?
            .execute([now.as_millis() - KEEP_MS])?;
    }
    tx.commit()?;
    Ok(())
}
