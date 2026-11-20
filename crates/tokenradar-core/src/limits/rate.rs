//! Burn rate and projection (architecture §7).
//!
//! - **Integer sources (Step 12t, ADR-062):** the statusline and the usage API report whole
//!   percentages, the API only every ~15 minutes. A slope over a short lookback then sees at
//!   most one step and overstates the rate several times. The rate comes from the **steps**
//!   instead: the instants where a source's running maximum steps up ("edges", the same idea as
//!   the calibration's intervals). See [`Steps`].
//! - **Fractional sources:** the Theil–Sen slope (median of pairwise slopes) of the current
//!   window's observations from the last 30 minutes, at least three of them. Robust against a
//!   single outlier.
//! - **Fallback:** `k0 × cost rate of the last 15 minutes`, with the prior
//!   `k0 = p_now / C_window` (architecture §9.2): the share of the window one dollar of local
//!   usage has cost so far. Always low confidence.
//! - **Calibrated fallback (Step 13, ADR-061):** where a calibration applies with at least
//!   medium confidence to the last 15 minutes' usage, `k × Σ α_f · cost_f` replaces `k0`. Still
//!   low confidence: extrapolating 15 minutes of usage over the rest of the window stays rough
//!   (user decision).

use serde::Serialize;
use specta::Type;

use crate::calibration::intervals::{Edge, MAX_EDGE_UNCERTAINTY_MS};
use crate::domain::Timestamp;

/// Observations older than this do not shape the slope.
pub const RATE_LOOKBACK_MS: i64 = 30 * 60 * 1000;
/// Cost window of the fallback.
pub const FALLBACK_LOOKBACK_MS: i64 = 15 * 60 * 1000;
pub const MIN_POINTS: usize = 3;
/// Below this (%/min) usage counts as stopped: no ETA.
const RATE_EPSILON: f64 = 1e-4;
/// Edges the step rate uses at most: the newest ones of the window.
pub const MAX_EDGES: usize = 4;
/// An edge rate drops one confidence level when its end edges' mean timing uncertainty exceeds
/// this share of the span between them.
const MAX_UNCERTAINTY_SHARE: f64 = 0.25;
/// Slack for float comparisons of percentages.
const PERCENT_EPS: f64 = 1e-9;

/// Ordered: `Low < Medium < High`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RateMethod {
    /// Slope of observed percentages.
    Observed,
    /// Points per minute between the edges of an integer source (Step 12t, ADR-062).
    Steps,
    /// Local cost rate times the window's percent-per-dollar.
    CostRate,
    /// Local cost rate times the learned percent-per-dollar (Step 13, ADR-061).
    Calibrated,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct Projection {
    /// Percentage points per minute, never negative.
    pub rate_per_min: f64,
    /// When 100 % is reached at this rate; `None` when usage has stopped.
    pub eta_100_at: Option<Timestamp>,
    pub will_hit_before_reset: bool,
    /// Percentage at `resets_at` at this rate; `None` without a reset time.
    pub projected_at_reset: Option<f64>,
    pub confidence: Confidence,
    pub method: RateMethod,
}

/// Theil–Sen slope in %/min, or `None` with fewer than [`MIN_POINTS`] distinct instants.
pub fn theil_sen(points: &[(Timestamp, f64)]) -> Option<f64> {
    if points.len() < MIN_POINTS {
        return None;
    }
    let mut slopes = Vec::with_capacity(points.len() * (points.len() - 1) / 2);
    for (i, &(ti, pi)) in points.iter().enumerate() {
        for &(tj, pj) in &points[i + 1..] {
            let dt = tj.as_millis() - ti.as_millis();
            if dt != 0 {
                #[allow(clippy::cast_precision_loss)]
                let minutes = dt as f64 / 60_000.0;
                slopes.push((pj - pi) / minutes);
            }
        }
    }
    if slopes.len() < MIN_POINTS - 1 {
        return None;
    }
    slopes.sort_by(f64::total_cmp);
    let mid = slopes.len() / 2;
    Some(if slopes.len() % 2 == 0 {
        f64::midpoint(slopes[mid - 1], slopes[mid])
    } else {
        slopes[mid]
    })
}

