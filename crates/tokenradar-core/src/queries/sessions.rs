//! Session trees (a main session with its subagents): the list, one session, and its share of
//! every window it touched.

use std::collections::{BTreeMap, HashMap};

use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::{
    Basis, Cursor, ProjectRef, QueryContext, Result, SessionName, TimeRange, TokenSums, page_size,
    session_name,
};
use crate::attribution::Method;
use crate::domain::{
    BUCKET_MS, LimitKind, MicroUsd, ModelFamily, ProjectId, Scope, SessionId, Timestamp,
};
use crate::pricing::Pricing;
use crate::store::Store;

/// Which sessions a list shows. Cost and tokens count the range only.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
pub struct SessionFilter {
    pub range: Option<TimeRange>,
    pub project: Option<ProjectId>,
    /// Sessions that used this model family in the range.
    pub family: Option<ModelFamily>,
    /// Only the sessions whose uuid the caller lists as running.
    pub live_only: bool,
    /// Matches title, project name and uuid.
    pub search: Option<String>,
}

/// Descending order of a session list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SessionSort {
    #[default]
    LastActive,
    Started,
    Cost,
    Tokens,
    /// Weekly share.
    Impact,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct SessionRow {
    pub id: SessionId,
    pub uuid: String,
    pub name: SessionName,
    pub project: Option<ProjectRef>,
    pub first_ts: Option<Timestamp>,
    pub last_ts: Option<Timestamp>,
    pub live: bool,
    /// Model names used in the range, most expensive first.
    pub models: Vec<String>,
    pub cost: MicroUsd,
    pub tokens: TokenSums,
    pub subagents: u32,
    /// `≈` points of weekly windows in the range, summed; `None` without a percentage.
    pub weekly_percent: Option<f64>,
    /// Points of 5-hour windows in the range, summed (accounts that have them).
    pub session_percent: Option<f64>,
    /// The weakest basis behind the two sums.
    pub impact_basis: Option<Basis>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct SessionPage {
    pub items: Vec<SessionRow>,
    pub next: Option<String>,
}

/// One window a session took part in.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ImpactRow {
    pub window_id: i64,
    pub kind: LimitKind,
    pub scope: Option<Scope>,
    pub starts_at: Timestamp,
    pub resets_at: Timestamp,
    pub peak_percent: Option<f64>,
    /// `None` for an estimated window.
    pub hit_limit: Option<bool>,
    pub estimated: bool,
    pub percent: Option<f64>,
    pub cost: MicroUsd,
    pub basis: Basis,
}

