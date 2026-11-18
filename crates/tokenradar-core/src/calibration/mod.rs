//! Calibration engine: learns how many percent of a limit one dollar of local usage costs
//! (architecture §9.2, ADR-008, ADR-061).
//!
//! - [`intervals`] turns integer readings into edge-to-edge intervals.
//! - [`estimator`] learns `k` and the family weights `α_f` from them.
//! - [`calibrate`] puts both together for every limit of an account, reading through
//!   [`CalibrationData`] so the app (pooled [`Store`]) and the backtest (read-only copy) share it.
//!
//! The runtime recomputes at most every [`RECOMPUTE_INTERVAL`] and only when new snapshots
//! arrived. [`should_store`] decides whether a result becomes a new row in `calibrations`.

pub mod estimator;
pub mod intervals;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::{
    AccountId, LimitKind, LimitObservation, MicroUsd, ModelFamily, Scope, Source, Timestamp,
};
use crate::limits::Confidence;
use crate::pricing::Pricing;
use crate::store::{ReadOnlyStore, Store, StoreError};

pub use estimator::{Drift, DriftDirection, Estimate, Obs};
pub use intervals::{Interval, Reading};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
/// Snapshots older than this are not read; at a three-day half-life their weight is < 1 %.
pub const LOOKBACK_MS: i64 = 21 * DAY_MS;
/// The statusline series is used when it yields at least this many intervals.
pub const MIN_SOURCE_INTERVALS: usize = 10;
/// Debounce of the runtime's recomputation.
pub const RECOMPUTE_INTERVAL: Duration = Duration::from_mins(15);
/// An interval is left out when models without a price carry more than this share of its cost.
pub const MAX_UNPRICED_SHARE: f64 = 0.10;
/// A new row is written when `k` moved at least this much …
pub const STORE_MIN_CHANGE: f64 = 0.02;
/// … or the newest row is this old.
pub const STORE_MAX_AGE_MS: i64 = DAY_MS;

/// Cost of one model within a time range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCost {
    pub model: String,
    pub family: ModelFamily,
    pub cost: MicroUsd,
}