/// Cost-based fallback rate in %/min: `k0 × cost of the last 15 min / 15`.
pub fn cost_rate(p_now: f64, window_cost: i64, recent_cost: i64) -> Option<f64> {
    if window_cost <= 0 || p_now <= 0.0 {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let k0 = p_now / window_cost as f64;
    #[allow(clippy::cast_precision_loss)]
    let per_min = recent_cost.max(0) as f64 / (FALLBACK_LOOKBACK_MS as f64 / 60_000.0);
    Some(k0 * per_min)
}

/// Builds the projection from a rate.
pub fn project(
    now: Timestamp,
    p_now: f64,
    rate_per_min: f64,
    resets_at: Option<Timestamp>,
    confidence: Confidence,
    method: RateMethod,
) -> Projection {
    let rate = rate_per_min.max(0.0);
    let eta_100_at = if p_now >= 100.0 {
        Some(now)
    } else if rate > RATE_EPSILON {
        #[allow(clippy::cast_possible_truncation)]
        let ms = ((100.0 - p_now) / rate * 60_000.0).min(1e15) as i64;
        Some(now.saturating_add_millis(ms))
    } else {
        None
    };
    let remaining_min = resets_at.map(|r| {
        #[allow(clippy::cast_precision_loss)]
        let ms = (r.as_millis() - now.as_millis()).max(0) as f64;
        ms / 60_000.0
    });
    let projected_at_reset = remaining_min.map(|m| p_now + rate * m);
    let will_hit_before_reset = match (eta_100_at, resets_at) {
        (Some(eta), Some(reset)) => eta < reset,
        _ => false,
    };
    Projection {
        rate_per_min: rate,
        eta_100_at,
        will_hit_before_reset,
        projected_at_reset,
        confidence,
        method,
    }
}

/// The projection for one limit. In order: the Theil–Sen slope of fractional readings, an edge
/// rate of at least medium confidence, the calibrated fallback, a low-confidence edge rate,
/// the `k0` prior. `None` when none works.
pub fn projection(
    now: Timestamp,
    p_now: f64,
    resets_at: Option<Timestamp>,
    points: &[(Timestamp, f64)],
    steps: Option<StepRate>,
    costs: Option<Fallback>,
) -> Option<Projection> {
    let at = |rate: f64, confidence: Confidence, method: RateMethod| {
        project(now, p_now, rate, resets_at, confidence, method)
    };
    let recent: Vec<_> = points
        .iter()
        .copied()
        .filter(|(t, _)| now.as_millis() - t.as_millis() <= RATE_LOOKBACK_MS)
        .collect();
    if let Some(slope) = theil_sen(&recent) {
        let confidence = if recent.len() >= 5 {
            Confidence::High
        } else {
            Confidence::Medium
        };
        return Some(at(slope, confidence, RateMethod::Observed));
    }
    if let Some(s) = steps.filter(|s| s.confidence != Confidence::Low) {
        return Some(at(s.per_min, s.confidence, RateMethod::Steps));
    }
    match (costs, steps) {
        (Some(Fallback::Calibrated { pct_per_min }), _) => {
            Some(at(pct_per_min, Confidence::Low, RateMethod::Calibrated))
        }
        (_, Some(s)) => Some(at(s.per_min, s.confidence, RateMethod::Steps)),
        (
            Some(Fallback::Prior {
                window_cost,
                recent_cost,
            }),
            None,
        ) => Some(at(
            cost_rate(p_now, window_cost, recent_cost)?,
            Confidence::Low,
            RateMethod::CostRate,
        )),
        (None, None) => None,
    }
}

/// A rate read off the edges of one integer source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepRate {
    /// Percentage points per minute, never negative.
    pub per_min: f64,
    pub confidence: Confidence,
    /// Sharp edges the rate rests on; picks the better of two sources.
    pub edges: usize,
    /// Time of the newest edge; breaks ties between sources.
    pub last_edge: Timestamp,
}

