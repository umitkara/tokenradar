//! Attribution tests (Step 14a): segments, conservation, the Other bucket against a simulator
//! with known external usage, methods, estimated windows and the store round trip.

use std::collections::BTreeMap;

use proptest::prelude::*;

use super::*;
use crate::calibration::intervals::{self, Reading};
use crate::domain::{AccountId, LimitKind, LimitWindow, Source};

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;
const T0: i64 = 1_800_000_000_000;

fn ts(ms: i64) -> Timestamp {
    Timestamp::from_millis(ms)
}

fn edge(at_ms: i64, percent: f64) -> Edge {
    Edge {
        before: ts(at_ms - MIN),
        after: ts(at_ms + MIN),
        percent,
    }
}

fn cal(k_per_usd: f64, alpha: &[(ModelFamily, f64)], confidence: f64) -> Calibration {
    Calibration {
        kind: LimitKind::WeeklyAll,
        scope: None,
        source: Source::Statusline,
        k: k_per_usd,
        alpha: alpha.iter().copied().collect(),
        fitted: alpha
            .iter()
            .map(|a| a.0)
            .chain([ModelFamily::Opus])
            .collect(),
        confidence,
        n_obs: 40,
        mad_rel: 0.04,
        drift: None,
        computed_at: ts(T0),
        version: 7,
    }
}

fn bucket(at_ms: i64, root: i64, family: ModelFamily, usd: f64) -> BucketCost {
    #[allow(clippy::cast_possible_truncation)]
    let micro = (usd * 1e6).round() as i64;
    BucketCost {
        bucket: ts(at_ms).bucket_5m(),
        root: SessionId(root),
        family,
        cost: MicroUsd(micro),
    }
}

fn input<'a>(
    end_ms: i64,
    peak: Option<f64>,
    edges: &'a [Edge],
    buckets: &'a [BucketCost],
    calibration: Option<&'a Calibration>,
) -> Input<'a> {
    Input {
        start: ts(T0),
        end: ts(end_ms),
        peak,
        edges,
        buckets,
        family: None,
        calibration,
    }
}

fn sum(o: &Outcome) -> f64 {
    o.shares.iter().filter_map(|s| s.percent).sum::<f64>() + o.other.unwrap_or(0.0)
}

// --- segments -------------------------------------------------------------------------------

#[test]
fn edges_group_by_slice_and_the_tail_joins_the_last_segment() {
    let edges = [
        edge(T0 + 2 * HOUR, 1.0),
        edge(T0 + 5 * HOUR, 2.0),
        edge(T0 + DAY + HOUR, 3.0),
        edge(T0 + DAY + 9 * HOUR, 5.0),
    ];
    let segs = segments(ts(T0), ts(T0 + 2 * DAY), &edges, 6.0);
    assert_eq!(
        segs,
        vec![
            Segment {
                from: ts(T0),
                to: ts(T0 + 5 * HOUR),
                dp: 2.0
            },
            Segment {
                from: ts(T0 + 5 * HOUR),
                to: ts(T0 + 2 * DAY),
                dp: 4.0
            },
        ]
    );
}

#[test]
fn uncertain_edges_are_spanned_and_no_edges_means_one_segment() {
    let mut vague = edge(T0 + DAY + HOUR, 3.0);
    vague.before = ts(T0 + DAY - 2 * HOUR);
    let edges = [edge(T0 + HOUR, 1.0), vague, edge(T0 + 2 * DAY + HOUR, 4.0)];
    let segs = segments(ts(T0), ts(T0 + 3 * DAY), &edges, 4.0);
    assert_eq!(segs.len(), 2);
    assert_eq!(segs[1].dp, 3.0);

    let segs = segments(ts(T0), ts(T0 + DAY), &[], 7.0);
    assert_eq!(
        segs,
        vec![Segment {
            from: ts(T0),
            to: ts(T0 + DAY),
            dp: 7.0
        }]
    );
}