/// What calibration reads.
pub trait CalibrationData {
    /// Snapshots with `from <= observed_at < to`, oldest first.
    fn snapshots(
        &self,
        account: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<Vec<LimitObservation>, StoreError>;
    /// Cost per model of the events with `from < ts <= to`.
    fn model_costs(
        &self,
        account: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<Vec<ModelCost>, StoreError>;
}

impl CalibrationData for Store {
    fn snapshots(
        &self,
        account: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<Vec<LimitObservation>, StoreError> {
        self.snapshots_between(account, from, to)
    }

    fn model_costs(
        &self,
        account: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<Vec<ModelCost>, StoreError> {
        self.model_costs_between(account, from, to)
    }
}

impl CalibrationData for ReadOnlyStore {
    fn snapshots(
        &self,
        account: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<Vec<LimitObservation>, StoreError> {
        self.snapshots_between(account, from, to)
    }

    fn model_costs(
        &self,
        account: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<Vec<ModelCost>, StoreError> {
        self.model_costs_between(account, from, to)
    }
}

/// Learned coefficients of one limit.
#[derive(Debug, Clone, PartialEq)]
pub struct Calibration {
    pub kind: LimitKind,
    pub scope: Option<Scope>,
    /// The series the intervals came from.
    pub source: Source,
    /// Percentage points per USD of `Σ α_f · cost_f`.
    pub k: f64,
    /// Family weights other than 1 (`α_opus = 1`). Empty for a single-family limit.
    pub alpha: BTreeMap<ModelFamily, f64>,
    /// Families whose weight was learned (it may have snapped to 1). A family that needs its
    /// own weight but is missing here is unknown, not 1.
    pub fitted: Vec<ModelFamily>,
    pub confidence: f64,
    pub n_obs: u32,
    pub mad_rel: f64,
    pub drift: Option<Drift>,
    pub computed_at: Timestamp,
    /// Row version once stored, 0 before.
    pub version: i64,
}

impl Calibration {
    pub fn level(&self) -> Confidence {
        estimator::level(self.confidence)
    }

    /// `Σ α_f · cost_f` in micro-USD, counting only the families the limit counts.
    pub fn weighted_cost(&self, costs: &[ModelCost]) -> f64 {
        let family = self.family();
        costs
            .iter()
            .filter(|c| family.is_none_or(|f| f == c.family))
            .map(|c| {
                #[allow(clippy::cast_precision_loss)]
                let micro = c.cost.0 as f64;
                micro * self.alpha.get(&c.family).copied().unwrap_or(1.0)
            })
            .sum()
    }

    /// Confidence for applying `k` to `costs`: the learned confidence times the share of these
    /// costs whose family weight is known. Usage of a family never seen in an interval makes
    /// the projection uncertain however well the past agrees (Step 13 backtest).
    pub fn confidence_for(&self, costs: &[ModelCost]) -> f64 {
        if self.family().is_some() {
            return self.confidence;
        }
        let (mut all, mut unknown) = (0.0, 0.0);
        for c in costs {
            #[allow(clippy::cast_precision_loss)]
            let micro = c.cost.0.max(0) as f64;
            all += micro;
            if estimator::needs_own_weight(c.family) && !self.fitted.contains(&c.family) {
                unknown += micro;
            }
        }
        if all <= 0.0 {
            self.confidence
        } else {
            self.confidence * (1.0 - unknown / all)
        }
    }

    /// The family a scoped limit counts; `None` when all local usage counts.
    pub fn family(&self) -> Option<ModelFamily> {
        scope_family(self.scope.as_ref())
    }

    /// The drift alarm while it is recent enough to show.
    pub fn visible_drift(&self, now: Timestamp) -> Option<Drift> {
        self.drift
            .filter(|d| now.as_millis() - d.detected_at.as_millis() <= estimator::DRIFT_VISIBLE_MS)
    }

    pub fn status(&self, account: AccountId, now: Timestamp) -> CalibrationStatus {
        CalibrationStatus {
            account,
            kind: self.kind.clone(),
            scope: self.scope.clone(),
            source: self.source,
            k: self.k,
            alpha: self
                .alpha
                .iter()
                .map(|(&family, &alpha)| FamilyWeight { family, alpha })
                .collect(),
            confidence: self.confidence,
            level: self.level(),
            n_obs: self.n_obs,
            drift: self.visible_drift(now),
            computed_at: self.computed_at,
            version: self.version,
        }
    }

    /// Same limit as `other`.
    pub fn same_limit(&self, other: &Self) -> bool {
        self.kind == other.kind && self.scope == other.scope
    }

    /// What the UI shows next to a projection.
    pub fn view(&self, now: Timestamp) -> CalibrationView {
        CalibrationView {
            confidence: self.confidence,
            level: self.level(),
            n_obs: self.n_obs,
            drift: self.visible_drift(now),
        }
    }
}

/// A family weight other than 1, for the UI and `doctor`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct FamilyWeight {
    pub family: ModelFamily,
    pub alpha: f64,
}

/// One limit's calibration as `get_calibration_status` and `CalibrationUpdated` carry it.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct CalibrationStatus {
    pub account: AccountId,
    pub kind: LimitKind,
    pub scope: Option<Scope>,
    pub source: Source,
    /// Percentage points per USD of `sum(alpha_f * cost_f)`.
    pub k: f64,
    pub alpha: Vec<FamilyWeight>,
    pub confidence: f64,
    pub level: Confidence,
    pub n_obs: u32,
    /// Only while recent enough to show.
    pub drift: Option<Drift>,
    pub computed_at: Timestamp,
    /// Row version; 0 if not stored yet.
    pub version: i64,
}

/// Calibration as the panel shows it (ADR-061).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct CalibrationView {
    pub confidence: f64,
    pub level: Confidence,
    pub n_obs: u32,
    /// Set while a recent change in `k` is being relearned.
    pub drift: Option<Drift>,
}

/// Stored next to `k` in `calibrations.alpha_json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StoredDetails {
    pub alpha: BTreeMap<ModelFamily, f64>,
    #[serde(default)]
    pub fitted: Vec<ModelFamily>,
    pub source: Source,
    pub mad_rel: f64,
    pub drift: Option<Drift>,
}

/// Which limits are calibrated and which family a scoped one counts.
fn calibrated(kind: &LimitKind, scope: Option<&Scope>) -> bool {
    match (kind, scope) {
        (LimitKind::Session | LimitKind::WeeklyAll, None) => true,
        (LimitKind::WeeklyScoped, Some(Scope::Model(_))) => scope_family(scope).is_some(),
        _ => false,
    }
}

fn scope_family(scope: Option<&Scope>) -> Option<ModelFamily> {
    match scope? {
        Scope::Model(name) => match name.to_ascii_lowercase().parse::<ModelFamily>() {
            Ok(ModelFamily::Synthetic | ModelFamily::Unknown) | Err(_) => None,
            Ok(f) => Some(f),
        },
        Scope::Surface(_) => None,
    }
}

/// The intervals of one limit from one source.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub kind: LimitKind,
    pub scope: Option<Scope>,
    pub source: Source,
    pub intervals: Vec<Interval>,
}

/// One series per calibrated limit: the statusline's when it has enough intervals (sharper
/// edges), the API's otherwise. Sources are never mixed (ADR-060 rounding differs).
pub fn series(snapshots: &[LimitObservation]) -> Vec<Series> {
    let mut by_limit: BTreeMap<(String, Option<String>), Vec<&LimitObservation>> = BTreeMap::new();
    for s in snapshots {
        if calibrated(&s.kind, s.scope.as_ref()) {
            by_limit
                .entry((
                    s.kind.as_str().to_owned(),
                    s.scope.as_ref().map(ToString::to_string),
                ))
                .or_default()
                .push(s);
        }
    }
    let mut out = Vec::new();
    for rows in by_limit.into_values() {
        let readings = |source: Source| -> Vec<Reading> {
            rows.iter()
                .filter(|s| s.source == source)
                .map(|s| Reading {
                    at: s.observed_at,
                    percent: s.percent,
                    resets_at: s.resets_at,
                })
                .collect()
        };
        let line = intervals::intervals(&readings(Source::Statusline));
        let (source, iv) = if line.len() >= MIN_SOURCE_INTERVALS {
            (Source::Statusline, line)
        } else {
            (Source::Api, intervals::intervals(&readings(Source::Api)))
        };
        if !iv.is_empty() {
            out.push(Series {
                kind: rows[0].kind.clone(),
                scope: rows[0].scope.clone(),
                source,
                intervals: iv,
            });
        }
    }
    out
}

/// Cost per family of `costs`, or `None` when unpriced models carry too much of it.
pub fn family_costs(
    costs: &[ModelCost],
    only: Option<ModelFamily>,
    unpriced: &BTreeSet<String>,
) -> Option<BTreeMap<ModelFamily, f64>> {
    let mut out: BTreeMap<ModelFamily, f64> = BTreeMap::new();
    let (mut all, mut unknown) = (0.0, 0.0);
    for c in costs {
        if only.is_some_and(|f| f != c.family) {
            continue;
        }
        #[allow(clippy::cast_precision_loss)]
        let micro = c.cost.0 as f64;
        all += micro;
        if unpriced.contains(&c.model) {
            unknown += micro;
        }
        *out.entry(c.family).or_default() += micro;
    }
    (all <= 0.0 || unknown / all <= MAX_UNPRICED_SHARE).then_some(out)
}

/// Turns a series into estimator observations.
pub fn observations(
    data: &impl CalibrationData,
    account: AccountId,
    series: &Series,
) -> Result<Vec<Obs>, StoreError> {
    let only = scope_family(series.scope.as_ref());
    let mut obs = Vec::with_capacity(series.intervals.len());
    let mut names = BTreeSet::new();
    let mut per_interval = Vec::with_capacity(series.intervals.len());
    for iv in &series.intervals {
        let costs = data.model_costs(account, iv.from, iv.to)?;
        names.extend(costs.iter().map(|c| c.model.clone()));
        per_interval.push((iv, costs));
    }
    let unpriced: BTreeSet<String> = Pricing::claude().unpriced(names).into_iter().collect();
    for (iv, costs) in per_interval {
        if let Some(cost) = family_costs(&costs, only, &unpriced) {
            obs.push(Obs {
                at: iv.to,
                dp: iv.dp,
                cost,
            });
        }
    }
    Ok(obs)
}

/// Calibrates every limit of `account` from the last [`LOOKBACK_MS`] of snapshots.
pub fn calibrate(
    data: &impl CalibrationData,
    account: AccountId,
    now: Timestamp,
) -> Result<Vec<Calibration>, StoreError> {
    let snapshots = data.snapshots(
        account,
        now.saturating_sub_millis(LOOKBACK_MS),
        now.saturating_add_millis(1),
    )?;
    let mut out = Vec::new();
    for s in series(&snapshots) {
        let fit = scope_family(s.scope.as_ref()).is_none();
        let obs = observations(data, account, &s)?;
        if let Some(e) = estimator::estimate(obs, now, fit) {
            out.push(Calibration {
                kind: s.kind,
                scope: s.scope,
                source: s.source,
                k: e.k * 1e6, // per micro-USD → per USD
                alpha: e.alpha,
                fitted: e.fitted,
                confidence: e.confidence,
                n_obs: e.n_obs,
                mad_rel: e.mad_rel,
                drift: e.drift,
                computed_at: now,
                version: 0,
            });
        }
    }
    Ok(out)
}

/// Whether `new` deserves a row: none yet, `k` moved by [`STORE_MIN_CHANGE`], the confidence
/// level, the family weights or the drift changed, or the newest row is a day old.
pub fn should_store(prev: Option<&Calibration>, new: &Calibration) -> bool {
    let Some(prev) = prev else {
        return true;
    };
    let moved = prev.k <= 0.0 || ((new.k - prev.k) / prev.k).abs() >= STORE_MIN_CHANGE;
    let alpha_moved = prev.alpha.keys().ne(new.alpha.keys())
        || prev.alpha.iter().any(|(f, a)| {
            new.alpha
                .get(f)
                .is_none_or(|b| ((b - a) / a).abs() >= STORE_MIN_CHANGE)
        });
    moved
        || alpha_moved
        || prev.level() != new.level()
        || prev.drift != new.drift
        || new.computed_at.as_millis() - prev.computed_at.as_millis() >= STORE_MAX_AGE_MS
}