/// Cost of one model in one 5-minute bucket.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct TimelineBucket {
    pub at: Timestamp,
    pub model: String,
    pub cost: MicroUsd,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct SubagentRow {
    pub id: SessionId,
    pub uuid: String,
    /// Agent title or type when known.
    pub title: Option<String>,
    pub cost: MicroUsd,
    pub tokens: TokenSums,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct CacheStats {
    /// `cache_read / (input + cache_read + cache_write)`, 0–1; `None` without input.
    pub hit_ratio: Option<f64>,
    pub write_5m: i64,
    pub write_1h: i64,
    /// What the cache reads would have cost as plain input, minus what they cost.
    pub savings: MicroUsd,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct SessionDetail {
    /// Totals over the session's whole life.
    pub session: SessionRow,
    pub git_branch: Option<String>,
    pub cc_version: Option<String>,
    pub entrypoint: Option<String>,
    /// The project's normalized path (`unresolved:<dir>` when the cwd is unknown).
    pub path: Option<String>,
    /// Cost per model per 5 minutes; merged into wider buckets beyond 1,500 buckets.
    pub timeline: Vec<TimelineBucket>,
    /// Width of a `timeline` bucket in ms.
    pub bucket_ms: i64,
    pub subagents: Vec<SubagentRow>,
    pub cache: CacheStats,
    pub impact: Vec<ImpactRow>,
}

fn sort_expr(sort: SessionSort) -> &'static str {
    match sort {
        SessionSort::LastActive => "COALESCE(s.last_ts, 0)",
        SessionSort::Started => "COALESCE(s.first_ts, 0)",
        SessionSort::Cost => "u.cost",
        SessionSort::Tokens => "u.tokens",
        SessionSort::Impact => "COALESCE(i.weekly, -1.0)",
    }
}

/// Session trees with usage in the range, one page after `cursor`, in `sort` order (then id),
/// all descending. `live`: uuids of the running sessions.
#[allow(clippy::too_many_lines)] // builds one statement from the filter, then reads it
pub fn list_sessions(
    store: &Store,
    ctx: &QueryContext<'_>,
    filter: &SessionFilter,
    sort: SessionSort,
    cursor: Option<&str>,
    limit: Option<u32>,
    live: &[String],
) -> Result<SessionPage> {
    if filter.live_only && live.is_empty() {
        return Ok(SessionPage {
            items: Vec::new(),
            next: None,
        });
    }
    let n = page_size(limit);
    let after = Cursor::decode(cursor).unwrap_or(Cursor::START);
    let (from, to) = super::bounds(filter.range);
    let mut args: Vec<Value> = vec![ctx.account.0.into(), from.into(), to.into()];
    let (family_col, family_having) = match filter.family {
        Some(f) => {
            args.push(f.as_str().to_owned().into());
            (
                format!(
                    ", MAX(u.model_id IN (SELECT id FROM models WHERE family = ?{})) AS fam",
                    args.len()
                ),
                "HAVING MAX(ps.fam) > 0",
            )
        }
        None => (String::new(), ""),
    };
    let mut wheres = Vec::new();
    let mut only = String::new();
    if let Some(p) = filter.project {
        args.push(p.0.into());
        wheres.push(format!("s.project_id = ?{}", args.len()));
        only = format!(
            "AND u.session_id IN ({})",
            super::project_sessions(args.len())
        );
    }
    if let Some(q) = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
    {
        args.push(super::like_pattern(q).into());
        let i = args.len();
        wheres.push(format!(
            "(s.title LIKE ?{i} ESCAPE '\\' OR p.display_name LIKE ?{i} ESCAPE '\\'
              OR s.uuid LIKE ?{i} ESCAPE '\\')"
        ));
    }
    if filter.live_only {
        let start = args.len() + 1;
        let marks: Vec<String> = (start..start + live.len())
            .map(|i| format!("?{i}"))
            .collect();
        args.extend(live.iter().map(|u| Value::from(u.clone())));
        wheres.push(format!("s.uuid IN ({})", marks.join(",")));
    }
    args.push(after.key.into());
    let k = args.len();
    args.push(after.id.into());
    let id = args.len();
    let where_sql = if wheres.is_empty() {
        String::new()
    } else {
        format!("AND {}", wheres.join(" AND "))
    };
    // Per session in key order first, then per tree: joining `sessions` for every bucket row
    // costs more than the aggregation itself.
    let sql = format!(
        "WITH ps AS MATERIALIZED (
           SELECT u.session_id AS sid, SUM(u.cost_micro_usd) AS cost,
                  SUM(u.input) AS input, SUM(u.output) AS output, SUM(u.cache_read) AS cr,
                  SUM(u.cache_w5m + u.cache_w1h) AS cw {family_col}
           FROM usage_5m u
           WHERE u.account_id = ?1 AND {b} >= ?2 AND {b} < ?3 {only}
           GROUP BY u.session_id),
         u AS MATERIALIZED (
           SELECT COALESCE(s.root_id, s.id) AS root, SUM(ps.cost) AS cost,
                  SUM(ps.input) AS input, SUM(ps.output) AS output, SUM(ps.cr) AS cr,
                  SUM(ps.cw) AS cw, SUM(ps.input + ps.output + ps.cr + ps.cw) AS tokens
           FROM ps JOIN sessions s ON s.id = ps.sid
           GROUP BY root {family_having}),
         i AS (
           SELECT a.root_session_id AS root,
                  SUM(CASE WHEN w.limit_kind = 'weekly_all' THEN a.est_percent END) AS weekly,
                  SUM(CASE WHEN w.limit_kind = 'session' THEN a.est_percent END) AS five
           FROM attributions a JOIN limit_windows w ON w.id = a.window_id
           WHERE w.account_id = ?1 AND w.scope IS NULL AND w.resets_at > ?2 AND w.starts_at < ?3
             AND a.root_session_id IN (SELECT root FROM u)
           GROUP BY a.root_session_id)
         SELECT * FROM (
           SELECT s.id, s.uuid, s.title, s.first_ts, s.last_ts, p.id, p.display_name,
                  u.cost, u.input, u.output, u.cr, u.cw, i.weekly, i.five,
                  (SELECT COUNT(*) FROM sessions c WHERE c.root_id = s.id),
                  {key} AS k
           FROM u JOIN sessions s ON s.id = u.root
           LEFT JOIN projects p ON p.id = s.project_id
           LEFT JOIN i ON i.root = s.id
           WHERE 1 {where_sql})
         WHERE k < ?{k} OR (k = ?{k} AND id < ?{id})
         ORDER BY k DESC, id DESC LIMIT {limit}",
        key = sort_expr(sort),
        limit = n + 1,
        b = if only.is_empty() {
            super::bucket_col(from, to)
        } else {
            super::TREE_BUCKET
        },
    );
    let (mut items, keys) = store.read(|c| {
        let mut items = Vec::new();
        let mut keys = Vec::new();
        let mut stmt = c.prepare_cached(&sql)?;
        let mut rows = stmt.query(params_from_iter(args))?;
        while let Some(r) = rows.next()? {
            let uuid: String = r.get(1)?;
            let title: Option<String> = r.get(2)?;
            let pid: Option<i64> = r.get(5)?;
            let pname: Option<String> = r.get(6)?;
            items.push(SessionRow {
                id: SessionId(r.get(0)?),
                name: session_name(title.as_deref(), pname.as_deref(), &uuid),
                live: live.contains(&uuid),
                uuid,
                project: pid.zip(pname).map(|(id, name)| ProjectRef {
                    id: ProjectId(id),
                    name,
                }),
                first_ts: r.get(3)?,
                last_ts: r.get(4)?,
                models: Vec::new(),
                cost: MicroUsd(r.get(7)?),
                tokens: TokenSums {
                    input: r.get(8)?,
                    output: r.get(9)?,
                    cache_read: r.get(10)?,
                    cache_write: r.get(11)?,
                },
                weekly_percent: r.get(12)?,
                session_percent: r.get(13)?,
                subagents: u32::try_from(r.get::<_, i64>(14)?).unwrap_or(0),
                impact_basis: None,
            });
            keys.push(r.get::<_, f64>(15)?);
        }
        Ok((items, keys))
    })?;
    let more = items.len() > n as usize;
    items.truncate(n as usize);
    let next = more
        .then(|| items.last().zip(keys.get(n as usize - 1)))
        .flatten()
        .map(|(row, key)| {
            Cursor {
                key: *key,
                id: row.id.0,
            }
            .encode()
        });
    store.read(|c| {
        for row in &mut items {
            row.models = tree_models(c, row.id, from, to)?;
            row.impact_basis = impact_basis(c, ctx, row.id, from, to)?;
        }
        Ok(())
    })?;
    Ok(SessionPage { items, next })
}