// --- conservation ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn shares_and_other_add_up_to_the_total(
        steps in proptest::collection::vec((1i64..600, 1u8..4), 0..30),
        usage in proptest::collection::vec((0i64..(7 * 24 * 12), 0i64..5, 0.0f64..3.0, any::<bool>()), 0..200),
        extra in 0.0f64..3.0,
        calibrated in any::<bool>(),
        k in 0.01f64..2.0,
    ) {
        let mut t = T0;
        let mut p = 0.0;
        let mut edges = Vec::new();
        for (gap, dp) in steps {
            t += gap * MIN;
            p += f64::from(dp);
            edges.push(edge(t, p));
        }
        let end = T0 + WEEK;
        let buckets: Vec<BucketCost> = usage
            .iter()
            .map(|&(b, root, usd, fable)| {
                let family = if fable { ModelFamily::Fable } else { ModelFamily::Opus };
                bucket(T0 + b * 5 * MIN, root, family, usd)
            })
            .collect();
        let c = cal(k, &[(ModelFamily::Fable, 2.0)], 0.9);
        let peak = p + extra;
        let o = attribute(&input(end, Some(peak), &edges, &buckets, calibrated.then_some(&c)));
        let total = o.total.expect("measured window has a total");
        prop_assert!((sum(&o) - total).abs() < 0.01, "Σ {} vs total {}", sum(&o), total);
        prop_assert!((total - peak).abs() < 1e-9);
        prop_assert!(o.other.unwrap_or(0.0) >= -1e-9);
        prop_assert!(o.shares.iter().all(|s| s.percent.unwrap_or(0.0) >= -1e-9));
    }
}

// --- the Other bucket against a simulator ---------------------------------------------------

