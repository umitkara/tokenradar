//! Indexer bookkeeping: per-file read positions (`source_files`) and the batch commit that
//! keeps them consistent with the events they produced.

use std::collections::HashMap;

use rusqlite::{Connection, Transaction, params};

use super::{Result, StoreError, catalog, events};
use crate::domain::{AccountId, SessionId, SessionKind, SessionSpec, Timestamp, UsageEvent};

/// Read position of one transcript file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFileState {
    pub account_id: AccountId,
    /// Path as the indexer builds it; the unique key of the row.
    pub path: String,
    pub kind: SessionKind,
    /// File size when the indexer last looked.
    pub size: u64,
    /// Modification time in epoch milliseconds when the indexer last looked.
    pub mtime: i64,
    /// End of the last complete line indexed. Never past `size`.
    pub offset: u64,
    pub session_id: Option<SessionId>,
    pub deleted_at: Option<Timestamp>,
}

/// Everything one indexer flush writes. Committed in a single transaction so that a file's
/// offset never moves past events that are not stored yet (crash consistency).
#[derive(Debug, Default)]
pub struct IndexBatch {
    pub sessions: Vec<SessionSpec>,
    pub events: Vec<UsageEvent>,
    pub files: Vec<SourceFileState>,
}

pub(crate) fn commit(conn: &mut Connection, batch: &IndexBatch) -> Result<events::UpsertStats> {
    let tx = conn.transaction()?;
    for spec in &batch.sessions {
        catalog::upsert_session(&tx, spec)?;
    }
    let stats = events::upsert(&tx, &batch.events)?;
    for file in &batch.files {
        upsert_file(&tx, file)?;
    }
    tx.commit()?;
    Ok(stats)
}

fn upsert_file(tx: &Transaction<'_>, f: &SourceFileState) -> Result<()> {
    tx.prepare_cached(
        "INSERT INTO source_files (account_id, path, kind, size, mtime, offset, session_id,
                                   deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT (path) DO UPDATE SET
           account_id = excluded.account_id,
           kind = excluded.kind,
           size = excluded.size,
           mtime = excluded.mtime,
           offset = excluded.offset,
           session_id = COALESCE(excluded.session_id, session_id),
           deleted_at = excluded.deleted_at",
    )?
    .execute(params![
        f.account_id,
        f.path,
        f.kind.as_str(),
        to_sql(f.size, "source_files.size")?,
        f.mtime,
        to_sql(f.offset, "source_files.offset")?,
        f.session_id,
        f.deleted_at,
    ])?;
    Ok(())
}

/// All file rows of an account, keyed by path.
pub(crate) fn files(
    conn: &Connection,
    account: AccountId,
) -> Result<HashMap<String, SourceFileState>> {
    let mut stmt = conn.prepare_cached(
        "SELECT path, kind, size, mtime, offset, session_id, deleted_at
         FROM source_files WHERE account_id = ?1",
    )?;
    let rows = stmt.query_map([account], |r| {
        Ok(SourceFileState {
            account_id: account,
            path: r.get(0)?,
            kind: catalog::parse_col(r, 1)?,
            size: from_sql(r.get(2)?),
            mtime: r.get(3)?,
            offset: from_sql(r.get(4)?),
            session_id: r.get(5)?,
            deleted_at: r.get(6)?,
        })
    })?;
    let mut out = HashMap::new();
    for row in rows {
        let row = row?;
        out.insert(row.path.clone(), row);
    }
    Ok(out)
}

/// Marks files as deleted. Their events and rollups stay (data-sources.md S3, pitfall 7).
pub(crate) fn mark_deleted(conn: &mut Connection, paths: &[String], at: Timestamp) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare_cached(
            "UPDATE source_files SET deleted_at = ?2 WHERE path = ?1 AND deleted_at IS NULL",
        )?;
        for path in paths {
            stmt.execute(params![path, at])?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn to_sql(v: u64, column: &'static str) -> Result<i64> {
    i64::try_from(v).map_err(|_| StoreError::Corrupt {
        column,
        detail: format!("{v} does not fit in an INTEGER"),
    })
}

/// Sizes and offsets are written from `u64` and never negative.
fn from_sql(v: i64) -> u64 {
    u64::try_from(v).unwrap_or(0)
}
