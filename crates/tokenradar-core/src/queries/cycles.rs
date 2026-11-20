//! The windows page (Step 17a, ADR-070): weekly cycles (one row per weekly window with its
//! days, top projects and scoped limits) and the 5-hour day strip.
//!
//! - **Days are 24-hour slices from the window's start**, like the time strip of the panel and
//!   the attribution slices (ADR-064), not local calendar days: a weekly window that resets on
//!   Wednesday 11:00 has seven days that each start at 11:00.
//! - **Day values are `≈`:** `k · Σ α · cost` of the slice with the weekly calibration, local
//!   usage only. Without a usable calibration a day has only its cost. The measured peak stays
//!   on the window row.

use std::collections::{BTreeSet, HashMap};

use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use specta::Type;

use super::zone::local_day_start;
use super::{
    Basis, ProjectRef, QueryContext, Result, SessionName, TimeRange, session_name,
    windows::{WindowRow, window_row},
};
use crate::attribution::Method;
use crate::domain::{LimitKind, LimitRef, MicroUsd, ProjectId, Scope, Timestamp};
use crate::limits::history::window_readings;
use crate::store::{Store, WINDOW_MATCH_TOLERANCE_MS};

const DAY_MS: i64 = 86_400_000;
const MINUTE_MS: i64 = 60_000;

/// One 24-hour slice of a weekly window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct CycleDay {
    pub starts_at: Timestamp,
    /// `≈` points of local usage; `None` without a usable calibration.
    pub percent: Option<f64>,
    pub cost: MicroUsd,
    /// The slice starts after now.
    pub future: bool,
}

/// A project's part of a weekly window (stored attribution).
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct CycleProject {
    /// `None` for sessions without a project.
    pub project: Option<ProjectRef>,
    pub percent: Option<f64>,
    pub cost: MicroUsd,
}

/// A scoped weekly limit (e.g. Fable) that reset with the row's window.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ScopedPeak {
    pub window_id: i64,
    pub scope: Scope,
    pub peak_percent: Option<f64>,
    pub hit_limit: Option<bool>,
    pub estimated: bool,
}

