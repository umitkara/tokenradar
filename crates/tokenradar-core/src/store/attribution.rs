//! `attributions` / `attribution_runs` (Step 14a, ADR-064) and the reads attribution needs.

use rusqlite::{Connection, OptionalExtension, params};

use super::Result;
use super::windows::WINDOW_MATCH_TOLERANCE_MS;
use crate::attribution::{BucketCost, Method, Share};
use crate::domain::{AccountId, LimitKind, MicroUsd, ModelFamily, SessionId, Timestamp};

/// One window's attribution as stored.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredAttribution {
    pub run: AttributionRun,
    /// Largest percentage (then cost) first; Other is not among them.
    pub shares: Vec<Share>,
}

/// How and when a window was attributed.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributionRun {
    pub window_id: i64,
    pub calibration_version: i64,
    pub method: Method,
    pub confidence: Option<f64>,
    pub total_percent: Option<f64>,
    pub other_percent: Option<f64>,
    pub breakdown_other: Option<f64>,
    pub suspect: bool,
    pub other_excess: bool,
    pub closed: bool,
    pub computed_at: Timestamp,
}

/// `α`-free cost per session tree, family and bucket with `from <= bucket_ts < to`.
pub(crate) fn bucket_costs(
    conn: &Connection,
    account: AccountId,
    from: Timestamp,
    to: Timestamp,
) -> Result<Vec<BucketCost>> {
    let mut stmt = conn.prepare_cached(
        "SELECT u.bucket_ts, COALESCE(s.root_id, s.id), m.family, SUM(u.cost_micro_usd)
         FROM usage_5m u
         JOIN sessions s ON s.id = u.session_id
         JOIN models m ON m.id = u.model_id
         WHERE u.account_id = ?1 AND u.bucket_ts >= ?2 AND u.bucket_ts < ?3
         GROUP BY 1, 2, 3",
    )?;
    let rows = stmt.query_map(params![account, from, to], |r| {
        let family: String = r.get(2)?;
        Ok(BucketCost {
            bucket: r.get(0)?,
            root: SessionId(r.get(1)?),
            family: family.parse().unwrap_or(ModelFamily::Unknown),
            cost: MicroUsd(r.get(3)?),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Start of the account's oldest `usage_5m` bucket.
pub(crate) fn earliest_bucket(conn: &Connection, account: AccountId) -> Result<Option<Timestamp>> {
    Ok(conn
        .prepare_cached("SELECT MIN(bucket_ts) FROM usage_5m WHERE account_id = ?1")?
        .query_row([account], |r| r.get(0))?)
}

/// The non-Claude-Code share of the newest `seven_day_breakdown` of the window starting at
/// `starts_at` (the API reports it a second off), in percent.
pub(crate) fn breakdown_other(
    conn: &Connection,
    account: AccountId,
    starts_at: Timestamp,
) -> Result<Option<f64>> {
    Ok(conn
        .prepare_cached(
            "SELECT SUM(COALESCE(percent, 0)) FROM surface_breakdown
             WHERE account_id = ?1 AND key <> 'claude_code'
               AND as_of = (SELECT MAX(as_of) FROM surface_breakdown
                            WHERE account_id = ?1
                              AND window_started_at BETWEEN ?2 - ?3 AND ?2 + ?3)",
        )?
        .query_row(
            params![account, starts_at, WINDOW_MATCH_TOLERANCE_MS],
            |r| r.get(0),
        )?)
}

/// Adds an estimated window unless one already covers this reset time.
pub(crate) fn insert_estimated_window(
    conn: &Connection,
    account: AccountId,
    kind: &LimitKind,
    starts_at: Timestamp,
    resets_at: Timestamp,
) -> Result<bool> {
    let n = conn
        .prepare_cached(
            "INSERT INTO limit_windows (account_id, limit_kind, scope, starts_at, resets_at,
                                        peak_percent, hit_limit, estimated)
             SELECT ?1, ?2, NULL, ?3, ?4, NULL, 0, 1
             WHERE NOT EXISTS (SELECT 1 FROM limit_windows
                               WHERE account_id = ?1 AND limit_kind = ?2 AND scope IS NULL
                                 AND resets_at BETWEEN ?4 - ?5 AND ?4 + ?5)",
        )?
        .execute(params![
            account,
            kind.as_str(),
            starts_at,
            resets_at,
            WINDOW_MATCH_TOLERANCE_MS
        ])?;
    Ok(n > 0)
}

/// Replaces a window's attribution. An estimated window's percentage becomes its peak.
pub(crate) fn write(conn: &mut Connection, run: &AttributionRun, shares: &[Share]) -> Result<()> {
    let tx = conn.transaction()?;
    tx.prepare_cached("DELETE FROM attributions WHERE window_id = ?1")?
        .execute([run.window_id])?;
    {
        let mut insert = tx.prepare_cached(
            "INSERT INTO attributions (window_id, root_session_id, est_percent, cost_micro_usd)
             VALUES (?1, ?2, ?3, ?4)",
        )?;
        for s in shares {
            insert.execute(params![run.window_id, s.root.0, s.percent, s.cost.0])?;
        }
        if let Some(other) = run.other_percent {
            insert.execute(params![run.window_id, None::<i64>, other, 0])?;
        }
    }
    tx.prepare_cached(
        "INSERT OR REPLACE INTO attribution_runs
           (window_id, calibration_version, method, confidence, total_percent, other_percent,
            breakdown_other, suspect, other_excess, closed, computed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )?
    .execute(params![
        run.window_id,
        run.calibration_version,
        run.method.as_str(),
        run.confidence,
        run.total_percent,
        run.other_percent,
        run.breakdown_other,
        run.suspect,
        run.other_excess,
        run.closed,
        run.computed_at
    ])?;
    tx.prepare_cached(
        "UPDATE limit_windows SET peak_percent = ?2 WHERE id = ?1 AND estimated = 1",
    )?
    .execute(params![run.window_id, run.total_percent])?;
    tx.commit()?;
    Ok(())
}

fn run_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<AttributionRun> {
    let method: String = r.get(2)?;
    Ok(AttributionRun {
        window_id: r.get(0)?,
        calibration_version: r.get(1)?,
        method: Method::parse(&method).unwrap_or(Method::CostOnly),
        confidence: r.get(3)?,
        total_percent: r.get(4)?,
        other_percent: r.get(5)?,
        breakdown_other: r.get(6)?,
        suspect: r.get(7)?,
        other_excess: r.get(8)?,
        closed: r.get(9)?,
        computed_at: r.get(10)?,
    })
}

const RUN_COLUMNS: &str = "r.window_id, r.calibration_version, r.method, r.confidence,
    r.total_percent, r.other_percent, r.breakdown_other, r.suspect, r.other_excess, r.closed,
    r.computed_at";

/// Runs of every window of an account.
pub(crate) fn runs(conn: &Connection, account: AccountId) -> Result<Vec<AttributionRun>> {
    let sql = format!(
        "SELECT {RUN_COLUMNS} FROM attribution_runs r
         JOIN limit_windows w ON w.id = r.window_id WHERE w.account_id = ?1"
    );
    let rows = conn
        .prepare_cached(&sql)?
        .query_map([account], run_from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

pub(crate) fn attribution(conn: &Connection, window_id: i64) -> Result<Option<StoredAttribution>> {
    let sql = format!("SELECT {RUN_COLUMNS} FROM attribution_runs r WHERE r.window_id = ?1");
    let Some(run) = conn
        .prepare_cached(&sql)?
        .query_row([window_id], run_from_row)
        .optional()?
    else {
        return Ok(None);
    };
    let shares = conn
        .prepare_cached(
            "SELECT root_session_id, est_percent, cost_micro_usd FROM attributions
             WHERE window_id = ?1 AND root_session_id IS NOT NULL
             ORDER BY COALESCE(est_percent, 0) DESC, cost_micro_usd DESC, root_session_id",
        )?
        .query_map([window_id], |r| {
            Ok(Share {
                root: SessionId(r.get(0)?),
                percent: r.get(1)?,
                cost: MicroUsd(r.get(2)?),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Some(StoredAttribution { run, shares }))
}
