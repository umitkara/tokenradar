//! The overview: this week (the open `weekly_all` window, else the last 7 days), its cost and
//! tokens, the top sessions and projects, "Other", calibration and the plan value.

use std::collections::HashMap;

use rusqlite::{Connection, Params, params};
use serde::Serialize;
use specta::Type;

use super::windows::other_view;
use super::{
    DAY_MS, OtherView, ProjectRef, QueryContext, Result, SessionName, TimeRange, TimeSeries,
    TokenSums, WEEK_MS, session_name,
};
use crate::attribution;
use crate::calibration::CalibrationView;
use crate::domain::{LimitKind, LimitRef, MicroUsd, ModelFamily, ProjectId, SessionId, Timestamp};
use crate::limits::LimitView;
use crate::store::Store;

/// Rows in each top list.
pub const TOP_N: usize = 5;

/// The week the overview is about.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct WeekSpan {
    pub starts_at: Timestamp,
    /// The window's reset, or `now` for the rolling fallback.
    pub ends_at: Timestamp,
    /// The open `weekly_all` window; `None` when the account has none (last 7 days).
    pub window_id: Option<i64>,
}

/// Cost and tokens of one model.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ModelSlice {
    pub model: String,
    pub family: ModelFamily,
    pub cost: MicroUsd,
    pub tokens: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Type)]