/// One `seven_day_breakdown` surface of the window, 0–100.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct SurfaceShare {
    pub key: String,
    pub percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct CycleRow {
    pub window: WindowRow,
    /// Seven slices; fewer only when the window is shorter than a week.
    pub days: Vec<CycleDay>,
    /// First measured reading at 100 %; `None` when it never got there or is unknown.
    pub hit_at: Option<Timestamp>,
    /// Index into `days` of `hit_at`.
    pub hit_day: Option<u8>,
    /// At most three, largest share (then cost) first.
    pub projects: Vec<CycleProject>,
    pub breakdown: Vec<SurfaceShare>,
    pub scoped: Vec<ScopedPeak>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct WeekCycles {
    /// Newest first.
    pub items: Vec<CycleRow>,
    /// Scopes of the scoped limits in `items`, one column each.
    pub scopes: Vec<Scope>,
    /// How the day values were computed: `estimated` with the calibration's confidence, or
    /// `cost_only`.
    pub day_basis: Basis,
}

/// Weekly windows overlapping `range`, newest first, with their days.
pub fn list_weeks(store: &Store, ctx: &QueryContext<'_>, range: TimeRange) -> Result<WeekCycles> {
    let ids: Vec<i64> = store.read(|c| {
        Ok(c.prepare_cached(
            "SELECT id FROM limit_windows
             WHERE account_id = ?1 AND limit_kind = 'weekly_all' AND scope IS NULL
               AND resets_at > ?2 AND starts_at < ?3
             ORDER BY resets_at DESC, id DESC",
        )?
        .query_map(
            params![ctx.account.0, range.from.as_millis(), range.to.as_millis()],
            |r| r.get(0),
        )?
        .collect::<rusqlite::Result<_>>()?)
    })?;
    let rows: Vec<WindowRow> = store.read(|c| {
        let mut out = Vec::with_capacity(ids.len());
        for id in &ids {
            if let Some(r) = window_row(c, ctx, *id)? {
                out.push(r);
            }
        }
        Ok(out)
    })?;
    let cal = ctx
        .calibrations
        .iter()
        .find(|c| c.kind == LimitKind::WeeklyAll && c.scope.is_none())
        .filter(|c| c.level() != crate::limits::Confidence::Low);
    let day_basis = match cal {
        Some(c) => Basis::new(Method::Estimated, Some(c.confidence), false),
        None => Basis::new(Method::CostOnly, None, false),
    };
    let weight = cal
        .map(|c| super::series::percent_weight(store, c))
        .transpose()?;
    let costs = slice_costs(store, ctx, &rows, weight.as_deref())?;
    let mut scope_keys = BTreeSet::new();
    let mut items = Vec::with_capacity(rows.len());
    for (window, slices) in rows.into_iter().zip(costs) {
        let days: Vec<CycleDay> = slices
            .iter()
            .enumerate()
            .map(|(i, (cost, pct))| {
                let starts_at = window
                    .starts_at
                    .saturating_add_millis(i64::try_from(i).unwrap_or(0) * DAY_MS);
                CycleDay {
                    starts_at,
                    percent: weight.is_some().then_some(*pct),
                    cost: MicroUsd(*cost),
                    future: starts_at > ctx.now,
                }
            })
            .collect();
        let hit_at = if window.hit_limit == Some(true) {
            first_full(store, ctx, &window)?
        } else {
            None
        };
        let hit_day = hit_at.and_then(|t| {
            u8::try_from((t.as_millis() - window.starts_at.as_millis()).div_euclid(DAY_MS)).ok()
        });
        let (projects, breakdown, scoped) = store.read(|c| {
            let projects = top_projects(c, window.id)?;
            let breakdown = super::overview::breakdown_rows(c, ctx, window.starts_at.as_millis())?
                .into_iter()
                .map(|(key, percent)| SurfaceShare { key, percent })
                .collect();
            let scoped = scoped_peaks(c, ctx, &window)?;
            Ok((projects, breakdown, scoped))
        })?;
        for s in &scoped {
            scope_keys.insert(s.scope.to_string());
        }
        items.push(CycleRow {
            window,
            days,
            hit_at,
            hit_day,
            projects,
            breakdown,
            scoped,
        });
    }
    Ok(WeekCycles {
        items,
        scopes: scope_keys
            .into_iter()
            .filter_map(|s| s.parse().ok())
            .collect(),
        day_basis,
    })
}

/// `(cost, ≈ points)` per 24-hour slice of each window, in one read over all of them.
fn slice_costs(
    store: &Store,
    ctx: &QueryContext<'_>,
    rows: &[WindowRow],
    weight: Option<&str>,
) -> Result<Vec<Vec<(i64, f64)>>> {
    let mut out: Vec<Vec<(i64, f64)>> = rows
        .iter()
        .map(|w| {
            let len = w.resets_at.as_millis() - w.starts_at.as_millis();
            vec![
                (0, 0.0);
                usize::try_from((len + DAY_MS - 1) / DAY_MS)
                    .unwrap_or(0)
                    .max(1)
            ]
        })
        .collect();
    let (Some(from), Some(to)) = (
        rows.iter().map(|w| w.starts_at.as_millis()).min(),
        rows.iter().map(|w| w.resets_at.as_millis()).max(),
    ) else {
        return Ok(out);
    };
    let to = to.min(ctx.now.as_millis() + 1);
    let col = super::bucket_col(from, to);
    let value = weight.map_or_else(|| "0".to_owned(), |w| format!("u.cost_micro_usd * {w}"));
    let sql = format!(
        "SELECT u.bucket_ts, u.cost_micro_usd, {value} FROM usage_5m u
         WHERE u.account_id = ?1 AND {col} >= ?2 AND {col} < ?3"
    );
    // Windows sorted by start for a binary search per bucket (weekly windows never overlap).
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by_key(|&i| rows[i].starts_at.as_millis());
    let starts: Vec<i64> = order
        .iter()
        .map(|&i| rows[i].starts_at.as_millis())
        .collect();
    store.read(|c| {
        let mut stmt = c.prepare_cached(&sql)?;
        let mut q = stmt.query(params![ctx.account.0, from, to])?;
        while let Some(r) = q.next()? {
            let t: i64 = r.get(0)?;
            let Some(pos) = starts.partition_point(|s| *s <= t).checked_sub(1) else {
                continue;
            };
            let i = order[pos];
            let w = &rows[i];
            if t >= w.resets_at.as_millis() {
                continue;
            }
            let slice = usize::try_from((t - w.starts_at.as_millis()) / DAY_MS).unwrap_or(0);
            if let Some(cell) = out[i].get_mut(slice) {
                cell.0 += r.get::<_, i64>(1)?;
                cell.1 += r.get::<_, f64>(2)?;
            }
        }
        Ok(())
    })?;
    Ok(out)
}

/// When the window's fused readings first reached 100 %.
fn first_full(store: &Store, ctx: &QueryContext<'_>, w: &WindowRow) -> Result<Option<Timestamp>> {
    let snaps = store.snapshots_between(
        ctx.account,
        w.starts_at,
        w.resets_at.saturating_add_millis(WINDOW_MATCH_TOLERANCE_MS),
    )?;
    let limit = LimitRef::new(w.kind.clone(), w.scope.clone());
    let len = w.resets_at.as_millis() - w.starts_at.as_millis();
    Ok(
        window_readings(&limit, w.resets_at, len, &snaps, ctx.now.min(w.resets_at))
            .into_iter()
            .find(|r| r.percent >= 100.0)
            .map(|r| r.at),
    )
}

fn top_projects(c: &rusqlite::Connection, window: i64) -> Result<Vec<CycleProject>> {
    Ok(c.prepare_cached(
        "SELECT p.id, p.display_name, SUM(a.est_percent), SUM(a.cost_micro_usd)
         FROM attributions a JOIN sessions s ON s.id = a.root_session_id
         LEFT JOIN projects p ON p.id = s.project_id
         WHERE a.window_id = ?1
         GROUP BY p.id
         ORDER BY SUM(a.est_percent) IS NULL, SUM(a.est_percent) DESC, SUM(a.cost_micro_usd) DESC
         LIMIT 3",
    )?
    .query_map([window], |r| {
        let id: Option<i64> = r.get(0)?;
        let name: Option<String> = r.get(1)?;
        Ok(CycleProject {
            project: id.zip(name).map(|(id, name)| ProjectRef {
                id: ProjectId(id),
                name,
            }),
            percent: r.get(2)?,
            cost: MicroUsd(r.get(3)?),
        })
    })?
    .collect::<rusqlite::Result<_>>()?)
}

/// `id, scope, peak_percent, hit_limit, estimated` of a scoped window.
type ScopedRaw = (i64, Option<String>, Option<f64>, bool, bool);

fn scoped_peaks(
    c: &rusqlite::Connection,
    ctx: &QueryContext<'_>,
    w: &WindowRow,
) -> Result<Vec<ScopedPeak>> {
    let rows: Vec<ScopedRaw> = c
        .prepare_cached(
            "SELECT id, scope, peak_percent, hit_limit, estimated FROM limit_windows
             WHERE account_id = ?1 AND limit_kind = 'weekly_scoped'
               AND resets_at BETWEEN ?2 - ?3 AND ?2 + ?3
             ORDER BY scope",
        )?
        .query_map(
            params![
                ctx.account.0,
                w.resets_at.as_millis(),
                WINDOW_MATCH_TOLERANCE_MS
            ],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get::<_, i64>(3)? != 0,
                    r.get::<_, i64>(4)? != 0,
                ))
            },
        )?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows
        .into_iter()
        .filter_map(|(id, scope, peak, hit, est)| {
            Some(ScopedPeak {
                window_id: id,
                scope: scope?.parse().ok()?,
                peak_percent: peak,
                hit_limit: (!est).then_some(hit),
                estimated: est,
            })
        })
        .collect())
}

