//! Combining the integer percentages of the statusline and the usage API (ADR-060).
//!
//! Both sources report whole percentages, but they round differently. Observed on real
//! data, not documented: the statusline rounds down, the API to the nearest point. So each
//! reading bounds the true value:
//!
//! - statusline `n` → `[n, n + 1)`
//! - API `m` → `[m − 0.5, m + 0.5)`
//! - a fractional value, or a source we know nothing about → exactly that value
//!
//! Within a window usage never goes down, so a reading's **lower** bound stays true for the
//! rest of the window; only its upper bound ages. The fused value is therefore the highest
//! lower bound seen in the window, shown the way the API (and `/usage`) round it. A newer
//! reading whose upper bound lies at or below that lower bound contradicts it: usage did go
//! down after all (or a source changed its rounding), and the bound restarts from that
//! reading.
//!
//! In practice the value is `max(statusline, API)` of the current window. No estimate goes
//! in: the result is a measurement. Only memory holds it; `limit_snapshots` keeps each
//! source's raw reading.

use std::collections::BTreeMap;

use crate::domain::{LimitObservation, Source, Timestamp};

use super::engine::same_window;

/// Slack for float comparisons of percentages.
const EPS: f64 = 1e-9;

/// What one reading says about the true value: `[lo, hi)`, or exactly `lo` when `exact`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
    pub exact: bool,
}

/// The bounds of a reading. `None` for estimates: they never take part.
pub fn interval(source: Source, percent: f64) -> Option<Interval> {
    let whole = (percent - percent.round()).abs() < EPS;
    let (lo, hi) = match source {
        Source::Estimate => return None,
        Source::Statusline if whole => (percent, percent + 1.0),
        // Never below 0: the API's 0 means `[0, 0.5)`.
        Source::Api if whole => ((percent - 0.5).max(0.0), percent + 0.5),
        // Fractional values, and Codex until its precision is known (step 20).
        _ => {
            return Some(Interval {
                lo: percent,
                hi: percent,
                exact: true,
            });
        }
    };
    Some(Interval {
        lo,
        hi,
        exact: false,
    })
}

/// The running lower bound of one limit's current window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bound {
    lower: f64,
    exact: bool,
    /// The source of the reading that set `lower`.
    source: Source,
    /// When the bound (re)started; earlier readings no longer count.
    since: Timestamp,
}

impl Bound {
    /// A bound from a single reading; `None` for an estimate.
    pub fn start(source: Source, percent: f64, at: Timestamp) -> Option<Self> {
        let iv = interval(source, percent)?;
        Some(Self {
            lower: iv.lo,
            exact: iv.exact,
            source,
            since: at,
        })
    }

    /// The percentage to show: the lower bound rounded the API's way, or the exact value.
    pub fn value(&self) -> f64 {
        if self.exact {
            self.lower
        } else {
            self.lower.round()
        }
    }

    /// The source whose reading set the value.
    pub fn source(&self) -> Source {
        self.source
    }

    /// Folds in a reading of the same window. `newest` says it is the latest reading so far:
    /// only such a reading may contradict the bound, an older one's upper bound describes the
    /// past. Readings from before the last restart are ignored. Returns whether the bound
    /// restarted.
    pub fn fold(&mut self, source: Source, percent: f64, at: Timestamp, newest: bool) -> bool {
        let Some(iv) = interval(source, percent) else {
            return false;
        };
        if at < self.since {
            return false;
        }
        let contradicts = if iv.exact {
            iv.hi < self.lower - EPS
        } else {
            iv.hi <= self.lower + EPS
        };
        if newest && contradicts {
            *self = Self {
                lower: iv.lo,
                exact: iv.exact,
                source,
                since: at,
            };
            return true;
        }
        let tie = (iv.lo - self.lower).abs() <= EPS;
        if iv.lo > self.lower + EPS || (tie && iv.exact && !self.exact) {
            self.lower = iv.lo;
            self.exact = iv.exact;
            self.source = source;
        }
        false
    }
}

/// The fused value after each reading of one window, oldest first. `readings` must be
/// sorted by time and belong to a single limit and window.
pub fn series(readings: &[(Timestamp, Source, f64)]) -> Vec<(Timestamp, f64)> {
    let mut bound: Option<Bound> = None;
    let mut out = Vec::with_capacity(readings.len());
    for &(at, source, percent) in readings {
        match &mut bound {
            Some(b) => {
                b.fold(source, percent, at, true);
            }
            None => bound = Bound::start(source, percent, at),
        }
        if let Some(b) = &bound {
            out.push((at, b.value()));
        }
    }
    out
}

/// Each limit's newest reading with the fused percentage of its window, from the newest
/// stored reading per `(limit, source)`. Readings from another window than the newest one are
/// left out. Keeps the input's order of limits.
pub fn fuse_latest(latest_by_source: &[LimitObservation]) -> Vec<LimitObservation> {
    type Key = (String, Option<String>);
    let mut groups: BTreeMap<Key, Vec<&LimitObservation>> = BTreeMap::new();
    let mut order: Vec<Key> = Vec::new();
    for o in latest_by_source {
        let key = (
            o.kind.as_str().to_owned(),
            o.scope.as_ref().map(ToString::to_string),
        );
        if !groups.contains_key(&key) {
            order.push(key.clone());
        }
        groups.entry(key).or_default().push(o);
    }
    order
        .into_iter()
        .filter_map(|key| {
            let mut group = groups.remove(&key)?;
            group.sort_by_key(|o| o.observed_at);
            let newest = (*group.last()?).clone();
            let readings: Vec<(Timestamp, Source, f64)> = group
                .iter()
                .filter(|o| same_window(o.resets_at, newest.resets_at))
                .map(|o| (o.observed_at, o.source, o.percent))
                .collect();
            let percent = series(&readings).last().map_or(newest.percent, |p| p.1);
            Some(LimitObservation { percent, ..newest })
        })
        .collect()
}

#[cfg(test)]
mod tests;
