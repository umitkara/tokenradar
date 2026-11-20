//! Step 17a: weekly cycles, the 5-hour strip, window contributions and alerts, the hour
//! profile, limit risk, the model mix trend and the export.

use super::*;
use crate::export::{ExportFormat, ExportRequest, export};
use tokenradar_i18n::Locale;

impl Env {
    fn scoped_reading(&self, at: i64, percent: f64, resets: i64) {
        self.store
            .writer()
            .record_snapshot(
                LimitObservation {
                    account: self.account,
                    kind: LimitKind::WeeklyScoped,
                    scope: Some(crate::domain::Scope::Model("Fable".into())),
                    percent,
                    resets_at: Some(ts(resets)),
                    severity: None,
                    source: Source::Api,
                    session_uuid: None,
                    observed_at: ts(at),
                },
                Some(WEEK),
            )
            .expect("snapshot");
    }

    fn five_hour(&self, at: i64, percent: f64, resets: i64) {
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

#[test]
fn weekly_cycles_have_days_hit_day_projects_and_scoped_columns() {
    let e = Env::new();
    let a = e.session("aaaaaaaa-1111", e.proj_a, Some("Alpha work"));
    let b = e.session("bbbbbbbb-2222", e.proj_b, Some("Beta work"));
    // Window T0 … T0 + WEEK. Day 0: $2 (alpha); day 2: $6 (beta).
    e.use_(a, e.opus, T0 + HOUR, 2.0, 0);
    e.use_(b, e.opus, T0 + 2 * DAY + HOUR, 6.0, 0);
    e.reading(T0 + 2 * HOUR, 20.0, T0 + WEEK);
    e.reading(T0 + 2 * DAY + 2 * HOUR, 100.0, T0 + WEEK);
    e.reading(T0 + 3 * DAY, 100.0, T0 + WEEK);
    e.scoped_reading(T0 + 3 * DAY, 40.0, T0 + WEEK);
    let cals = [cal(10.0, 0.9, 1)];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + 4 * DAY);
    attribution::refresh(&e.store, e.account, &cals, ctx.now, true, true).expect("refresh");

    let weeks = list_weeks(&e.store, &ctx, TimeRange::ALL).expect("weeks");
    assert_eq!(weeks.items.len(), 1);
    let row = &weeks.items[0];
    assert_eq!(row.days.len(), 7);
    assert_eq!(row.days[0].cost, MicroUsd(2_000_000));
    assert!(
        (row.days[0].percent.expect("pct") - 20.0).abs() < 1e-9,
        "k · $2"
    );
    assert!(
        (row.days[2].percent.expect("pct") - 60.0).abs() < 1e-9,
        "k · $6"
    );
    assert_eq!(row.days[1].cost, MicroUsd(0));
    assert!(!row.days[4].future && row.days[5].future, "now is day 4");
    assert_eq!(row.hit_day, Some(2), "first reading at 100 % is on day 2");
    assert_eq!(
        row.projects[0].project.as_ref().map(|p| p.id),
        Some(e.proj_b)
    );
    assert_eq!(row.projects.len(), 2);
    assert_eq!(row.scoped.len(), 1);
    assert_eq!(row.scoped[0].peak_percent, Some(40.0));
    assert_eq!(weeks.scopes.len(), 1);
    assert!(weeks.day_basis.approx);

    // Without a usable calibration the days have cost only.
    let ctx = e.ctx(&[], &zone, T0 + 4 * DAY);
    let weeks = list_weeks(&e.store, &ctx, TimeRange::ALL).expect("weeks");
    assert_eq!(weeks.items[0].days[2].percent, None);
    assert_eq!(weeks.items[0].days[2].cost, MicroUsd(6_000_000));
    assert_eq!(weeks.day_basis.method, Method::CostOnly);
}

#[test]
fn strip_cuts_five_hour_windows_at_local_midnight() {
    let e = Env::new();
    let zone = FixedZone::offset(3 * HOUR);
    let ctx = e.ctx(&[], &zone, T0 + 2 * DAY);
    let empty = window_strip(&e.store, &ctx, TimeRange::ALL).expect("strip");
    assert!(!empty.any, "no 5-hour windows: the strip hides itself");

    // T0 = Wed 10:00 UTC = 13:00 local. Window 19:00–00:00 UTC is 22:00–03:00 local.
    let s = e.session("cccccccc-3333", e.proj_a, Some("Night"));
    e.use_(s, e.opus, T0 + 9 * HOUR + 10 * MIN, 1.0, 0);
    e.five_hour(T0 + 9 * HOUR + 15 * MIN, 30.0, T0 + 14 * HOUR);
    e.five_hour(T0 + 10 * HOUR, 100.0, T0 + 14 * HOUR);
    let strip = window_strip(&e.store, &ctx, TimeRange::ALL).expect("strip");
    assert!(strip.any);
    assert_eq!(strip.windows.len(), 1);
    assert_eq!(
        strip.days.len(),
        3,
        "Wed, Thu and Fri (today), newest first"
    );
    let thu = &strip.days[1];
    let wed = &strip.days[2];
    assert_eq!(wed.segments.len(), 1);
    assert_eq!(wed.segments[0].from_minute, 22 * 60);
    assert_eq!(wed.segments[0].to_minute, 24 * 60);
    assert_eq!(thu.segments[0].from_minute, 0);
    assert_eq!(thu.segments[0].to_minute, 3 * 60);
    assert_eq!(strip.days[0].segments.len(), 0);
    assert_eq!(strip.windows[0].window.hit_limit, Some(true));
}

#[test]
fn window_detail_builds_up_shares_and_lists_alerts() {
    let e = Env::new();
    let a = e.session("aaaaaaaa-1111", e.proj_a, Some("Alpha"));
    let b = e.session("bbbbbbbb-2222", e.proj_b, Some("Beta"));
    e.use_(a, e.opus, T0 + HOUR, 3.0, 0);
    e.use_(b, e.opus, T0 + DAY, 1.0, 0);
    e.use_(a, e.opus, T0 + 2 * DAY, 1.0, 0);
    e.reading(T0 + 2 * DAY + HOUR, 44.0, T0 + WEEK);
    let cals = [cal(10.0, 0.9, 1)];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + 3 * DAY);
    let fired = crate::notifier::Fired {
        rule: crate::notifier::Rule::Threshold,
        kind: "weekly_all".into(),
        scope: None,
        resets_at: ts(T0 + WEEK),
        threshold: Some(75),
        at: ts(T0 + 2 * DAY),
    };
    e.store
        .writer()
        .record_notifications(
            vec![(fired.rule.as_str().to_owned(), fired.dedupe_key(), fired.at)],
            ctx.now,
        )
        .expect("log");
    let id = list_windows(&e.store, &ctx, &WindowFilter::default(), None, None)
        .expect("list")
        .items[0]
        .id;
    let d = get_window(&e.store, &ctx, id)
        .expect("detail")
        .expect("window");
    assert_eq!(d.contrib.unit, ContribUnit::Percent);
    assert_eq!(d.contrib.series.len(), 2, "two sessions, no rest");
    for (series, share) in d.contrib.series.iter().zip(&d.shares) {
        let last = *series.values.last().expect("values");
        assert!(
            (last - share.percent.expect("pct")).abs() < 1e-9,
            "a series ends at its stored share"
        );
        assert!(series.values.windows(2).all(|w| w[1] >= w[0]), "cumulative");
    }
    assert_eq!(d.contrib.at.len(), d.contrib.series[0].values.len());
    assert_eq!(d.alerts.len(), 1);
    assert_eq!(d.alerts[0].threshold, Some(75));
}

