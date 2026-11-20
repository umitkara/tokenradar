//! Reading, staleness and writing around [`super::attribute`] (Step 14a, ADR-064).
//!
//! - [`refresh`] runs on the limits thread: windows without an attribution get one, windows
//!   that closed since get their final one, the open window is redone at most every
//!   [`LIVE_REFRESH_MS`] and only when new data arrived (no idle wake-ups).
//! - [`ensure`] is for viewing: a window attributed with an older calibration is redone then,
//!   not in the background.
//! - [`fill_estimated`] adds weekly windows for the weeks the logs cover but no snapshot did.

use std::collections::BTreeMap;

use super::{Input, LIVE_REFRESH_MS, Method, Share, attribute, cross_check};
use crate::calibration::intervals::{self, Edge, MAX_EDGE_UNCERTAINTY_MS, Reading};
use crate::calibration::{Calibration, CalibrationData, scope_family};
use crate::domain::{AccountId, LimitKind, LimitWindow, Source, Timestamp};
use crate::store::{
    AttributionRun, ReadOnlyStore, Store, StoreError, StoredAttribution, WINDOW_MATCH_TOLERANCE_MS,
};

const WEEK_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// What attribution reads; the app's pooled [`Store`] and a read-only copy share it.
pub trait AttributionData: CalibrationData {
    fn windows(&self, account: AccountId) -> Result<Vec<LimitWindow>, StoreError>;
    fn bucket_costs(
        &self,
        account: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<Vec<super::BucketCost>, StoreError>;
    fn breakdown_other(
        &self,
        account: AccountId,
        starts_at: Timestamp,
    ) -> Result<Option<f64>, StoreError>;
    fn earliest_bucket(&self, account: AccountId) -> Result<Option<Timestamp>, StoreError>;
}

macro_rules! impl_attribution_data {
    ($($ty:ty),+) => {$(
        impl AttributionData for $ty {
            fn windows(&self, account: AccountId) -> Result<Vec<LimitWindow>, StoreError> {
                self.limit_windows(account)
            }
            fn bucket_costs(
                &self,
                account: AccountId,
                from: Timestamp,
                to: Timestamp,
            ) -> Result<Vec<super::BucketCost>, StoreError> {
                <$ty>::bucket_costs(self, account, from, to)
            }
            fn breakdown_other(
                &self,
                account: AccountId,
                starts_at: Timestamp,
            ) -> Result<Option<f64>, StoreError> {
                <$ty>::breakdown_other(self, account, starts_at)
            }
            fn earliest_bucket(&self, account: AccountId) -> Result<Option<Timestamp>, StoreError> {
                <$ty>::earliest_bucket(self, account)
            }
        }
    )+};
}

impl_attribution_data!(Store, ReadOnlyStore);

fn calibration_for<'a>(cals: &'a [Calibration], w: &LimitWindow) -> Option<&'a Calibration> {
    cals.iter().find(|c| c.kind == w.kind && c.scope == w.scope)
}

/// Sharp edges of the window's source: the one with more of them, the later last edge on a
/// tie (Step 12t's rule).
pub fn window_edges(data: &impl AttributionData, w: &LimitWindow) -> Result<Vec<Edge>, StoreError> {
    let snaps = data.snapshots(
        w.account,
        w.starts_at,
        w.resets_at.saturating_add_millis(WINDOW_MATCH_TOLERANCE_MS),
    )?;
    let mut best: Vec<Edge> = Vec::new();
    for source in [Source::Statusline, Source::Api] {
        let readings: Vec<Reading> = snaps
            .iter()
            .filter(|s| s.kind == w.kind && s.scope == w.scope && s.source == source)
            .map(|s| Reading {
                at: s.observed_at,
                percent: s.percent,
                resets_at: s.resets_at,
            })
            .collect();
        let edges: Vec<Edge> = intervals::edges(&readings)
            .into_iter()
            .find(|(r, _)| {
                (r.as_millis() - w.resets_at.as_millis()).abs() <= WINDOW_MATCH_TOLERANCE_MS
            })
            .map(|(_, e)| e)
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.uncertainty_ms() <= MAX_EDGE_UNCERTAINTY_MS)
            .collect();
        let better = edges.len() > best.len()
            || (edges.len() == best.len()
                && !edges.is_empty()
                && edges.last().map(Edge::at) > best.last().map(Edge::at));
        if better {
            best = edges;
        }
    }
    Ok(best)
}

/// Attributes one window from the data, without writing.
pub fn compute(
    data: &impl AttributionData,
    w: &LimitWindow,
    cals: &[Calibration],
    now: Timestamp,
) -> Result<(AttributionRun, Vec<Share>), StoreError> {
    let end = now.min(w.resets_at);
    let cal = calibration_for(cals, w);
    let edges = if w.estimated {
        Vec::new()
    } else {
        window_edges(data, w)?
    };
    let buckets = data.bucket_costs(w.account, w.starts_at, end.max(w.starts_at))?;
    let outcome = attribute(&Input {
        start: w.starts_at,
        end: end.max(w.starts_at),
        peak: if w.estimated { None } else { w.peak_percent },
        edges: &edges,
        buckets: &buckets,
        family: scope_family(w.scope.as_ref()),
        calibration: cal,
    });
    let (mut breakdown, mut suspect, mut excess) = (None, false, false);
    if w.kind == LimitKind::WeeklyAll
        && outcome.method == Method::Calibrated
        && let (Some(total), Some(other)) = (outcome.total, outcome.other)
        && total > 0.0
    {
        breakdown = data.breakdown_other(w.account, w.starts_at)?;
        if let Some(b) = breakdown {
            (suspect, excess) = cross_check(other / total * 100.0, b);
        }
    }
    let run = AttributionRun {
        window_id: w.id,
        calibration_version: cal.map_or(0, |c| c.version),
        method: outcome.method,
        confidence: outcome.confidence,
        total_percent: outcome.total,
        other_percent: outcome.other,
        breakdown_other: breakdown,
        suspect,
        other_excess: excess,
        closed: now >= w.resets_at,
        computed_at: now,
    };
    Ok((run, outcome.shares))
}

