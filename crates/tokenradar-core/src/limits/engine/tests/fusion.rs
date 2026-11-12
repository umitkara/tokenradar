//! The engine with integer readings of both sources (ADR-060).

use super::*;

const MIN_MS: i64 = 60 * SEC;
/// The weekly window of these tests ends three days after `T0`.
const RESET: i64 = 3 * 24 * 3600 * SEC;

impl Fx {
    fn weekly(&self, percent: f64, source: Source, at: i64) -> LimitObservation {
        self.obs(LimitKind::WeeklyAll, percent, RESET, source, at)
    }
}

fn weekly_view(e: &mut LimitEngine, now: i64) -> LimitView {
    current(e, t(now))
        .into_iter()
        .find(|l| l.kind == LimitKind::WeeklyAll)
        .expect("weekly")
}

fn thresholds(events: &[LimitEvent]) -> Vec<u8> {
    events
        .iter()
        .filter_map(|e| match e {
            LimitEvent::ThresholdCrossed { threshold, .. } => Some(*threshold),
            _ => None,
        })
        .collect()
}

#[test]
fn a_statusline_one_below_the_api_shows_the_api_value_and_keeps_it() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    e.ingest(&fx.weekly(6.0, Source::Api, 0), t(0));
    e.ingest(&fx.weekly(5.0, Source::Statusline, MIN_MS), t(MIN_MS));
    let l = weekly_view(&mut e, MIN_MS);
    assert_eq!(
        (l.percent, l.source, l.combined_with),
        (6.0, Source::Statusline, Some(Source::Api))
    );
    // Long after the API reading, the statusline still says 5: still 6, no step down.
    for m in [5, 20, 60, 180] {
        e.ingest(
            &fx.weekly(5.0, Source::Statusline, m * MIN_MS),
            t(m * MIN_MS),
        );
        assert_eq!(weekly_view(&mut e, m * MIN_MS).percent, 6.0, "minute {m}");
    }
    // Each source's raw reading is stored as it came.
    let stored: Vec<(Source, f64)> = fx
        .snapshots()
        .iter()
        .map(|o| (o.source, o.percent))
        .collect();
    assert!(stored.contains(&(Source::Api, 6.0)));
    assert!(stored.contains(&(Source::Statusline, 5.0)));
    assert!(!stored.contains(&(Source::Statusline, 6.0)));
}

#[test]
fn the_api_poll_does_not_make_the_value_jump_back_and_forth() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    let mut seen = Vec::new();
    for (i, (p, s)) in [
        (5.0, Source::Statusline),
        (6.0, Source::Api),
        (5.0, Source::Statusline),
        (5.0, Source::Statusline),
        (6.0, Source::Api),
        (5.0, Source::Statusline),
    ]
    .into_iter()
    .enumerate()
    {
        let at = i64::try_from(i).expect("small") * MIN_MS;
        e.ingest(&fx.weekly(p, s, at), t(at));
        seen.push(weekly_view(&mut e, at).percent);
    }
    assert_eq!(seen, vec![5.0, 6.0, 6.0, 6.0, 6.0, 6.0]);
}

#[test]
fn a_rising_statusline_leads_without_waiting_for_the_api() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    e.ingest(&fx.weekly(6.0, Source::Api, 0), t(0));
    e.ingest(&fx.weekly(5.0, Source::Statusline, MIN_MS), t(MIN_MS));
    e.ingest(
        &fx.weekly(6.0, Source::Statusline, 2 * MIN_MS),
        t(2 * MIN_MS),
    );
    let l = weekly_view(&mut e, 2 * MIN_MS);
    // Both say 6 now: no other source is needed to explain the value.
    assert_eq!((l.percent, l.combined_with), (6.0, None));
    e.ingest(
        &fx.weekly(7.0, Source::Statusline, 3 * MIN_MS),
        t(3 * MIN_MS),
    );
    assert_eq!(weekly_view(&mut e, 3 * MIN_MS).percent, 7.0);
}

#[test]
fn equal_readings_need_no_second_source() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    e.ingest(&fx.weekly(6.0, Source::Statusline, 0), t(0));
    e.ingest(&fx.weekly(6.0, Source::Api, MIN_MS), t(MIN_MS));
    let l = weekly_view(&mut e, MIN_MS);
    assert_eq!(
        (l.percent, l.source, l.combined_with),
        (6.0, Source::Api, None)
    );
}

#[test]
fn a_contradicting_newer_reading_restarts_the_value() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    e.ingest(&fx.weekly(6.0, Source::Api, 0), t(0));
    e.ingest(&fx.weekly(3.0, Source::Statusline, MIN_MS), t(MIN_MS));
    let l = weekly_view(&mut e, MIN_MS);
    assert_eq!((l.percent, l.combined_with), (3.0, None));
}