#[test]
fn hour_profile_averages_weekdays_and_weekends_separately() {
    let e = Env::new();
    let s = e.session("dddddddd-4444", e.proj_a, None);
    // T0 is a Wednesday; T0 + 3 days a Saturday. Both at 10:00 UTC.
    e.use_(s, e.opus, T0, 5.0, 0);
    e.use_(s, e.opus, T0 + 3 * DAY, 4.0, 0);
    let zone = FixedZone::utc();
    let ctx = e.ctx(&[], &zone, T0 + 7 * DAY);
    let h = get_heatmap(
        &e.store,
        &ctx,
        // Wed 00:00 to the next Wed 00:00.
        TimeRange::new(ts(T0), ts(T0 - 10 * HOUR + 7 * DAY)),
        HeatMetric::Cost,
    )
    .expect("heatmap");
    assert_eq!(h.hours.len(), 24);
    let ten = h.hours[10];
    // Wed–Tue: five weekdays and two weekend days from the first usage on.
    assert!(
        (ten.weekday - 1_000_000.0).abs() < 1e-6,
        "$5 over 5 weekdays"
    );
    assert!(
        (ten.weekend - 2_000_000.0).abs() < 1e-6,
        "$4 over 2 weekend days"
    );
    assert_eq!(h.hours[11].weekday, 0.0);
}

#[test]
fn limit_risk_needs_ten_windows_and_a_clear_span() {
    let e = Env::new();
    let zone = FixedZone::utc();
    let ctx = e.ctx(&[], &zone, T0 + 30 * DAY);
    let none = limit_risk(&e.store, &ctx, TimeRange::ALL).expect("risk");
    assert!(!none.any && !none.enough && none.span.is_none());

    // Nine windows: three at 14:00 that hit, six at 08:00 that do not.
    let day0 = T0 - 10 * HOUR; // Wed 00:00 UTC
    let add = |d: i64, hour: i64, peak: f64| {
        let start = day0 + d * DAY + hour * HOUR;
        e.five_hour(start + 10 * MIN, peak, start + 5 * HOUR);
    };
    for d in 0..3 {
        add(d, 14, 100.0);
    }
    for d in 0..6 {
        add(d, 8, 40.0);
    }
    let few = limit_risk(&e.store, &ctx, TimeRange::ALL).expect("risk");
    assert!(few.any);
    assert_eq!(few.windows, 9);
    assert!(!few.enough, "below the minimum of ten");
    assert!(few.span.is_none());
    // Three more, at 15:00 (two hit, one 92 %).
    add(3, 15, 100.0);
    add(4, 15, 100.0);
    add(5, 15, 92.0);
    let r = limit_risk(&e.store, &ctx, TimeRange::ALL).expect("risk");
    assert!(r.enough);
    assert_eq!(r.hit, 5);
    assert_eq!(r.hours[15].high, 3, "a 92 % window counts as high");
    let span = r.span.expect("span");
    assert!(span.from_hour <= 14 && span.to_hour >= 16, "{span:?}");
}