/// A 5-hour window in the strip.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct StripWindow {
    pub window: WindowRow,
    /// The three largest session shares, for the hover.
    pub top: Vec<StripSession>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct StripSession {
    pub name: SessionName,
    pub percent: Option<f64>,
}

/// The part of a window that falls on one local day, in minutes from local midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct StripSegment {
    /// Index into [`WindowStrip::windows`].
    pub window: u32,
    pub from_minute: u16,
    /// Exclusive; at most the day's length (1500 on a 25-hour day).
    pub to_minute: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct StripDay {
    /// Local midnight.
    pub day: Timestamp,
    /// The day's length in minutes (1440, or 1380/1500 when the clocks change).
    pub minutes: u16,
    pub segments: Vec<StripSegment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct WindowStrip {
    /// The account has 5-hour windows at all (ADR-041: verified Ultra has none).
    pub any: bool,
    pub windows: Vec<StripWindow>,
    /// Every local day from the first window in range to today, newest first.
    pub days: Vec<StripDay>,
}

/// 5-hour windows overlapping `range`, cut into local days.
pub fn window_strip(
    store: &Store,
    ctx: &QueryContext<'_>,
    range: TimeRange,
) -> Result<WindowStrip> {
    let any = store.read(|c| {
        Ok(c.prepare_cached(
            "SELECT 1 FROM limit_windows WHERE account_id = ?1 AND limit_kind = 'session' LIMIT 1",
        )?
        .query_row([ctx.account.0], |_| Ok(()))
        .optional()?
        .is_some())
    })?;
    let mut strip = WindowStrip {
        any,
        windows: Vec::new(),
        days: Vec::new(),
    };
    if !any {
        return Ok(strip);
    }
    let to = range.to.min(ctx.now);
    strip.windows = store.read(|c| {
        let rows = super::windows::session_rows(c, ctx, range.from, to)?;
        // The three largest shares of every window in one statement.
        let mut top: HashMap<i64, Vec<StripSession>> = HashMap::new();
        let mut stmt = c.prepare_cached(
            "SELECT window_id, uuid, title, project, est_percent FROM (
               SELECT a.window_id, s.uuid, s.title, p.display_name AS project, a.est_percent,
                      ROW_NUMBER() OVER (
                        PARTITION BY a.window_id
                        ORDER BY a.est_percent IS NULL, a.est_percent DESC, a.cost_micro_usd DESC
                      ) AS n
               FROM limit_windows w
               JOIN attributions a ON a.window_id = w.id
               JOIN sessions s ON s.id = a.root_session_id
               LEFT JOIN projects p ON p.id = s.project_id
               WHERE w.account_id = ?1 AND w.limit_kind = 'session'
                 AND w.resets_at > ?2 AND w.starts_at < ?3)
             WHERE n <= 3 ORDER BY window_id, n",
        )?;
        let found = stmt.query_map(
            params![ctx.account.0, range.from.as_millis(), to.as_millis()],
            |r| {
                let uuid: String = r.get(1)?;
                let title: Option<String> = r.get(2)?;
                let project: Option<String> = r.get(3)?;
                Ok((
                    r.get::<_, i64>(0)?,
                    StripSession {
                        name: session_name(title.as_deref(), project.as_deref(), &uuid),
                        percent: r.get(4)?,
                    },
                ))
            },
        )?;
        for row in found {
            let (id, session) = row?;
            top.entry(id).or_default().push(session);
        }
        Ok(rows
            .into_iter()
            .map(|window| StripWindow {
                top: top.remove(&window.id).unwrap_or_default(),
                window,
            })
            .collect())
    })?;
    let Some(first) = strip.windows.first().map(|w| w.window.starts_at) else {
        return Ok(strip);
    };
    strip.days = cut_days(ctx, &strip.windows, first.max(range.from), to);
    Ok(strip)
}

