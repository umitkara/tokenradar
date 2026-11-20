//! Limit windows: the list (stored attribution, keyset by `resets_at`) and one window with its
//! shares, readings and the "Other" check.

use std::collections::HashMap;
use std::fmt::Write as _;

use rusqlite::{Connection, params_from_iter, types::Value};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::{
    Basis, Cursor, ProjectRef, QueryContext, Result, SessionName, TimeRange, page_size,
    session_name,
};
use crate::attribution::{self, Method};
use crate::domain::{LimitKind, LimitRef, LimitWindow, MicroUsd, Scope, SessionId, Timestamp};
use crate::limits::history::window_readings;
use crate::store::{Store, StoredAttribution};

/// Which windows a list shows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
pub struct WindowFilter {
    /// Windows overlapping this range; all when `None`.
    pub range: Option<TimeRange>,
    /// One limit; all when `None`.
    pub limit: Option<LimitRef>,
}

/// One window in a list.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct WindowRow {
    pub id: i64,
    pub kind: LimitKind,
    pub scope: Option<Scope>,
    pub starts_at: Timestamp,
    pub resets_at: Timestamp,
    /// Measured peak; for an estimated window the `≈` estimate (`basis.approx`), or `None`
    /// when it has no percentage.
    pub peak_percent: Option<f64>,
    /// `None` for an estimated window: whether it hit the limit is unknown.
    pub hit_limit: Option<bool>,
    pub estimated: bool,
    /// `resets_at` is still ahead.
    pub open: bool,
    /// Cost the limit counts (only its family for a scoped limit).
    pub cost: MicroUsd,
    pub sessions: u32,
    /// "Other" share in points; `None` when not computed (ADR-064).
    pub other_percent: Option<f64>,
    /// `None` until the window is attributed.
    pub basis: Option<Basis>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct WindowPage {
    pub items: Vec<WindowRow>,
    /// Pass back for the next page; `None` on the last one.
    pub next: Option<String>,
}

/// One session tree's part of a window.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ShareRow {
    pub session_id: SessionId,
    pub name: SessionName,
    pub project: Option<ProjectRef>,
    /// `≈` points; `None` when the window has no percentage to split.
    pub percent: Option<f64>,
    pub cost: MicroUsd,
}

/// The window's "Other" and its check against `seven_day_breakdown` (ADR-064).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct OtherView {
    pub percent: f64,
    /// Share of the window's total, 0–100.
    pub share_of_total: f64,
    /// The breakdown's non-Claude-Code share, 0–100, when the API reported one.
    pub breakdown_other: Option<f64>,
    /// "Other" is more than 10 points below the breakdown: the calibration counts local use
    /// too high.
    pub suspect: bool,
    /// "Other" is more than 10 points above it: another computer or the calibration (info).
    pub other_excess: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct WindowDetail {
    pub window: WindowRow,
    pub shares: Vec<ShareRow>,
    pub other: Option<OtherView>,
    /// Measured readings, fused (ADR-060) and thinned; empty for an estimated window.
    pub points: Vec<super::SeriesPoint>,
    /// The sessions' shares building up over the window (Step 17a).
    pub contrib: Contrib,
    /// Notifications fired for this window (the log keeps 30 days).
    pub alerts: Vec<AlertRow>,
}

/// What [`Contrib`] values count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ContribUnit {
    /// `≈` points: each session's stored share spread over time like its cost.
    Percent,
    /// Micro-USD, when the window's shares have no percentage.
    Cost,
}

/// One stacked series of [`Contrib`].
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ContribSeries {
    /// `None` for "the other sessions" (everything after the largest four).
    pub session_id: Option<SessionId>,
    pub name: Option<SessionName>,
    /// Running total at each of [`Contrib::at`].
    pub values: Vec<f64>,
}

/// Cumulative session contributions over the window, largest share at the bottom.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct Contrib {
    pub unit: ContribUnit,
    /// Bucket ends, oldest first; up to now for an open window.
    pub at: Vec<Timestamp>,
    pub series: Vec<ContribSeries>,
}

/// A notification the window caused.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct AlertRow {
    /// `threshold`, `projection`, `final` or `reset`.
    pub rule: String,
    pub threshold: Option<u8>,
    pub at: Timestamp,
}

/// Sessions a contribution chart names; the rest share one series. Four, because the design
/// has four validated categorical colours (design-language.md); a fifth would need a new hue.
pub const CONTRIB_SESSIONS: usize = 4;
/// Buckets a contribution chart has at most.
const CONTRIB_BUCKETS: i64 = 168;