/// Model names a session tree used in `[from, to)`, most expensive first.
fn tree_models(c: &Connection, root: SessionId, from: i64, to: i64) -> Result<Vec<String>> {
    Ok(c.prepare_cached(&format!(
        "SELECT m.name FROM usage_5m u JOIN models m ON m.id = u.model_id
         WHERE u.account_id = (SELECT account_id FROM sessions WHERE id = ?1)
           AND u.session_id IN (SELECT id FROM sessions WHERE id = ?1 OR root_id = ?1)
           AND {b} >= ?2 AND {b} < ?3
         GROUP BY m.name ORDER BY SUM(u.cost_micro_usd) DESC",
        b = super::TREE_BUCKET
    ))?
    .query_map(params![root.0, from, to], |r| r.get(0))?
    .collect::<rusqlite::Result<_>>()?)
}

/// Basis of every unscoped window share of a tree in the range, combined.
fn impact_basis(
    c: &Connection,
    ctx: &QueryContext<'_>,
    root: SessionId,
    from: i64,
    to: i64,
) -> Result<Option<Basis>> {
    let rows: Vec<(String, Option<f64>, i64, String)> = c
        .prepare_cached(
            "SELECT r.method, r.confidence, r.calibration_version, w.limit_kind
             FROM attributions a JOIN attribution_runs r ON r.window_id = a.window_id
             JOIN limit_windows w ON w.id = a.window_id
             WHERE a.root_session_id = ?1 AND w.scope IS NULL
               AND w.resets_at > ?2 AND w.starts_at < ?3 AND a.est_percent IS NOT NULL",
        )?
        .query_map(params![root.0, from, to], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Basis::weakest(rows.into_iter().filter_map(
        |(m, conf, version, kind)| {
            let kind = LimitKind::from(kind.as_str());
            let current = ctx
                .calibrations
                .iter()
                .find(|c| c.kind == kind && c.scope.is_none())
                .map_or(0, |c| c.version);
            Method::parse(&m).map(|m| Basis::new(m, conf, version != current))
        },
    )))
}

