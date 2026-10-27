//! Limit windows, the API's surface breakdown and the usage aggregates the limit engine needs.

use rusqlite::{Connection, OptionalExtension, params};

use super::Result;
use crate::domain::{
    AccountId, LimitKind, LimitObservation, LimitWindow, MicroUsd, Scope, SessionId, Timestamp,
};
use crate::limits::RootUsage;

/// Two `resets_at` closer than this are the same window. The API reports microseconds that
/// drift between calls and the statusline whole seconds; real windows are hours apart.
pub const WINDOW_MATCH_TOLERANCE_MS: i64 = 5 * 60 * 1000;

/// Records `obs` in its window: creates the window on a new `resets_at`, otherwise raises
/// `peak_percent` and `hit_limit`. `length_ms` is the window's length (5 h, 7 d).
pub(crate) fn upsert_window(
    conn: &Connection,
    obs: &LimitObservation,
    length_ms: i64,
) -> Result<()> {
    let Some(resets_at) = obs.resets_at else {
        return Ok(());
    };
    let scope = obs.scope.as_ref().map(ToString::to_string);
    let hit = i64::from(obs.percent >= 100.0);
    let existing: Option<i64> = conn
        .prepare_cached(
            "SELECT id FROM limit_windows
             WHERE account_id = ?1 AND limit_kind = ?2 AND scope IS ?3
               AND resets_at BETWEEN ?4 - ?5 AND ?4 + ?5
             ORDER BY ABS(resets_at - ?4) LIMIT 1",
        )?
        .query_row(
            params![
                obs.account,
                obs.kind.as_str(),
                scope,
                resets_at,
                WINDOW_MATCH_TOLERANCE_MS
            ],
            |r| r.get(0),
        )
        .optional()?;
    match existing {
        Some(id) => {
            conn.prepare_cached(
                "UPDATE limit_windows
                 SET peak_percent = MAX(COALESCE(peak_percent, ?2), ?2),
                     hit_limit = MAX(hit_limit, ?3),
                     estimated = 0
                 WHERE id = ?1",
            )?
            .execute(params![id, obs.percent, hit])?;
        }
        None => {
            conn.prepare_cached(
                "INSERT INTO limit_windows
                   (account_id, limit_kind, scope, starts_at, resets_at, peak_percent, hit_limit)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?
            .execute(params![
                obs.account,
                obs.kind.as_str(),
                scope,
                resets_at.saturating_sub_millis(length_ms),
                resets_at,
                obs.percent,
                hit
            ])?;
        }
    }
    Ok(())
}

pub(crate) fn windows(conn: &Connection, account: AccountId) -> Result<Vec<LimitWindow>> {
    let rows = conn
        .prepare_cached(
            "SELECT id, account_id, limit_kind, scope, starts_at, resets_at, peak_percent,
                    hit_limit, estimated
             FROM limit_windows WHERE account_id = ?1 ORDER BY limit_kind, scope, resets_at",
        )?
        .query_map([account], |r| {
            let kind: String = r.get(2)?;
            let scope: Option<String> = r.get(3)?;
            Ok(LimitWindow {
                id: r.get(0)?,
                account: r.get(1)?,
                kind: LimitKind::from(kind.as_str()),
                scope: scope.and_then(|s| s.parse::<Scope>().ok()),
                starts_at: r.get(4)?,
                resets_at: r.get(5)?,
                peak_percent: r.get(6)?,
                hit_limit: r.get::<_, i64>(7)? != 0,
                estimated: r.get::<_, i64>(8)? != 0,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// Stores one `seven_day_breakdown`. Repeated polls report the same `as_of`; those are kept
/// once.
pub(crate) fn insert_breakdown(
    conn: &mut Connection,
    account: AccountId,
    as_of: Timestamp,
    window_started_at: Option<Timestamp>,
    rows: &[(String, Option<f64>)],
) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO surface_breakdown
               (account_id, as_of, window_started_at, key, percent)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for (key, percent) in rows {
            stmt.execute(params![account, as_of, window_started_at, key, percent])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Cost and tokens per session tree (`COALESCE(root_id, id)`) with `from <= ts < to`,
/// largest cost first.
pub(crate) fn root_usage_between(
    conn: &Connection,
    account: AccountId,
    from: Timestamp,
    to: Timestamp,
) -> Result<Vec<RootUsage>> {
    let rows = conn
        .prepare_cached(
            "SELECT COALESCE(s.root_id, s.id) AS root,
                    SUM(e.cost_micro_usd),
                    SUM(e.input + e.output + e.cache_w5m + e.cache_w1h)
             FROM usage_events e JOIN sessions s ON s.id = e.session_id
             WHERE e.account_id = ?1 AND e.ts >= ?2 AND e.ts < ?3
             GROUP BY root
             ORDER BY 2 DESC, root",
        )?
        .query_map(params![account, from, to], |r| {
            Ok(RootUsage {
                root: SessionId(r.get(0)?),
                cost: MicroUsd(r.get(1)?),
                tokens: u64::try_from(r.get::<_, i64>(2)?).unwrap_or(0),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// Total cost with `from <= ts < to`.
pub(crate) fn cost_between(
    conn: &Connection,
    account: AccountId,
    from: Timestamp,
    to: Timestamp,
) -> Result<MicroUsd> {
    let cost: i64 = conn
        .prepare_cached(
            "SELECT COALESCE(SUM(cost_micro_usd), 0) FROM usage_events
             WHERE account_id = ?1 AND ts >= ?2 AND ts < ?3",
        )?
        .query_row(params![account, from, to], |r| r.get(0))?;
    Ok(MicroUsd(cost))
}

/// Display data of a session: title and project name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLabel {
    pub id: SessionId,
    pub title: Option<String>,
    pub project: Option<String>,
}

pub(crate) fn session_labels(conn: &Connection, ids: &[SessionId]) -> Result<Vec<SessionLabel>> {
    let mut stmt = conn.prepare_cached(
        "SELECT s.title, p.display_name FROM sessions s
         LEFT JOIN projects p ON p.id = s.project_id WHERE s.id = ?1",
    )?;
    let mut out = Vec::with_capacity(ids.len());
    for &id in ids {
        if let Some((title, project)) = stmt
            .query_row([id], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?
        {
            out.push(SessionLabel { id, title, project });
        }
    }
    Ok(out)
}