/// The newest sharp edges of one integer source in the current window (Step 12t, ADR-062).
///
/// - **Running maximum:** a lower reading (the stale payload of an idle session) says nothing;
///   an equal one moves `seen_at`, the last time the old value was still shown.
/// - **Edge:** the maximum stepped up within `(seen_at, at]`. Edges known less precisely than
///   [`MAX_EDGE_UNCERTAINTY_MS`] (app closed, a 401 wait) are not kept; the kept neighbours
///   span them with a larger point difference.
/// - **The first reading is not an edge:** recording may start mid-window.
///
/// Memory stays at [`MAX_EDGES`] edges however long the window is.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Steps {
    max: f64,
    /// Last reading that showed `max`; `None` before the first reading.
    seen_at: Option<Timestamp>,
    edges: Vec<Edge>,
}

impl Steps {
    /// Takes one whole-percent reading. Readings older than the last one are ignored.
    pub fn observe(&mut self, at: Timestamp, percent: f64) {
        let Some(seen) = self.seen_at else {
            self.max = percent;
            self.seen_at = Some(at);
            return;
        };
        if at < seen {
            return;
        }
        if percent > self.max + PERCENT_EPS {
            let edge = Edge {
                before: seen,
                after: at,
                percent,
            };
            if edge.uncertainty_ms() <= MAX_EDGE_UNCERTAINTY_MS {
                if self.edges.len() == MAX_EDGES {
                    self.edges.remove(0);
                }
                self.edges.push(edge);
            }
            self.max = percent;
            self.seen_at = Some(at);
        } else if percent >= self.max - PERCENT_EPS {
            self.seen_at = Some(at);
        }
    }

    /// The edges kept, oldest first.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// `(last − first edge's points) / time between them`, capped once usage slows down; `None`
    /// with fewer than two edges.
    ///
    /// - **Stop cap:** the newest reading still shows the last edge's value, so less than one
    ///   point was used since that edge: `rate ≤ 1 / (seen_at − last edge)`. A projection fades
    ///   by itself once usage stops.
    /// - **Confidence:** one interval low, two medium, three high; one level lower when the end
    ///   edges' mean timing uncertainty exceeds a quarter of the span.
    pub fn rate(&self) -> Option<StepRate> {
        let (first, last) = (self.edges.first()?, self.edges.last()?);
        let span = last.at().as_millis() - first.at().as_millis();
        let dp = last.percent - first.percent;
        if self.edges.len() < 2 || span <= 0 || dp <= 0.0 {
            return None;
        }
        #[allow(clippy::cast_precision_loss)]
        let mut per_min = dp / (span as f64 / 60_000.0);
        let since = self.seen_at?.as_millis() - last.at().as_millis();
        if since > 0 {
            #[allow(clippy::cast_precision_loss)]
            let cap = 1.0 / (since as f64 / 60_000.0);
            per_min = per_min.min(cap);
        }
        let mut confidence = match self.edges.len() - 1 {
            1 => Confidence::Low,
            2 => Confidence::Medium,
            _ => Confidence::High,
        };
        #[allow(clippy::cast_precision_loss)]
        let blur = i64::midpoint(first.uncertainty_ms(), last.uncertainty_ms()) as f64;
        #[allow(clippy::cast_precision_loss)]
        let limit = MAX_UNCERTAINTY_SHARE * span as f64;
        if blur > limit {
            confidence = confidence.lower();
        }
        Some(StepRate {
            per_min,
            confidence,
            edges: self.edges.len(),
            last_edge: last.at(),
        })
    }
}

impl Confidence {
    /// One level down; `Low` stays.
    #[must_use]
    pub fn lower(self) -> Self {
        match self {
            Self::High => Self::Medium,
            Self::Medium | Self::Low => Self::Low,
        }
    }
}

/// The better of several sources' edge rates: more edges, then the newest edge.
pub fn best_step_rate(rates: impl IntoIterator<Item = StepRate>) -> Option<StepRate> {
    rates.into_iter().max_by_key(|r| (r.edges, r.last_edge))
}

/// Inputs of the cost-based fallback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fallback {
    /// `k0 = p_now / window_cost` times the cost of the last [`FALLBACK_LOOKBACK_MS`].
    Prior { window_cost: i64, recent_cost: i64 },
    /// Already a rate: the calibrated `k` times the weighted recent cost.
    Calibrated { pct_per_min: f64 },
}

