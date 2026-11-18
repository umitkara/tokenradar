//! Observation intervals from integer limit readings (Step 13, ADR-061).
//!
//! Both sources report whole percentages, so a pair of arbitrary snapshots carries up to one
//! point of quantisation error. The instants where the percentage *steps up* do not: between
//! two consecutive steps ("edges") exactly `Δp` points were used. An interval runs from one
//! edge to the next.
//!
//! - **Running maximum.** Usage never falls inside a window, but the statusline of an idle
//!   session repeats an older, lower value. Edges are therefore steps of the window's running
//!   maximum; a lower reading says nothing and is skipped.
//! - **Edge time.** The step happened somewhere between the last reading that still showed the
//!   old maximum and the first that showed the new one. The midpoint stands for it; the
//!   distance between the two is the edge's uncertainty.
//! - **Uncertain edges merge.** An edge seen only after a long gap (app closed, a 401 wait) is
//!   not used as a boundary; its neighbours span it with a larger `Δp`. This also fixes the
//!   stale first reading after a gap, which otherwise makes one point look expensive and the
//!   next one cheap.
//! - **The first reading of a window is not an edge.** Recording may start mid-window.

use crate::domain::Timestamp;
use crate::store::WINDOW_MATCH_TOLERANCE_MS;

/// Edges known less precisely than this are not used as interval boundaries.
pub const MAX_EDGE_UNCERTAINTY_MS: i64 = 45 * 60 * 1000;

/// One reading of one source for one limit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub at: Timestamp,
    pub percent: f64,
    pub resets_at: Option<Timestamp>,
}

/// A step of the running maximum: the value became `percent` within `(before, after]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub before: Timestamp,
    pub after: Timestamp,
    pub percent: f64,
}

impl Edge {
    pub fn at(&self) -> Timestamp {
        Timestamp::from_millis(i64::midpoint(
            self.before.as_millis(),
            self.after.as_millis(),
        ))
    }

    pub fn uncertainty_ms(&self) -> i64 {
        self.after.as_millis() - self.before.as_millis()
    }
}

/// `dp` points were used within `(from, to]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub from: Timestamp,
    pub to: Timestamp,
    pub dp: f64,
    /// The window the interval belongs to.
    pub resets_at: Timestamp,
}

struct Window {
    resets_at: Timestamp,
    max: f64,
    /// Last reading that showed `max`.
    seen_at: Timestamp,
    edges: Vec<Edge>,
}

/// Edges per window, in the order the windows were first seen. `readings` must be sorted by
/// time. Readings without a reset time are ignored.
pub fn edges(readings: &[Reading]) -> Vec<(Timestamp, Vec<Edge>)> {
    let mut windows: Vec<Window> = Vec::new();
    for r in readings {
        let Some(resets_at) = r.resets_at else {
            continue;
        };
        // A stale session may still report the previous window, so match any window.
        let found = windows.iter_mut().find(|w| {
            (w.resets_at.as_millis() - resets_at.as_millis()).abs() <= WINDOW_MATCH_TOLERANCE_MS
        });
        let Some(w) = found else {
            windows.push(Window {
                resets_at,
                max: r.percent,
                seen_at: r.at,
                edges: Vec::new(),
            });
            continue;
        };
        if r.percent > w.max {
            w.edges.push(Edge {
                before: w.seen_at,
                after: r.at,
                percent: r.percent,
            });
            w.max = r.percent;
            w.seen_at = r.at;
        } else if r.percent >= w.max {
            w.seen_at = r.at;
        }
    }
    windows
        .into_iter()
        .map(|w| (w.resets_at, w.edges))
        .collect()
}

