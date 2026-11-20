//! Query latency on the synthetic year (Step 14b target: every query ≤ 50 ms p95).
//!
//! The database comes from `examples/gen_synthetic.rs`: `TR_SYNTH_DB` names it, otherwise
//! `%TEMP%/tr-synth.db`, generated on the first run (about two minutes). Before Criterion's
//! own runs, each query runs [`SAMPLES`] times and the p95 is printed and checked.

#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

#[path = "../examples/synthetic/mod.rs"]
mod synthetic;

use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use criterion::{Criterion, criterion_group, criterion_main};
use tokenradar_core::domain::{LimitKind, LimitRef, Scope, SessionId, Timestamp};
use tokenradar_core::queries::{
    self, FixedZone, HeatMetric, PlanInput, ProjectFilter, ProjectSort, QueryContext,
    SessionFilter, SessionSort, TimeRange, WindowFilter,
};
use tokenradar_core::store::Store;

const SAMPLES: usize = 40;
const BUDGET: Duration = Duration::from_millis(50);
const DAY: i64 = synthetic::DAY;

fn database() -> PathBuf {
    let path = std::env::var_os("TR_SYNTH_DB")
        .map_or_else(|| std::env::temp_dir().join("tr-synth.db"), PathBuf::from);
    if !path.exists() {
        eprintln!("generating {} …", path.display());
        synthetic::generate(&path, &synthetic::Params::default());
    }
    path
}

type Query<'a> = Box<dyn Fn() + 'a>;

#[allow(clippy::too_many_lines)] // one closure per query
fn cases<'a>(store: &'a Store, ctx: &'a QueryContext<'a>) -> Vec<(&'static str, Query<'a>)> {
    let now = ctx.now;
    let year = TimeRange::new(Timestamp::from_millis(now.as_millis() - 365 * DAY), now);
    let month = TimeRange::last_days(now, 30);
    let weekly = LimitRef::new(LimitKind::WeeklyAll, None);
    let fable = LimitRef::new(LimitKind::WeeklyScoped, Some(Scope::Model("Fable".into())));
    let window = queries::list_windows(store, ctx, &WindowFilter::default(), None, Some(1))
        .expect("windows")
        .items[0]
        .id;
    let top = queries::list_sessions(
        store,
        ctx,
        &SessionFilter::default(),
        SessionSort::Cost,
        None,
        Some(1),
        &[],
    )
    .expect("sessions")
    .items[0]
        .id;
    // The default temporary-folder patterns, with a made-up %TEMP%.
    let temp = queries::temp_like_patterns(
        &tokenradar_core::config::DEFAULT_TEMP_DIRS.map(str::to_owned),
        |_| Some(r"C:\Users\dev\AppData\Local\Temp".to_owned()),
    );
    let hide_temp = ProjectFilter {
        hide_temp: true,
        ..ProjectFilter::default()
    };
    let busiest = queries::list_projects(
        store,
        ctx,
        &ProjectFilter::default(),
        ProjectSort::Cost,
        None,
        Some(1),
        &[],
    )
    .expect("projects")
    .items[0]
        .id;
    let session_page2 = queries::list_sessions(
        store,
        ctx,
        &SessionFilter::default(),
        SessionSort::LastActive,
        None,
        None,
        &[],
    )
    .expect("sessions")
    .next;
    let plan = PlanInput {
        subscription: Some("max"),
        tier: Some("default_claude_max_20x"),
        user_fee_usd: None,
    };
    vec![
        (
            "list_windows",
            Box::new(move || {
                black_box(
                    queries::list_windows(store, ctx, &WindowFilter::default(), None, None)
                        .expect("q"),
                );
            }),
        ),
        (
            "get_window",
            Box::new(move || {
                black_box(queries::get_window(store, ctx, window).expect("q"));
            }),
        ),
        (
            "list_sessions/last_active",
            Box::new(move || {
                black_box(
                    queries::list_sessions(
                        store,
                        ctx,
                        &SessionFilter::default(),
                        SessionSort::LastActive,
                        None,
                        None,
                        &[],
                    )
                    .expect("q"),
                );
            }),
        ),
        (
            "list_sessions/page2",
            Box::new(move || {
                black_box(
                    queries::list_sessions(
                        store,
                        ctx,
                        &SessionFilter::default(),
                        SessionSort::LastActive,
                        session_page2.as_deref(),
                        None,
                        &[],
                    )
                    .expect("q"),
                );
            }),
        ),
        (
            "list_sessions/cost",
            Box::new(move || {
                black_box(
                    queries::list_sessions(
                        store,
                        ctx,
                        &SessionFilter::default(),
                        SessionSort::Cost,
                        None,
                        None,
                        &[],
                    )
                    .expect("q"),
                );
            }),
        ),
        (
            "list_sessions/impact_month_search",
            Box::new(move || {
                let f = SessionFilter {
                    range: Some(month),
                    search: Some("task 1".into()),
                    ..SessionFilter::default()
                };
                black_box(
                    queries::list_sessions(store, ctx, &f, SessionSort::Impact, None, None, &[])
                        .expect("q"),
                );
            }),
        ),
        (
            "get_session",
            Box::new(move || {
                black_box(queries::get_session(store, ctx, top, &[]).expect("q"));
            }),
        ),
        (
            "get_session_impact",
            Box::new(move || {
                black_box(queries::get_session_impact(store, ctx, SessionId(top.0)).expect("q"));
            }),
        ),
        (
            "list_projects/cost",
            Box::new(move || {
                black_box(
                    queries::list_projects(
                        store,
                        ctx,
                        &ProjectFilter::default(),
                        ProjectSort::Cost,
                        None,
                        None,
                        &[],
                    )
                    .expect("q"),
                );
            }),
        ),
        (
            "list_projects/hide_temp",
            Box::new(move || {
                black_box(
                    queries::list_projects(
                        store,
                        ctx,
                        &hide_temp,
                        ProjectSort::Cost,
                        None,
                        None,
                        &temp,
                    )
                    .expect("q"),
                );
            }),
        ),
        (
            "project_tree/cost",
            Box::new(move || {
                black_box(
                    queries::project_tree(
                        store,
                        ctx,
                        &ProjectFilter::default(),
                        ProjectSort::Cost,
                        &[],
                    )
                    .expect("q"),
                );
            }),
        ),
        (
            "get_project",
            Box::new(move || {
                black_box(queries::get_project(store, ctx, busiest).expect("q"));
            }),
        ),
        (
            "get_timeseries/weekly_year",
            Box::new(move || {
                black_box(queries::get_timeseries(store, ctx, &weekly, year, None).expect("q"));
            }),
        ),
        (
            "get_timeseries/fable_month",
            Box::new(move || {
                black_box(queries::get_timeseries(store, ctx, &fable, month, None).expect("q"));
            }),
        ),
        (
            "get_heatmap/cost_year",
            Box::new(move || {
                black_box(queries::get_heatmap(store, ctx, year, HeatMetric::Cost).expect("q"));
            }),
        ),
        (
            "get_heatmap/percent_year",
            Box::new(move || {
                black_box(queries::get_heatmap(store, ctx, year, HeatMetric::Percent).expect("q"));
            }),
        ),
        (
            "get_overview",
            Box::new(move || {
                black_box(
                    queries::get_overview(store, ctx, Vec::new(), Vec::new(), &plan).expect("q"),
                );
            }),
        ),
    ]
}