/// xorshift64* with Box–Muller; deterministic per seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        #[allow(clippy::cast_precision_loss)]
        let v = (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64;
        v / 9_007_199_254_740_992.0
    }

    fn gauss(&mut self) -> f64 {
        let (u1, u2) = (self.next().max(1e-12), self.next());
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

/// A week of local usage in three sessions (busy daytime hours, 4 % noise per bucket around
/// `k`), plus external usage in a share of the hours, read every 5 minutes as whole percents
/// rounded to the nearest point. Returns (Other / total in percent, external share in percent).
fn simulate(seed: u64, external_share: f64) -> (f64, f64) {
    const K: f64 = 0.05; // points per USD
    let mut rng = Rng(seed);
    let mut buckets = Vec::new();
    let mut readings = Vec::new();
    let (mut local, mut external) = (0.0, 0.0);
    let mut true_p = 0.0;
    let mut ext_hour = false;
    for b in 0..(WEEK / (5 * MIN)) {
        let at = T0 + b * 5 * MIN;
        let hour = (b * 5 * MIN / HOUR) % 24;
        if b % 12 == 0 {
            ext_hour = rng.next() < 0.15;
        }
        let busy = (8..22).contains(&hour) && rng.next() < 0.8;
        if busy {
            let usd = 1.0 + rng.next() * 2.0;
            let root = 1 + i64::from(rng.next() < 0.4) + i64::from(rng.next() < 0.2);
            buckets.push(bucket(at, root, ModelFamily::Opus, usd));
            let dp = K * usd * (1.0 + 0.04 * rng.gauss());
            local += dp;
            true_p += dp;
        }
        if ext_hour && external_share > 0.0 {
            // Spread so the expected external total is `external_share` of the week.
            let dp =
                0.75 * external_share / (1.0 - external_share) * (1.0 + 0.3 * rng.gauss()).max(0.0);
            external += dp;
            true_p += dp;
        }
        readings.push(Reading {
            at: ts(at + 4 * MIN),
            percent: true_p.round(),
            resets_at: Some(ts(T0 + WEEK)),
        });
    }
    let edges = intervals::edges(&readings)
        .into_iter()
        .next()
        .map(|(_, e)| e)
        .unwrap_or_default();
    let peak = true_p.round();
    let c = cal(K, &[], 0.9);
    let o = attribute(&input(T0 + WEEK, Some(peak), &edges, &buckets, Some(&c)));
    let total = o.total.expect("total");
    (
        o.other.expect("other") / total * 100.0,
        external / (local + external) * 100.0,
    )
}

#[test]
fn other_catches_external_usage_within_two_points() {
    for seed in 1..=8 {
        for share in [0.0, 0.05, 0.15, 0.30] {
            let (other, truth) = simulate(seed * 7919, share);
            assert!(
                (other - truth).abs() <= 2.0,
                "seed {seed}, share {share}: Other {other:.2} vs external {truth:.2}"
            );
        }
    }
}

// --- methods --------------------------------------------------------------------------------

#[test]
fn local_usage_is_split_by_weighted_cost() {
    // Fable weighs twice: 1 USD of Fable and 2 USD of Opus are equal shares.
    let c = cal(1.0, &[(ModelFamily::Fable, 2.0)], 0.9);
    let edges = [edge(T0 + 2 * HOUR, 4.5)];
    let buckets = [
        bucket(T0 + HOUR, 1, ModelFamily::Fable, 1.0),
        bucket(T0 + HOUR, 2, ModelFamily::Opus, 2.0),
    ];
    let o = attribute(&input(T0 + 2 * HOUR, Some(4.5), &edges, &buckets, Some(&c)));
    assert_eq!(o.method, Method::Calibrated);
    let p: Vec<f64> = o.shares.iter().map(|s| s.percent.expect("pct")).collect();
    // 4.5 shown: 4 used (k · 4 USD), the head's half point of rounding follows the shares.
    assert!(
        (p[0] - 2.25).abs() < 1e-9 && (p[1] - 2.25).abs() < 1e-9,
        "{p:?}"
    );
    assert!(o.other.expect("other").abs() < 1e-9);
}

#[test]
fn usage_beyond_local_cost_goes_to_other() {
    let c = cal(1.0, &[], 0.9);
    let edges = [edge(T0 + 2 * HOUR, 1.0), edge(T0 + 3 * DAY, 11.0)];
    let buckets = [
        bucket(T0 + HOUR, 1, ModelFamily::Opus, 0.5),
        bucket(T0 + 2 * DAY, 1, ModelFamily::Opus, 4.0),
    ];
    let o = attribute(&input(T0 + 3 * DAY, Some(11.0), &edges, &buckets, Some(&c)));
    // Second segment: Δp 10 against k · 4 USD = 4 → 6 points from elsewhere.
    assert!((o.other.expect("other") - 6.0).abs() < 1e-9, "{o:?}");
    assert!((sum(&o) - 11.0).abs() < 1e-9);
}

#[test]
fn low_confidence_splits_proportionally_without_other() {
    let c = cal(1.0, &[], 0.2);
    let buckets = [
        bucket(T0 + HOUR, 1, ModelFamily::Opus, 3.0),
        bucket(T0 + HOUR, 2, ModelFamily::Opus, 1.0),
    ];
    let o = attribute(&input(T0 + DAY, Some(20.0), &[], &buckets, Some(&c)));
    assert_eq!(o.method, Method::Proportional);
    assert_eq!(o.other, None);
    assert_eq!(o.confidence, Some(0.2));
    let p: Vec<f64> = o.shares.iter().map(|s| s.percent.expect("pct")).collect();
    assert_eq!(p, vec![15.0, 5.0]);
}

#[test]
fn unfitted_family_lowers_the_window_confidence() {
    let mut c = cal(1.0, &[], 0.6);
    c.fitted = vec![ModelFamily::Opus];
    let buckets = [
        bucket(T0 + HOUR, 1, ModelFamily::Opus, 1.0),
        bucket(T0 + HOUR, 2, ModelFamily::Fable, 3.0),
    ];
    let o = attribute(&input(T0 + DAY, Some(10.0), &[], &buckets, Some(&c)));
    assert_eq!(o.method, Method::Proportional);
    assert!((o.confidence.expect("conf") - 0.15).abs() < 1e-9);
}

#[test]
fn scoped_limit_counts_only_its_family() {
    let c = cal(1.0, &[], 0.9);
    let edges = [edge(T0 + 2 * HOUR, 2.5)];
    let buckets = [
        bucket(T0 + HOUR, 1, ModelFamily::Fable, 2.0),
        bucket(T0 + HOUR, 2, ModelFamily::Opus, 50.0),
    ];
    let mut i = input(T0 + 2 * HOUR, Some(2.5), &edges, &buckets, Some(&c));
    i.family = Some(ModelFamily::Fable);
    let o = attribute(&i);
    assert_eq!(o.shares.len(), 1);
    assert_eq!(o.shares[0].root, SessionId(1));
    assert_eq!(o.shares[0].cost, MicroUsd(2_000_000));
}

#[test]
fn nothing_local_means_all_other() {
    let c = cal(1.0, &[], 0.9);
    let o = attribute(&input(T0 + DAY, Some(5.0), &[], &[], Some(&c)));
    assert_eq!(o.other, Some(5.0));
    let o = attribute(&input(T0 + DAY, Some(5.0), &[], &[], None));
    assert_eq!((o.method, o.other), (Method::Proportional, Some(5.0)));
}

#[test]
fn windows_without_snapshots_are_estimated_or_cost_only() {
    let c = cal(2.0, &[(ModelFamily::Fable, 2.0)], 0.9);
    let buckets = [
        bucket(T0 + HOUR, 1, ModelFamily::Fable, 1.0),
        bucket(T0 + HOUR, 2, ModelFamily::Opus, 1.0),
    ];
    let o = attribute(&input(T0 + WEEK, None, &[], &buckets, Some(&c)));
    assert_eq!(o.method, Method::Estimated);
    assert_eq!(o.total, Some(6.0));
    assert_eq!(o.other, None);

    let o = attribute(&input(T0 + WEEK, None, &[], &buckets, None));
    assert_eq!((o.method, o.total), (Method::CostOnly, None));
    assert!(o.shares.iter().all(|s| s.percent.is_none()));
}

#[test]
fn cross_check_flags_only_other_well_below_the_breakdown() {
    assert_eq!(cross_check(2.0, 14.0), (true, false));
    assert_eq!(cross_check(5.0, 14.0), (false, false));
    // Claude Code on another computer: Other above the breakdown is no contradiction.
    assert_eq!(cross_check(30.0, 1.0), (false, true));
}

// --- estimated windows ----------------------------------------------------------------------

fn weekly(id: i64, resets_ms: i64) -> LimitWindow {
    LimitWindow {
        id,
        account: AccountId(1),
        kind: LimitKind::WeeklyAll,
        scope: None,
        starts_at: ts(resets_ms - WEEK),
        resets_at: ts(resets_ms),
        peak_percent: Some(10.0),
        hit_limit: false,
        estimated: false,
    }
}

#[test]
fn estimated_weeks_fill_gaps_and_the_time_before_the_first_window() {
    let r = T0 + 10 * WEEK;
    let windows = [weekly(1, r), weekly(2, r + 3 * WEEK)];
    let est = estimated_weekly_windows(&windows, Some(ts(r - 2 * WEEK - DAY)));
    let resets: Vec<i64> = est.iter().map(|w| w.1.as_millis()).collect();
    assert_eq!(
        resets,
        vec![r - 2 * WEEK, r - WEEK, r + WEEK, r + 2 * WEEK],
        "two weeks back (usage starts a day into the first) and the two-week gap"
    );
    assert!(
        est.iter()
            .all(|(s, e)| e.as_millis() - s.as_millis() == WEEK)
    );

    // Nothing to anchor on, or no usage: nothing.
    assert_eq!(estimated_weekly_windows(&[], Some(ts(T0))), vec![]);
    assert_eq!(estimated_weekly_windows(&windows, None), vec![]);
    // Usage starting inside the first window: nothing before it.
    assert_eq!(
        estimated_weekly_windows(&windows[..1], Some(ts(r - DAY))),
        vec![]
    );
}

// --- store round trip -----------------------------------------------------------------------

mod store {
    use super::*;
    use crate::domain::{
        AccountSpec, LimitObservation, MsgKey, Provider, SessionKind, SessionSpec, Usage,
        UsageEvent,
    };
    use crate::store::Store;

    struct Env {
        _dir: tempfile::TempDir,
        store: Store,
        account: AccountId,
        main: SessionId,
        agent: SessionId,
        other: SessionId,
        opus: crate::domain::ModelId,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(dir.path().join("db")).expect("open");
        let w = store.writer();
        let account = w
            .upsert_account(AccountSpec {
                provider: Provider::Claude,
                label: "Personal".into(),
                config_dir: "C:/u/.claude".into(),
                plan: None,
                tier: None,
                environment: "native".into(),
            })
            .expect("account");
        let main = w
            .upsert_session(SessionSpec::new(account, "m", SessionKind::Main))
            .expect("main");
        let mut spec = SessionSpec::new(account, "agent-1", SessionKind::Subagent);
        spec.parent_id = Some(main);
        spec.root_id = Some(main);
        let agent = w.upsert_session(spec).expect("agent");
        let other = w
            .upsert_session(SessionSpec::new(account, "o", SessionKind::Main))
            .expect("other");
        let opus = w
            .upsert_model("claude-opus-5-5".into(), ModelFamily::Opus)
            .expect("model");
        Env {
            _dir: dir,
            store,
            account,
            main,
            agent,
            other,
            opus,
        }
    }

    impl Env {
        fn event(&self, key: u32, session: SessionId, at_ms: i64, usd: f64) -> UsageEvent {
            #[allow(clippy::cast_possible_truncation)]
            let micro = (usd * 1e6) as i64;
            UsageEvent {
                account_id: self.account,
                session_id: session,
                model_id: self.opus,
                ts: ts(at_ms),
                msg_key: MsgKey::from_ids(&format!("m{key}"), &format!("r{key}")),
                usage: Usage::default(),
                cost: MicroUsd(micro),
            }
        }

        fn reading(&self, at_ms: i64, percent: f64, resets_ms: i64) {
            self.store
                .writer()
                .record_snapshot(
                    LimitObservation {
                        account: self.account,
                        kind: LimitKind::WeeklyAll,
                        scope: None,
                        percent,
                        resets_at: Some(ts(resets_ms)),
                        severity: None,
                        source: Source::Statusline,
                        session_uuid: None,
                        observed_at: ts(at_ms),
                    },
                    Some(WEEK),
                )
                .expect("snapshot");
        }
    }

    #[test]
    fn subagents_count_with_their_main_session() {
        let e = env();
        e.store
            .writer()
            .upsert_events(vec![
                e.event(1, e.main, T0 + MIN, 1.0),
                e.event(2, e.agent, T0 + 2 * MIN, 2.0),
                e.event(3, e.other, T0 + 3 * MIN, 0.5),
            ])
            .expect("events");
        let b = e
            .store
            .bucket_costs(e.account, ts(T0 - HOUR), ts(T0 + HOUR))
            .expect("buckets");
        let by: BTreeMap<SessionId, i64> = b.iter().fold(BTreeMap::new(), |mut m, b| {
            *m.entry(b.root).or_default() += b.cost.0;
            m
        });
        assert_eq!(by.get(&e.main), Some(&3_000_000));
        assert_eq!(by.get(&e.other), Some(&500_000));
        assert_eq!(by.len(), 2);
    }

    #[test]
    fn refresh_attributes_finalizes_and_estimates() {
        let e = env();
        let r = T0 + WEEK;
        // Earlier usage, before any snapshot: one estimated week.
        e.store
            .writer()
            .upsert_events(vec![
                e.event(1, e.main, T0 - 3 * DAY, 10.0),
                e.event(2, e.main, T0 + HOUR, 2.0),
                e.event(3, e.other, T0 + 5 * HOUR, 2.0),
            ])
            .expect("events");
        for (i, p) in [0.0, 0.0, 1.0, 1.0, 2.0, 2.0].into_iter().enumerate() {
            e.reading(T0 + i64::try_from(i).expect("small") * 2 * HOUR, p, r);
        }
        let cals = vec![cal(0.5, &[], 0.9)];
        assert_eq!(attribution_fill(&e), 1);
        assert_eq!(attribution_fill(&e), 0, "idempotent");

        let now = ts(T0 + DAY);
        let done = refresh(&e.store, e.account, &cals, now, false).expect("refresh");
        assert_eq!(done.recomputed, 2);
        let windows = e.store.limit_windows(e.account).expect("windows");
        let (est, live): (Vec<_>, Vec<_>) = windows.iter().partition(|w| w.estimated);
        let est_run = e.store.attribution(est[0].id).expect("read").expect("run");
        assert_eq!(est_run.run.method, Method::Estimated);
        assert!(est_run.run.closed);
        assert_eq!(est_run.run.total_percent, Some(5.0));
        let est_window = e
            .store
            .limit_windows(e.account)
            .expect("windows")
            .into_iter()
            .find(|w| w.id == est[0].id)
            .expect("window");
        assert_eq!(
            est_window.peak_percent,
            Some(5.0),
            "estimate becomes the peak"
        );

        let live_run = e.store.attribution(live[0].id).expect("read").expect("run");
        assert_eq!(live_run.run.method, Method::Calibrated);
        assert_eq!(live_run.run.calibration_version, 7);
        assert!(!live_run.run.closed);
        let total: f64 = live_run
            .shares
            .iter()
            .filter_map(|s| s.percent)
            .sum::<f64>()
            + live_run.run.other_percent.unwrap_or(0.0);
        assert!((total - 2.0).abs() < 0.01);

        // No new data: nothing to do. New data within the minute: nothing either.
        let again = refresh(&e.store, e.account, &cals, ts(T0 + DAY + 2 * MIN), false);
        assert_eq!(again.expect("refresh").recomputed, 0);
        let soon = refresh(&e.store, e.account, &cals, ts(T0 + DAY + 30_000), true);
        assert_eq!(soon.expect("refresh").recomputed, 0);
        let later = refresh(&e.store, e.account, &cals, ts(T0 + DAY + 2 * MIN), true);
        assert_eq!(later.expect("refresh").recomputed, 1);

        // After the reset the window is finalized once.
        let after = ts(r + MIN);
        assert_eq!(
            refresh(&e.store, e.account, &cals, after, false)
                .expect("refresh")
                .recomputed,
            1
        );
        let final_run = e.store.attribution(live[0].id).expect("read").expect("run");
        assert!(final_run.run.closed);
        assert_eq!(
            refresh(&e.store, e.account, &cals, ts(r + DAY), true)
                .expect("refresh")
                .recomputed,
            0
        );

        // A new calibration version is applied when the window is viewed.
        let mut newer = cals.clone();
        newer[0].version = 8;
        newer[0].k = 0.25;
        let viewed = ensure(&e.store, live[0].id, e.account, &newer, ts(r + DAY))
            .expect("ensure")
            .expect("window");
        assert_eq!(viewed.run.calibration_version, 8);
        assert!(
            ensure(&e.store, 9999, e.account, &newer, ts(r + DAY))
                .expect("ensure")
                .is_none()
        );
    }

    fn attribution_fill(e: &Env) -> usize {
        fill_estimated(&e.store, e.account).expect("fill")
    }

    #[test]
    fn breakdown_drives_the_cross_check() {
        let e = env();
        let r = T0 + WEEK;
        e.store
            .writer()
            .upsert_events(vec![e.event(1, e.main, T0 + HOUR, 2.0)])
            .expect("events");
        for (i, p) in [0.0, 1.0, 1.0, 2.0].into_iter().enumerate() {
            e.reading(T0 + i64::try_from(i).expect("small") * HOUR, p, r);
        }
        // The API reports the window start a second early.
        e.store
            .writer()
            .insert_breakdown(
                e.account,
                ts(T0 + 3 * HOUR),
                Some(ts(T0 - 1000)),
                vec![
                    ("claude_code".into(), Some(60.0)),
                    ("chat".into(), Some(30.0)),
                    ("cowork".into(), Some(10.0)),
                ],
            )
            .expect("breakdown");
        let done = refresh(
            &e.store,
            e.account,
            &[cal(1.0, &[], 0.9)],
            ts(T0 + 4 * HOUR),
            false,
        )
        .expect("refresh");
        assert_eq!(done.newly_suspect.len(), 1);
        let w = &e.store.limit_windows(e.account).expect("windows")[0];
        let a = e.store.attribution(w.id).expect("read").expect("run");
        assert_eq!(a.run.breakdown_other, Some(40.0));
        assert!(a.run.suspect && !a.run.other_excess);
    }
}