#[test]
fn a_late_api_reading_within_thirty_seconds_lifts_the_value_but_does_not_lead() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    e.ingest(&fx.weekly(5.0, Source::Statusline, 10 * SEC), t(10 * SEC));
    e.ingest(&fx.weekly(6.0, Source::Api, 20 * SEC), t(20 * SEC));
    let l = weekly_view(&mut e, 20 * SEC);
    assert_eq!(
        (l.percent, l.source, l.observed_at),
        (6.0, Source::Statusline, t(10 * SEC))
    );
}

#[test]
fn a_threshold_is_crossed_once_at_the_89_90_border() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    let mut crossed = Vec::new();
    for (i, (p, s)) in [
        (89.0, Source::Statusline),
        (90.0, Source::Api),
        (89.0, Source::Statusline),
        (89.0, Source::Statusline),
        (90.0, Source::Api),
        (89.0, Source::Statusline),
        (90.0, Source::Statusline),
    ]
    .into_iter()
    .enumerate()
    {
        let at = i64::try_from(i).expect("small") * MIN_MS;
        crossed.extend(thresholds(&e.ingest(&fx.weekly(p, s, at), t(at))));
    }
    assert_eq!(crossed, vec![90]);
}

#[test]
fn a_new_window_starts_from_its_own_readings() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    e.ingest(&fx.weekly(60.0, Source::Api, 0), t(0));
    let next = fx.obs(
        LimitKind::WeeklyAll,
        2.0,
        RESET + WEEK_MS,
        Source::Statusline,
        MIN_MS,
    );
    e.ingest(&next, t(MIN_MS));
    let l = weekly_view(&mut e, MIN_MS);
    assert_eq!((l.percent, l.combined_with), (2.0, None));
    // A late reading of the old window changes nothing.
    e.ingest(&fx.weekly(61.0, Source::Api, 30 * SEC), t(2 * MIN_MS));
    assert_eq!(weekly_view(&mut e, 2 * MIN_MS).percent, 2.0);
}

#[test]
fn after_a_reset_estimate_the_first_reading_starts_afresh() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    e.ingest(&fx.weekly(80.0, Source::Api, 0), t(0));
    e.ingest(&fx.weekly(79.0, Source::Statusline, MIN_MS), t(MIN_MS));
    e.tick(t(RESET + 1));
    let l = weekly_view(&mut e, RESET + 1);
    assert_eq!(
        (l.source, l.percent, l.combined_with),
        (Source::Estimate, 0.0, None)
    );
    let first = fx.obs(
        LimitKind::WeeklyAll,
        1.0,
        RESET + WEEK_MS,
        Source::Statusline,
        RESET + MIN_MS,
    );
    let events = e.ingest(&first, t(RESET + MIN_MS));
    let l = weekly_view(&mut e, RESET + MIN_MS);
    assert_eq!((l.source, l.percent), (Source::Statusline, 1.0));
    assert_eq!(thresholds(&events), Vec::<u8>::new());
}

#[test]
fn the_combined_value_survives_a_restart() {
    let fx = fixture();
    {
        let mut e = fx.engine(t(0));
        e.ingest(&fx.weekly(6.0, Source::Api, 0), t(0));
        // Hours of statusline readings later; the API reading is far older than 30 min.
        e.ingest(
            &fx.weekly(5.0, Source::Statusline, 120 * MIN_MS),
            t(120 * MIN_MS),
        );
    }
    let mut e = fx.engine(t(121 * MIN_MS));
    let l = weekly_view(&mut e, 121 * MIN_MS);
    assert_eq!(
        (l.percent, l.source, l.combined_with),
        (6.0, Source::Statusline, Some(Source::Api))
    );
    // The heartbeat of both sources goes on from the stored readings.
    e.ingest(&fx.weekly(6.0, Source::Api, 122 * MIN_MS), t(122 * MIN_MS));
    let api_rows = fx
        .snapshots()
        .iter()
        .filter(|o| o.source == Source::Api)
        .count();
    assert_eq!(api_rows, 2, "a changed time after more than 5 min is due");
}

#[test]
fn the_combined_value_adds_no_phantom_slope() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    // Ten minutes of the same true value, read by both sources in turn: the step from 5 to
    // 6 at the first API reading is not usage.
    for i in 0..10 {
        let (p, s) = if i % 2 == 0 {
            (5.0, Source::Statusline)
        } else {
            (6.0, Source::Api)
        };
        e.ingest(&fx.weekly(p, s, i * MIN_MS), t(i * MIN_MS));
    }
    let l = weekly_view(&mut e, 10 * MIN_MS);
    let rate = l.projection.map_or(0.0, |p| p.rate_per_min);
    assert!(rate.abs() < 1e-9, "no usage, no slope: {rate}");
}
