//! The tray panel's detail view (ADR-048): the Claude Code mod's report (ADR-046) inside the
//! app. Time against use for each limit, and for one limit, the "focus", the 24-hour periods
//! of its window and its measured readings for the usage chart.
//!
//! Local midnights are not here: they are calendar presentation, drawn by the frontend in the
//! system's time zone next to the day names it formats (ADR-048).

use serde::Serialize;
use specta::Type;

use super::engine::window_length_ms;
use super::history::window_readings;
use super::pace;
use super::view::{LimitView, PanelSnapshot};
use crate::domain::{LimitKind, LimitObservation, LimitRef, Scope, Source, Timestamp};

const DAY_MS: i64 = 86_400_000;

/// Everything the detail view adds to [`PanelSnapshot`].
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct PanelDetail {
    /// Measured limits with a window still running, in the snapshot's order.
    pub pace: Vec<PaceView>,
    /// The limit the time strip and the chart are about: the weekly limit, otherwise the
    /// 5-hour window; `None` when neither has a measured reading with a reset ahead.
    pub focus: Option<FocusView>,
    pub generated_at: Timestamp,
}

/// One limit's use against the time gone (ADR-044).
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct PaceView {
    pub kind: LimitKind,
    pub scope: Option<Scope>,
    pub percent: f64,
    /// Share of the window that has passed, 0–100.
    pub elapsed_percent: f64,
    /// `percent − elapsed_percent`: positive when use runs ahead of the time line.
    pub ahead: f64,
    /// At least [`pace::PACE_AHEAD`] points ahead, past the window's first seventh (ADR-045).
    pub warn: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct FocusView {
    pub kind: LimitKind,
    pub scope: Option<Scope>,
    pub window_start: Timestamp,
    pub resets_at: Timestamp,
    /// Every 24 hours from the window's start with the time line's value there; weekly only.
    pub periods: Vec<PeriodMark>,
    /// Measured readings of this window, oldest first, thinned for a chart; the last one is
    /// the panel's current value. Empty without stored readings. Never an estimate.
    pub points: Vec<ChartPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct PeriodMark {
    pub at: Timestamp,
    /// Where the time line stands at `at`: k/7 of the week.
    pub time_percent: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct ChartPoint {
    pub at: Timestamp,
    pub percent: f64,
    pub source: Source,
}

/// The detail view for `snapshot` at `now`. `snapshots` are the account's stored readings,
/// oldest first, covering at least the last week.
pub fn panel_detail(
    snapshot: &PanelSnapshot,
    snapshots: &[LimitObservation],
    now: Timestamp,
) -> PanelDetail {
    let pace = snapshot
        .limits
        .iter()
        .filter_map(|l| pace_of(l, now))
        .collect();
    let focus_limit = [LimitKind::WeeklyAll, LimitKind::Session]
        .iter()
        .find_map(|kind| {
            snapshot
                .limits
                .iter()
                .find(|l| l.kind == *kind && l.scope.is_none() && measured_window(l, now).is_some())
        });
    PanelDetail {
        pace,
        focus: focus_limit.and_then(|l| focus_of(l, snapshots, now)),
        generated_at: now,
    }
}

/// `(resets_at, length)` of a measured limit whose window is still running.
fn measured_window(limit: &LimitView, now: Timestamp) -> Option<(Timestamp, i64)> {
    if limit.source == Source::Estimate {
        return None;
    }
    let length = window_length_ms(&limit.kind)?;
    let resets_at = limit.resets_at.filter(|r| *r > now)?;
    Some((resets_at, length))
}

fn pace_of(limit: &LimitView, now: Timestamp) -> Option<PaceView> {
    let (resets_at, length) = measured_window(limit, now)?;
    let elapsed = pace::elapsed_at(Some(resets_at), length, now)?;
    Some(PaceView {
        kind: limit.kind.clone(),
        scope: limit.scope.clone(),
        percent: limit.percent,
        elapsed_percent: elapsed,
        ahead: limit.percent - elapsed,
        warn: pace::warns(limit.percent, elapsed),
    })
}

fn focus_of(
    limit: &LimitView,
    snapshots: &[LimitObservation],
    now: Timestamp,
) -> Option<FocusView> {
    let (resets_at, length) = measured_window(limit, now)?;
    let start = resets_at.saturating_sub_millis(length);
    let periods = if length > DAY_MS {
        (1..length / DAY_MS)
            .map(|k| PeriodMark {
                at: start.saturating_add_millis(k * DAY_MS),
                #[allow(clippy::cast_precision_loss)] // a handful of days
                time_percent: (k * DAY_MS) as f64 / length as f64 * 100.0,
            })
            .collect()
    } else {
        Vec::new()
    };
    let limit_ref = LimitRef::new(limit.kind.clone(), limit.scope.clone());
    let mut points: Vec<ChartPoint> =
        window_readings(&limit_ref, resets_at, length, snapshots, now)
            .map(|o| ChartPoint {
                at: o.observed_at,
                percent: o.percent,
                source: o.source,
            })
            .collect();
    // The panel's value may be newer than the last stored reading (a heartbeat apart).
    if points.last().is_none_or(|p| p.at < limit.observed_at) && limit.observed_at >= start {
        points.push(ChartPoint {
            at: limit.observed_at,
            percent: limit.percent,
            source: limit.source,
        });
    }
    Some(FocusView {
        kind: limit.kind.clone(),
        scope: limit.scope.clone(),
        window_start: start,
        resets_at,
        periods,
        points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::AccountId;
    use crate::limits::view::{ApiStatusView, Freshness, IndexState, Severity, StatuslineState};
    use crate::limits::{SESSION_WINDOW_MS, WEEK_MS};
    use crate::providers::claude::usage_api::{ApiState, PollDiagnostics};

    const HOUR: i64 = 3_600_000;

    fn view(kind: LimitKind, percent: f64, resets_at: i64, at: i64, source: Source) -> LimitView {
        LimitView {
            kind,
            scope: None,
            percent,
            resets_at: Some(Timestamp::from_millis(resets_at)),
            window_start: None,
            severity: Severity::of(percent),
            provider_severity: None,
            source,
            estimate_reason: None,
            observed_at: Timestamp::from_millis(at),
            freshness: Freshness::Fresh,
            projection: None,
        }
    }

    fn snapshot(limits: Vec<LimitView>) -> PanelSnapshot {
        PanelSnapshot {
            account: None,
            limits,
            absent: Vec::new(),
            live_sessions: Vec::new(),
            top_sessions: Vec::new(),
            index_state: IndexState::default(),
            api: ApiStatusView {
                state: ApiState::Off,
                diagnostics: PollDiagnostics::default(),
            },
            statusline: StatuslineState::default(),
            generated_at: Timestamp::from_millis(0),
        }
    }

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
    fn the_weekly_limit_is_the_focus_with_its_periods_and_readings() {
        let reset = 20 * WEEK_MS;
        let start = reset - WEEK_MS;
        let now = start + 50 * HOUR;
        let weekly = view(
            LimitKind::WeeklyAll,
            30.0,
            reset,
            now - 1000,
            Source::Statusline,
        );
        let session = view(
            LimitKind::Session,
            10.0,
            now + HOUR,
            now,
            Source::Statusline,
        );
        let snaps = vec![
            obs(LimitKind::WeeklyAll, 70.0, start, start - HOUR),
            obs(LimitKind::WeeklyAll, 5.0, reset, start + HOUR),
            obs(LimitKind::WeeklyAll, 20.0, reset, start + 30 * HOUR),
        ];
        let d = panel_detail(
            &snapshot(vec![session, weekly]),
            &snaps,
            Timestamp::from_millis(now),
        );
        let focus = d.focus.expect("focus");
        assert_eq!(focus.kind, LimitKind::WeeklyAll);
        assert_eq!(focus.window_start.as_millis(), start);
        let marks: Vec<i64> = focus
            .periods
            .iter()
            .map(|p| p.at.as_millis() - start)
            .collect();
        assert_eq!(marks, (1..7).map(|k| k * 24 * HOUR).collect::<Vec<_>>());
        assert!((focus.periods[0].time_percent - 100.0 / 7.0).abs() < 1e-9);
        // Stored readings of this window, then the panel's newer value.
        let got: Vec<f64> = focus.points.iter().map(|p| p.percent).collect();
        assert_eq!(got, vec![5.0, 20.0, 30.0]);
        assert_eq!(d.pace.len(), 2);
    }

    #[test]
    fn without_a_weekly_reading_the_five_hour_window_is_the_focus_without_periods() {
        let now = 30 * WEEK_MS;
        let session = view(LimitKind::Session, 40.0, now + HOUR, now, Source::Api);
        let estimate = view(LimitKind::WeeklyAll, 0.0, now + HOUR, now, Source::Estimate);
        let d = panel_detail(
            &snapshot(vec![estimate, session]),
            &[],
            Timestamp::from_millis(now),
        );
        let focus = d.focus.expect("focus");
        assert_eq!(focus.kind, LimitKind::Session);
        assert_eq!(
            focus.window_start.as_millis(),
            now + HOUR - SESSION_WINDOW_MS
        );
        assert_eq!(focus.periods, Vec::new());
        assert_eq!(focus.points.len(), 1);
        // The estimate has no pace: it is not a measurement (ADR-044).
        assert_eq!(d.pace.len(), 1);
        assert_eq!(d.pace[0].kind, LimitKind::Session);
    }

    #[test]
    fn pace_warns_ahead_of_time_but_not_in_the_first_seventh() {
        let reset = 40 * WEEK_MS;
        let start = reset - WEEK_MS;
        // Two hours in, 12 % used: far ahead, still quiet.
        let early = start + 2 * HOUR;
        let w = view(LimitKind::WeeklyAll, 12.0, reset, early, Source::Statusline);
        let d = panel_detail(&snapshot(vec![w]), &[], Timestamp::from_millis(early));
        assert!(!d.pace[0].warn);
        assert!(d.pace[0].ahead > 10.0);
        // Two days in (28.6 % of the time), 45 % used.
        let later = start + 48 * HOUR;
        let w = view(LimitKind::WeeklyAll, 45.0, reset, later, Source::Statusline);
        let d = panel_detail(&snapshot(vec![w]), &[], Timestamp::from_millis(later));
        assert!(d.pace[0].warn);
        assert!((d.pace[0].elapsed_percent - 200.0 / 7.0).abs() < 1e-9);
    }

    #[test]
    fn a_passed_reset_gives_no_focus_and_no_pace() {
        let now = 50 * WEEK_MS;
        let past = view(
            LimitKind::WeeklyAll,
            80.0,
            now - 1,
            now - HOUR,
            Source::Statusline,
        );
        let d = panel_detail(&snapshot(vec![past]), &[], Timestamp::from_millis(now));
        assert!(d.focus.is_none());
        assert_eq!(d.pace, Vec::new());
    }
}
