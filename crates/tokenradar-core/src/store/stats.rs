//! Counts per account, for the settings window and `doctor`. Numbers only: no paths, names or
//! titles leave this module.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use specta::Type;

use super::Result;
use crate::domain::{AccountId, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct AccountStats {
    pub account_id: AccountId,
    /// Main sessions; subagents are part of their parent.
    pub sessions: u64,
    pub projects: u64,
    /// Transcript files that still exist.
    pub files: u64,
    /// Raw usage events kept (older ones are pruned, ADR-020).
    pub events: u64,
    /// Claude Code's version in the most recent session.
    pub last_cc_version: Option<String>,
    pub last_activity: Option<Timestamp>,
}

pub(crate) fn account_stats(conn: &Connection, account: AccountId) -> Result<AccountStats> {
    let count = |sql: &str| -> Result<u64> {
        let n: i64 = conn
            .prepare_cached(sql)?
            .query_row([account], |r| r.get(0))?;
        Ok(u64::try_from(n).unwrap_or(0))
    };
    let last_activity: Option<Timestamp> = conn
        .prepare_cached("SELECT MAX(last_ts) FROM sessions WHERE account_id = ?1")?
        .query_row([account], |r| r.get(0))?;
    let last_cc_version: Option<String> = conn
        .prepare_cached(
            "SELECT cc_version FROM sessions
             WHERE account_id = ?1 AND cc_version IS NOT NULL
             ORDER BY last_ts DESC LIMIT 1",
        )?
        .query_row([account], |r| r.get(0))
        .optional()?;
    Ok(AccountStats {
        account_id: account,
        sessions: count("SELECT COUNT(*) FROM sessions WHERE account_id = ?1 AND kind = 'main'")?,
        projects: count("SELECT COUNT(*) FROM projects WHERE account_id = ?1")?,
        files: count(
            "SELECT COUNT(*) FROM source_files WHERE account_id = ?1 AND deleted_at IS NULL",
        )?,
        events: count("SELECT COUNT(*) FROM usage_events WHERE account_id = ?1")?,
        last_cc_version,
        last_activity,
    })
}

/// Rows per table, for `doctor`.
pub(crate) fn table_counts(conn: &Connection) -> Result<Vec<(String, u64)>> {
    const TABLES: [&str; 9] = [
        "accounts",
        "projects",
        "sessions",
        "source_files",
        "usage_events",
        "usage_5m",
        "limit_snapshots",
        "limit_windows",
        "notification_log",
    ];
    let mut out = Vec::with_capacity(TABLES.len());
    for table in TABLES {
        let n: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?;
        out.push((table.to_owned(), u64::try_from(n).unwrap_or(0)));
    }
    Ok(out)
}
