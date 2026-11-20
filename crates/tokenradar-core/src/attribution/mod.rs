//! Attribution of limit windows to session trees (architecture §9.3, Step 14a, ADR-064).
//!
//! The question it answers: how many points of a window did each session tree (a main session
//! with its subagents) use, and how many came from somewhere this machine's logs cannot see
//! ("Other": claude.ai, another computer, another tool)?
//!
//! - **Segments.** The window's sharp edges (Step 13, [`crate::calibration::intervals`]) are
//!   grouped into 24-hour slices aligned to the window start. A segment runs from the last edge
//!   of one slice to the last edge of the next slice that has one; its `Δp` is known exactly.
//!   The head starts at the window start (0 %). The tail up to `min(now, resets_at)` joins the
//!   last segment with `peak − last edge`, so the shares always add up to the shown percentage.
//! - **Calibrated** (calibration at least medium confidence): per segment
//!   `local = min(Δp, k · Σ α · cost)`, local is split by `α`-weighted cost, the rest is Other.
//!   Per edge interval the `min` collects quantisation noise into Other (+3–4 points on real
//!   data); per 24 hours it tracks `seven_day_breakdown` (Step 14 discovery). The head is
//!   measured from 0 and so carries the sources' rounding ([`ROUNDING_OFFSET`]); that half
//!   point goes to the shares, not to Other.
//! - **Proportional** (no usable calibration): the window's percentage is split by cost over
//!   the whole window, as in v0.1. Other is not computed.
//! - **Estimated / cost only** (no snapshot covered the window): `k · Σ α · cost` per session
//!   when calibrated, otherwise cost alone.
//!
//! [`attribute`] is pure; [`refresh`] reads, decides what is stale and writes.

mod refresh;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use serde::Serialize;
use specta::Type;

use crate::calibration::Calibration;
use crate::calibration::intervals::{Edge, MAX_EDGE_UNCERTAINTY_MS};
use crate::domain::{MicroUsd, ModelFamily, SessionId, Timestamp};
use crate::limits::Confidence;

pub use refresh::{
    AttributionData, MAX_PER_REFRESH, Refreshed, compute, ensure, estimated_weekly_windows,
    fill_estimated, refresh, window_edges,
};

/// Width of a slice that edges are grouped by.
pub const SLICE_MS: i64 = 24 * 60 * 60 * 1000;
/// "Other" this many points below the breakdown's non-Claude-Code share is a contradiction.
pub const CROSS_CHECK_POINTS: f64 = 10.0;
/// The edge to `n` points comes when the true value passes `n − ROUNDING_OFFSET`: both the
/// statusline and the API round to the nearest point (Step 14a discovery: cumulative
/// `k · Σ α · cost` at the first edges ≈ 0.5, 1.5, 2.5 …).
pub const ROUNDING_OFFSET: f64 = 0.5;
/// An open window is recomputed at most this often.
pub const LIVE_REFRESH_MS: i64 = 60_000;

/// Cost of one session tree's model family in one `usage_5m` bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BucketCost {
    pub bucket: Timestamp,
    pub root: SessionId,
    pub family: ModelFamily,
    pub cost: MicroUsd,
}

/// `dp` points were used within `(from, to]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub from: Timestamp,
    pub to: Timestamp,
    pub dp: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Calibrated,
    Proportional,
    Estimated,
    CostOnly,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Calibrated => "calibrated",
            Self::Proportional => "proportional",
            Self::Estimated => "estimated",
            Self::CostOnly => "cost_only",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "calibrated" => Self::Calibrated,
            "proportional" => Self::Proportional,
            "estimated" => Self::Estimated,
            "cost_only" => Self::CostOnly,
            _ => return None,
        })
    }
}

/// One session tree's part of a window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Share {
    pub root: SessionId,
    /// `None` when the window has no percentage to split.
    pub percent: Option<f64>,
    /// Cost the limit counts (only its family for a scoped limit).
    pub cost: MicroUsd,
}

