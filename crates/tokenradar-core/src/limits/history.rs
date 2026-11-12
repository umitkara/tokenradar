//! The current window's readings of each limit, thinned for a chart (ADR-046).
//!
//! `tokenradar history --json` serves this to the Claude Code mod, which draws the usage curve
//! against the time line. It reads stored snapshots only, so estimates never appear: every
//! point is a measurement with its source.

use serde::Serialize;

use super::engine::window_length_ms;
use super::fusion;
use crate::domain::{LimitObservation, LimitRef, Source, Timestamp};
use crate::store::WINDOW_MATCH_TOLERANCE_MS;

/// Version of the `history --json` document.
pub const HISTORY_SCHEMA: u8 = 1;
/// A window is cut into this many equal buckets and keeps the last reading of each: one per
/// hour of the week, one per ~107 s of the 5-hour window.
pub const HISTORY_BUCKETS: i64 = 168;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryReply {
    pub schema: u8,
    pub generated_at: Timestamp,
    pub series: Vec<HistorySeries>,
}

/// One limit's current window, `window_start <= at <= generated_at`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistorySeries {
    /// `session`, `weekly_all`, `weekly_scoped`, …
    pub kind: String,
    pub scope: Option<String>,
    pub window_start: Timestamp,
    pub resets_at: Timestamp,
    /// Oldest first.
    pub points: Vec<HistoryPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryPoint {
    pub at: Timestamp,
    pub percent: f64,
    /// `statusline`, `api`, `codex_log`.
    pub source: String,
}

/// The series of every limit whose newest reading names a reset still ahead: its readings
/// from the window's start, those of the same window only (a `resets_at` within the match
/// tolerance), the last of each bucket. Limits in `absent` (ADR-041) are left out.
pub fn window_series(
    latest: &[LimitObservation],
    snapshots: &[LimitObservation],
    absent: &[LimitRef],
    now: Timestamp,
) -> Vec<HistorySeries> {
    let mut out = Vec::new();
    for newest in latest {
        let limit = LimitRef::of(newest);
        if absent.contains(&limit) {
            continue;
        }
        let (Some(resets_at), Some(length)) = (newest.resets_at, window_length_ms(&newest.kind))
        else {
            continue;
        };
        if resets_at <= now {
            continue;
        }
        let start = resets_at.saturating_sub_millis(length);
        let points = window_readings(&limit, resets_at, length, snapshots, now)
            .into_iter()
            .map(|o| HistoryPoint {
                at: o.at,
                percent: o.percent,
                source: o.source.as_str().to_owned(),
            })
            .collect();
        out.push(HistorySeries {
            kind: newest.kind.as_str().to_owned(),
            scope: newest.scope.as_ref().map(ToString::to_string),
            window_start: start,
            resets_at,
            points,
        });
    }
    out
}

/// One point of a window's curve: a stored reading's time and source, with the fused value
/// of the window up to it (ADR-060).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowReading {
    pub at: Timestamp,
    pub percent: f64,
    pub source: Source,
}

