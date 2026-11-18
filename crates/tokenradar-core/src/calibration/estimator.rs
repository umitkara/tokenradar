//! Robust estimate of `k` (percent per dollar) and the family weights `α_f` (Step 13, ADR-061).
//!
//! Model: `Δp ≈ k · Σ_f α_f · ΔC_f`, with `α_opus = 1`.
//!
//! - **`k`:** time-decayed, cost-weighted median of the ratios `r_i = Δp_i / Σ α ΔC_i`.
//!   Weight `w_i = ΔC_i · 2^(−age_i / half-life)` (raw cost, independent of α), half-life
//!   3 days. The median shrugs off
//!   intervals with usage that is not local (claude.ai, another device).
//! - **`α_f`:** the value that makes the ratios agree best: coordinate search on a log grid
//!   minimising the weighted relative MAD. Only for a family with enough intervals where it
//!   carries at least 30 % of the cost, and enough where it does not; otherwise `α = 1`. A fit
//!   within ±15 % of 1 is snapped to 1: H1 holds for that family.
//! - **Confidence** in `[0, 1]`: count × precision × freshness × fitted share (see
//!   [`confidence`]).
//! - **Drift:** two-sided CUSUM on robust z-scores of `ln(r_i / k_before_i)`, where
//!   `k_before_i` is learned from the intervals before `i` only. Scores are clipped to ±3, so
//!   an alarm needs several agreeing intervals, never a single outlier. After an alarm the estimate
//!   uses the intervals since the change with a one-day half-life, so it relearns fast. The
//!   family weights stay those of all intervals: a few intervals cannot pin them down, and a
//!   change of the limit's size moves every family alike.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::{ModelFamily, Timestamp};
use crate::limits::Confidence;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
/// Half-life of an interval's weight.
pub const HALF_LIFE_MS: i64 = 3 * DAY_MS;
/// Half-life right after a drift alarm.
pub const DRIFT_HALF_LIFE_MS: i64 = DAY_MS;
/// Intervals below this share of the median cost are noise (a single tiny request).
pub const NOISE_FLOOR_SHARE: f64 = 0.05;
/// Intervals a family needs, both where it carries the cost and where it does not, for its α.
pub const MIN_FAMILY_OBS: usize = 5;
/// A fitted α this close to 1 counts as 1 (H1 holds for the family).
pub const ALPHA_SNAP: f64 = 0.15;
/// Usable intervals for full count confidence (the plan's "≥ 30 intervals").
pub const FULL_COUNT: f64 = 30.0;
/// A drift alarm stays visible this long.
pub const DRIFT_VISIBLE_MS: i64 = 3 * DAY_MS;
/// CUSUM allowance and alarm threshold, in robust standard deviations.
const CUSUM_SLACK: f64 = 0.5;
const CUSUM_ALARM: f64 = 6.0;
/// z-scores are clipped to this, so no single outlier can raise an alarm on its own.
const CUSUM_CLIP: f64 = 3.0;
/// Floor of the robust spread of `ln r`. Below it, ordinary swings (an hour of claude.ai use)
/// would look like a change; a change worth reporting is far larger than ±15 %.
const CUSUM_MIN_SIGMA: f64 = 0.15;
/// A family "carries" an interval from this cost share on (for α fitting).
const CARRIES_SHARE: f64 = 0.3;
/// Intervals before the first CUSUM step.
const CUSUM_WARMUP: usize = 10;

/// The reference family: `α = 1` by definition.
pub const REFERENCE: ModelFamily = ModelFamily::Opus;

/// One usable interval: `dp` points for this cost per family (micro-USD).
#[derive(Debug, Clone, PartialEq)]
pub struct Obs {
    pub at: Timestamp,
    pub dp: f64,
    pub cost: BTreeMap<ModelFamily, f64>,
}

impl Obs {
    fn total(&self) -> f64 {
        self.cost.values().sum()
    }

    fn weighted(&self, alpha: &BTreeMap<ModelFamily, f64>) -> f64 {
        self.cost
            .iter()
            .map(|(f, c)| c * alpha.get(f).copied().unwrap_or(1.0))
            .sum()
    }

