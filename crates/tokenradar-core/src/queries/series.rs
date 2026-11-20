//! The percentage time series of one limit (fused readings, LTTB-thinned, with window bands
//! and session markers) and the week-day × hour heat map.

use std::collections::{BTreeMap, HashMap};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use specta::Type;

use super::zone::{spans, weekday_hour};
use super::{QueryContext, Result, SessionName, TimeRange, session_name};
use crate::domain::{LimitKind, LimitRef, ModelFamily, SessionId, Source, Timestamp};
use crate::limits::fusion;
use crate::store::{Store, WINDOW_MATCH_TOLERANCE_MS};

/// Session markers a series carries at most (largest cost first).
pub const MAX_MARKERS: i64 = 200;
const HOUR_MS: i64 = 3_600_000;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct SeriesPoint {
    pub at: Timestamp,
    /// Fused value (ADR-060). Never an estimate.
    pub percent: f64,
    pub source: Source,
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn point_xy(p: &SeriesPoint) -> (f64, f64) {
    (p.at.as_millis() as f64, p.percent)
}

/// A window inside the range, drawn as a band.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct WindowBand {
    pub window_id: i64,
    pub starts_at: Timestamp,
    pub resets_at: Timestamp,
    pub peak_percent: Option<f64>,
    /// `None` for an estimated window.
    pub hit_limit: Option<bool>,
    /// No readings: draw dashed.
    pub estimated: bool,
}