/// `limit`'s readings in the window ending at `resets_at`, oldest first: those of the same
/// window only (a `resets_at` within the match tolerance), from its start up to `now`, the
/// last of each of [`HISTORY_BUCKETS`] buckets. Each carries the fused value, so the curve
/// follows the panel instead of zigzagging between the sources' roundings. `snapshots` must
/// come oldest first.
pub fn window_readings(
    limit: &LimitRef,
    resets_at: Timestamp,
    length: i64,
    snapshots: &[LimitObservation],
    now: Timestamp,
) -> Vec<WindowReading> {
    let start = resets_at.saturating_sub_millis(length);
    let mine: Vec<(Timestamp, Source, f64)> = snapshots
        .iter()
        .filter(|o| {
            let same_window = o.resets_at.is_some_and(|r| {
                (r.as_millis() - resets_at.as_millis()).abs() <= WINDOW_MATCH_TOLERANCE_MS
            });
            LimitRef::of(o) == *limit
                && same_window
                && o.observed_at >= start
                && o.observed_at <= now
        })
        .map(|o| (o.observed_at, o.source, o.percent))
        .collect();
    let fused = fusion::series(&mine);
    let bucket = (length / HISTORY_BUCKETS).max(1);
    let mut kept: Vec<WindowReading> = Vec::new();
    let mut last_bucket = None;
    for (&(at, source, _), &(_, percent)) in mine.iter().zip(&fused) {
        let point = WindowReading {
            at,
            percent,
            source,
        };
        let b = (at.as_millis() - start.as_millis()) / bucket;
        if last_bucket == Some(b) {
            // The later reading of a bucket wins.
            if let Some(slot) = kept.last_mut() {
                *slot = point;
            }
        } else {
            kept.push(point);
            last_bucket = Some(b);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AccountId, LimitKind, Scope, Source};
    use crate::limits::WEEK_MS;

    const HOUR: i64 = 3_600_000;

    fn obs(kind: LimitKind, percent: f64, resets_at: i64, at: i64) -> LimitObservation {
        LimitObservation {
            account: AccountId(1),
            kind,
            scope: None,
            percent,
            resets_at: Some(Timestamp::from_millis(resets_at)),
            severity: None,
            source: Source::Statusline,
            session_uuid: None,
            observed_at: Timestamp::from_millis(at),
        }
    }

    #[test]
    fn keeps_the_current_window_thinned_to_one_reading_per_bucket() {
        let reset = 10 * WEEK_MS;
        let start = reset - WEEK_MS;
        let now = Timestamp::from_millis(start + 30 * HOUR);
        let snaps = vec![
            // The previous window: another reset.
            obs(LimitKind::WeeklyAll, 80.0, start, start - HOUR),
            obs(LimitKind::WeeklyAll, 1.0, reset, start + 10 * 60_000),
            obs(LimitKind::WeeklyAll, 2.0, reset, start + 50 * 60_000),
            // Seconds of drift in `resets_at` are the same window.
            obs(LimitKind::WeeklyAll, 5.0, reset + 2_000, start + 2 * HOUR),
            obs(LimitKind::WeeklyAll, 9.0, reset, start + 29 * HOUR),
        ];
        let latest = vec![snaps[4].clone()];
        let series = window_series(&latest, &snaps, &[], now);
        assert_eq!(series.len(), 1);
        let s = &series[0];
        assert_eq!(s.kind, "weekly_all");
        assert_eq!(s.window_start.as_millis(), start);
        let got: Vec<f64> = s.points.iter().map(|p| p.percent).collect();
        assert_eq!(got, vec![2.0, 5.0, 9.0]);
        assert_eq!(s.points[0].source, "statusline");
    }

    #[test]
    fn leaves_out_absent_past_and_windowless_limits() {
        let now = 100 * WEEK_MS;
        let session = obs(LimitKind::Session, 10.0, now + HOUR, now - HOUR);
        let past = obs(LimitKind::WeeklyAll, 40.0, now - 1, now - 2 * HOUR);
        let spend = obs(LimitKind::Spend, 3.0, now + HOUR, now - HOUR);
        let mut scoped = obs(LimitKind::WeeklyScoped, 7.0, now + HOUR, now - HOUR);
        scoped.scope = Some(Scope::Model("Fable".into()));
        let latest = vec![session.clone(), past, spend, scoped.clone()];
        let absent = vec![LimitRef::new(LimitKind::Session, None)];
        let series = window_series(
            &latest,
            &[session, scoped],
            &absent,
            Timestamp::from_millis(now),
        );
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].scope.as_deref(), Some("model:Fable"));
        assert_eq!(series[0].window_start.as_millis(), now + HOUR - WEEK_MS);
        assert_eq!(series[0].points.len(), 1);
    }

    #[test]
    fn the_curve_carries_the_fused_value_not_each_source_rounding() {
        let reset = 10 * WEEK_MS;
        let start = reset - WEEK_MS;
        let now = Timestamp::from_millis(start + 10 * HOUR);
        let mut snaps: Vec<LimitObservation> = (0..6)
            .map(|h| obs(LimitKind::WeeklyAll, 5.0, reset, start + h * HOUR))
            .collect();
        // The API's 6 an hour in: the statusline's later 5s mean [5.5, 6).
        snaps[1].source = Source::Api;
        snaps[1].percent = 6.0;
        let series = window_series(&[snaps[5].clone()], &snaps, &[], now);
        let got: Vec<(f64, &str)> = series[0]
            .points
            .iter()
            .map(|p| (p.percent, p.source.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                (5.0, "statusline"),
                (6.0, "api"),
                (6.0, "statusline"),
                (6.0, "statusline"),
                (6.0, "statusline"),
                (6.0, "statusline"),
            ]
        );
    }
}