    fn share(&self, family: ModelFamily) -> f64 {
        let total = self.total();
        if total <= 0.0 {
            0.0
        } else {
            self.cost.get(&family).copied().unwrap_or(0.0) / total
        }
    }
}

/// The direction of a detected change in `k`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum DriftDirection {
    /// One dollar now costs more of the limit.
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
pub struct Drift {
    /// When the CUSUM crossed its threshold.
    pub detected_at: Timestamp,
    /// Where the change most likely began.
    pub since: Timestamp,
    pub direction: DriftDirection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Estimate {
    /// Percentage points per USD of `Σ α ΔC`.
    pub k: f64,
    /// Fitted family weights other than 1.
    pub alpha: BTreeMap<ModelFamily, f64>,
    /// Families with enough intervals for their own weight (which may have snapped to 1).
    pub fitted: Vec<ModelFamily>,
    pub confidence: f64,
    pub n_obs: u32,
    /// Weighted MAD of the ratios over `k`.
    pub mad_rel: f64,
    /// The latest drift alarm, if any.
    pub drift: Option<Drift>,
}

/// Maps a confidence score to the three levels the UI shows.
pub fn level(confidence: f64) -> Confidence {
    if confidence > 0.7 {
        Confidence::High
    } else if confidence >= 0.4 {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

fn decay(now: Timestamp, at: Timestamp, half_life_ms: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let age = (now.as_millis() - at.as_millis()).max(0) as f64;
    #[allow(clippy::cast_precision_loss)]
    let half = half_life_ms as f64;
    (-age / half * std::f64::consts::LN_2).exp()
}

/// Weighted median of `(value, weight)`; `None` without positive weight.
pub fn weighted_median(pairs: &mut [(f64, f64)]) -> Option<f64> {
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let total: f64 = pairs.iter().map(|p| p.1).sum();
    if total <= 0.0 {
        return None;
    }
    let mut acc = 0.0;
    for &(v, w) in pairs.iter() {
        acc += w;
        if acc >= total / 2.0 {
            return Some(v);
        }
    }
    pairs.last().map(|p| p.0)
}

/// Weighted median of the ratios, `MAD / median`, and the `(ratio, weight)` pairs.
type RatioStats = (f64, f64, Vec<(f64, f64)>);

/// `(median, MAD / median)` of the ratios under `alpha`, weighted by raw cost and decay.
fn ratio_stats(
    obs: &[&Obs],
    alpha: &BTreeMap<ModelFamily, f64>,
    now: Timestamp,
    half_life_ms: i64,
) -> Option<RatioStats> {
    let mut pairs: Vec<(f64, f64)> = obs
        .iter()
        .filter_map(|o| {
            // The weight must not depend on α: otherwise a small α shrinks the family's
            // intervals out of the median and "improves" the fit by ignoring them.
            let c = o.weighted(alpha);
            (c > 0.0).then(|| (o.dp / c, o.total() * decay(now, o.at, half_life_ms)))
        })
        .collect();
    let k = weighted_median(&mut pairs)?;
    if k <= 0.0 {
        return None;
    }
    let mut dev: Vec<(f64, f64)> = pairs.iter().map(|&(r, w)| ((r - k).abs(), w)).collect();
    let mad = weighted_median(&mut dev).unwrap_or(0.0);
    Some((k, mad / k, pairs))
}

/// Grid of candidate α values: 2^(i/10) for i in −20..=30 (0.25 … 8).
fn alpha_grid() -> impl Iterator<Item = f64> {
    (-20..=30).map(|i| 2f64.powf(f64::from(i) / 10.0))
}

/// Whether a family gets its own α: every one but the reference and the placeholders.
pub fn needs_own_weight(family: ModelFamily) -> bool {
    !matches!(
        family,
        ModelFamily::Opus | ModelFamily::Synthetic | ModelFamily::Unknown
    )
}

/// Families that can get their own α from these intervals.
fn fittable(obs: &[&Obs]) -> Vec<ModelFamily> {
    let mut families: Vec<ModelFamily> = obs
        .iter()
        .flat_map(|o| o.cost.keys().copied())
        .filter(|f| needs_own_weight(*f))
        .collect();
    families.sort();
    families.dedup();
    families
        .into_iter()
        .filter(|&f| {
            let carried = obs.iter().filter(|o| o.share(f) >= CARRIES_SHARE).count();
            carried >= MIN_FAMILY_OBS && obs.len() - carried >= MIN_FAMILY_OBS
        })
        .collect()
}

fn fit_alpha(
    obs: &[&Obs],
    now: Timestamp,
    half_life_ms: i64,
) -> (BTreeMap<ModelFamily, f64>, Vec<ModelFamily>) {
    let families = fittable(obs);
    let mut alpha: BTreeMap<ModelFamily, f64> = families.iter().map(|&f| (f, 1.0)).collect();
    for _round in 0..2 {
        for &f in &families {
            let mut best: Option<(f64, f64)> = None;
            for a in alpha_grid() {
                alpha.insert(f, a);
                if let Some((_, mad, _)) = ratio_stats(obs, &alpha, now, half_life_ms)
                    && best.is_none_or(|(m, _)| mad < m)
                {
                    best = Some((mad, a));
                }
            }
            alpha.insert(f, best.map_or(1.0, |b| b.1));
        }
    }
    alpha.retain(|_, a| (*a - 1.0).abs() > ALPHA_SNAP);
    (alpha, families)
}

/// Confidence in `[0, 1]`, the product of four factors:
///
/// - **count:** `min(1, n / 30)`.
/// - **precision:** `1 − se / 0.15`, clamped, with `se = 1.4826 · MAD_rel / √n_eff` (the
///   relative standard error of the median under a normal approximation) and
///   `n_eff = (Σw)² / Σw²`.
/// - **freshness:** 1 while the newest interval is under a day old, then halves every three
///   days.
/// - **fitted share:** one minus the recent cost share of families that would need their own α
///   but have too few intervals for one. A limit used mostly with such a family is uncertain
///   however well the rest agrees.
pub fn confidence(n: usize, mad_rel: f64, n_eff: f64, newest_age_ms: i64, unfitted: f64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let count = (n as f64 / FULL_COUNT).min(1.0);
    let se = 1.4826 * mad_rel / n_eff.max(1.0).sqrt();
    let precision = (1.0 - se / 0.15).clamp(0.0, 1.0);
    #[allow(clippy::cast_precision_loss)]
    let stale_days = (newest_age_ms - DAY_MS).max(0) as f64 / DAY_MS as f64;
    let freshness = 0.5f64.powf(stale_days / 3.0);
    let fitted = (1.0 - unfitted).clamp(0.0, 1.0);
    (count * precision * freshness * fitted).clamp(0.0, 1.0)
}

/// Last CUSUM alarm over `obs` (sorted by time) under `alpha`.
fn cusum(obs: &[&Obs], alpha: &BTreeMap<ModelFamily, f64>) -> Option<Drift> {
    let logs: Vec<f64> = obs
        .iter()
        .filter_map(|o| {
            let c = o.weighted(alpha);
            (c > 0.0).then(|| (o.dp / c).ln())
        })
        .collect();
    if logs.len() <= CUSUM_WARMUP {
        return None;
    }
    let mut sorted = logs.clone();
    sorted.sort_by(f64::total_cmp);
    let med = sorted[sorted.len() / 2];
    let mut dev: Vec<f64> = logs.iter().map(|l| (l - med).abs()).collect();
    dev.sort_by(f64::total_cmp);
    let sigma = (1.4826 * dev[dev.len() / 2]).max(CUSUM_MIN_SIGMA);

    let (mut up, mut down) = (0.0f64, 0.0f64);
    let (mut up_start, mut down_start) = (0usize, 0usize);
    let mut last = None;
    let mut base = 0usize;
    for i in CUSUM_WARMUP..obs.len() {
        if i - base < 3 {
            continue;
        }
        let Some((k_before, _, _)) = ratio_stats(&obs[base..i], alpha, obs[i].at, HALF_LIFE_MS)
        else {
            continue;
        };
        let c = obs[i].weighted(alpha);
        if c <= 0.0 {
            continue;
        }
        let z = (((obs[i].dp / c) / k_before).ln() / sigma).clamp(-CUSUM_CLIP, CUSUM_CLIP);
        if up == 0.0 {
            up_start = i;
        }
        if down == 0.0 {
            down_start = i;
        }
        up = (up + z - CUSUM_SLACK).max(0.0);
        down = (down - z - CUSUM_SLACK).max(0.0);
        let alarm = if up > CUSUM_ALARM {
            Some((DriftDirection::Up, up_start))
        } else if down > CUSUM_ALARM {
            Some((DriftDirection::Down, down_start))
        } else {
            None
        };
        if let Some((direction, start)) = alarm {
            last = Some(Drift {
                detected_at: obs[i].at,
                since: obs[start].at,
                direction,
            });
            // Learn the new level from the change on.
            base = start;
            up = 0.0;
            down = 0.0;
        }
    }
    last
}

/// Estimates `k` from `obs` (any order). `fit_families` enables α fitting (off for a limit
/// that counts a single family). `None` without a usable interval.
pub fn estimate(mut obs: Vec<Obs>, now: Timestamp, fit_families: bool) -> Option<Estimate> {
    obs.retain(|o| o.dp > 0.0 && o.total() > 0.0 && o.at <= now);
    obs.sort_by_key(|o| o.at);
    let mut totals: Vec<f64> = obs.iter().map(Obs::total).collect();
    if totals.is_empty() {
        return None;
    }
    totals.sort_by(f64::total_cmp);
    let floor = totals[totals.len() / 2] * NOISE_FLOOR_SHARE;
    let usable: Vec<&Obs> = obs.iter().filter(|o| o.total() >= floor).collect();
    if usable.is_empty() {
        return None;
    }

    let (alpha, fitted) = if fit_families {
        fit_alpha(&usable, now, HALF_LIFE_MS)
    } else {
        (BTreeMap::new(), Vec::new())
    };
    let drift = cusum(&usable, &alpha);

    // After a drift, relearn from the change on with a short memory.
    let (basis, half_life): (Vec<&Obs>, i64) = match drift {
        Some(d) if now.as_millis() - d.detected_at.as_millis() <= DRIFT_VISIBLE_MS => {
            let since: Vec<&Obs> = usable.iter().copied().filter(|o| o.at >= d.since).collect();
            if since.len() >= 3 {
                (since, DRIFT_HALF_LIFE_MS)
            } else {
                (usable.clone(), HALF_LIFE_MS)
            }
        }
        _ => (usable.clone(), HALF_LIFE_MS),
    };
    let (k, mad_rel, pairs) = ratio_stats(&basis, &alpha, now, half_life)?;

    let sw: f64 = pairs.iter().map(|p| p.1).sum();
    let sw2: f64 = pairs.iter().map(|p| p.1 * p.1).sum();
    let n_eff = if sw2 > 0.0 { sw * sw / sw2 } else { 0.0 };
    let newest_age = now.as_millis() - basis.last().map_or(0, |o| o.at.as_millis());

    // Recent cost share of families that would need an α but have none.
    let unfitted = if fit_families {
        let mut need = 0.0;
        let mut all = 0.0;
        for o in &basis {
            let w = decay(now, o.at, half_life);
            for (f, c) in &o.cost {
                all += c * w;
                if needs_own_weight(*f) && !fitted.contains(f) {
                    need += c * w;
                }
            }
        }
        if all > 0.0 { need / all } else { 0.0 }
    } else {
        0.0
    };

    Some(Estimate {
        k,
        alpha,
        fitted,
        confidence: confidence(basis.len(), mad_rel, n_eff, newest_age, unfitted),
        n_obs: u32::try_from(basis.len()).unwrap_or(u32::MAX),
        mad_rel,
        drift,
    })
}
