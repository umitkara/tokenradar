//! What the periodic summaries say (Step 17b): the past weekly limit window next to the one
//! before it, and the past local day. The notifier turns these into text; nothing here is
//! localized.

use rusqlite::{OptionalExtension as _, params};
use serde::Serialize;
use specta::Type;

use super::{
    Basis, CycleProject, ProjectRef, QueryContext, Result, ScopedPeak, TimeRange, WindowRow,
    list_weeks,
};
use crate::attribution::Method;
use crate::domain::{LimitKind, MicroUsd, ProjectId, Timestamp};
use crate::limits::Confidence;
use crate::store::Store;

const DAY_MS: i64 = 86_400_000;

/// A weekly limit window as the scheduler needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeekBounds {
    pub starts_at: Timestamp,
    pub resets_at: Timestamp,
}

/// The newest weekly window that has reset by `now`, and the reset of the one still open.
pub fn week_bounds(
    store: &Store,
    ctx: &QueryContext<'_>,
) -> Result<(Option<WeekBounds>, Option<Timestamp>)> {
    store.read(|c| {
        let closed = c
            .prepare_cached(
                "SELECT starts_at, resets_at FROM limit_windows
                 WHERE account_id = ?1 AND limit_kind = 'weekly_all' AND scope IS NULL
                   AND resets_at <= ?2
                 ORDER BY resets_at DESC LIMIT 1",
            )?
            .query_row(params![ctx.account.0, ctx.now.as_millis()], |r| {
                Ok(WeekBounds {
                    starts_at: Timestamp::from_millis(r.get(0)?),
                    resets_at: Timestamp::from_millis(r.get(1)?),
                })
            })
            .optional()?;
        let open = c
            .prepare_cached(
                "SELECT resets_at FROM limit_windows
                 WHERE account_id = ?1 AND limit_kind = 'weekly_all' AND scope IS NULL
                   AND resets_at > ?2
                 ORDER BY resets_at LIMIT 1",
            )?
            .query_row(params![ctx.account.0, ctx.now.as_millis()], |r| {
                r.get::<_, i64>(0)
            })
            .optional()?
            .map(Timestamp::from_millis);
        Ok((closed, open))
    })
}

/// 5-hour windows that reset inside the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct FiveHourHits {
    pub windows: u32,
    /// Measured windows that reached 100 %; estimated ones are never counted.
    pub hit: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct WeeklyDigest {
    pub window: WindowRow,
    /// First measured reading at 100 %.
    pub hit_at: Option<Timestamp>,
    pub scoped: Vec<ScopedPeak>,
    /// `None` when the account had no 5-hour window that week (ADR-041).
    pub five_hour: Option<FiveHourHits>,
    /// The project with the largest share; sessions without a project are skipped.
    pub top_project: Option<CycleProject>,
    /// The window before, for the comparison line.
    pub previous: Option<WindowRow>,
}

impl WeeklyDigest {
    /// Nothing happened that week: no usage and no reading above zero.
    pub fn is_empty(&self) -> bool {
        self.window.cost.0 == 0 && self.window.peak_percent.is_none_or(|p| p <= 0.0)
    }

    /// Stored attribution exists, so the project line can be trusted.
    pub fn attributed(&self) -> bool {
        self.window.basis.is_some()
    }
}

