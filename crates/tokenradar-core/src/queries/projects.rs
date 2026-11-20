//! Projects: the list with a weekly cost sparkline, the same rows as a folder tree, and one
//! project's weekly trend, model mix and most expensive sessions. A subagent has no project of
//! its own: its usage counts for its main session's project.
//!
//! Temporary folders (Step 16b) are left out when the filter asks: the caller passes the
//! config's patterns as `LIKE` patterns ([`super::temp_like_patterns`]), matched against
//! `path_norm`.

use std::collections::HashMap;

use rusqlite::{OptionalExtension, params, params_from_iter, types::Value};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::{
    Cursor, QueryContext, Result, SessionFilter, SessionRow, SessionSort, TimeRange, WEEK_MS,
    page_size,
};
use crate::domain::{MicroUsd, ProjectId, Timestamp};
use crate::store::Store;

/// Weeks in a project sparkline and in the detail's trend.
pub const TREND_WEEKS: usize = 8;
const TREND_WEEKS_I64: i64 = 8;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
pub struct ProjectFilter {
    /// Cost and impact count this range; projects without usage in it are left out.
    pub range: Option<TimeRange>,
    /// Matches the name and parts of the path.
    pub search: Option<String>,
    /// Leave out projects in temporary folders (the `temp` patterns of the call).
    #[serde(default)]
    pub hide_temp: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSort {
    #[default]
    LastActive,
    Cost,
    Impact,
    Sessions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ProjectRow {
    pub id: ProjectId,
    pub name: String,
    /// The path as recorded (schema v5), otherwise the normalized one; `unresolved:<dir>` when
    /// the cwd is unknown.
    pub path: String,
    /// Main sessions with usage in the range.
    pub sessions: u32,
    pub last_active: Option<Timestamp>,
    pub cost: MicroUsd,
    /// `≈` points of weekly windows in the range, summed over the project's sessions.
    pub weekly_percent: Option<f64>,
    /// Cost per 7 days over the last [`TREND_WEEKS`] weeks, oldest first.
    pub trend: Vec<MicroUsd>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ProjectPage {
    pub items: Vec<ProjectRow>,
    pub next: Option<String>,
    /// On the first page of a list that hides temporary folders: how many projects with usage
    /// in the range it hides.
    pub hidden: Option<u32>,
}

/// A folder or project of [`ProjectTree`]. A folder with one child and no project of its own
/// is merged with that child (`D:\Projects\Tools`), so every level branches.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ProjectNode {
    /// 0 at the top.
    pub depth: u32,
    /// What this node adds to its parent's path (`Tools\aktif`, or the project's name).
    pub label: String,
    /// The full path up to here.
    pub path: String,
    /// The project at exactly this path, with its own totals.
    pub project: Option<ProjectRow>,
    /// Totals of this node and everything under it.
    pub projects: u32,
    pub sessions: u32,
    pub last_active: Option<Timestamp>,
    pub cost: MicroUsd,
    pub weekly_percent: Option<f64>,
    pub trend: Vec<MicroUsd>,
    /// Nodes directly under this one; they follow it in [`ProjectTree::nodes`].
    pub children: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ProjectTree {
    /// Depth first, each level in `sort` order, descending.
    pub nodes: Vec<ProjectNode>,
    /// As in [`ProjectPage::hidden`].
    pub hidden: Option<u32>,
}

/// One weekly window of a project.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ProjectWeek {
    pub window_id: i64,
    pub starts_at: Timestamp,
    pub resets_at: Timestamp,
    pub estimated: bool,
    pub percent: Option<f64>,
    pub cost: MicroUsd,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ProjectDetail {
    /// All-time totals.
    pub project: ProjectRow,
    /// The last [`TREND_WEEKS`] weekly windows, oldest first.
    pub weeks: Vec<ProjectWeek>,
    pub models: Vec<super::ModelSlice>,
    pub top_sessions: Vec<SessionRow>,
}

fn sort_expr(sort: ProjectSort) -> &'static str {
    match sort {
        ProjectSort::LastActive => "COALESCE(last, 0)",
        ProjectSort::Cost => "cost",
        ProjectSort::Impact => "COALESCE(weekly, -1.0)",
        ProjectSort::Sessions => "sessions",
    }
}

/// Project rows over `[?2, ?3)`; `b` is [`super::bucket_col`] of that range. `only`: the
/// parameter holding the one project to read, which then reads just its sessions' keys.
fn rows_sql(b: &str, only: Option<usize>) -> String {
    let only = only.map_or_else(String::new, |i| {
        format!("AND u.session_id IN ({})", super::project_sessions(i))
    });
    // Per session in key order first: joining `sessions` twice for every bucket row costs
    // more than the aggregation.
    format!(
        "
    WITH ps AS MATERIALIZED (
      SELECT u.session_id AS sid, SUM(u.cost_micro_usd) AS cost, MAX(u.bucket_ts) AS last
      FROM usage_5m u
      WHERE u.account_id = ?1 AND {b} >= ?2 AND {b} < ?3 {only}
      GROUP BY u.session_id),
    u AS MATERIALIZED (
      SELECT t.project_id AS pid, t.id AS root, SUM(ps.cost) AS cost, MAX(ps.last) AS last
      FROM ps JOIN sessions s ON s.id = ps.sid
      JOIN sessions t ON t.id = COALESCE(s.root_id, s.id)
      WHERE t.project_id IS NOT NULL
      GROUP BY pid, root),
    i AS (
      SELECT s.project_id AS pid, SUM(a.est_percent) AS weekly
      FROM attributions a JOIN limit_windows w ON w.id = a.window_id
      JOIN sessions s ON s.id = a.root_session_id
      WHERE w.account_id = ?1 AND w.limit_kind = 'weekly_all' AND w.scope IS NULL
        AND w.resets_at > ?2 AND w.starts_at < ?3
      GROUP BY s.project_id)
    SELECT p.id AS id, p.display_name, COALESCE(p.display_path, p.path_norm),
           COUNT(DISTINCT u.root) AS sessions,
           MAX(u.last) AS last, SUM(u.cost) AS cost, i.weekly AS weekly
    FROM u JOIN projects p ON p.id = u.pid LEFT JOIN i ON i.pid = p.id"
    )
}

/// The `WHERE` of the project rows for `filter`, its parameters appended to `args`. `hidden`:
/// the projects the temp patterns leave out instead of those they keep.
fn conditions(
    filter: &ProjectFilter,
    temp: &[String],
    hidden: bool,
    args: &mut Vec<Value>,
) -> String {
    let mut conds = Vec::new();
    if let Some(q) = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
    {
        args.push(super::like_pattern(q).into());
        let i = args.len();
        conds.push(format!(
            "(p.display_name LIKE ?{i} ESCAPE '\\' OR p.path_norm LIKE ?{i} ESCAPE '\\')"
        ));
    }
    if filter.hide_temp {
        let first = args.len() + 1;
        args.extend(temp.iter().map(|t| Value::from(t.clone())));
        match super::temp::like_any("p.path_norm", first, temp.len()) {
            Some(any) if hidden => conds.push(any),
            Some(any) => conds.push(format!("NOT {any}")),
            // No pattern hides nothing.
            None if hidden => conds.push("0".to_owned()),
            None => {}
        }
    }
    if conds.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conds.join(" AND "))
    }
}