const ROW_SQL: &str = "
    SELECT w.id, w.limit_kind, w.scope, w.starts_at, w.resets_at, w.peak_percent, w.hit_limit,
           w.estimated, r.method, r.confidence, r.other_percent, r.calibration_version,
           (SELECT COUNT(*) FROM attributions a
             WHERE a.window_id = w.id AND a.root_session_id IS NOT NULL),
           (SELECT COALESCE(SUM(a.cost_micro_usd), 0) FROM attributions a
             WHERE a.window_id = w.id)
    FROM limit_windows w LEFT JOIN attribution_runs r ON r.window_id = w.id";

struct RawRow {
    window: LimitWindow,
    method: Option<String>,
    confidence: Option<f64>,
    other: Option<f64>,
    version: Option<i64>,
    sessions: i64,
    cost: i64,
}

fn raw_row(r: &rusqlite::Row<'_>, account: crate::domain::AccountId) -> rusqlite::Result<RawRow> {
    let kind: String = r.get(1)?;
    let scope: Option<String> = r.get(2)?;
    Ok(RawRow {
        window: LimitWindow {
            id: r.get(0)?,
            account,
            kind: LimitKind::from(kind.as_str()),
            scope: scope.and_then(|s| s.parse().ok()),
            starts_at: r.get(3)?,
            resets_at: r.get(4)?,
            peak_percent: r.get(5)?,
            hit_limit: r.get::<_, i64>(6)? != 0,
            estimated: r.get::<_, i64>(7)? != 0,
        },
        method: r.get(8)?,
        confidence: r.get(9)?,
        other: r.get(10)?,
        version: r.get(11)?,
        sessions: r.get(12)?,
        cost: r.get(13)?,
    })
}

fn to_row(ctx: &QueryContext<'_>, raw: RawRow) -> WindowRow {
    let w = raw.window;
    let current = ctx.calibration_for(&w).map_or(0, |c| c.version);
    let basis = raw
        .method
        .as_deref()
        .and_then(Method::parse)
        .map(|m| Basis::new(m, raw.confidence, raw.version.is_some_and(|v| v != current)));
    WindowRow {
        id: w.id,
        open: ctx.now < w.resets_at,
        hit_limit: (!w.estimated).then_some(w.hit_limit),
        peak_percent: w.peak_percent,
        estimated: w.estimated,
        cost: MicroUsd(raw.cost),
        sessions: u32::try_from(raw.sessions).unwrap_or(0),
        other_percent: raw.other,
        basis,
        kind: w.kind,
        scope: w.scope,
        starts_at: w.starts_at,
        resets_at: w.resets_at,
    }
}

/// Windows newest first (`resets_at`, then id), one page after `cursor`.
pub fn list_windows(
    store: &Store,
    ctx: &QueryContext<'_>,
    filter: &WindowFilter,
    cursor: Option<&str>,
    limit: Option<u32>,
) -> Result<WindowPage> {
    let n = page_size(limit);
    let after = Cursor::decode(cursor).unwrap_or(Cursor::START);
    let (from, to) = super::bounds(filter.range);
    let mut sql = format!(
        "{ROW_SQL} WHERE w.account_id = ?1 AND w.resets_at > ?2 AND w.starts_at < ?3
           AND (w.resets_at < ?4 OR (w.resets_at = ?4 AND w.id < ?5))"
    );
    #[allow(clippy::cast_possible_truncation)]
    let key = if after.key.is_finite() {
        after.key as i64
    } else {
        i64::MAX
    };
    let mut args: Vec<Value> = vec![
        ctx.account.0.into(),
        from.into(),
        to.into(),
        key.into(),
        after.id.into(),
    ];
    if let Some(l) = &filter.limit {
        sql.push_str(" AND w.limit_kind = ?6 AND w.scope IS ?7");
        args.push(l.kind.as_str().to_owned().into());
        args.push(
            l.scope
                .as_ref()
                .map_or(Value::Null, |s| Value::Text(s.to_string())),
        );
    }
    let _ = write!(sql, " ORDER BY w.resets_at DESC, w.id DESC LIMIT {}", n + 1);
    let account = ctx.account;
    let mut raws: Vec<RawRow> = store.read(|c| {
        Ok(c.prepare_cached(&sql)?
            .query_map(params_from_iter(args), |r| raw_row(r, account))?
            .collect::<rusqlite::Result<_>>()?)
    })?;
    let more = raws.len() > n as usize;
    raws.truncate(n as usize);
    let items: Vec<WindowRow> = raws.into_iter().map(|r| to_row(ctx, r)).collect();
    #[allow(clippy::cast_precision_loss)]
    let next = more.then(|| items.last()).flatten().map(|w| {
        Cursor {
            key: w.resets_at.as_millis() as f64,
            id: w.id,
        }
        .encode()
    });
    Ok(WindowPage { items, next })
}