/// Every local day from `first` to `to`, newest first, with the windows' parts on each.
fn cut_days(
    ctx: &QueryContext<'_>,
    windows: &[StripWindow],
    first: Timestamp,
    to: Timestamp,
) -> Vec<StripDay> {
    // Oldest first while cutting.
    let mut day = local_day_start(ctx.zone, first);
    let last = local_day_start(ctx.zone, to);
    let mut days: Vec<StripDay> = Vec::new();
    let mut index: HashMap<i64, usize> = HashMap::new();
    while day <= last {
        let next = local_day_start(ctx.zone, day.saturating_add_millis(DAY_MS + 3 * 3_600_000));
        let minutes =
            u16::try_from((next.as_millis() - day.as_millis()) / MINUTE_MS).unwrap_or(1440);
        index.insert(day.as_millis(), days.len());
        days.push(StripDay {
            day,
            minutes,
            segments: Vec::new(),
        });
        day = next;
    }
    for (i, w) in windows.iter().enumerate() {
        let (a, b) = (
            w.window.starts_at.as_millis(),
            w.window.resets_at.as_millis(),
        );
        let mut d = local_day_start(ctx.zone, Timestamp::from_millis(a.max(first.as_millis())));
        while d.as_millis() < b {
            let Some(&di) = index.get(&d.as_millis()) else {
                break;
            };
            let day = &mut days[di];
            let end = d.as_millis() + i64::from(day.minutes) * MINUTE_MS;
            let from = a.max(d.as_millis());
            let until = b.min(end);
            if until > from {
                day.segments.push(StripSegment {
                    window: u32::try_from(i).unwrap_or(u32::MAX),
                    from_minute: u16::try_from((from - d.as_millis()) / MINUTE_MS).unwrap_or(0),
                    to_minute: u16::try_from((until - d.as_millis() + MINUTE_MS - 1) / MINUTE_MS)
                        .unwrap_or(day.minutes),
                });
            }
            d = Timestamp::from_millis(end);
        }
    }
    days.reverse();
    days
}
