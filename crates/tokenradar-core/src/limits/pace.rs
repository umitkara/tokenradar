//! Time against use (ADR-044, ADR-045): how much of a window has passed, and whether use runs
//! ahead of it. Shared by the statusline's `⏱`, the Claude Code mod's numbers and the panel's
//! detail view, so all three agree.

use crate::domain::Timestamp;

/// Use this many points ahead of the time line is worth a mark (ADR-044).
pub const PACE_AHEAD: f64 = 10.0;
/// No pace warning in the window's first seventh: a day of the week, ~43 minutes of the
/// 5-hour window (ADR-045).
pub const PACE_QUIET: f64 = 100.0 / 7.0;

/// The share of a window of `length` ms that has passed, from the time left until its reset.
/// `None` when the reset is unknown, passed or further away than the window is long.
#[allow(clippy::cast_precision_loss)] // window lengths are far below 2^52 ms
pub fn elapsed(resets_in: Option<i64>, length: i64) -> Option<f64> {
    let left = resets_in?;
    (left > 0 && left <= length).then(|| (1.0 - left as f64 / length as f64) * 100.0)
}

/// [`elapsed`] at `now` for a window ending at `resets_at`.
pub fn elapsed_at(resets_at: Option<Timestamp>, length: i64, now: Timestamp) -> Option<f64> {
    elapsed(resets_at.map(|r| r.as_millis() - now.as_millis()), length)
}

/// Whether `percent` used with `elapsed` of the time gone deserves a warning: far enough
/// ahead, and past the quiet start.
pub fn warns(percent: f64, elapsed: f64) -> bool {
    elapsed >= PACE_QUIET && percent - elapsed >= PACE_AHEAD
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400_000;
    const WEEK: i64 = 7 * DAY;

    #[test]
    fn elapsed_comes_from_the_time_left() {
        assert_eq!(elapsed(Some(WEEK), WEEK), Some(0.0));
        assert!((elapsed(Some(WEEK / 2), WEEK).expect("half a week") - 50.0).abs() < 1e-9);
        assert_eq!(elapsed(Some(0), WEEK), None);
        assert_eq!(elapsed(Some(WEEK + 1), WEEK), None);
        assert_eq!(elapsed(None, WEEK), None);
        let now = Timestamp::from_millis(10 * WEEK);
        let reset = Timestamp::from_millis(10 * WEEK + 6 * DAY);
        let got = elapsed_at(Some(reset), WEEK, now).expect("a day in");
        assert!((got - 100.0 / 7.0).abs() < 1e-9);
    }

    #[test]
    fn warns_only_ahead_and_after_the_first_seventh() {
        // Two hours in, 12 % used: far ahead, but quiet (ADR-045).
        assert!(!warns(12.0, 1.2));
        assert!(!warns(PACE_QUIET + 9.9, PACE_QUIET));
        assert!(warns(30.0, 20.0));
        assert!(!warns(40.0, 43.0));
    }
}