pub struct TokenBreakdown {
    pub tokens: TokenSums,
    pub cache_write_5m: i64,
    pub cache_write_1h: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct TopSession {
    pub session_id: SessionId,
    pub name: SessionName,
    pub project: Option<ProjectRef>,
    /// `≈` points of this week's window; `None` without a percentage.
    pub percent: Option<f64>,
    pub cost: MicroUsd,
    /// Cost per local day of the week, oldest first.
    pub daily: Vec<MicroUsd>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct TopProject {
    pub project: ProjectRef,
    pub percent: Option<f64>,
    pub cost: MicroUsd,
    pub daily: Vec<MicroUsd>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PlanFeeSource {
    /// `pricing/plans.toml`.
    Table,
    /// The user's `plan_fee_usd`.
    User,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct PlanValue {
    /// API-equivalent cost since local midnight on the 1st.
    pub month_cost: MicroUsd,
    pub month_start: Timestamp,
    /// The plan's display name from the table, when known.
    pub plan: Option<String>,
    pub fee: Option<MicroUsd>,
    pub fee_source: Option<PlanFeeSource>,
    /// `month_cost / fee`.
    pub ratio: Option<f64>,
    /// Share of the local calendar month gone, 0–1: early in a month the ratio is low because
    /// the month has barely started, not because the plan is worth less.
    pub month_elapsed: f64,
    /// API-equivalent cost of the whole previous local month; `None` when it had no usage.
    pub previous_month_cost: Option<MicroUsd>,
    /// `previous_month_cost / fee`: a finished month's ratio, measured, next to the running one.
    pub previous_ratio: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct OverviewOther {
    /// This week's "Other" and its check; `None` when not computed (low confidence).
    pub other: Option<OtherView>,
    /// `seven_day_breakdown` rows of this week (key, percent), largest first.
    pub breakdown: Vec<(String, f64)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct Overview {
    /// The panel's current limits, passed through.
    pub limits: Vec<LimitView>,
    /// Limits this account does not have (ADR-041), passed through like `limits`.
    pub absent: Vec<LimitRef>,
    pub week: WeekSpan,
    /// Last 7 days per limit with readings (5-hour and weekly).
    pub series: Vec<TimeSeries>,
    pub cost: MicroUsd,
    /// The whole previous week.
    pub previous_cost: Option<MicroUsd>,
    /// The previous week up to the same time into it as now is into this one: what the
    /// week-over-week change compares with (a running week against a whole one always looks
    /// lower).
    pub previous_cost_to_date: Option<MicroUsd>,
    /// `cost / previous_cost_to_date − 1` (`0.12` is 12 % more); `None` without a previous
    /// week or when it cost nothing up to this point.
    pub change: Option<f64>,
    pub tokens: TokenBreakdown,
    pub models: Vec<ModelSlice>,
    pub top_sessions: Vec<TopSession>,
    pub top_projects: Vec<TopProject>,
    /// Basis of the week's shares.
    pub basis: Option<super::Basis>,
    pub other: OverviewOther,
    pub calibration: Option<CalibrationView>,
    pub plan: PlanValue,
    pub generated_at: Timestamp,
}

/// Plan inputs: `.credentials.json` metadata and the user's fee.
#[derive(Debug, Clone, Default)]
pub struct PlanInput<'a> {
    pub subscription: Option<&'a str>,
    pub tier: Option<&'a str>,
    pub user_fee_usd: Option<f64>,
}

/// The overview at `ctx.now`. `limits` are the panel's (fusion lives there).
#[allow(clippy::too_many_lines)] // one linear assembly of the page's parts, in display order
pub fn get_overview(
    store: &Store,
    ctx: &QueryContext<'_>,
    limits: Vec<LimitView>,
    absent: Vec<LimitRef>,
    plan: &PlanInput<'_>,
) -> Result<Overview> {
    let now = ctx.now;
    let weekly = store
        .limit_windows(ctx.account)?
        .into_iter()
        .filter(|w| w.kind == LimitKind::WeeklyAll && w.scope.is_none())
        .collect::<Vec<_>>();
    let open = weekly
        .iter()
        .filter(|w| !w.estimated && w.starts_at <= now && now < w.resets_at)
        .max_by_key(|w| w.resets_at);
    let week = match open {
        Some(w) => WeekSpan {
            starts_at: w.starts_at,
            ends_at: w.resets_at,
            window_id: Some(w.id),
        },
        None => WeekSpan {
            starts_at: now.saturating_sub_millis(WEEK_MS),
            ends_at: now,
            window_id: None,
        },
    };
    let (from, to) = (week.starts_at.as_millis(), now.as_millis());
    let previous = weekly
        .iter()
        .find(|w| (w.resets_at.as_millis() - from).abs() <= crate::store::WINDOW_MATCH_TOLERANCE_MS)
        .map(|w| (w.starts_at.as_millis(), w.resets_at.as_millis()))
        .or_else(|| open.is_none().then_some((from - WEEK_MS, from)));

    let stored = match week.window_id {
        Some(id) => attribution::ensure(store, id, ctx.account, ctx.calibrations, now)?,
        None => None,
    };
    let basis = stored.as_ref().map(|s| {
        let current = open
            .and_then(|w| ctx.calibration_for(w))
            .map_or(0, |c| c.version);
        super::Basis::new(
            s.run.method,
            s.run.confidence,
            s.run.calibration_version != current,
        )
    });

    let first_day = super::local_day_start(ctx.zone, week.starts_at).as_millis();
    let days = usize::try_from((to - first_day) / DAY_MS + 1)
        .unwrap_or(1)
        .clamp(1, 8);

    let (cost, (previous_cost, previous_cost_to_date), tokens, models, root_cost, daily, projects) =
        store.read(|c| {
            let cost = sum_cost(c, ctx, from, to)?;
            let previous_cost = match previous {
                Some((a, b)) => (
                    Some(sum_cost(c, ctx, a, b)?),
                    Some(sum_cost(c, ctx, a, b.min(a + (to - from)))?),
                ),
                None => (None, None),
            };
            let tokens = token_breakdown(c, ctx, from, to)?;
            let models = model_slices(
                c,
                &format!(
                    "WHERE u.account_id = ?1 AND {b} >= ?2 AND {b} < ?3",
                    b = super::bucket_col(from, to)
                ),
                params![ctx.account.0, from, to],
            )?;
            let root_cost = super::sessions::root_costs(c, ctx.account, from, to)?;
            let daily = daily_by_root(c, ctx, first_day, to, days)?;
            let projects = root_projects(c, &root_cost)?;
            Ok((
                cost,
                previous_cost,
                tokens,
                models,
                root_cost,
                daily,
                projects,
            ))
        })?;

    // Top sessions: by share when the week is attributed, otherwise by cost.
    let mut ranked: Vec<(i64, Option<f64>, i64)> = match &stored {
        Some(s) if s.run.total_percent.is_some() => s
            .shares
            .iter()
            .map(|sh| {
                (
                    sh.root.0,
                    sh.percent,
                    root_cost.get(&sh.root.0).copied().unwrap_or(sh.cost.0),
                )
            })
            .collect(),
        _ => root_cost.iter().map(|(&r, &c)| (r, None, c)).collect(),
    };
    ranked.sort_by(|a, b| {
        b.1.unwrap_or(-1.0)
            .total_cmp(&a.1.unwrap_or(-1.0))
            .then(b.2.cmp(&a.2))
            .then(a.0.cmp(&b.0))
    });
    let labels = store.read(|c| labels(c, ranked.iter().take(TOP_N).map(|r| r.0)))?;
    let empty = vec![MicroUsd(0); days];
    let top_sessions = ranked
        .iter()
        .take(TOP_N)
        .filter_map(|&(root, percent, cost)| {
            let (name, project) = labels.get(&root)?.clone();
            Some(TopSession {
                session_id: SessionId(root),
                name,
                project,
                percent,
                cost: MicroUsd(cost),
                daily: daily.get(&root).cloned().unwrap_or_else(|| empty.clone()),
            })
        })
        .collect();

    // Top projects: shares and costs of their session trees.
    let mut by_project: HashMap<i64, (ProjectRef, Option<f64>, i64, Vec<MicroUsd>)> =
        HashMap::new();
    for &(root, percent, cost) in &ranked {
        let Some(p) = projects.get(&root) else {
            continue;
        };
        let e = by_project
            .entry(p.id.0)
            .or_insert_with(|| (p.clone(), None, 0, empty.clone()));
        if let Some(v) = percent {
            e.1 = Some(e.1.unwrap_or(0.0) + v);
        }
        e.2 += cost;
        if let Some(d) = daily.get(&root) {
            for (slot, v) in e.3.iter_mut().zip(d) {
                slot.0 += v.0;
            }
        }
    }
    let mut top_projects: Vec<TopProject> = by_project
        .into_values()
        .map(|(project, percent, cost, daily)| TopProject {
            project,
            percent,
            cost: MicroUsd(cost),
            daily,
        })
        .collect();
    top_projects.sort_by(|a, b| {
        b.percent
            .unwrap_or(-1.0)
            .total_cmp(&a.percent.unwrap_or(-1.0))
            .then(b.cost.cmp(&a.cost))
            .then(a.project.id.cmp(&b.project.id))
    });
    top_projects.truncate(TOP_N);

    let breakdown = match open {
        Some(w) => store.read(|c| breakdown_rows(c, ctx, w.starts_at.as_millis()))?,
        None => Vec::new(),
    };
    let other = OverviewOther {
        other: stored.as_ref().and_then(other_view),
        breakdown,
    };

    let range = TimeRange::last_days(now, 7);
    let mut series = Vec::new();
    for kind in [LimitKind::Session, LimitKind::WeeklyAll] {
        let limit = LimitRef::new(kind, None);
        let s = super::get_timeseries(store, ctx, &limit, range, None)?;
        if !s.points.is_empty() || !s.windows.is_empty() {
            series.push(s);
        }
    }

    let calibration = ctx
        .calibrations
        .iter()
        .find(|c| c.kind == LimitKind::WeeklyAll && c.scope.is_none())
        .map(|c| c.view(now));

    let month = MonthSpan::around(ctx.zone, now);
    let (month_cost, previous_month_cost) = store.read(|c| {
        Ok((
            sum_cost(c, ctx, month.start.as_millis(), to)?,
            sum_cost(c, ctx, month.previous.as_millis(), month.start.as_millis())?,
        ))
    })?;
    let plan = plan_value(
        month_cost,
        Some(previous_month_cost).filter(|c| c.0 > 0),
        &month,
        now,
        plan,
    );

    #[allow(clippy::cast_precision_loss)]
    let change = previous_cost_to_date
        .filter(|p| p.0 > 0)
        .map(|p| cost.0 as f64 / p.0 as f64 - 1.0);

    Ok(Overview {
        limits,
        absent,
        week,
        series,
        cost,
        previous_cost,
        previous_cost_to_date,
        change,
        tokens,
        models,
        top_sessions,
        top_projects,
        basis,
        other,
        calibration,
        plan,
        generated_at: now,
    })
}

/// The local calendar month around `now`, with the previous one's start and the next one's.
struct MonthSpan {
    previous: Timestamp,
    start: Timestamp,
    next: Timestamp,
}

impl MonthSpan {
    fn around(zone: &dyn super::LocalZone, now: Timestamp) -> Self {
        let start = super::local_month_start(zone, now);
        Self {
            previous: super::local_month_start(zone, start.saturating_sub_millis(1)),
            start,
            // Any instant 32 days on lies in the next month.
            next: super::local_month_start(
                zone,
                Timestamp::from_millis(start.as_millis() + 32 * DAY_MS),
            ),
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn elapsed(&self, now: Timestamp) -> f64 {
        let span = (self.next.as_millis() - self.start.as_millis()).max(1);
        ((now.as_millis() - self.start.as_millis()) as f64 / span as f64).clamp(0.0, 1.0)
    }
}

fn plan_value(
    month_cost: MicroUsd,
    previous_month_cost: Option<MicroUsd>,
    month: &MonthSpan,
    now: Timestamp,
    input: &PlanInput<'_>,
) -> PlanValue {
    let table = super::plan_fee(input.subscription, input.tier);
    let (fee, source) = match input.user_fee_usd.filter(|v| v.is_finite() && *v > 0.0) {
        Some(v) => (Some(super::plans::usd(v)), Some(PlanFeeSource::User)),
        None => (
            table.as_ref().map(|t| t.monthly),
            table.as_ref().map(|_| PlanFeeSource::Table),
        ),
    };
    #[allow(clippy::cast_precision_loss)]
    let ratio_of = |cost: MicroUsd| fee.filter(|f| f.0 > 0).map(|f| cost.0 as f64 / f.0 as f64);
    PlanValue {
        month_cost,
        month_start: month.start,
        plan: table.map(|t| t.name),
        fee,
        fee_source: source,
        ratio: ratio_of(month_cost),
        month_elapsed: month.elapsed(now),
        previous_month_cost,
        previous_ratio: previous_month_cost.and_then(ratio_of),
    }
}

fn sum_cost(c: &Connection, ctx: &QueryContext<'_>, from: i64, to: i64) -> Result<MicroUsd> {
    Ok(MicroUsd(
        c.prepare_cached(&format!(
            "SELECT COALESCE(SUM(u.cost_micro_usd), 0) FROM usage_5m u
             WHERE u.account_id = ?1 AND {b} >= ?2 AND {b} < ?3",
            b = super::bucket_col(from, to)
        ))?
        .query_row(params![ctx.account.0, from, to], |r| r.get(0))?,
    ))
}

fn token_breakdown(
    c: &Connection,
    ctx: &QueryContext<'_>,
    from: i64,
    to: i64,
) -> Result<TokenBreakdown> {
    Ok(c.prepare_cached(&format!(
        "SELECT COALESCE(SUM(u.input), 0), COALESCE(SUM(u.output), 0),
                COALESCE(SUM(u.cache_read), 0), COALESCE(SUM(u.cache_w5m), 0),
                COALESCE(SUM(u.cache_w1h), 0)
         FROM usage_5m u WHERE u.account_id = ?1 AND {b} >= ?2 AND {b} < ?3",
        b = super::bucket_col(from, to)
    ))?
    .query_row(params![ctx.account.0, from, to], |r| {
        let (w5, w1): (i64, i64) = (r.get(3)?, r.get(4)?);
        Ok(TokenBreakdown {
            tokens: TokenSums {
                input: r.get(0)?,
                output: r.get(1)?,
                cache_read: r.get(2)?,
                cache_write: w5 + w1,
            },
            cache_write_5m: w5,
            cache_write_1h: w1,
        })
    })?)
}

/// Cost and tokens per model of the `usage_5m` rows `filter` keeps (`u` is the table alias,
/// joins go before the `WHERE`), most expensive first.
pub(crate) fn model_slices(
    c: &Connection,
    filter: &str,
    args: impl Params,
) -> Result<Vec<ModelSlice>> {
    let sql = format!(
        "SELECT m.name, m.family, SUM(u.cost_micro_usd),
                SUM(u.input + u.output + u.cache_read + u.cache_w5m + u.cache_w1h)
         FROM usage_5m u JOIN models m ON m.id = u.model_id {filter}
         GROUP BY m.id ORDER BY 3 DESC, m.name"
    );
    Ok(c.prepare(&sql)?
        .query_map(args, |r| {
            let family: String = r.get(1)?;
            Ok(ModelSlice {
                model: r.get(0)?,
                family: family.parse().unwrap_or(ModelFamily::Unknown),
                cost: MicroUsd(r.get(2)?),
                tokens: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}

/// Cost per session tree per local day from `first_day` (local midnight), `days` slots. Local
/// days are 24 h here; a DST day shifts the boundary by an hour, which a sparkline hides.
fn daily_by_root(
    c: &Connection,
    ctx: &QueryContext<'_>,
    first_day: i64,
    to: i64,
    days: usize,
) -> Result<HashMap<i64, Vec<MicroUsd>>> {
    let rows: Vec<(i64, i64, i64)> = c
        .prepare_cached(&format!(
            "SELECT COALESCE(s.root_id, s.id), (u.bucket_ts - ?2) / {DAY_MS}, SUM(u.cost_micro_usd)
             FROM usage_5m u JOIN sessions s ON s.id = u.session_id
             WHERE u.account_id = ?1 AND {b} >= ?2 AND {b} < ?3 GROUP BY 1, 2",
            b = super::bucket_col(first_day, to)
        ))?
        .query_map(params![ctx.account.0, first_day, to], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut out: HashMap<i64, Vec<MicroUsd>> = HashMap::new();
    for (root, day, cost) in rows {
        let v = out.entry(root).or_insert_with(|| vec![MicroUsd(0); days]);
        if let Some(slot) = usize::try_from(day).ok().and_then(|d| v.get_mut(d)) {
            slot.0 += cost;
        }
    }
    Ok(out)
}

fn root_projects(c: &Connection, roots: &HashMap<i64, i64>) -> Result<HashMap<i64, ProjectRef>> {
    let mut stmt = c.prepare_cached(
        "SELECT p.id, p.display_name FROM sessions s JOIN projects p ON p.id = s.project_id
         WHERE s.id = ?1",
    )?;
    let mut out = HashMap::new();
    for &root in roots.keys() {
        if let Some((id, name)) =
            rusqlite::OptionalExtension::optional(stmt.query_row([root], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            }))?
        {
            out.insert(
                root,
                ProjectRef {
                    id: ProjectId(id),
                    name,
                },
            );
        }
    }
    Ok(out)
}

type Label = (SessionName, Option<ProjectRef>);

fn labels(c: &Connection, roots: impl Iterator<Item = i64>) -> Result<HashMap<i64, Label>> {
    let mut stmt = c.prepare_cached(
        "SELECT s.uuid, s.title, p.id, p.display_name FROM sessions s
         LEFT JOIN projects p ON p.id = s.project_id WHERE s.id = ?1",
    )?;
    let mut out = HashMap::new();
    for root in roots {
        let (uuid, title, pid, pname): (String, Option<String>, Option<i64>, Option<String>) =
            stmt.query_row([root], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
        out.insert(
            root,
            (
                session_name(title.as_deref(), pname.as_deref(), &uuid),
                pid.zip(pname).map(|(id, name)| ProjectRef {
                    id: ProjectId(id),
                    name,
                }),
            ),
        );
    }
    Ok(out)
}

/// The newest `seven_day_breakdown` of the window starting at `starts_at`, largest first.
fn breakdown_rows(
    c: &Connection,
    ctx: &QueryContext<'_>,
    starts_at: i64,
) -> Result<Vec<(String, f64)>> {
    Ok(c.prepare_cached(
        "SELECT key, COALESCE(percent, 0) FROM surface_breakdown
         WHERE account_id = ?1
           AND as_of = (SELECT MAX(as_of) FROM surface_breakdown
                        WHERE account_id = ?1 AND window_started_at BETWEEN ?2 - ?3 AND ?2 + ?3)
         ORDER BY 2 DESC, key",
    )?
    .query_map(
        params![
            ctx.account.0,
            starts_at,
            crate::store::WINDOW_MATCH_TOLERANCE_MS
        ],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?
    .collect::<rusqlite::Result<_>>()?)
}