pub(crate) fn window_row(
    c: &Connection,
    ctx: &QueryContext<'_>,
    id: i64,
) -> Result<Option<WindowRow>> {
    use rusqlite::OptionalExtension;
    let sql = format!("{ROW_SQL} WHERE w.id = ?1 AND w.account_id = ?2");
    let account = ctx.account;
    let raw = c
        .prepare_cached(&sql)?
        .query_row(rusqlite::params![id, account.0], |r| raw_row(r, account))
        .optional()?;
    Ok(raw.map(|r| to_row(ctx, r)))
}

/// The 5-hour windows overlapping `[from, to)`, oldest first, in one statement (the strip).
pub(crate) fn session_rows(
    c: &Connection,
    ctx: &QueryContext<'_>,
    from: Timestamp,
    to: Timestamp,
) -> Result<Vec<WindowRow>> {
    let sql = format!(
        "{ROW_SQL} WHERE w.account_id = ?1 AND w.limit_kind = 'session'
           AND w.resets_at > ?2 AND w.starts_at < ?3 ORDER BY w.starts_at"
    );
    let account = ctx.account;
    let raws = c
        .prepare_cached(&sql)?
        .query_map(
            rusqlite::params![account.0, from.as_millis(), to.as_millis()],
            |r| raw_row(r, account),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(raws.into_iter().map(|r| to_row(ctx, r)).collect())
}

/// One window, attributed first if its attribution is missing or stale
/// ([`attribution::ensure`]). `None` for an unknown window.
pub fn get_window(store: &Store, ctx: &QueryContext<'_>, id: i64) -> Result<Option<WindowDetail>> {
    let Some(stored) = attribution::ensure(store, id, ctx.account, ctx.calibrations, ctx.now)?
    else {
        return Ok(None);
    };
    let Some(window) = store.read(|c| window_row(c, ctx, id))? else {
        return Ok(None);
    };
    let shares = share_rows(store, &stored)?;
    let other = other_view(&stored);
    let points = if window.estimated {
        Vec::new()
    } else {
        readings(store, ctx, &window)?
    };
    let contrib = contrib(store, ctx, &window, &shares)?;
    let alerts = alerts(store, &window)?;
    Ok(Some(WindowDetail {
        window,
        shares,
        other,
        points,
        contrib,
        alerts,
    }))
}

/// Each named session's share (or cost) accumulated like its cost, bucket by bucket.
fn contrib(
    store: &Store,
    ctx: &QueryContext<'_>,
    w: &WindowRow,
    shares: &[ShareRow],
) -> Result<Contrib> {
    const FIVE_MIN: i64 = 300_000;
    let (from, to) = (w.starts_at.as_millis(), w.resets_at.as_millis());
    let len = (to - from).max(FIVE_MIN);
    let step = ((len / CONTRIB_BUCKETS + FIVE_MIN - 1) / FIVE_MIN).max(1) * FIVE_MIN;
    let end = to.min(ctx.now.as_millis().max(from));
    let n = usize::try_from((end - from + step - 1) / step).unwrap_or(0);
    let unit = if shares.iter().any(|s| s.percent.is_some()) {
        ContribUnit::Percent
    } else {
        ContribUnit::Cost
    };
    let named: Vec<&ShareRow> = shares.iter().take(CONTRIB_SESSIONS).collect();
    let slot: HashMap<i64, usize> = named
        .iter()
        .enumerate()
        .map(|(i, s)| (s.session_id.0, i))
        .collect();
    let rest = named.len();
    // Cost per (series, bucket); a scoped limit counts only its family.
    let mut cost = vec![vec![0i64; n]; rest + 1];
    let family = crate::calibration::scope_family(w.scope.as_ref());
    let family_sql = if family.is_some() {
        " AND u.model_id IN (SELECT id FROM models WHERE family = ?4)"
    } else {
        ""
    };
    let sql = format!(
        "SELECT COALESCE(s.root_id, s.id), u.bucket_ts, u.cost_micro_usd
         FROM usage_5m u JOIN sessions s ON s.id = u.session_id
         WHERE u.account_id = ?1 AND {col} >= ?2 AND {col} < ?3{family_sql}",
        col = super::bucket_col(from, end)
    );
    store.read(|c| {
        let mut stmt = c.prepare_cached(&sql)?;
        let mut args: Vec<Value> = vec![ctx.account.0.into(), from.into(), end.into()];
        if let Some(f) = family {
            args.push(f.as_str().to_owned().into());
        }
        let mut rows = stmt.query(params_from_iter(args))?;
        while let Some(r) = rows.next()? {
            let root: i64 = r.get(0)?;
            let b = usize::try_from((r.get::<_, i64>(1)? - from) / step).unwrap_or(0);
            let series = slot.get(&root).copied().unwrap_or(rest);
            if let Some(cell) = cost[series].get_mut(b) {
                *cell += r.get::<_, i64>(2)?;
            }
        }
        Ok(())
    })?;
    // The final value of each series: its share, or its cost.
    #[allow(clippy::cast_precision_loss)]
    let finals: Vec<f64> = (0..=rest)
        .map(|i| match (unit, named.get(i)) {
            (ContribUnit::Percent, Some(s)) => s.percent.unwrap_or(0.0),
            (ContribUnit::Percent, None) => shares
                .iter()
                .skip(CONTRIB_SESSIONS)
                .filter_map(|s| s.percent)
                .sum(),
            (ContribUnit::Cost, _) => cost[i].iter().sum::<i64>() as f64,
        })
        .collect();
    let mut series = Vec::with_capacity(rest + 1);
    for (i, buckets) in cost.iter().enumerate() {
        let total: i64 = buckets.iter().sum();
        if i == rest && total == 0 && finals[i] == 0.0 {
            continue;
        }
        let mut run = 0i64;
        #[allow(clippy::cast_precision_loss)]
        let values = buckets
            .iter()
            .map(|c| {
                run += c;
                if total > 0 {
                    finals[i] * run as f64 / total as f64
                } else {
                    0.0
                }
            })
            .collect();
        series.push(ContribSeries {
            session_id: named.get(i).map(|s| s.session_id),
            name: named.get(i).map(|s| s.name.clone()),
            values,
        });
    }
    let at = (1..=n)
        .map(|i| Timestamp::from_millis((from + i64::try_from(i).unwrap_or(0) * step).min(end)))
        .collect();
    Ok(Contrib { unit, at, series })
}

fn alerts(store: &Store, w: &WindowRow) -> Result<Vec<AlertRow>> {
    let kind = w.kind.as_str();
    let scope = w.scope.as_ref().map(ToString::to_string);
    Ok(store
        .notifications_since(w.starts_at)?
        .into_iter()
        .filter_map(|(rule, key, at)| crate::notifier::Fired::parse(&rule, &key, at))
        .filter(|f| {
            f.kind == kind
                && f.scope == scope
                && (f.resets_at.as_millis() - w.resets_at.as_millis()).abs()
                    <= crate::store::WINDOW_MATCH_TOLERANCE_MS
        })
        .map(|f| AlertRow {
            rule: f.rule.as_str().to_owned(),
            threshold: f.threshold,
            at: f.at,
        })
        .collect())
}

pub(crate) fn other_view(stored: &StoredAttribution) -> Option<OtherView> {
    let r = &stored.run;
    let other = r.other_percent?;
    let total = r.total_percent.filter(|t| *t > 0.0)?;
    Some(OtherView {
        percent: other,
        share_of_total: other / total * 100.0,
        breakdown_other: r.breakdown_other,
        suspect: r.suspect,
        other_excess: r.other_excess,
    })
}

fn share_rows(store: &Store, stored: &StoredAttribution) -> Result<Vec<ShareRow>> {
    store.read(|c| {
        let mut stmt = c.prepare_cached(
            "SELECT s.uuid, s.title, p.id, p.display_name FROM sessions s
             LEFT JOIN projects p ON p.id = s.project_id WHERE s.id = ?1",
        )?;
        let mut out = Vec::with_capacity(stored.shares.len());
        for share in &stored.shares {
            let (uuid, title, pid, pname): (String, Option<String>, Option<i64>, Option<String>) =
                stmt.query_row([share.root.0], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                })?;
            out.push(ShareRow {
                session_id: share.root,
                name: session_name(title.as_deref(), pname.as_deref(), &uuid),
                project: pid.zip(pname).map(|(id, name)| ProjectRef {
                    id: crate::domain::ProjectId(id),
                    name,
                }),
                percent: share.percent,
                cost: share.cost,
            });
        }
        Ok(out)
    })
}

fn readings(
    store: &Store,
    ctx: &QueryContext<'_>,
    w: &WindowRow,
) -> Result<Vec<super::SeriesPoint>> {
    let snaps = store.snapshots_between(
        ctx.account,
        w.starts_at,
        w.resets_at
            .saturating_add_millis(crate::store::WINDOW_MATCH_TOLERANCE_MS),
    )?;
    let limit = LimitRef::new(w.kind.clone(), w.scope.clone());
    let length = w.resets_at.as_millis() - w.starts_at.as_millis();
    let end = ctx.now.min(w.resets_at);
    let points: Vec<super::SeriesPoint> = window_readings(&limit, w.resets_at, length, &snaps, end)
        .into_iter()
        .map(|r| super::SeriesPoint {
            at: r.at,
            percent: r.percent,
            source: r.source,
        })
        .collect();
    Ok(super::lttb(
        &points,
        super::MAX_SERIES_POINTS as usize,
        super::series::point_xy,
    ))
}
