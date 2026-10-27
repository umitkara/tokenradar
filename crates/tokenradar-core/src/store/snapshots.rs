//! Limit snapshots: every observed limit reading except estimates (architecture §7).

use rusqlite::{Connection, Row, params};

use super::catalog::parse_col;
use super::{Result, StoreError};
use crate::domain::{AccountId, LimitKind, LimitObservation, Scope, Source, Timestamp};

pub(crate) fn insert(conn: &Connection, obs: &LimitObservation) -> Result<i64> {
    if obs.source == Source::Estimate {
        return Err(StoreError::EstimateNotPersisted);
    }
    let id = conn
        .prepare_cached(
            "INSERT INTO limit_snapshots (account_id, limit_kind, scope, percent, resets_at,
                                          severity, source, session_uuid, observed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             RETURNING id",
        )?
        .query_row(
            params![
                obs.account,
                obs.kind.as_str(),
                obs.scope.as_ref().map(ToString::to_string),
                obs.percent,
                obs.resets_at,
                obs.severity,
                obs.source.as_str(),
                obs.session_uuid,
                obs.observed_at
            ],
            |r| r.get(0),
        )?;
    Ok(id)
}

const COLUMNS: &str = "account_id, limit_kind, scope, percent, resets_at, severity, source, \
                       session_uuid, observed_at";

fn from_row(r: &Row<'_>) -> rusqlite::Result<LimitObservation> {
    let kind: String = r.get(1)?;
    let scope: Option<String> = r.get(2)?;
    Ok(LimitObservation {
        account: r.get(0)?,
        kind: LimitKind::from(kind.as_str()),
        scope: scope.map(|s| s.parse::<Scope>()).transpose().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
        })?,
        percent: r.get(3)?,
        resets_at: r.get(4)?,
        severity: r.get(5)?,
        source: parse_col::<Source>(r, 6)?,
        session_uuid: r.get(7)?,
        observed_at: r.get(8)?,
    })
}

/// The newest snapshot per `(limit_kind, scope)` of an account. Ties on `observed_at` go to
/// the later insert.
pub(crate) fn latest(conn: &Connection, account: AccountId) -> Result<Vec<LimitObservation>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM (
           SELECT *, ROW_NUMBER() OVER (
             PARTITION BY limit_kind, scope ORDER BY observed_at DESC, id DESC) AS rn
           FROM limit_snapshots WHERE account_id = ?1)
         WHERE rn = 1
         ORDER BY limit_kind, scope"
    );
    let rows = conn
        .prepare_cached(&sql)?
        .query_map([account], from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// Snapshots of an account with `from <= observed_at < to`, oldest first.
pub(crate) fn between(
    conn: &Connection,
    account: AccountId,
    from: Timestamp,
    to: Timestamp,
) -> Result<Vec<LimitObservation>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM limit_snapshots
         WHERE account_id = ?1 AND observed_at >= ?2 AND observed_at < ?3
         ORDER BY observed_at, id"
    );
    let rows = conn
        .prepare_cached(&sql)?
        .query_map(params![account, from, to], from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}