fn base_args(ctx: &QueryContext<'_>, from: i64, to: i64) -> Vec<Value> {
    vec![ctx.account.0.into(), from.into(), to.into()]
}

/// How many projects with usage in the range the temp patterns hide; `None` when the filter
/// hides none.
fn count_hidden(
    store: &Store,
    ctx: &QueryContext<'_>,
    filter: &ProjectFilter,
    temp: &[String],
) -> Result<Option<u32>> {
    if !filter.hide_temp {
        return Ok(None);
    }
    let (from, to) = super::bounds(filter.range);
    let mut args = base_args(ctx, from, to);
    // Always a `WHERE`: with `hidden`, the temp condition is never left out.
    let cond = conditions(filter, temp, true, &mut args);
    // Only "has usage in the range" matters here, so each matching project is one keyed
    // probe per session instead of the list's whole aggregation.
    let sql = format!(
        "SELECT COUNT(*) FROM projects p {cond}
           AND EXISTS (SELECT 1 FROM usage_5m u
             WHERE u.account_id = ?1 AND {b} >= ?2 AND {b} < ?3
               AND u.session_id IN (
                 SELECT id FROM sessions WHERE project_id = p.id
                 UNION ALL SELECT id FROM sessions
                 WHERE root_id IN (SELECT id FROM sessions WHERE project_id = p.id)))",
        b = super::TREE_BUCKET,
    );
    let n: i64 = store.read(|c| {
        Ok(c.prepare_cached(&sql)?
            .query_row(params_from_iter(args), |r| r.get(0))?)
    })?;
    Ok(Some(u32::try_from(n).unwrap_or(u32::MAX)))
}