/// The summary of the weekly window that reset at `resets_at`; `None` when it is not stored.
pub fn weekly_digest(
    store: &Store,
    ctx: &QueryContext<'_>,
    week: WeekBounds,
) -> Result<Option<WeeklyDigest>> {
    // This window and the one before; the open one starts at `resets_at` and is left out.
    let range = TimeRange::new(
        week.starts_at.saturating_sub_millis(7 * DAY_MS),
        week.resets_at,
    );
    let mut rows = list_weeks(store, ctx, range)?.items.into_iter();
    let Some(row) = rows.find(|r| {
        (r.window.resets_at.as_millis() - week.resets_at.as_millis()).abs()
            <= crate::store::WINDOW_MATCH_TOLERANCE_MS
    }) else {
        return Ok(None);
    };
    let previous = rows.next().map(|r| r.window);
    let five_hour = store.read(|c| {
        let (windows, hit): (u32, u32) = c
            .prepare_cached(
                "SELECT COUNT(*), COALESCE(SUM(hit_limit = 1 AND estimated = 0), 0)
                 FROM limit_windows
                 WHERE account_id = ?1 AND limit_kind = ?2
                   AND resets_at > ?3 AND resets_at <= ?4",
            )?
            .query_row(
                params![
                    ctx.account.0,
                    LimitKind::Session.as_str(),
                    week.starts_at.as_millis(),
                    week.resets_at.as_millis()
                ],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
        Ok((windows > 0).then_some(FiveHourHits { windows, hit }))
    })?;
    let top_project = row.projects.iter().find(|p| p.project.is_some()).cloned();
    Ok(Some(WeeklyDigest {
        window: row.window,
        hit_at: row.hit_at,
        scoped: row.scoped,
        five_hour,
        top_project,
        previous,
    }))
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct DailyDigest {
    pub from: Timestamp,
    pub to: Timestamp,
    pub cost: MicroUsd,
    /// `≈` points of the weekly limit, from the weekly calibration; `None` without a usable
    /// one.
    pub percent: Option<f64>,
    pub basis: Basis,
    /// Session trees with usage that day.
    pub sessions: u32,
    pub top_project: Option<ProjectRef>,
}

impl DailyDigest {
    pub fn is_empty(&self) -> bool {
        self.cost.0 == 0 && self.sessions == 0
    }
}

/// The summary of `[range.from, range.to)`, normally one local day.
pub fn daily_digest(
    store: &Store,
    ctx: &QueryContext<'_>,
    range: TimeRange,
) -> Result<DailyDigest> {
    let cal = ctx
        .calibrations
        .iter()
        .find(|c| c.kind == LimitKind::WeeklyAll && c.scope.is_none())
        .filter(|c| c.level() != Confidence::Low);
    let weight = cal
        .map(|c| super::series::percent_weight(store, c))
        .transpose()?
        .unwrap_or_else(|| "0".to_owned());
    let sql = format!(
        "SELECT COALESCE(SUM(u.cost_micro_usd), 0),
                COALESCE(SUM(u.cost_micro_usd * ({weight})), 0.0),
                COUNT(DISTINCT COALESCE(s.root_id, s.id))
         FROM usage_5m u JOIN sessions s ON s.id = u.session_id
         WHERE u.account_id = ?1 AND u.bucket_ts >= ?2 AND u.bucket_ts < ?3"
    );
    let args = params![ctx.account.0, range.from.as_millis(), range.to.as_millis()];
    let (cost, percent, sessions, top) = store.read(|c| {
        let (cost, percent, sessions): (i64, f64, u32) = c
            .prepare_cached(&sql)?
            .query_row(args, |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        // A session's project is its root's.
        let top = c
            .prepare_cached(
                "SELECT p.id, p.display_name
                 FROM usage_5m u
                 JOIN sessions s ON s.id = u.session_id
                 JOIN sessions r ON r.id = COALESCE(s.root_id, s.id)
                 JOIN projects p ON p.id = r.project_id
                 WHERE u.account_id = ?1 AND u.bucket_ts >= ?2 AND u.bucket_ts < ?3
                 GROUP BY p.id ORDER BY SUM(u.cost_micro_usd) DESC, p.id LIMIT 1",
            )?
            .query_row(args, |r| {
                Ok(ProjectRef {
                    id: ProjectId(r.get(0)?),
                    name: r.get(1)?,
                })
            })
            .optional()?;
        Ok((cost, percent, sessions, top))
    })?;
    let basis = match cal {
        Some(c) => Basis::new(Method::Estimated, Some(c.confidence), false),
        None => Basis::new(Method::CostOnly, None, false),
    };
    Ok(DailyDigest {
        from: range.from,
        to: range.to,
        cost: MicroUsd(cost),
        percent: cal.map(|_| percent),
        basis,
        sessions,
        top_project: top,
    })
}