/// What to attribute.
#[derive(Debug, Clone, Copy)]
pub struct Input<'a> {
    pub start: Timestamp,
    /// `min(now, resets_at)`.
    pub end: Timestamp,
    /// The window's shown percentage; `None` when no snapshot covered it.
    pub peak: Option<f64>,
    /// Edges of the window's source, oldest first. Uncertain ones are skipped here.
    pub edges: &'a [Edge],
    pub buckets: &'a [BucketCost],
    /// The family a scoped limit counts.
    pub family: Option<ModelFamily>,
    pub calibration: Option<&'a Calibration>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub method: Method,
    /// Calibration confidence for this window's usage; `None` without a calibration.
    pub confidence: Option<f64>,
    /// Σ shares + Other.
    pub total: Option<f64>,
    pub other: Option<f64>,
    /// Largest percentage (then cost) first.
    pub shares: Vec<Share>,
}

/// Splits a window into segments with known `Δp` (module docs).
pub fn segments(start: Timestamp, end: Timestamp, edges: &[Edge], peak: f64) -> Vec<Segment> {
    let mut out = Vec::new();
    let (mut from, mut p_from) = (start, 0.0_f64);
    let mut last_in_slice: Option<(i64, Edge)> = None;
    let sharp = edges.iter().filter(|e| {
        e.uncertainty_ms() <= MAX_EDGE_UNCERTAINTY_MS && e.at() > start && e.at() <= end
    });
    let mut close = |e: Edge, from: &mut Timestamp, p_from: &mut f64| {
        if e.percent > *p_from {
            out.push(Segment {
                from: *from,
                to: e.at(),
                dp: e.percent - *p_from,
            });
            *from = e.at();
            *p_from = e.percent;
        }
    };
    for e in sharp {
        let slice = (e.at().as_millis() - start.as_millis()).div_euclid(SLICE_MS);
        if let Some((s, prev)) = last_in_slice
            && s != slice
        {
            close(prev, &mut from, &mut p_from);
        }
        last_in_slice = Some((slice, *e));
    }
    if let Some((_, e)) = last_in_slice {
        close(e, &mut from, &mut p_from);
    }
    // The tail after the last edge holds less than a point of usage plus the other source's
    // rounding (ADR-060); on its own its `min` would push that rounding into Other. It joins
    // the last segment instead.
    let tail = (peak - p_from).max(0.0);
    match out.last_mut() {
        Some(last) => {
            last.to = end.max(last.to);
            last.dp += tail;
        }
        None => out.push(Segment {
            from,
            to: end.max(from),
            dp: tail,
        }),
    }
    out
}

/// Whether a calibration may split a window: at least medium confidence for its usage.
fn usable(cal: Option<&Calibration>, confidence: Option<f64>) -> bool {
    cal.is_some()
        && confidence.is_some_and(|c| crate::calibration::estimator::level(c) >= Confidence::Medium)
}