/// Brings the account's attributions up to date (module docs), newest window first and at
/// most [`MAX_PER_REFRESH`] of them; [`Refreshed::more`] says to call again soon. `new_data`:
/// snapshots or usage arrived since the last call.
pub fn refresh(
    store: &Store,
    account: AccountId,
    cals: &[Calibration],
    now: Timestamp,
    new_data: bool,
) -> Result<Refreshed, StoreError> {
    let runs: BTreeMap<i64, AttributionRun> = store
        .attribution_runs(account)?
        .into_iter()
        .map(|r| (r.window_id, r))
        .collect();
    let mut out = Refreshed::default();
    let mut windows = store.limit_windows(account)?;
    windows.sort_by_key(|w| std::cmp::Reverse(w.resets_at));
    for w in windows {
        let prev = runs.get(&w.id);
        let due = match prev {
            None => true,
            Some(r) if r.closed => false,
            Some(r) => {
                now >= w.resets_at
                    || (new_data && now.as_millis() - r.computed_at.as_millis() >= LIVE_REFRESH_MS)
            }
        };
        if !due {
            continue;
        }
        if out.recomputed >= MAX_PER_REFRESH {
            out.more = true;
            break;
        }
        let (run, shares) = compute(store, &w, cals, now)?;
        if run.suspect && !prev.is_some_and(|p| p.suspect) {
            out.newly_suspect.push(w.id);
        }
        store.writer().write_attribution(run, shares)?;
        out.recomputed += 1;
    }
    Ok(out)
}

/// Windows one [`refresh`] recomputes at most (a few ms each), so a first run over a long
/// history does not hold the limits thread.
pub const MAX_PER_REFRESH: usize = 64;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Refreshed {
    pub recomputed: usize,
    /// Due windows were left for the next call.
    pub more: bool,
    /// Windows whose cross-check turned suspect in this call.
    pub newly_suspect: Vec<i64>,
}

/// A window's attribution for viewing, recomputed first when its calibration changed or the
/// open window's copy is older than [`LIVE_REFRESH_MS`]. `None` for an unknown window.
pub fn ensure(
    store: &Store,
    window_id: i64,
    account: AccountId,
    cals: &[Calibration],
    now: Timestamp,
) -> Result<Option<StoredAttribution>, StoreError> {
    let Some(w) = store
        .limit_windows(account)?
        .into_iter()
        .find(|w| w.id == window_id)
    else {
        return Ok(None);
    };
    let stored = store.attribution(window_id)?;
    let version = calibration_for(cals, &w).map_or(0, |c| c.version);
    let stale = match &stored {
        None => true,
        Some(a) if a.run.calibration_version != version => true,
        Some(a) => {
            !a.run.closed
                && (now >= w.resets_at
                    || now.as_millis() - a.run.computed_at.as_millis() >= LIVE_REFRESH_MS)
        }
    };
    if !stale {
        return Ok(stored);
    }
    let (run, shares) = compute(store, &w, cals, now)?;
    store.writer().write_attribution(run, shares)?;
    store.attribution(window_id)
}

/// `(starts_at, resets_at)` of the weekly windows the logs cover but no window row does:
/// gaps between known `weekly_all` windows and the weeks before the first, back to the oldest
/// usage. The reset time is fixed per account, so known windows step back by 7 days.
pub fn estimated_weekly_windows(
    windows: &[LimitWindow],
    earliest_usage: Option<Timestamp>,
) -> Vec<(Timestamp, Timestamp)> {
    let mut weekly: Vec<&LimitWindow> = windows
        .iter()
        .filter(|w| w.kind == LimitKind::WeeklyAll && w.scope.is_none())
        .collect();
    weekly.sort_by_key(|w| w.resets_at);
    let (Some(first), Some(earliest)) = (weekly.first(), earliest_usage) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for pair in weekly.windows(2) {
        let mut r = pair[0].resets_at.as_millis() + WEEK_MS;
        while r < pair[1].resets_at.as_millis() - WINDOW_MATCH_TOLERANCE_MS {
            out.push((
                Timestamp::from_millis(r - WEEK_MS),
                Timestamp::from_millis(r),
            ));
            r += WEEK_MS;
        }
    }
    let mut r = first.resets_at.as_millis() - WEEK_MS;
    while r > earliest.as_millis() {
        out.push((
            Timestamp::from_millis(r - WEEK_MS),
            Timestamp::from_millis(r),
        ));
        r -= WEEK_MS;
    }
    out.sort();
    out
}

/// Adds the estimated weekly windows; returns how many were added.
pub fn fill_estimated(store: &Store, account: AccountId) -> Result<usize, StoreError> {
    let windows = store.limit_windows(account)?;
    let earliest = store.earliest_bucket(account)?;
    let mut added = 0;
    for (starts, resets) in estimated_weekly_windows(&windows, earliest) {
        if store
            .writer()
            .insert_estimated_window(account, LimitKind::WeeklyAll, starts, resets)?
        {
            added += 1;
        }
    }
    Ok(added)
}