/// `k × Σ α · cost` of the last [`FALLBACK_LOOKBACK_MS`], in %/min. `weighted_micro_usd` is the
/// weighted cost in micro-dollars, `k` in percent per dollar.
pub fn calibrated_rate(k: f64, weighted_micro_usd: f64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let minutes = FALLBACK_LOOKBACK_MS as f64 / 60_000.0;
    (k * weighted_micro_usd / 1e6 / minutes).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;

    fn t(min: f64) -> Timestamp {
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        Timestamp::from_millis(1_793_000_000_000 + (min * MIN as f64) as i64)
    }

    #[test]
    fn theil_sen_recovers_a_known_slope_despite_noise_and_an_outlier() {
        // 0.5 %/min with ±0.2 noise, sampled every 2 min, one wild outlier.
        let noise = [0.1, -0.2, 0.15, 0.0, -0.1, 0.2, -0.15, 0.05, -0.05, 0.1];
        let mut points: Vec<_> = noise
            .iter()
            .enumerate()
            .map(|(i, n)| {
                #[allow(clippy::cast_precision_loss)]
                let m = i as f64 * 2.0;
                (t(m), 10.0 + 0.5 * m + n)
            })
            .collect();
        points[4].1 = 90.0;
        let slope = theil_sen(&points).expect("slope");
        assert!((slope - 0.5).abs() < 0.05, "slope {slope}");
    }

    #[test]
    fn theil_sen_handles_integer_steps() {
        // The API's whole percentages: 20, 20, 21, 21, 22 over 8 minutes ≈ 0.25 %/min.
        let points = [
            (t(0.0), 20.0),
            (t(2.0), 20.0),
            (t(4.0), 21.0),
            (t(6.0), 21.0),
            (t(8.0), 22.0),
        ];
        let slope = theil_sen(&points).expect("slope");
        assert!((slope - 0.25).abs() < 0.01, "slope {slope}");
    }

    #[test]
    fn theil_sen_needs_three_points() {
        assert_eq!(theil_sen(&[(t(0.0), 1.0), (t(1.0), 2.0)]), None);
        // Same instant three times: no slope.
        assert_eq!(
            theil_sen(&[(t(0.0), 1.0), (t(0.0), 2.0), (t(0.0), 3.0)]),
            None
        );
    }

    #[test]
    fn projection_reports_eta_and_whether_the_reset_comes_first() {
        let now = t(0.0);
        // 60 % at 1 %/min: full in 40 min, reset in 60 min.
        let p = project(
            now,
            60.0,
            1.0,
            Some(t(60.0)),
            Confidence::High,
            RateMethod::Observed,
        );
        assert_eq!(p.eta_100_at, Some(t(40.0)));
        assert!(p.will_hit_before_reset);
        assert!((p.projected_at_reset.expect("reset") - 120.0).abs() < 1e-9);
        // Reset first.
        let p = project(
            now,
            60.0,
            0.5,
            Some(t(60.0)),
            Confidence::High,
            RateMethod::Observed,
        );
        assert!(!p.will_hit_before_reset);
        assert!((p.projected_at_reset.expect("reset") - 90.0).abs() < 1e-9);
        // Stopped: no ETA, a negative slope is clamped.
        let p = project(
            now,
            60.0,
            -0.3,
            Some(t(60.0)),
            Confidence::Low,
            RateMethod::Observed,
        );
        assert_eq!((p.rate_per_min, p.eta_100_at), (0.0, None));
        // Already full.
        let p = project(now, 100.0, 0.0, None, Confidence::Low, RateMethod::Observed);
        assert_eq!(p.eta_100_at, Some(now));
    }

    #[test]
    fn falls_back_to_the_cost_rate_with_low_confidence() {
        let now = t(30.0);
        // 40 % for $10 so far; $1.5 in the last 15 min → 0.4 %/min.
        let p = projection(
            now,
            40.0,
            Some(t(120.0)),
            &[(t(29.0), 40.0)],
            None,
            Some(Fallback::Prior {
                window_cost: 10_000_000,
                recent_cost: 1_500_000,
            }),
        )
        .expect("projection");
        assert_eq!(p.method, RateMethod::CostRate);
        assert_eq!(p.confidence, Confidence::Low);
        assert!((p.rate_per_min - 0.4).abs() < 1e-9);
        // No cost in the window: nothing to scale.
        let none = Fallback::Prior {
            window_cost: 0,
            recent_cost: 0,
        };
        assert_eq!(projection(now, 40.0, None, &[], None, Some(none)), None);
        assert_eq!(projection(now, 40.0, None, &[], None, None), None);
    }

    #[test]
    fn a_calibrated_fallback_keeps_low_confidence_and_says_so() {
        let now = t(30.0);
        // 0.05 %/$ × $3 weighted in the last 15 min = 0.01 %/min.
        let rate = calibrated_rate(0.05, 3_000_000.0);
        assert!((rate - 0.01).abs() < 1e-12);
        let p = projection(
            now,
            40.0,
            Some(t(120.0)),
            &[],
            None,
            Some(Fallback::Calibrated { pct_per_min: rate }),
        )
        .expect("projection");
        assert_eq!(p.method, RateMethod::Calibrated);
        assert_eq!(p.confidence, Confidence::Low);
        // The observed slope still wins when there are enough points.
        let points = [(t(10.0), 30.0), (t(20.0), 35.0), (t(29.0), 40.0)];
        let p = projection(
            now,
            40.0,
            None,
            &points,
            None,
            Some(Fallback::Calibrated { pct_per_min: rate }),
        )
        .expect("projection");
        assert_eq!(p.method, RateMethod::Observed);
    }

    #[test]
    fn old_points_are_ignored() {
        let now = t(60.0);
        let points = [
            (t(0.0), 1.0),
            (t(10.0), 2.0),
            (t(55.0), 50.0),
            (t(58.0), 51.0),
        ];
        // Only two points inside 30 min: no observed slope.
        assert_eq!(projection(now, 51.0, None, &points, None, None), None);
    }

    // --- Step edges (Step 12t) -------------------------------------------------------

    fn steps_of(readings: &[(f64, f64)]) -> Steps {
        let mut s = Steps::default();
        for &(m, p) in readings {
            s.observe(t(m), p);
        }
        s
    }

    fn step_rate(per_min: f64, confidence: Confidence) -> StepRate {
        StepRate {
            per_min,
            confidence,
            edges: 3,
            last_edge: t(0.0),
        }
    }

    #[test]
    fn no_edge_or_a_single_edge_gives_no_rate() {
        assert_eq!(Steps::default().rate(), None);
        // Flat: no edge.
        assert_eq!(
            steps_of(&[(0.0, 20.0), (15.0, 20.0), (30.0, 20.0)]).rate(),
            None
        );
        // The first reading is not an edge, so 20 → 21 is one edge: no interval yet.
        let s = steps_of(&[(0.0, 20.0), (15.0, 20.0), (30.0, 21.0)]);
        assert_eq!(s.edges().len(), 1);
        assert_eq!(s.rate(), None);
    }

    #[test]
    fn edges_sit_between_the_readings_around_a_step() {
        let s = steps_of(&[(0.0, 20.0), (10.0, 20.0), (20.0, 21.0), (30.0, 21.0)]);
        let e = s.edges()[0];
        assert_eq!((e.before, e.after, e.percent), (t(10.0), t(20.0), 21.0));
        assert_eq!(e.at(), t(15.0));
    }

    #[test]
    fn a_steady_rate_is_read_off_the_edges() {
        // 1 point per hour, readings every 15 min: edges at 52.5, 112.5 and 172.5 min.
        let s = steps_of(&[
            (0.0, 20.0),
            (45.0, 20.0),
            (60.0, 21.0),
            (105.0, 21.0),
            (120.0, 22.0),
            (165.0, 22.0),
            (180.0, 23.0),
        ]);
        let r = s.rate().expect("rate");
        assert!((r.per_min - 1.0 / 60.0).abs() < 1e-12, "{}", r.per_min);
        // Two intervals; 15 min blur against 120 min of span stays sharp.
        assert_eq!(r.confidence, Confidence::Medium);
        assert_eq!(r.edges, 3);
    }

    #[test]
    fn confidence_grows_with_intervals_and_drops_with_blur() {
        let edges = |gap: f64, n: u32| {
            let mut readings = vec![(0.0, 0.0)];
            for i in 1..=n {
                let m = f64::from(i) * 60.0;
                readings.push((m - gap, f64::from(i - 1)));
                readings.push((m, f64::from(i)));
            }
            steps_of(&readings).rate().expect("rate").confidence
        };
        assert_eq!(edges(5.0, 2), Confidence::Low);
        assert_eq!(edges(5.0, 3), Confidence::Medium);
        assert_eq!(edges(5.0, 4), Confidence::High);
        // A step at every reading 40 min apart: 40 min of blur against 120 min of span is
        // more than a quarter, so three intervals read medium, not high.
        let blurred = steps_of(&[
            (0.0, 0.0),
            (40.0, 1.0),
            (80.0, 2.0),
            (120.0, 3.0),
            (160.0, 4.0),
        ]);
        let r = blurred.rate().expect("rate");
        assert_eq!(r.edges, 4);
        assert_eq!(r.confidence, Confidence::Medium);
    }

    #[test]
    fn only_the_newest_edges_count_so_a_new_pace_shows_quickly() {
        // 1 point per hour for five hours, then 1 point per 10 minutes.
        let mut readings = Vec::new();
        for h in 0..=5 {
            #[allow(clippy::cast_precision_loss)]
            let m = f64::from(h) * 60.0;
            readings.push((m, f64::from(h)));
            readings.push((m + 55.0, f64::from(h)));
        }
        for i in 1..=4 {
            let m = 360.0 + f64::from(i) * 10.0;
            readings.push((m - 5.0, 5.0 + f64::from(i - 1)));
            readings.push((m, 5.0 + f64::from(i)));
        }
        let s = steps_of(&readings);
        assert_eq!(s.edges().len(), MAX_EDGES);
        let r = s.rate().expect("rate");
        assert!((r.per_min - 0.1).abs() < 1e-9, "{}", r.per_min);
    }

    #[test]
    fn stopped_usage_caps_the_rate_until_the_projection_fades() {
        let mut s = steps_of(&[
            (0.0, 0.0),
            (55.0, 0.0),
            (60.0, 1.0),
            (115.0, 1.0),
            (120.0, 2.0),
            (175.0, 2.0),
            (180.0, 3.0),
        ]);
        let busy = s.rate().expect("rate").per_min;
        assert!((busy - 1.0 / 60.0).abs() < 1e-12);
        // No new step for five hours: less than one point in 300 min.
        s.observe(t(477.5), 3.0);
        let idle = s.rate().expect("rate").per_min;
        assert!((idle - 1.0 / 300.0).abs() < 1e-12, "{idle}");
        // A weekly window at 90 % with two days left no longer fills before its reset.
        let p = project(
            t(477.5),
            90.0,
            idle,
            Some(t(477.5 + 2.0 * 24.0 * 60.0)),
            Confidence::High,
            RateMethod::Steps,
        );
        assert!(!p.will_hit_before_reset);
    }

    #[test]
    fn stale_lower_readings_and_out_of_order_ones_change_nothing() {
        let mut s = steps_of(&[(0.0, 20.0), (10.0, 21.0)]);
        // An idle session's older payload.
        s.observe(t(20.0), 18.0);
        // A reading older than the last one.
        s.observe(t(5.0), 25.0);
        s.observe(t(30.0), 22.0);
        let e = s.edges();
        assert_eq!(e.len(), 2);
        // The stale 18 did not move `seen_at`: the second edge starts at 10 min.
        assert_eq!((e[1].before, e[1].after), (t(10.0), t(30.0)));
    }

    #[test]
    fn edges_seen_only_after_a_long_gap_are_skipped_and_spanned() {
        // The app was closed between 70 and 200 min.
        let s = steps_of(&[
            (0.0, 0.0),
            (55.0, 0.0),
            (60.0, 1.0),
            (70.0, 1.0),
            (200.0, 3.0),
            (235.0, 3.0),
            (240.0, 4.0),
        ]);
        let e = s.edges();
        assert_eq!(e.len(), 2, "{e:?}");
        // 3 points between 57.5 and 237.5 min.
        let r = s.rate().expect("rate");
        assert!((r.per_min - 3.0 / 180.0).abs() < 1e-12, "{}", r.per_min);
    }

    #[test]
    fn whole_percent_polls_every_fifteen_minutes_give_a_steady_projection() {
        // The finding of Step 12t, synthetic: a weekly limit read off the usage API, which
        // rounds to the nearest point, at irregular 5–19 min intervals; true use 1 point per
        // hour from 40.2 %. Full after ~59.8 h, the reset comes after 75 h.
        let rate = 1.0 / 60.0;
        let reset = t(75.0 * 60.0);
        let gaps = [15.0, 5.0, 19.0, 11.0, 15.0, 8.0, 17.0, 14.0];
        let mut steps = Steps::default();
        let mut minute = 0.0;
        let mut i = 0;
        let mut checked = 0;
        while minute < 30.0 * 60.0 {
            let truth: f64 = 40.2 + rate * minute;
            let shown = truth.round();
            steps.observe(t(minute), shown);
            if steps.edges().len() == MAX_EDGES {
                let r = steps.rate().expect("rate");
                assert!(
                    (r.per_min / rate - 1.0).abs() < 0.2,
                    "{minute}: {}",
                    r.per_min
                );
                let p = projection(t(minute), shown, Some(reset), &[], Some(r), None)
                    .expect("projection");
                assert!(p.will_hit_before_reset, "flipped at {minute} min");
                assert_eq!(p.method, RateMethod::Steps);
                checked += 1;
            }
            minute += gaps[i % gaps.len()];
            i += 1;
        }
        assert!(checked > 100, "{checked}");
        // The old 30-minute Theil–Sen on the same kind of readings, right after a step:
        // n, n, n+1, n+1 every 15 min reads 2.5–3 times the true rate.
        let old = theil_sen(&[
            (t(0.0), 50.0),
            (t(10.0), 50.0),
            (t(20.0), 51.0),
            (t(30.0), 51.0),
        ])
        .expect("slope");
        assert!(old / rate > 2.0, "{}", old / rate);
    }

    #[test]
    fn edge_rates_rank_between_the_slope_and_the_cost_fallbacks() {
        let now = t(30.0);
        let calibrated = Some(Fallback::Calibrated { pct_per_min: 0.5 });
        let prior = Some(Fallback::Prior {
            window_cost: 10_000_000,
            recent_cost: 1_500_000,
        });
        let sure = Some(step_rate(0.2, Confidence::Medium));
        let rough = Some(step_rate(0.2, Confidence::Low));
        let method = |steps, costs| {
            projection(now, 40.0, None, &[], steps, costs).map(|p| (p.method, p.confidence))
        };
        // Medium or better beats the calibrated fallback.
        assert_eq!(
            method(sure, calibrated),
            Some((RateMethod::Steps, Confidence::Medium))
        );
        // A low-confidence edge rate yields to the calibrated fallback ...
        assert_eq!(
            method(rough, calibrated),
            Some((RateMethod::Calibrated, Confidence::Low))
        );
        // ... but beats the `k0` prior.
        assert_eq!(
            method(rough, prior),
            Some((RateMethod::Steps, Confidence::Low))
        );
        assert_eq!(method(None, prior).map(|m| m.0), Some(RateMethod::CostRate));
        // The slope of fractional readings comes first.
        let points = [(t(10.0), 30.0), (t(20.0), 35.0), (t(29.0), 40.0)];
        let p = projection(now, 40.0, None, &points, sure, calibrated).expect("projection");
        assert_eq!(p.method, RateMethod::Observed);
    }

    #[test]
    fn the_source_with_more_edges_wins_then_the_newer_one() {
        let a = StepRate {
            edges: 2,
            last_edge: t(50.0),
            ..step_rate(0.1, Confidence::Low)
        };
        let b = StepRate {
            edges: 4,
            last_edge: t(10.0),
            ..step_rate(0.2, Confidence::High)
        };
        let c = StepRate {
            last_edge: t(20.0),
            ..b
        };
        assert_eq!(best_step_rate([a, b]), Some(b));
        assert_eq!(best_step_rate([b, c]), Some(c));
        assert_eq!(best_step_rate([]), None);
    }
}