/// Projects with usage in the range, one page after `cursor`, in `sort` order, descending.
/// `temp`: `LIKE` patterns of temporary folders, used when `filter.hide_temp`.
pub fn list_projects(
    store: &Store,
    ctx: &QueryContext<'_>,
    filter: &ProjectFilter,
    sort: ProjectSort,
    cursor: Option<&str>,
    limit: Option<u32>,
    temp: &[String],
) -> Result<ProjectPage> {
    let n = page_size(limit);
    let after = Cursor::decode(cursor).unwrap_or(Cursor::START);
    let (from, to) = super::bounds(filter.range);
    let mut args = base_args(ctx, from, to);
    let cond = conditions(filter, temp, false, &mut args);
    args.push(after.key.into());
    let k = args.len();
    args.push(after.id.into());
    let id = args.len();
    let sql = format!(
        "SELECT * FROM (
           SELECT *, {key} AS k FROM ({rows} {cond} GROUP BY p.id))
         WHERE k < ?{k} OR (k = ?{k} AND id < ?{id})
         ORDER BY k DESC, id DESC LIMIT {limit}",
        key = sort_expr(sort),
        limit = n + 1,
        rows = rows_sql(super::bucket_col(from, to), None),
    );
    let (mut items, keys) = store.read(|c| {
        let mut items = Vec::new();
        let mut keys = Vec::new();
        let mut stmt = c.prepare_cached(&sql)?;
        let mut rows = stmt.query(params_from_iter(args))?;
        while let Some(r) = rows.next()? {
            items.push(row_of(r)?);
            keys.push(r.get::<_, f64>(7)?);
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
    fill_trends(store, ctx, &mut items)?;
    let hidden = if cursor.is_none() {
        count_hidden(store, ctx, filter, temp)?
    } else {
        None
    };
    Ok(ProjectPage {
        items,
        next,
        hidden,
    })
}

/// Every project of the list as a folder tree; see [`ProjectNode`].
pub fn project_tree(
    store: &Store,
    ctx: &QueryContext<'_>,
    filter: &ProjectFilter,
    sort: ProjectSort,
    temp: &[String],
) -> Result<ProjectTree> {
    let (from, to) = super::bounds(filter.range);
    let mut args = base_args(ctx, from, to);
    let cond = conditions(filter, temp, false, &mut args);
    let sql = format!(
        "{rows} {cond} GROUP BY p.id",
        rows = rows_sql(super::bucket_col(from, to), None),
    );
    let mut items = store.read(|c| {
        Ok(c.prepare_cached(&sql)?
            .query_map(params_from_iter(args), row_of)?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    })?;
    fill_trends(store, ctx, &mut items)?;
    Ok(ProjectTree {
        nodes: super::tree::build(items, sort),
        hidden: count_hidden(store, ctx, filter, temp)?,
    })
}

fn row_of(r: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRow> {
    Ok(ProjectRow {
        id: ProjectId(r.get(0)?),
        name: r.get(1)?,
        path: r.get(2)?,
        sessions: u32::try_from(r.get::<_, i64>(3)?).unwrap_or(0),
        last_active: r.get(4)?,
        cost: MicroUsd(r.get(5)?),
        weekly_percent: r.get(6)?,
        trend: Vec::new(),
    })
}

/// Cost per 7 days ending at `now`, [`TREND_WEEKS`] of them, for the listed projects.
fn fill_trends(store: &Store, ctx: &QueryContext<'_>, items: &mut [ProjectRow]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let now = ctx.now.as_millis();
    let start = now - TREND_WEEKS_I64 * WEEK_MS;
    let ids: Vec<String> = items.iter().map(|p| p.id.0.to_string()).collect();
    // One project (the detail) reads its sessions' keys; a page reads the weeks by time.
    let (only, b) = match items {
        [one] => (
            format!(
                "AND u.session_id IN ({})",
                super::project_sessions(4).replace("?4", &one.id.0.to_string())
            ),
            super::TREE_BUCKET,
        ),
        _ => (String::new(), super::bucket_col(start, now)),
    };
    let sql = format!(
        "WITH ps AS MATERIALIZED (
           SELECT u.session_id AS sid, (u.bucket_ts - ?2) / {WEEK_MS} AS wk,
                  SUM(u.cost_micro_usd) AS cost
           FROM usage_5m u
           WHERE u.account_id = ?1 AND {b} >= ?2 AND {b} < ?3 {only}
           GROUP BY 1, 2)
         SELECT t.project_id, ps.wk, SUM(ps.cost)
         FROM ps JOIN sessions s ON s.id = ps.sid
         JOIN sessions t ON t.id = COALESCE(s.root_id, s.id)
         WHERE t.project_id IN ({ids})
         GROUP BY 1, 2",
        ids = ids.join(","),
    );
    let rows: Vec<(i64, i64, i64)> = store.read(|c| {
        Ok(c.prepare(&sql)?
            .query_map(params![ctx.account.0, start, now], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?
            .collect::<rusqlite::Result<_>>()?)
    })?;
    let mut by: HashMap<i64, Vec<MicroUsd>> = HashMap::new();
    for (pid, week, cost) in rows {
        let v = by
            .entry(pid)
            .or_insert_with(|| vec![MicroUsd(0); TREND_WEEKS]);
        if let Some(slot) = usize::try_from(week).ok().and_then(|w| v.get_mut(w)) {
            slot.0 += cost;
        }
    }
    for p in items {
        p.trend = by
            .remove(&p.id.0)
            .unwrap_or_else(|| vec![MicroUsd(0); TREND_WEEKS]);
    }
    Ok(())
}

/// One project, all time; `None` for an unknown id.
pub fn get_project(
    store: &Store,
    ctx: &QueryContext<'_>,
    id: ProjectId,
) -> Result<Option<ProjectDetail>> {
    let sql = format!(
        "{} WHERE p.id = ?4 GROUP BY p.id",
        rows_sql(super::TREE_BUCKET, Some(4))
    );
    let row = store.read(|c| {
        Ok(c.prepare_cached(&sql)?
            .query_row(params![ctx.account.0, 0i64, i64::MAX, id.0], row_of)
            .optional()?)
    })?;
    let Some(mut project) = row else {
        return Ok(None);
    };
    fill_trends(store, ctx, std::slice::from_mut(&mut project))?;
    let weeks = store.read(|c| {
        let mut rows: Vec<ProjectWeek> = c
            .prepare_cached(
                "SELECT w.id, w.starts_at, w.resets_at, w.estimated,
                        SUM(a.est_percent), COALESCE(SUM(a.cost_micro_usd), 0)
                 FROM limit_windows w
                 LEFT JOIN attributions a ON a.window_id = w.id
                   AND a.root_session_id IN (SELECT id FROM sessions WHERE project_id = ?2)
                 WHERE w.account_id = ?1 AND w.limit_kind = 'weekly_all' AND w.scope IS NULL
                   AND w.starts_at <= ?3
                 GROUP BY w.id ORDER BY w.resets_at DESC LIMIT ?4",
            )?
            .query_map(
                params![ctx.account.0, id.0, ctx.now, TREND_WEEKS_I64],
                |r| {
                    Ok(ProjectWeek {
                        window_id: r.get(0)?,
                        starts_at: r.get(1)?,
                        resets_at: r.get(2)?,
                        estimated: r.get::<_, i64>(3)? != 0,
                        percent: r.get(4)?,
                        cost: MicroUsd(r.get(5)?),
                    })
                },
            )?
            .collect::<rusqlite::Result<_>>()?;
        rows.reverse();
        Ok(rows)
    })?;
    let models = store.read(|c| {
        super::overview::model_slices(
            c,
            &format!(
                "WHERE u.account_id = ?1 AND u.session_id IN ({})",
                super::project_sessions(2)
            ),
            params![ctx.account.0, id.0],
        )
    })?;
    let top = super::list_sessions(
        store,
        ctx,
        &SessionFilter {
            project: Some(id),
            ..SessionFilter::default()
        },
        SessionSort::Cost,
        None,
        Some(5),
        &[],
    )?;
    Ok(Some(ProjectDetail {
        project,
        weeks,
        models,
        top_sessions: top.items,
    }))
}