/// Every window a session tree has a share in, newest first.
pub fn get_session_impact(
    store: &Store,
    ctx: &QueryContext<'_>,
    id: SessionId,
) -> Result<Vec<ImpactRow>> {
    store.read(|c| impact_rows(c, ctx, id))
}

fn impact_rows(c: &Connection, ctx: &QueryContext<'_>, id: SessionId) -> Result<Vec<ImpactRow>> {
    let rows = c
        .prepare_cached(
            "SELECT w.id, w.limit_kind, w.scope, w.starts_at, w.resets_at, w.peak_percent,
                    w.hit_limit, w.estimated, a.est_percent, a.cost_micro_usd,
                    r.method, r.confidence, r.calibration_version
             FROM attributions a JOIN limit_windows w ON w.id = a.window_id
             JOIN attribution_runs r ON r.window_id = a.window_id
             WHERE a.root_session_id = ?1 AND w.account_id = ?2
             ORDER BY w.resets_at DESC, w.id DESC",
        )?
        .query_map(params![id.0, ctx.account.0], |r| {
            let kind: String = r.get(1)?;
            let scope: Option<String> = r.get(2)?;
            let method: String = r.get(10)?;
            let estimated = r.get::<_, i64>(7)? != 0;
            Ok((
                ImpactRow {
                    window_id: r.get(0)?,
                    kind: LimitKind::from(kind.as_str()),
                    scope: scope.and_then(|s| s.parse().ok()),
                    starts_at: r.get(3)?,
                    resets_at: r.get(4)?,
                    peak_percent: r.get(5)?,
                    hit_limit: (!estimated).then_some(r.get::<_, i64>(6)? != 0),
                    estimated,
                    percent: r.get(8)?,
                    cost: MicroUsd(r.get(9)?),
                    basis: Basis::new(Method::CostOnly, None, false),
                },
                Method::parse(&method).unwrap_or(Method::CostOnly),
                r.get::<_, Option<f64>>(11)?,
                r.get::<_, i64>(12)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows
        .into_iter()
        .map(|(mut row, method, conf, version)| {
            let current = ctx
                .calibrations
                .iter()
                .find(|c| c.kind == row.kind && c.scope == row.scope)
                .map_or(0, |c| c.version);
            row.basis = Basis::new(method, conf, version != current);
            row
        })
        .collect())
}

/// One session tree over its whole life; `None` for an unknown id. A subagent id opens its
/// main session.
pub fn get_session(
    store: &Store,
    ctx: &QueryContext<'_>,
    id: SessionId,
    live: &[String],
) -> Result<Option<SessionDetail>> {
    let root: Option<i64> = store.read(|c| {
        Ok(c.prepare_cached(
            "SELECT COALESCE(root_id, id) FROM sessions WHERE id = ?1 AND account_id = ?2",
        )?
        .query_row(params![id.0, ctx.account.0], |r| r.get(0))
        .optional()?)
    })?;
    let Some(root) = root.map(SessionId) else {
        return Ok(None);
    };
    let row = store.read(|c| tree_row(c, ctx, root, live))?;
    let Some(session) = row else {
        return Ok(None);
    };
    store
        .read(|c| {
            let (git_branch, cc_version, entrypoint, path): (
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
            ) = c
                .prepare_cached(
                    "SELECT s.git_branch, s.cc_version, s.entrypoint, p.path_norm FROM sessions s
             LEFT JOIN projects p ON p.id = s.project_id WHERE s.id = ?1",
                )?
                .query_row([root.0], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                })?;
            let (timeline, bucket_ms) = timeline(c, root)?;
            Ok(SessionDetail {
                git_branch,
                cc_version,
                entrypoint,
                path,
                timeline,
                bucket_ms,
                subagents: subagents(c, root)?,
                cache: cache_stats(c, root)?,
                impact: impact_rows(c, ctx, root)?,
                session,
            })
        })
        .map(Some)
}

/// The list row of one tree, all time.
fn tree_row(
    c: &Connection,
    ctx: &QueryContext<'_>,
    root: SessionId,
    live: &[String],
) -> Result<Option<SessionRow>> {
    let row = c
        .prepare_cached(
            "SELECT s.uuid, s.title, s.first_ts, s.last_ts, p.id, p.display_name,
                    COALESCE(SUM(u.cost_micro_usd), 0), COALESCE(SUM(u.input), 0),
                    COALESCE(SUM(u.output), 0), COALESCE(SUM(u.cache_read), 0),
                    COALESCE(SUM(u.cache_w5m + u.cache_w1h), 0),
                    (SELECT COUNT(*) FROM sessions c WHERE c.root_id = s.id)
             FROM sessions s LEFT JOIN projects p ON p.id = s.project_id
             LEFT JOIN usage_5m u
               ON u.account_id = s.account_id
              AND u.session_id IN (SELECT id FROM sessions WHERE id = s.id OR root_id = s.id)
             WHERE s.id = ?1 GROUP BY s.id",
        )?
        .query_row([root.0], |r| {
            let uuid: String = r.get(0)?;
            let title: Option<String> = r.get(1)?;
            let pid: Option<i64> = r.get(4)?;
            let pname: Option<String> = r.get(5)?;
            Ok(SessionRow {
                id: root,
                name: session_name(title.as_deref(), pname.as_deref(), &uuid),
                live: live.contains(&uuid),
                uuid,
                project: pid.zip(pname).map(|(id, name)| ProjectRef {
                    id: ProjectId(id),
                    name,
                }),
                first_ts: r.get(2)?,
                last_ts: r.get(3)?,
                models: Vec::new(),
                cost: MicroUsd(r.get(6)?),
                tokens: TokenSums {
                    input: r.get(7)?,
                    output: r.get(8)?,
                    cache_read: r.get(9)?,
                    cache_write: r.get(10)?,
                },
                subagents: u32::try_from(r.get::<_, i64>(11)?).unwrap_or(0),
                weekly_percent: None,
                session_percent: None,
                impact_basis: None,
            })
        })
        .optional()?;
    let Some(mut row) = row else {
        return Ok(None);
    };
    row.models = tree_models(c, root, 0, i64::MAX)?;
    let impact = impact_rows(c, ctx, root)?;
    let sum = |kind: LimitKind| {
        let v: Vec<f64> = impact
            .iter()
            .filter(|i| i.kind == kind && i.scope.is_none())
            .filter_map(|i| i.percent)
            .collect();
        (!v.is_empty()).then(|| v.iter().sum())
    };
    row.weekly_percent = sum(LimitKind::WeeklyAll);
    row.session_percent = sum(LimitKind::Session);
    row.impact_basis = Basis::weakest(
        impact
            .iter()
            .filter(|i| i.scope.is_none() && i.percent.is_some())
            .map(|i| i.basis),
    );
    Ok(Some(row))
}

/// Buckets per model, merged into wider buckets so at most 1,500 bucket times remain.
fn timeline(c: &Connection, root: SessionId) -> Result<(Vec<TimelineBucket>, i64)> {
    let rows: Vec<(i64, String, i64)> = c
        .prepare_cached(
            "SELECT u.bucket_ts, m.name, SUM(u.cost_micro_usd) FROM usage_5m u
             JOIN models m ON m.id = u.model_id
             WHERE u.account_id = (SELECT account_id FROM sessions WHERE id = ?1)
           AND u.session_id IN (SELECT id FROM sessions WHERE id = ?1 OR root_id = ?1)
             GROUP BY +u.bucket_ts, 2 ORDER BY 1",
        )?
        .query_map([root.0], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let (Some(first), Some(last)) = (rows.first().map(|r| r.0), rows.last().map(|r| r.0)) else {
        return Ok((Vec::new(), BUCKET_MS));
    };
    let span = last - first + BUCKET_MS;
    let max = i64::from(super::MAX_SERIES_POINTS);
    // Whole 5-minute buckets, wide enough that at most `max` remain.
    let buckets = (span + BUCKET_MS - 1) / BUCKET_MS;
    let width = ((buckets + max - 1) / max).max(1) * BUCKET_MS;
    let mut merged: BTreeMap<(i64, String), i64> = BTreeMap::new();
    for (at, model, cost) in rows {
        let b = first + (at - first) / width * width;
        *merged.entry((b, model)).or_default() += cost;
    }
    Ok((
        merged
            .into_iter()
            .map(|((at, model), cost)| TimelineBucket {
                at: Timestamp::from_millis(at),
                model,
                cost: MicroUsd(cost),
            })
            .collect(),
        width,
    ))
}

fn subagents(c: &Connection, root: SessionId) -> Result<Vec<SubagentRow>> {
    let mut rows: Vec<SubagentRow> = c
        .prepare_cached(
            "SELECT s.id, s.uuid, s.title, COALESCE(SUM(u.cost_micro_usd), 0),
                    COALESCE(SUM(u.input), 0), COALESCE(SUM(u.output), 0),
                    COALESCE(SUM(u.cache_read), 0), COALESCE(SUM(u.cache_w5m + u.cache_w1h), 0)
             FROM sessions s
             LEFT JOIN usage_5m u ON u.account_id = s.account_id AND u.session_id = s.id
             WHERE s.root_id = ?1 GROUP BY s.id ORDER BY 4 DESC, s.id",
        )?
        .query_map([root.0], |r| {
            Ok(SubagentRow {
                id: SessionId(r.get(0)?),
                uuid: r.get(1)?,
                title: r.get(2)?,
                cost: MicroUsd(r.get(3)?),
                tokens: TokenSums {
                    input: r.get(4)?,
                    output: r.get(5)?,
                    cache_read: r.get(6)?,
                    cache_write: r.get(7)?,
                },
                models: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut stmt = c.prepare_cached(
        "SELECT m.name FROM usage_5m u JOIN models m ON m.id = u.model_id
         WHERE u.account_id = (SELECT account_id FROM sessions WHERE id = ?1) AND u.session_id = ?1
         GROUP BY m.name ORDER BY SUM(u.cost_micro_usd) DESC",
    )?;
    for row in &mut rows {
        row.models = stmt
            .query_map([row.id.0], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
    }
    Ok(rows)
}

/// Hit ratio and the saving of cache reads against full input price, per model.
fn cache_stats(c: &Connection, root: SessionId) -> Result<CacheStats> {
    let rows: Vec<(String, i64, i64, i64, i64)> = c
        .prepare_cached(
            "SELECT m.name, SUM(u.input), SUM(u.cache_read), SUM(u.cache_w5m), SUM(u.cache_w1h)
             FROM usage_5m u JOIN models m ON m.id = u.model_id
             WHERE u.account_id = (SELECT account_id FROM sessions WHERE id = ?1)
           AND u.session_id IN (SELECT id FROM sessions WHERE id = ?1 OR root_id = ?1)
             GROUP BY m.name",
        )?
        .query_map([root.0], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let pricing = Pricing::claude();
    let (mut input, mut read, mut w5, mut w1) = (0i64, 0i64, 0i64, 0i64);
    let mut savings = 0.0;
    for (model, i, r, a, b) in rows {
        input += i;
        read += r;
        w5 += a;
        w1 += b;
        let (price, _) = pricing.lookup(&model);
        #[allow(clippy::cast_precision_loss)]
        {
            savings += r as f64 * price.input * (1.0 - price.cache_read_mult);
        }
    }
    let all = input + read + w5 + w1;
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    Ok(CacheStats {
        hit_ratio: (all > 0).then(|| read as f64 / all as f64),
        write_5m: w5,
        write_1h: w1,
        savings: MicroUsd(savings.round() as i64),
    })
}

/// Root ids with their cost in `[from, to)`, for the overview and project details.
pub(crate) fn root_costs(
    c: &Connection,
    account: crate::domain::AccountId,
    from: i64,
    to: i64,
) -> Result<HashMap<i64, i64>> {
    let rows = c
        .prepare_cached(&format!(
            "SELECT COALESCE(s.root_id, s.id), SUM(u.cost_micro_usd)
             FROM usage_5m u JOIN sessions s ON s.id = u.session_id
             WHERE u.account_id = ?1 AND {b} >= ?2 AND {b} < ?3 GROUP BY 1",
            b = super::bucket_col(from, to)
        ))?
        .query_map(params![account.0, from, to], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}