/// `Σ α · cost` of one bucket in micro-USD (0 outside the limit's family).
fn weight(b: &BucketCost, family: Option<ModelFamily>, cal: Option<&Calibration>) -> f64 {
    if family.is_some_and(|f| f != b.family) {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let micro = b.cost.0.max(0) as f64;
    micro
        * cal
            .and_then(|c| c.alpha.get(&b.family).copied())
            .unwrap_or(1.0)
}

/// Attributes one window (module docs). Shares + Other equal the total exactly up to float
/// rounding.
pub fn attribute(input: &Input<'_>) -> Outcome {
    let family = input.family;
    let counted: Vec<&BucketCost> = input
        .buckets
        .iter()
        .filter(|b| family.is_none_or(|f| f == b.family))
        .collect();
    let mut cost: BTreeMap<SessionId, i64> = BTreeMap::new();
    for b in &counted {
        *cost.entry(b.root).or_default() += b.cost.0;
    }
    let confidence = input
        .calibration
        .map(|c| c.confidence_for_families(counted.iter().map(|b| (b.family, b.cost))));
    let cal = input
        .calibration
        .filter(|_| usable(input.calibration, confidence));
    let mut pct: BTreeMap<SessionId, f64> = BTreeMap::new();

    let (method, total, other) = match (input.peak, cal) {
        (Some(peak), Some(cal)) => {
            let other = calibrated(input, peak, cal, &mut pct);
            let total = pct.values().sum::<f64>() + other;
            (Method::Calibrated, Some(total), Some(other))
        }
        (Some(peak), None) => {
            let mut w: BTreeMap<SessionId, f64> = BTreeMap::new();
            for b in &counted {
                *w.entry(b.root).or_default() += weight(b, family, None);
            }
            let sum: f64 = w.values().sum();
            if sum > 0.0 {
                for (root, x) in w {
                    pct.insert(root, peak * x / sum);
                }
                (Method::Proportional, Some(peak), None)
            } else {
                // Nothing local to split it over: all of it came from elsewhere.
                (
                    Method::Proportional,
                    Some(peak),
                    (peak > 0.0).then_some(peak),
                )
            }
        }
        (None, Some(cal)) => {
            for b in &counted {
                *pct.entry(b.root).or_default() += cal.k * weight(b, family, Some(cal)) / 1e6;
            }
            let total = pct.values().sum();
            (Method::Estimated, Some(total), None)
        }
        (None, None) => (Method::CostOnly, None, None),
    };

    let mut shares: Vec<Share> = cost
        .into_iter()
        .map(|(root, c)| Share {
            root,
            percent: total.map(|_| pct.get(&root).copied().unwrap_or(0.0)),
            cost: MicroUsd(c),
        })
        .collect();
    shares.sort_by(|a, b| {
        b.percent
            .unwrap_or(0.0)
            .total_cmp(&a.percent.unwrap_or(0.0))
            .then(b.cost.cmp(&a.cost))
            .then(a.root.cmp(&b.root))
    });
    Outcome {
        method,
        confidence,
        total,
        other,
        shares,
    }
}

/// Per-segment calibrated split; returns Other.
fn calibrated(
    input: &Input<'_>,
    peak: f64,
    cal: &Calibration,
    pct: &mut BTreeMap<SessionId, f64>,
) -> f64 {
    let segs = segments(input.start, input.end, input.edges, peak);
    // Weighted cost per segment and root. A bucket belongs to the first segment whose end
    // edge is after the bucket's start.
    let mut per: Vec<BTreeMap<SessionId, f64>> = vec![BTreeMap::new(); segs.len()];
    for b in input.buckets {
        let w = weight(b, input.family, Some(cal));
        if w <= 0.0 || b.bucket < input.start {
            continue;
        }
        let i = segs.partition_point(|s| s.to <= b.bucket);
        if let Some(m) = per.get_mut(i) {
            *m.entry(b.root).or_default() += w;
        }
    }
    let mut other = 0.0;
    let mut rounding = 0.0;
    for (i, (seg, roots)) in segs.iter().zip(per).enumerate() {
        // Both sources round to the nearest point: the edge to `n` comes at a true `n − 0.5`.
        // Differences between edges cancel it; the head, measured from 0, carries it.
        let dp = if i == 0 {
            let cut = ROUNDING_OFFSET.min(seg.dp);
            rounding += cut;
            seg.dp - cut
        } else {
            seg.dp
        };
        let sum: f64 = roots.values().sum();
        if sum <= 0.0 {
            other += dp;
            continue;
        }
        let local = (cal.k * sum / 1e6).min(dp);
        other += dp - local;
        for (root, w) in roots {
            *pct.entry(root).or_default() += local * w / sum;
        }
    }
    // The rounding is no one's usage; it follows the shares so the total stays the shown value.
    let local: f64 = pct.values().sum();
    if local > 0.0 {
        for v in pct.values_mut() {
            *v += rounding * *v / local;
        }
    } else {
        other += rounding;
    }
    other
}

/// `(suspect, other_excess)`: Other's share of the total against the breakdown's
/// non-Claude-Code share, both in percent. The breakdown counts Claude Code on another
/// computer as Claude Code, so it is only a lower bound for Other: only falling well below it
/// is a contradiction (ADR-064).
pub fn cross_check(other_share: f64, breakdown_other: f64) -> (bool, bool) {
    (
        other_share < breakdown_other - CROSS_CHECK_POINTS,
        other_share > breakdown_other + CROSS_CHECK_POINTS,
    )
}
