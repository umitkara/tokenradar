//! The patterns page (Step 17a, ADR-070): the hour profile next to the heat map, the 5-hour
//! limit risk by start hour and the weekly model mix.
//!
//! Day and week boundaries are local ([`LocalZone`](super::LocalZone)); the first day of the
//! week comes from the caller's locale (Monday-zero), the boundaries themselves are computed
//! here.

use std::collections::HashMap;

use rusqlite::params;
use serde::Serialize;
use specta::Type;

use super::zone::{local_day_start, spans, to_utc, weekday_hour};
use super::{QueryContext, Result, TimeRange};
use crate::domain::{MicroUsd, ModelFamily, Timestamp};
use crate::store::Store;

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;

/// Windows a limit-risk statement needs at least (Step 17: "≥ 10 windows").
pub const MIN_RISK_WINDOWS: u32 = 10;
/// Windows the span of a statement must hold at least.
const MIN_SPAN_WINDOWS: u32 = 5;
/// Hours a statement names.
const SPAN_HOURS: u8 = 3;

/// One hour of the day: weekday and weekend averages per calendar day of that kind.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct HourSplit {
    pub hour: u8,
    /// Monday–Friday total divided by the Monday–Friday days in the range.
    pub weekday: f64,
    /// Saturday and Sunday total divided by the weekend days in the range.
    pub weekend: f64,
}

/// Hour profile from local-hour totals: `(weekday, hour) → total` cells.
pub(crate) fn hour_splits(
    zone: &dyn super::LocalZone,
    cells: &[(u8, u8, f64)],
    from: Timestamp,
    to: Timestamp,
) -> Vec<HourSplit> {
    let (mut weekdays, mut weekends) = (0u32, 0u32);
    let mut day = local_day_start(zone, from);
    while day < to {
        let local = day.as_millis() + zone.offset_ms(day);
        if weekday_hour(local).0 >= 5 {
            weekends += 1;
        } else {
            weekdays += 1;
        }
        day = local_day_start(zone, day.saturating_add_millis(DAY_MS + 3 * HOUR_MS));
    }
    let mut out: Vec<HourSplit> = (0..24u8)
        .map(|hour| HourSplit {
            hour,
            weekday: 0.0,
            weekend: 0.0,
        })
        .collect();
    for &(d, h, v) in cells {
        let o = &mut out[usize::from(h)];
        if d >= 5 {
            o.weekend += v;
        } else {
            o.weekday += v;
        }
    }
    for o in &mut out {
        o.weekday /= f64::from(weekdays.max(1));
        o.weekend /= f64::from(weekends.max(1));
    }
    out
}

/// 5-hour windows that started in one local hour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct RiskHour {
    pub hour: u8,
    pub windows: u32,
    /// Windows that reached the limit.
    pub hit: u32,
    /// Windows that reached 90 % (the limit included).
    pub high: u32,
}

/// The hours whose windows hit the limit most often, `from_hour` to `to_hour` (exclusive,
/// may pass midnight).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct RiskSpan {
    pub from_hour: u8,
    pub to_hour: u8,
    pub windows: u32,
    pub hit: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct LimitRisk {
    /// The account has 5-hour windows at all (ADR-041).
    pub any: bool,
    /// Measured 5-hour windows that started in the range.
    pub windows: u32,
    pub hit: u32,
    /// `windows >= MIN_RISK_WINDOWS`: rates and the statement may be shown.
    pub enough: bool,
    pub minimum: u32,
    /// All 24 hours.
    pub hours: Vec<RiskHour>,
    /// Set only with enough data and a span clearly above the overall rate.
    pub span: Option<RiskSpan>,
}

/// Limit hits of measured 5-hour windows by local start hour.
pub fn limit_risk(store: &Store, ctx: &QueryContext<'_>, range: TimeRange) -> Result<LimitRisk> {
    let rows: Vec<(i64, Option<f64>, bool)> = store.read(|c| {
        Ok(c.prepare_cached(
            "SELECT starts_at, peak_percent, hit_limit FROM limit_windows
             WHERE account_id = ?1 AND limit_kind = 'session' AND estimated = 0
               AND starts_at >= ?2 AND starts_at < ?3",
        )?
        .query_map(
            params![ctx.account.0, range.from.as_millis(), range.to.as_millis()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0)),
        )?
        .collect::<rusqlite::Result<_>>()?)
    })?;
    let any = !rows.is_empty()
        || store.read(|c| {
            Ok(c.prepare_cached(
                "SELECT EXISTS (SELECT 1 FROM limit_windows
                 WHERE account_id = ?1 AND limit_kind = 'session')",
            )?
            .query_row([ctx.account.0], |r| r.get::<_, bool>(0))?)
        })?;
    let mut hours: Vec<RiskHour> = (0..24u8)
        .map(|hour| RiskHour {
            hour,
            windows: 0,
            hit: 0,
            high: 0,
        })
        .collect();
    for (start, peak, hit) in &rows {
        let at = Timestamp::from_millis(*start);
        let (_, h) = weekday_hour(start + ctx.zone.offset_ms(at));
        let o = &mut hours[usize::from(h)];
        o.windows += 1;
        o.hit += u32::from(*hit);
        o.high += u32::from(*hit || peak.is_some_and(|p| p >= 90.0));
    }
    let windows = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    let hit: u32 = hours.iter().map(|h| h.hit).sum();
    let enough = windows >= MIN_RISK_WINDOWS;
    let span = if enough {
        risk_span(&hours, windows, hit)
    } else {
        None
    };
    Ok(LimitRisk {
        any,
        windows,
        hit,
        enough,
        minimum: MIN_RISK_WINDOWS,
        hours,
        span,
    })
}