/// Edge-to-edge intervals of all windows, oldest window first.
pub fn intervals(readings: &[Reading]) -> Vec<Interval> {
    let mut out = Vec::new();
    for (resets_at, edges) in edges(readings) {
        let sharp: Vec<&Edge> = edges
            .iter()
            .filter(|e| e.uncertainty_ms() <= MAX_EDGE_UNCERTAINTY_MS)
            .collect();
        for pair in sharp.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let dp = b.percent - a.percent;
            if dp > 0.0 && b.at() > a.at() {
                out.push(Interval {
                    from: a.at(),
                    to: b.at(),
                    dp,
                    resets_at,
                });
            }
        }
    }
    out.sort_by_key(|i| i.to);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;
    const RESET: i64 = 10_000 * MIN;

    fn r(min: i64, percent: f64) -> Reading {
        Reading {
            at: Timestamp::from_millis(min * MIN),
            percent,
            resets_at: Some(Timestamp::from_millis(RESET)),
        }
    }

    #[test]
    fn edges_are_steps_of_the_running_maximum() {
        // 10 at 0 and 5, 11 at 10: one edge in (5, 10]. A stale 10 at 12 changes nothing.
        let rs = [
            r(0, 10.0),
            r(5, 10.0),
            r(10, 11.0),
            r(12, 10.0),
            r(15, 11.0),
            r(20, 12.0),
        ];
        let (_, e) = &edges(&rs)[0];
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].before.as_millis(), 5 * MIN);
        assert_eq!(e[0].after.as_millis(), 10 * MIN);
        // The last reading showing the old maximum (15, not the stale 12) bounds the next edge.
        assert_eq!(e[1].before.as_millis(), 15 * MIN);
        assert_eq!(e[1].at().as_millis(), 17 * MIN + 30_000);
    }

    #[test]
    fn the_first_reading_is_not_an_edge() {
        let rs = [r(0, 40.0), r(5, 40.0)];
        assert_eq!(edges(&rs)[0].1, Vec::<Edge>::new());
        assert_eq!(intervals(&rs), Vec::<Interval>::new());
    }

    #[test]
    fn intervals_run_from_edge_to_edge() {
        let rs = [r(0, 1.0), r(10, 2.0), r(70, 2.0), r(80, 3.0), r(90, 5.0)];
        let iv = intervals(&rs);
        assert_eq!(iv.len(), 2);
        assert_eq!(iv[0].from.as_millis(), 5 * MIN);
        assert_eq!(iv[0].to.as_millis(), 75 * MIN);
        assert!((iv[0].dp - 1.0).abs() < 1e-9);
        assert!((iv[1].dp - 2.0).abs() < 1e-9);
    }

    #[test]
    fn an_uncertain_edge_is_spanned_by_its_neighbours() {
        // The step to 3 is seen only after a 5-hour gap.
        let rs = [r(0, 1.0), r(10, 2.0), r(20, 2.0), r(320, 3.0), r(330, 4.0)];
        let iv = intervals(&rs);
        assert_eq!(iv.len(), 1);
        assert!((iv[0].dp - 2.0).abs() < 1e-9);
        assert_eq!(iv[0].from.as_millis(), 5 * MIN);
        assert_eq!(iv[0].to.as_millis(), 325 * MIN);
    }

    #[test]
    fn windows_are_kept_apart_and_matched_with_tolerance() {
        let next = |min: i64, p: f64, jitter: i64| Reading {
            at: Timestamp::from_millis(min * MIN),
            percent: p,
            resets_at: Some(Timestamp::from_millis(RESET + 7 * 24 * 60 * MIN + jitter)),
        };
        let rs = [
            r(0, 50.0),
            r(10, 51.0),
            next(20, 0.0, 0),
            r(25, 51.0), // stale session, old window
            next(30, 1.0, 900),
            r(35, 52.0),
            next(40, 2.0, -900),
        ];
        let w = edges(&rs);
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].1.len(), 2);
        assert_eq!(w[1].1.len(), 2);
        let iv = intervals(&rs);
        assert_eq!(iv.len(), 2);
        assert!(iv.iter().all(|i| (i.dp - 1.0).abs() < 1e-9));
    }
}