#[test]
fn model_mix_weeks_follow_the_locale_week_start() {
    let e = Env::new();
    let s = e.session("eeeeeeee-5555", e.proj_a, None);
    // T0 is Wednesday 2026-11-04; T0 + 4 days is Sunday 2026-11-08.
    e.use_(s, e.opus, T0, 3.0, 0);
    e.use_(s, e.sonnet, T0 + 4 * DAY, 1.0, 0);
    let zone = FixedZone::utc();
    let ctx = e.ctx(&[], &zone, T0 + 6 * DAY);
    let monday = model_mix_trend(&e.store, &ctx, TimeRange::ALL, 0).expect("mix");
    assert_eq!(
        monday.families,
        vec![Some(ModelFamily::Opus), Some(ModelFamily::Sonnet)]
    );
    assert_eq!(monday.weeks.len(), 2, "Mon 2 Nov and Mon 9 Nov");
    assert_eq!(
        monday.weeks[0].parts,
        vec![MicroUsd(3_000_000), MicroUsd(1_000_000)]
    );
    assert_eq!(
        monday.weeks[0].starts_at,
        Timestamp::parse_iso("2026-11-02T00:00:00Z").expect("ts")
    );
    let sunday = model_mix_trend(&e.store, &ctx, TimeRange::ALL, 6).expect("mix");
    assert_eq!(sunday.weeks.len(), 2, "Sun 1 Nov and Sun 8 Nov");
    assert_eq!(
        sunday.weeks[1].parts,
        vec![MicroUsd(0), MicroUsd(1_000_000)]
    );
}

#[test]
fn csv_export_is_excel_friendly_and_can_hide_paths() {
    let e = Env::new();
    let s = e.session("ffffffff-6666", e.proj_a, Some("=HYPERLINK(\"x\")"));
    e.use_(s, e.opus, T0, 1.5, 0);
    let zone = FixedZone::offset(3 * HOUR);
    let ctx = e.ctx(&[], &zone, T0 + DAY);
    let req = ExportRequest::Sessions {
        filter: SessionFilter::default(),
    };
    let tr = export(&e.store, &ctx, &req, ExportFormat::Csv, false, Locale::Tr).expect("csv");
    let text = String::from_utf8(tr).expect("utf-8");
    assert!(text.starts_with('\u{feff}'), "BOM for Excel");
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    let header = lines.next().expect("header");
    assert!(header.starts_with("Oturum kimliği;Oturum;Proje;Proje yolu;"));
    let row = lines.next().expect("row");
    assert!(row.contains(";1,50;"), "decimal comma: {row}");
    assert!(row.contains("d:/work/alpha"));
    assert!(
        row.contains("\"'=HYPERLINK(\"\"x\"\")\""),
        "a formula-like title is defused: {row}"
    );

    let hidden = export(&e.store, &ctx, &req, ExportFormat::Csv, true, Locale::En).expect("csv");
    let text = String::from_utf8(hidden).expect("utf-8");
    assert!(!text.contains("d:/work"), "paths hidden");
    assert!(text.contains("Session ID,Session,Project,Started,"));
    assert!(text.contains(",1.50,"));

    let json = export(
        &e.store,
        &ctx,
        &ExportRequest::Daily {
            range: TimeRange::ALL,
        },
        ExportFormat::Json,
        false,
        Locale::Tr,
    )
    .expect("json");
    let v: serde_json::Value = serde_json::from_slice(&json).expect("json");
    assert_eq!(v[0]["date"], "2026-11-04");
    assert_eq!(v[0]["cost_usd"], 1.5);
    assert_eq!(v[0]["sessions"], 1);
    let daily = export(
        &e.store,
        &ctx,
        &ExportRequest::Daily {
            range: TimeRange::ALL,
        },
        ExportFormat::Csv,
        false,
        Locale::Tr,
    )
    .expect("csv");
    let text = String::from_utf8(daily).expect("utf-8");
    assert!(
        text.contains("\r\n2026-11-04;1;1,50;100;10;0;0\r\n"),
        "local day, Turkish numbers: {text}"
    );

    let windows = export(
        &e.store,
        &ctx,
        &ExportRequest::Windows { range: None },
        ExportFormat::Json,
        false,
        Locale::En,
    )
    .expect("json");
    assert_eq!(&windows[..], b"[]\n");
}