/// The three-hour span with the highest hit rate, if it holds enough windows, at least two
/// hits and a rate at least 1.5 times the overall one.
fn risk_span(hours: &[RiskHour], windows: u32, hit: u32) -> Option<RiskSpan> {
    if hit < 2 {
        return None;
    }
    let overall = f64::from(hit) / f64::from(windows);
    let mut best: Option<(f64, RiskSpan)> = None;
    for start in 0..24u8 {
        let (mut w, mut h) = (0u32, 0u32);
        for i in 0..SPAN_HOURS {
            let x = &hours[usize::from((start + i) % 24)];
            w += x.windows;
            h += x.hit;
        }
        if w < MIN_SPAN_WINDOWS || h < 2 {
            continue;
        }
        let rate = f64::from(h) / f64::from(w);
        if rate < overall * 1.5 {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|(r, s)| rate.total_cmp(r).then(w.cmp(&s.windows)).is_gt())
        {
            best = Some((
                rate,
                RiskSpan {
                    from_hour: start,
                    to_hour: (start + SPAN_HOURS) % 24,
                    windows: w,
                    hit: h,
                },
            ));
        }
    }
    best.map(|(_, s)| s)
}

/// One local week of the model mix.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct MixWeek {
    /// Local midnight of the week's first day.
    pub starts_at: Timestamp,
    pub total: MicroUsd,
    /// Cost per [`MixTrend::families`] entry, same order.
    pub parts: Vec<MicroUsd>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct MixTrend {
    /// Opus, Fable, Sonnet, Haiku when used, then `None` for every other family; the order
    /// of the stack from the bottom.
    pub families: Vec<Option<ModelFamily>>,
    /// Oldest first, every week from the first usage in range, empty weeks included.
    pub weeks: Vec<MixWeek>,
}

const MIX_ORDER: [ModelFamily; 4] = [
    ModelFamily::Opus,
    ModelFamily::Fable,
    ModelFamily::Sonnet,
    ModelFamily::Haiku,
];

/// Cost per model family and local week; weeks begin on `week_start` (Monday = 0).
pub fn model_mix_trend(
    store: &Store,
    ctx: &QueryContext<'_>,
    range: TimeRange,
    week_start: u8,
) -> Result<MixTrend> {
    let week_start = i64::from(week_start.min(6));
    let to = range.to.min(ctx.now);
    let families: HashMap<i64, Option<usize>> = store.read(|c| {
        Ok(c.prepare_cached("SELECT id, family FROM models")?
            .query_map([], |r| {
                let fam: String = r.get(1)?;
                let f: ModelFamily = fam.parse().unwrap_or(ModelFamily::Unknown);
                Ok((r.get(0)?, MIX_ORDER.iter().position(|m| *m == f)))
            })?
            .collect::<rusqlite::Result<_>>()?)
    })?;
    let Some(first) = store.earliest_bucket(ctx.account)? else {
        return Ok(MixTrend {
            families: Vec::new(),
            weeks: Vec::new(),
        });
    };
    let from = first.max(range.from);
    // Week number of a local instant: weeks counted from the epoch's first `week_start` day.
    let week_of = |local: i64| (local.div_euclid(DAY_MS) + 3 - week_start).div_euclid(7);
    let mut cost: HashMap<i64, [i64; 5]> = HashMap::new();
    for (a, b, offset) in spans(ctx.zone, from.as_millis(), to.as_millis()) {
        let col = super::bucket_col(a, b);
        let sql = format!(
            "SELECT u.bucket_ts, u.model_id, u.cost_micro_usd FROM usage_5m u
             WHERE u.account_id = ?1 AND {col} >= ?2 AND {col} < ?3"
        );
        store.read(|c| {
            let mut stmt = c.prepare_cached(&sql)?;
            let mut rows = stmt.query(params![ctx.account.0, a, b])?;
            while let Some(r) = rows.next()? {
                let week = week_of(r.get::<_, i64>(0)? + offset);
                let slot = families
                    .get(&r.get::<_, i64>(1)?)
                    .copied()
                    .flatten()
                    .unwrap_or(4);
                cost.entry(week).or_default()[slot] += r.get::<_, i64>(2)?;
            }
            Ok(())
        })?;
    }
    let used: Vec<usize> = (0..5)
        .filter(|&i| cost.values().any(|c| c[i] > 0))
        .collect();
    let first_week = week_of(from.as_millis() + ctx.zone.offset_ms(from));
    let last_week = week_of(to.as_millis() + ctx.zone.offset_ms(to));
    let weeks = (first_week..=last_week)
        .map(|wk| {
            let c = cost.get(&wk).copied().unwrap_or_default();
            let local = (wk * 7 - 3 + week_start) * DAY_MS;
            MixWeek {
                starts_at: to_utc(ctx.zone, local),
                total: MicroUsd(c.iter().sum()),
                parts: used.iter().map(|&i| MicroUsd(c[i])).collect(),
            }
        })
        .collect();
    Ok(MixTrend {
        families: used.iter().map(|&i| MIX_ORDER.get(i).copied()).collect(),
        weeks,
    })
}
