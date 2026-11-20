//! Step 17b: what the periodic summaries read from the store.

use super::*;

impl Env {
    fn five_hour_reading(&self, at: i64, percent: f64, resets: i64) {
        self.store
            .writer()
            .record_snapshot(
                LimitObservation {
                    account: self.account,
                    kind: LimitKind::Session,
                    scope: None,
                    percent,
                    resets_at: Some(ts(resets)),
                    severity: None,
                    source: Source::Statusline,
                    session_uuid: None,
                    observed_at: ts(at),
                },
                Some(5 * HOUR),
            )
            .expect("snapshot");
    }
}

/// Last week (`T0 - WEEK … T0`), the week that just reset (`T0 … T0 + WEEK`) and the open one.
fn three_weeks() -> Env {
    let e = Env::new();
    let a = e.session("aaaaaaaa-1111", e.proj_a, Some("Alpha work"));
    let b = e.session("bbbbbbbb-2222", e.proj_b, Some("Beta work"));
    e.use_(a, e.opus, T0 - WEEK + HOUR, 5.0, 0);
    e.reading(T0 - WEEK + 2 * HOUR, 30.0, T0);
    // The closed week: $2 alpha on day 0, $6 beta on day 2, full on day 2.
    e.use_(a, e.opus, T0 + HOUR, 2.0, 0);
    e.use_(b, e.opus, T0 + 2 * DAY + HOUR, 6.0, 0);
    e.reading(T0 + 2 * HOUR, 20.0, T0 + WEEK);
    e.reading(T0 + 2 * DAY + 2 * HOUR, 100.0, T0 + WEEK);
    // Two 5-hour windows, one of them full.
    e.five_hour_reading(T0 + HOUR, 40.0, T0 + 4 * HOUR);
    e.five_hour_reading(T0 + 2 * DAY + 2 * HOUR, 100.0, T0 + 2 * DAY + 4 * HOUR);
    // The open week.
    e.reading(T0 + WEEK + HOUR, 3.0, T0 + 2 * WEEK);
    e
}

#[test]
fn the_weekly_digest_reads_the_week_that_reset_and_the_one_before() {
    let e = three_weeks();
    let cals = [cal(10.0, 0.9, 1)];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + WEEK + 2 * HOUR);
    attribution::refresh(&e.store, e.account, &cals, ctx.now, true, true).expect("refresh");

    let (closed, open) = week_bounds(&e.store, &ctx).expect("bounds");
    let week = closed.expect("a closed week");
    assert_eq!(week.resets_at, ts(T0 + WEEK));
    assert_eq!(open, Some(ts(T0 + 2 * WEEK)));

    let d = weekly_digest(&e.store, &ctx, week)
        .expect("digest")
        .expect("stored");
    assert!(!d.is_empty() && d.attributed());
    assert_eq!(d.window.peak_percent, Some(100.0));
    assert_eq!(d.window.hit_limit, Some(true));
    assert_eq!(d.hit_at, Some(ts(T0 + 2 * DAY + 2 * HOUR)));
    assert_eq!(d.window.cost, MicroUsd(8_000_000));
    assert_eq!(d.five_hour, Some(FiveHourHits { windows: 2, hit: 1 }));
    assert_eq!(
        d.top_project.and_then(|p| p.project).map(|p| p.id),
        Some(e.proj_b)
    );
    let prev = d.previous.expect("the week before");
    assert_eq!(prev.peak_percent, Some(30.0));
    assert_eq!(prev.cost, MicroUsd(5_000_000));
}

#[test]
fn a_week_without_five_hour_windows_says_so() {
    let e = Env::new();
    e.reading(T0 + HOUR, 10.0, T0 + WEEK);
    let cals = [];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + WEEK + HOUR);
    let week = week_bounds(&e.store, &ctx)
        .expect("bounds")
        .0
        .expect("closed");
    let d = weekly_digest(&e.store, &ctx, week)
        .expect("digest")
        .expect("stored");
    assert_eq!(d.five_hour, None);
    assert!(d.previous.is_none());
    assert!(!d.attributed());
}

#[test]
fn the_daily_digest_sums_one_local_day() {
    let e = three_weeks();
    let cals = [cal(10.0, 0.9, 1)];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + 3 * DAY);
    let day = local_day_start(&zone, ts(T0 + 2 * DAY + HOUR));
    let d = daily_digest(
        &e.store,
        &ctx,
        TimeRange::new(day, next_local_day_start(&zone, day)),
    )
    .expect("daily");
    assert_eq!(d.cost, MicroUsd(6_000_000));
    assert_eq!(d.sessions, 1);
    assert!(
        (d.percent.expect("calibrated") - 60.0).abs() < 1e-9,
        "k · $6"
    );
    assert_eq!(d.top_project.map(|p| p.id), Some(e.proj_b));

    // Without a usable calibration: cost only.
    let none = [];
    let ctx = e.ctx(&none, &zone, T0 + 3 * DAY);
    let d = daily_digest(
        &e.store,
        &ctx,
        TimeRange::new(ts(T0 + 5 * DAY), ts(T0 + 6 * DAY)),
    )
    .expect("daily");
    assert!(d.is_empty() && d.percent.is_none());
}
