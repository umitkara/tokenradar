use super::*;
use crate::domain::{AccountId, LimitKind, Scope};

const S: Source = Source::Statusline;
const A: Source = Source::Api;

fn t(ms: i64) -> Timestamp {
    Timestamp::from_millis(ms)
}

/// The fused value after `readings`, folded in order as the newest each time.
fn fused(readings: &[(Source, f64)]) -> f64 {
    let points: Vec<(Timestamp, Source, f64)> = readings
        .iter()
        .enumerate()
        .map(|(i, &(s, p))| (t(i64::try_from(i).expect("small") * 1000), s, p))
        .collect();
    series(&points).last().expect("a value").1
}

#[test]
fn each_source_bounds_the_true_value_its_own_way() {
    let iv = |s, p| interval(s, p).expect("real");
    assert_eq!(
        iv(S, 5.0),
        Interval {
            lo: 5.0,
            hi: 6.0,
            exact: false
        }
    );
    assert_eq!(
        iv(A, 5.0),
        Interval {
            lo: 4.5,
            hi: 5.5,
            exact: false
        }
    );
    // The API's 0 cannot mean a negative value.
    assert_eq!(iv(A, 0.0).lo, 0.0);
    // Fractions, and Codex, count as exact readings.
    assert!(iv(S, 5.3).exact && iv(A, 5.3).exact);
    assert!(iv(Source::CodexLog, 5.0).exact);
    assert_eq!(interval(Source::Estimate, 5.0), None);
}

#[test]
fn a_lone_reading_shows_as_it_is() {
    assert_eq!(fused(&[(S, 5.0)]), 5.0);
    assert_eq!(fused(&[(A, 6.0)]), 6.0);
    assert_eq!(fused(&[(A, 0.0)]), 0.0);
    assert_eq!(fused(&[(A, 100.0)]), 100.0);
    assert_eq!(fused(&[(S, 7.25)]), 7.25);
}

#[test]
fn equal_readings_agree() {
    assert_eq!(fused(&[(S, 6.0), (A, 6.0)]), 6.0);
    assert_eq!(fused(&[(A, 6.0), (S, 6.0)]), 6.0);
}

#[test]
fn a_statusline_one_below_the_api_shows_the_api_value() {
    // True value in [5.5, 6): `/usage` says 6.
    assert_eq!(fused(&[(A, 6.0), (S, 5.0)]), 6.0);
    assert_eq!(fused(&[(S, 5.0), (A, 6.0)]), 6.0);
    // And stays there while the statusline repeats itself.
    assert_eq!(fused(&[(S, 5.0), (A, 6.0), (S, 5.0), (S, 5.0)]), 6.0);
}

#[test]
fn a_rising_statusline_leads_without_waiting_for_the_api() {
    assert_eq!(fused(&[(A, 6.0), (S, 5.0), (S, 6.0)]), 6.0);
    assert_eq!(fused(&[(A, 6.0), (S, 5.0), (S, 6.0), (S, 7.0)]), 7.0);
}

#[test]
fn a_newer_reading_that_contradicts_the_bound_restarts_it() {
    // The API's 6 means at least 5.5; a statusline 4 means below 5: usage went down.
    assert_eq!(fused(&[(A, 6.0), (S, 4.0)]), 4.0);
    // Statusline 6 means at least 6; an API 5 means below 5.5.
    assert_eq!(fused(&[(S, 6.0), (A, 5.0)]), 5.0);
    // An exact reading below the bound, too.
    assert_eq!(fused(&[(A, 6.0), (S, 5.2)]), 5.2);
    // After the restart the bound grows again from there.
    assert_eq!(fused(&[(A, 6.0), (S, 4.0), (A, 5.0)]), 5.0);
}

