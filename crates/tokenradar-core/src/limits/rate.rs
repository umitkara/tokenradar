//! Burn rate and projection (architecture §7).
//!
//! - **Primary:** the Theil–Sen slope (median of pairwise slopes) of the current window's
//!   observations from the last 30 minutes, at least three of them. Robust against a single
//!   outlier and against the 1-point steps of the API's integer percentages.
//! - **Fallback:** `k0 × cost rate of the last 15 minutes`, with the prior
//!   `k0 = p_now / C_window` (architecture §9.2): the share of the window one dollar of local
//!   usage has cost so far. Always low confidence.

use serde::Serialize;
use specta::Type;

use crate::domain::Timestamp;

/// Observations older than this do not shape the slope.
pub const RATE_LOOKBACK_MS: i64 = 30 * 60 * 1000;
/// Cost window of the fallback.
pub const FALLBACK_LOOKBACK_MS: i64 = 15 * 60 * 1000;
pub const MIN_POINTS: usize = 3;
/// Below this (%/min) usage counts as stopped: no ETA.
const RATE_EPSILON: f64 = 1e-4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
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
    /// Local cost rate times the window's percent-per-dollar.
    CostRate,
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

/// The projection for one limit: Theil–Sen when enough points exist, the cost fallback
/// otherwise, `None` when neither works.
pub fn projection(
    now: Timestamp,
    p_now: f64,
    resets_at: Option<Timestamp>,
    points: &[(Timestamp, f64)],
    costs: Option<(i64, i64)>,
) -> Option<Projection> {
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
        return Some(project(
            now,
            p_now,
            slope,
            resets_at,
            confidence,
            RateMethod::Observed,
        ));
    }
    let (window_cost, recent_cost) = costs?;
    let rate = cost_rate(p_now, window_cost, recent_cost)?;
    Some(project(
        now,
        p_now,
        rate,
        resets_at,
        Confidence::Low,
        RateMethod::CostRate,
    ))
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
            Some((10_000_000, 1_500_000)),
        )
        .expect("projection");
        assert_eq!(p.method, RateMethod::CostRate);
        assert_eq!(p.confidence, Confidence::Low);
        assert!((p.rate_per_min - 0.4).abs() < 1e-9);
        // No cost in the window: nothing to scale.
        assert_eq!(projection(now, 40.0, None, &[], Some((0, 0))), None);
        assert_eq!(projection(now, 40.0, None, &[], None), None);
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
        assert_eq!(projection(now, 51.0, None, &points, None), None);
    }
}
