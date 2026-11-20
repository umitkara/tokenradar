//! Largest-Triangle-Three-Buckets thinning (Steinarsson 2013): keeps the first and last point
//! and, per bucket, the point that spans the largest triangle with its neighbours. A chart of
//! the result keeps the peaks and steps a uniform pick would drop.

/// Thins `points` (x ascending) to at most `max` points; returns them unchanged when they
/// already fit or `max < 3`.
#[allow(clippy::cast_precision_loss)]
pub fn lttb<T: Copy>(points: &[T], max: usize, xy: impl Fn(&T) -> (f64, f64)) -> Vec<T> {
    let n = points.len();
    if max < 3 || n <= max {
        return points.to_vec();
    }
    let mut out = Vec::with_capacity(max);
    out.push(points[0]);
    // Inner points split into `max - 2` buckets of `every` points (every > 1).
    let every = (n - 2) as f64 / (max - 2) as f64;
    // `i · every` is non-negative and below `n`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let edge = |i: usize| ((i as f64 * every) as usize + 1).min(n - 1);
    let mut a = 0usize;
    for i in 0..max - 2 {
        let (start, end) = (edge(i), edge(i + 1));
        // Third corner: the next bucket's average, or the last point after the final bucket.
        let (next_start, next_end) = if i + 2 < max - 1 {
            (end, edge(i + 2))
        } else {
            (n - 1, n)
        };
        let (mut ax, mut ay) = (0.0, 0.0);
        for p in &points[next_start..next_end] {
            let (x, y) = xy(p);
            ax += x;
            ay += y;
        }
        let count = (next_end - next_start) as f64;
        let (ax, ay) = (ax / count, ay / count);
        let (px, py) = xy(&points[a]);
        let mut best = start;
        let mut best_area = -1.0;
        for (j, p) in points.iter().enumerate().take(end).skip(start) {
            let (x, y) = xy(p);
            let area = ((px - ax) * (y - py) - (px - x) * (ay - py)).abs();
            if area > best_area {
                best_area = area;
                best = j;
            }
        }
        out.push(points[best]);
        a = best;
    }
    out.push(points[n - 1]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::cast_precision_loss)]
    fn xy(p: &(i64, f64)) -> (f64, f64) {
        (p.0 as f64, p.1)
    }

    #[test]
    fn short_series_pass_through() {
        let pts = vec![(0, 1.0), (1, 2.0)];
        assert_eq!(lttb(&pts, 10, xy), pts);
    }

    #[test]
    fn keeps_ends_peak_and_bound() {
        let pts: Vec<(i64, f64)> = (0..10_000)
            .map(|i| {
                (
                    i,
                    if i == 4_321 {
                        99.0
                    } else {
                        f64::from(i32::try_from(i % 7).expect("small"))
                    },
                )
            })
            .collect();
        let out = lttb(&pts, 1_500, xy);
        assert_eq!(out.len(), 1_500);
        assert_eq!(out.first(), pts.first());
        assert_eq!(out.last(), pts.last());
        assert!(out.iter().any(|p| p.0 == 4_321), "the spike survives");
        assert!(out.windows(2).all(|w| w[0].0 < w[1].0), "x stays ascending");
    }
}