fn p95(f: &dyn Fn()) -> (Duration, Duration) {
    f();
    let mut t: Vec<Duration> = (0..SAMPLES)
        .map(|_| {
            let s = Instant::now();
            f();
            s.elapsed()
        })
        .collect();
    t.sort();
    (t[SAMPLES / 2], t[(SAMPLES * 95).div_ceil(100) - 1])
}

fn bench(c: &mut Criterion) {
    let store = Store::open(database()).expect("open");
    let (account, cals) = synthetic::load(&store);
    let zone = FixedZone::offset(synthetic::LOCAL_OFFSET);
    let ctx = QueryContext {
        account,
        now: synthetic::Params::default().end,
        calibrations: &cals,
        zone: &zone,
    };
    let cases = cases(&store, &ctx);
    let mut over = Vec::new();
    println!("{:<36} {:>10} {:>10}", "query", "p50 ms", "p95 ms");
    for (name, f) in &cases {
        let (p50, p95) = p95(f.as_ref());
        println!(
            "{name:<36} {:>10.2} {:>10.2}",
            p50.as_secs_f64() * 1e3,
            p95.as_secs_f64() * 1e3
        );
        if p95 > BUDGET {
            over.push(*name);
        }
    }
    let mut g = c.benchmark_group("queries");
    g.sample_size(20).measurement_time(Duration::from_secs(2));
    for (name, f) in &cases {
        g.bench_function(*name, |b| b.iter(f));
    }
    g.finish();
    assert!(over.is_empty(), "over the 50 ms p95 budget: {over:?}");
}

criterion_group!(benches, bench);
criterion_main!(benches);