/// A session tree with a share in the limit's windows in the range; `start`–`end` is its
/// life clipped to the range (the windows' span when the session's times are unknown).
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct SessionMarker {
    pub session_id: SessionId,
    pub name: SessionName,
    pub start: Timestamp,
    pub end: Timestamp,
    /// `≈` points of this limit's windows in the range.
    pub percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct TimeSeries {
    pub limit: LimitRef,
    pub range: TimeRange,
    /// Oldest first, at most the requested number of points.
    pub points: Vec<SeriesPoint>,
    pub windows: Vec<WindowBand>,
    pub markers: Vec<SessionMarker>,
}

/// `limit`'s readings in `range`, fused per window and thinned to `max_points` (at most
/// [`super::MAX_SERIES_POINTS`]).
pub fn get_timeseries(
    store: &Store,
    ctx: &QueryContext<'_>,
    limit: &LimitRef,
    range: TimeRange,
    max_points: Option<u32>,
) -> Result<TimeSeries> {
    let max = max_points
        .unwrap_or(super::MAX_SERIES_POINTS)
        .clamp(3, super::MAX_SERIES_POINTS) as usize;
    let to = range.to.min(ctx.now);
    let scope = limit.scope.as_ref().map(ToString::to_string);
    // Only this limit's columns, along `ix_snap` and in its order: a year of readings of every
    // limit as full snapshots took most of the time.
    let readings: Vec<(Timestamp, String, f64, Option<i64>)> = store.read(|c| {
        Ok(c.prepare_cached(
            "SELECT observed_at, source, percent, resets_at FROM limit_snapshots
             WHERE account_id = ?1 AND limit_kind = ?2 AND scope IS ?3
               AND observed_at >= ?4 AND observed_at < ?5 AND source <> 'estimate'
             ORDER BY observed_at, id",
        )?
        .query_map(
            params![ctx.account.0, limit.kind.as_str(), scope, range.from, to],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?
        .collect::<rusqlite::Result<_>>()?)
    })?;
    // Fusion is per window: group the readings by `resets_at`.
    let mut by_window: BTreeMap<i64, Vec<(Timestamp, Source, f64)>> = BTreeMap::new();
    for (at, source, percent, resets) in readings {
        let Ok(source) = source.parse::<Source>() else {
            continue;
        };
        let key = resets.map_or(i64::MIN, |r| r / WINDOW_MATCH_TOLERANCE_MS);
        by_window
            .entry(key)
            .or_default()
            .push((at, source, percent));
    }
    let mut points: Vec<SeriesPoint> = Vec::new();
    for readings in by_window.values() {
        for (&(at, source, _), &(_, percent)) in readings.iter().zip(&fusion::series(readings)) {
            points.push(SeriesPoint {
                at,
                percent,
                source,
            });
        }
    }
    points.sort_by_key(|p| p.at);
    let points = super::lttb(&points, max, point_xy);

    let windows: Vec<WindowBand> = store.read(|c| {
        Ok(c.prepare_cached(
            "SELECT id, starts_at, resets_at, peak_percent, hit_limit, estimated
             FROM limit_windows
             WHERE account_id = ?1 AND limit_kind = ?2 AND scope IS ?3
               AND resets_at > ?4 AND starts_at < ?5 ORDER BY resets_at",
        )?
        .query_map(
            params![ctx.account.0, limit.kind.as_str(), scope, range.from, to],
            |r| {
                let estimated = r.get::<_, i64>(5)? != 0;
                Ok(WindowBand {
                    window_id: r.get(0)?,
                    starts_at: r.get(1)?,
                    resets_at: r.get(2)?,
                    peak_percent: r.get(3)?,
                    hit_limit: (!estimated).then_some(r.get::<_, i64>(4)? != 0),
                    estimated,
                })
            },
        )?
        .collect::<rusqlite::Result<_>>()?)
    })?;
    let markers = markers(store, ctx, limit, range.from, to)?;
    Ok(TimeSeries {
        limit: limit.clone(),
        range,
        points,
        windows,
        markers,
    })
}

fn markers(
    store: &Store,
    ctx: &QueryContext<'_>,
    limit: &LimitRef,
    from: Timestamp,
    to: Timestamp,
) -> Result<Vec<SessionMarker>> {
    let scope = limit.scope.as_ref().map(ToString::to_string);
    // From the limit's attributions, not from usage: a few rows per window instead of a scan
    // of the range, and the markers are the sessions that moved this limit.
    store.read(|c| {
        let mut stmt = c.prepare_cached(
            "WITH sh AS (
               SELECT a.root_session_id AS root, SUM(a.cost_micro_usd) AS cost,
                      SUM(a.est_percent) AS pct, MIN(w.starts_at) AS ws, MAX(w.resets_at) AS we
               FROM limit_windows w JOIN attributions a ON a.window_id = w.id
               WHERE w.account_id = ?1 AND w.limit_kind = ?5 AND w.scope IS ?6
                 AND w.resets_at > ?2 AND w.starts_at < ?3 AND a.root_session_id IS NOT NULL
               GROUP BY a.root_session_id ORDER BY cost DESC LIMIT ?4)
             SELECT sh.root, MAX(COALESCE(s.first_ts, sh.ws), ?2),
                    MIN(COALESCE(s.last_ts, sh.we), ?3), s.uuid, s.title, p.display_name, sh.pct
             FROM sh JOIN sessions s ON s.id = sh.root
             LEFT JOIN projects p ON p.id = s.project_id
             ORDER BY 2, 1",
        )?;
        let rows = stmt
            .query_map(
                params![
                    ctx.account.0,
                    from,
                    to,
                    MAX_MARKERS,
                    limit.kind.as_str(),
                    scope
                ],
                |r| {
                    let uuid: String = r.get(3)?;
                    let title: Option<String> = r.get(4)?;
                    let project: Option<String> = r.get(5)?;
                    Ok(SessionMarker {
                        session_id: SessionId(r.get(0)?),
                        start: r.get(1)?,
                        end: r.get(2)?,
                        name: session_name(title.as_deref(), project.as_deref(), &uuid),
                        percent: r.get(6)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    })
}

/// What a heat map cell adds up.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum HeatMetric {
    #[default]
    Cost,
    Tokens,
    /// `≈` points of the weekly limit: `k · Σ α · cost` with its calibration.
    Percent,
}

/// One week-day × hour cell (local time).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct HeatCell {
    /// Monday = 0.
    pub weekday: u8,
    pub hour: u8,
    /// Cost in micro-USD, tokens, or points.
    pub total: f64,
    /// `total / active_days`; 0 without activity.
    pub mean: f64,
    /// Local dates with usage in this cell.
    pub active_days: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct Heatmap {
    pub metric: HeatMetric,
    pub range: TimeRange,
    /// All 168 cells, Monday 00:00 first.
    pub cells: Vec<HeatCell>,
    pub max_total: f64,
    /// Set for [`HeatMetric::Percent`]; `None` without a usable calibration (then every
    /// total is 0).
    pub basis: Option<super::Basis>,
}

/// Usage per local week-day and hour in `range`.
pub fn get_heatmap(
    store: &Store,
    ctx: &QueryContext<'_>,
    range: TimeRange,
    metric: HeatMetric,
) -> Result<Heatmap> {
    let to = range.to.min(ctx.now);
    let cal = ctx
        .calibrations
        .iter()
        .find(|c| c.kind == LimitKind::WeeklyAll && c.scope.is_none());
    let usable = cal.filter(|c| c.level() != crate::limits::Confidence::Low);
    // Local-hour index → value.
    let mut hours: HashMap<i64, f64> = HashMap::new();
    // The percent metric weighs cost per model in SQL (`k · α · cost`): no join, one group
    // per hour.
    let weight = match (metric, usable) {
        (HeatMetric::Percent, Some(c)) => Some(percent_weight(store, c)?),
        _ => None,
    };
    let value_sql = match (metric, &weight) {
        (HeatMetric::Tokens, _) => {
            "u.input + u.output + u.cache_read + u.cache_w5m + u.cache_w1h".to_owned()
        }
        (HeatMetric::Percent, Some(w)) => format!("u.cost_micro_usd * {w}"),
        _ => "u.cost_micro_usd".to_owned(),
    };
    if !(metric == HeatMetric::Percent && usable.is_none()) {
        // Zone spans are probed hourly: start at the oldest usage, not at the range's start.
        let from = store
            .earliest_bucket(ctx.account)?
            .map_or(to, |e| e.max(range.from));
        for (a, b, offset) in spans(ctx.zone, from.as_millis(), to.as_millis()) {
            let col = super::bucket_col(a, b);
            // Summed here, not with GROUP BY: grouping by the hour sorts every row in a
            // temporary tree, which costs more than reading them.
            let sql = format!(
                "SELECT u.bucket_ts, {value_sql} FROM usage_5m u
                 WHERE u.account_id = ?1 AND {col} >= ?2 AND {col} < ?3"
            );
            store.read(|c| {
                let mut stmt = c.prepare_cached(&sql)?;
                let mut rows = stmt.query(params![ctx.account.0, a, b])?;
                while let Some(r) = rows.next()? {
                    let hour = (r.get::<_, i64>(0)? + offset).div_euclid(HOUR_MS);
                    *hours.entry(hour).or_default() += r.get::<_, f64>(1)?;
                }
                Ok(())
            })?;
        }
    }
    let mut cells: Vec<HeatCell> = (0..7u8)
        .flat_map(|d| {
            (0..24u8).map(move |h| HeatCell {
                weekday: d,
                hour: h,
                total: 0.0,
                mean: 0.0,
                active_days: 0,
            })
        })
        .collect();
    for (hour, v) in hours {
        if v <= 0.0 {
            continue;
        }
        let (d, h) = weekday_hour(hour * HOUR_MS);
        let cell = &mut cells[usize::from(d) * 24 + usize::from(h)];
        cell.total += v;
        cell.active_days += 1;
    }
    let mut max_total = 0.0f64;
    for c in &mut cells {
        if c.active_days > 0 {
            c.mean = c.total / f64::from(c.active_days);
        }
        max_total = max_total.max(c.total);
    }
    let basis = (metric == HeatMetric::Percent).then(|| match usable {
        Some(c) => super::Basis::new(
            crate::attribution::Method::Estimated,
            Some(c.confidence),
            false,
        ),
        None => super::Basis::new(crate::attribution::Method::CostOnly, None, false),
    });
    Ok(Heatmap {
        metric,
        range,
        cells,
        max_total,
        basis,
    })
}

/// `CASE` over model ids giving `k · α` per micro-USD; 0 for families a scoped limit does not
/// count.
fn percent_weight(store: &Store, c: &crate::calibration::Calibration) -> Result<String> {
    use std::fmt::Write as _;
    let models: Vec<(i64, String)> = store.read(|conn| {
        Ok(conn
            .prepare_cached("SELECT id, family FROM models")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?)
    })?;
    let mut sql = String::from("CASE u.model_id");
    for (id, fam) in models {
        let f: ModelFamily = fam.parse().unwrap_or(ModelFamily::Unknown);
        let w = if c.family().is_some_and(|only| only != f) {
            0.0
        } else {
            c.k * c.alpha.get(&f).copied().unwrap_or(1.0) / 1_000_000.0
        };
        let _ = write!(sql, " WHEN {id} THEN {w:e}");
    }
    let _ = write!(sql, " ELSE {:e} END", c.k / 1_000_000.0);
    Ok(sql)
}