#[test]
fn an_older_reading_only_raises_the_bound() {
    let mut b = Bound::start(A, 6.0, t(1_000)).expect("real");
    b.fold(S, 5.0, t(2_000), true);
    // Older than the newest and lower: its upper bound describes the past, no restart.
    assert!(!b.fold(S, 3.0, t(1_500), false));
    assert_eq!(b.value(), 6.0);
    // Older but higher: still a lower bound for now.
    b.fold(S, 7.0, t(1_800), false);
    assert_eq!(b.value(), 7.0);
    // From before the bound started: ignored.
    b.fold(S, 9.0, t(500), false);
    assert_eq!(b.value(), 7.0);
    // Newer and lower: a restart.
    assert!(b.fold(S, 3.0, t(3_000), true));
    assert_eq!((b.value(), b.source()), (3.0, S));
    // Readings from before the restart no longer count.
    b.fold(A, 6.0, t(2_500), false);
    assert_eq!(b.value(), 3.0);
}

#[test]
fn the_bound_remembers_which_source_set_it() {
    let mut b = Bound::start(S, 5.0, t(0)).expect("real");
    b.fold(A, 6.0, t(1), true);
    assert_eq!((b.value(), b.source()), (6.0, A));
    // A tie keeps the source that got there first.
    b.fold(S, 5.0, t(2), true);
    assert_eq!(b.source(), A);
    b.fold(S, 6.0, t(3), true);
    assert_eq!((b.value(), b.source()), (6.0, S));
}

#[test]
fn fractional_values_are_exact() {
    // 5.3 exactly, then an API 6 (at least 5.5): 6.
    assert_eq!(fused(&[(S, 5.3), (A, 6.0)]), 6.0);
    // An exact 5.5 ties the API's lower bound and wins: it is the true value.
    assert_eq!(fused(&[(A, 6.0), (S, 5.5)]), 5.5);
    assert_eq!(fused(&[(A, 6.0), (Source::CodexLog, 5.7)]), 5.7);
}

#[test]
fn estimates_take_no_part() {
    assert_eq!(Bound::start(Source::Estimate, 0.0, t(0)), None);
    let mut b = Bound::start(A, 6.0, t(0)).expect("real");
    assert!(!b.fold(Source::Estimate, 0.0, t(1), true));
    assert_eq!(b.value(), 6.0);
}

#[test]
fn the_series_of_alternating_sources_never_goes_down() {
    let points: Vec<(Timestamp, Source, f64)> =
        [(S, 5.0), (A, 6.0), (S, 5.0), (A, 6.0), (S, 6.0), (S, 7.0)]
            .iter()
            .enumerate()
            .map(|(i, &(s, p))| (t(i64::try_from(i).expect("small")), s, p))
            .collect();
    let values: Vec<f64> = series(&points).iter().map(|p| p.1).collect();
    assert_eq!(values, vec![5.0, 6.0, 6.0, 6.0, 6.0, 7.0]);
}

fn obs(kind: LimitKind, source: Source, percent: f64, resets_at: i64, at: i64) -> LimitObservation {
    LimitObservation {
        account: AccountId(1),
        kind,
        scope: None,
        percent,
        resets_at: Some(t(resets_at)),
        severity: None,
        source,
        session_uuid: None,
        observed_at: t(at),
    }
}

#[test]
fn fuse_latest_combines_the_sources_of_each_limit() {
    let week = 1_000_000;
    let mut scoped = obs(LimitKind::WeeklyScoped, A, 3.0, week, 50);
    scoped.scope = Some(Scope::Model("Fable".into()));
    let latest = vec![
        obs(LimitKind::WeeklyAll, A, 6.0, week, 10),
        obs(LimitKind::WeeklyAll, S, 5.0, week + 2_000, 20),
        scoped.clone(),
    ];
    let got = fuse_latest(&latest);
    assert_eq!(got.len(), 2);
    // The newest leads, the value is combined.
    assert_eq!(
        (got[0].source, got[0].percent, got[0].observed_at),
        (S, 6.0, t(20))
    );
    assert_eq!(got[1], scoped);
}

#[test]
fn fuse_latest_leaves_out_another_window() {
    let latest = vec![
        // Last week's API reading.
        obs(LimitKind::WeeklyAll, A, 90.0, 1_000_000, 10),
        obs(LimitKind::WeeklyAll, S, 2.0, 2_000_000, 20),
    ];
    let got = fuse_latest(&latest);
    assert_eq!((got[0].source, got[0].percent), (S, 2.0));
}
