use super::*;
use crate::domain::{
    AccountSpec, ModelFamily, MsgKey, Provider, SessionKind, SessionSpec, TokenCounts, Usage,
    UsageEvent,
};
use crate::live_sessions::LiveSession;

const T0: i64 = 1_793_000_000_000;
const SEC: i64 = 1000;

fn t(ms: i64) -> Timestamp {
    Timestamp::from_millis(T0 + ms)
}

struct Fx {
    _dir: tempfile::TempDir,
    store: Store,
    account: AccountId,
    sessions: [SessionId; 2],
}

fn fixture() -> Fx {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path().join("t.db")).expect("store");
    let account = store
        .writer()
        .upsert_account(AccountSpec {
            provider: Provider::Claude,
            label: "Claude".into(),
            config_dir: "C:/test/.claude".into(),
            plan: Some("max".into()),
            tier: None,
            environment: "native".into(),
        })
        .expect("account");
    let session = |uuid: &str| {
        store
            .writer()
            .upsert_session(SessionSpec::new(account, uuid, SessionKind::Main))
            .expect("session")
    };
    let sessions = [session("aaaa-0"), session("bbbb-1")];
    Fx {
        _dir: dir,
        store,
        account,
        sessions,
    }
}

impl Fx {
    fn engine(&self, now: Timestamp) -> LimitEngine {
        LimitEngine::new(
            self.store.clone(),
            vec![AccountMeta {
                id: self.account,
                label: "Claude".into(),
                plan: Some("max".into()),
                tier: Some("default_claude_max_20x".into()),
            }],
            false,
            now,
        )
        .expect("engine")
    }

    fn obs(
        &self,
        kind: LimitKind,
        percent: f64,
        resets_at: i64,
        source: Source,
        at: i64,
    ) -> LimitObservation {
        LimitObservation {
            account: self.account,
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

    fn session(&self, percent: f64, source: Source, at: i64) -> LimitObservation {
        self.obs(LimitKind::Session, percent, 3 * 3600 * SEC, source, at)
    }

    fn snapshots(&self) -> Vec<LimitObservation> {
        self.store
            .snapshots_between(self.account, t(-WEEK_MS), t(WEEK_MS * 3))
            .expect("snapshots")
    }

    /// One usage event of `cost` micro-dollars for a session at `at`.
    fn spend(&self, key: u32, session: usize, cost: i64, at: i64) {
        let model = self
            .store
            .writer()
            .upsert_model("claude-opus-5-5".into(), ModelFamily::Opus)
            .expect("model");
        self.store
            .writer()
            .upsert_events(vec![UsageEvent {
                account_id: self.account,
                session_id: self.sessions[session],
                model_id: model,
                ts: t(at),
                msg_key: MsgKey::from_ids(&format!("m{key}"), "r"),
                usage: Usage {
                    tokens: TokenCounts {
                        input: 100,
                        output: 400,
                        cache_read: 10_000,
                        cache_w5m: 500,
                        cache_w1h: 0,
                        thinking: 0,
                    },
                    ..Usage::default()
                },
                cost: MicroUsd(cost),
            }])
            .expect("events");
    }
}

fn current(engine: &mut LimitEngine, now: Timestamp) -> Vec<LimitView> {
    engine.snapshot(now).0.limits
}

#[test]
fn statusline_beats_api_within_thirty_seconds() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    e.ingest(&fx.session(41.3, Source::Statusline, 0), t(0));
    e.ingest(&fx.session(41.0, Source::Api, 10 * SEC), t(10 * SEC));
    let l = &current(&mut e, t(10 * SEC))[0];
    assert_eq!((l.source, l.percent), (Source::Statusline, 41.3));

    // Later than 30 s: the newest reading wins whatever its source.
    e.ingest(&fx.session(43.0, Source::Api, 45 * SEC), t(45 * SEC));
    let l = &current(&mut e, t(45 * SEC))[0];
    assert_eq!((l.source, l.percent), (Source::Api, 43.0));

    // A statusline reading arriving late but within 30 s of the API one still wins.
    e.ingest(&fx.session(43.4, Source::Statusline, 40 * SEC), t(46 * SEC));
    let l = &current(&mut e, t(46 * SEC))[0];
    assert_eq!((l.source, l.percent), (Source::Statusline, 43.4));

    // Every real reading was stored, winner or not.
    assert_eq!(fx.snapshots().len(), 4);
}

#[test]
fn unchanged_readings_are_written_as_a_five_minute_heartbeat() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    for i in 0..=60 {
        // One statusline call every 10 s for 10 minutes, value unchanged.
        e.ingest(
            &fx.session(20.0, Source::Statusline, i * 10 * SEC),
            t(i * 10 * SEC),
        );
    }
    let at: Vec<i64> = fx
        .snapshots()
        .iter()
        .map(|s| (s.observed_at.as_millis() - T0) / SEC)
        .collect();
    assert_eq!(at, vec![0, 300, 600]);
    // A change is written at once.
    e.ingest(
        &fx.session(20.5, Source::Statusline, 610 * SEC),
        t(610 * SEC),
    );
    assert_eq!(fx.snapshots().len(), 4);
    // Sources are deduplicated separately.
    e.ingest(&fx.session(20.0, Source::Api, 615 * SEC), t(615 * SEC));
    assert_eq!(fx.snapshots().len(), 5);
}

#[test]
fn estimates_are_never_stored() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    e.ingest(&fx.session(10.0, Source::Estimate, 0), t(0));
    assert_eq!(fx.snapshots(), Vec::new());
    assert_eq!(current(&mut e, t(0))[0].source, Source::Estimate);
}

#[test]
fn a_new_resets_at_opens_a_new_window() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    let r1 = 3600 * SEC;
    // The API's resets_at jitters by microseconds between calls: same window.
    e.ingest(&fx.obs(LimitKind::Session, 30.0, r1, Source::Api, 0), t(0));
    e.ingest(
        &fx.obs(LimitKind::Session, 55.0, r1 + 942, Source::Api, 120 * SEC),
        t(120 * SEC),
    );
    e.ingest(
        &fx.obs(
            LimitKind::Session,
            100.0,
            r1 - 1000,
            Source::Statusline,
            600 * SEC,
        ),
        t(600 * SEC),
    );
    // Next window, five hours later.
    let r2 = r1 + SESSION_WINDOW_MS;
    e.ingest(
        &fx.obs(LimitKind::Session, 4.0, r2, Source::Api, r1 + 60 * SEC),
        t(r1 + 60 * SEC),
    );
    e.ingest(
        &fx.obs(LimitKind::WeeklyAll, 66.0, WEEK_MS, Source::Api, 0),
        t(0),
    );

    let windows = fx.store.limit_windows(fx.account).expect("windows");
    let summary: Vec<_> = windows
        .iter()
        .map(|w| {
            (
                w.kind.as_str().to_owned(),
                w.resets_at.as_millis() - T0,
                w.resets_at.as_millis() - w.starts_at.as_millis(),
                w.peak_percent,
                w.hit_limit,
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![
            (
                "session".to_owned(),
                r1,
                SESSION_WINDOW_MS,
                Some(100.0),
                true
            ),
            (
                "session".to_owned(),
                r2,
                SESSION_WINDOW_MS,
                Some(4.0),
                false
            ),
            ("weekly_all".to_owned(), WEEK_MS, WEEK_MS, Some(66.0), false),
        ]
    );
}

#[test]
fn a_passed_reset_drops_the_limit_to_an_estimated_zero() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    let reset = 3600 * SEC;
    e.ingest(
        &fx.obs(LimitKind::Session, 80.0, reset, Source::Statusline, 0),
        t(0),
    );
    e.ingest(
        &fx.obs(LimitKind::WeeklyAll, 50.0, reset, Source::Api, 0),
        t(0),
    );
    assert_eq!(e.tick(t(reset - 1)), Vec::new());
    assert_eq!(
        e.next_wake(t(0)),
        Some(t(10 * 60 * SEC)),
        "freshness step first"
    );

    let events = e.tick(t(reset + 1));
    assert_eq!(events.len(), 2);
    assert!(matches!(
        &events[0],
        LimitEvent::Reset { kind: LimitKind::Session, previous_percent, .. } if (*previous_percent - 80.0).abs() < 1e-9
    ));
    let limits = current(&mut e, t(reset + 1));
    assert_eq!(limits[0].source, Source::Estimate);
    assert_eq!(limits[0].estimate_reason, Some(EstimateReason::Reset));
    assert_eq!((limits[0].percent, limits[0].resets_at), (0.0, None));
    // The weekly window repeats a week later.
    assert_eq!(limits[1].resets_at, Some(t(reset + WEEK_MS)));
    // Once only.
    assert_eq!(e.tick(t(reset + 2)), Vec::new());
    // The estimate was not stored.
    assert_eq!(fx.snapshots().len(), 2);

    // A new reading replaces the estimate.
    e.ingest(
        &fx.obs(
            LimitKind::Session,
            3.0,
            reset + SESSION_WINDOW_MS,
            Source::Statusline,
            reset + 60 * SEC,
        ),
        t(reset + 60 * SEC),
    );
    assert_eq!(
        current(&mut e, t(reset + 60 * SEC))[0].source,
        Source::Statusline
    );
}

#[test]
fn resets_missed_while_closed_apply_without_events() {
    let fx = fixture();
    {
        let mut e = fx.engine(t(0));
        e.ingest(&fx.session(70.0, Source::Statusline, 0), t(0));
    }
    let later = t(4 * 3600 * SEC);
    let mut e = fx.engine(later);
    assert_eq!(e.tick(later), Vec::new());
    let l = &current(&mut e, later)[0];
    assert_eq!(
        (l.source, l.estimate_reason),
        (Source::Estimate, Some(EstimateReason::Reset))
    );
}

#[test]
fn the_last_values_survive_a_restart() {
    let fx = fixture();
    {
        let mut e = fx.engine(t(0));
        e.ingest(&fx.session(33.0, Source::Statusline, 0), t(0));
    }
    let mut e = fx.engine(t(60 * SEC));
    let snap = e.snapshot(t(60 * SEC)).0;
    assert_eq!(snap.limits[0].percent, 33.0);
    assert_eq!(snap.statusline.last_seen, Some(t(0)));
    assert_eq!(snap.limits[0].freshness, Freshness::Fresh);
    let snap = e.snapshot(t(20 * 60 * SEC)).0;
    assert_eq!(snap.limits[0].freshness, Freshness::Aging);
    let snap = e.snapshot(t(40 * 60 * SEC)).0;
    assert_eq!(snap.limits[0].freshness, Freshness::Stale);
}

#[test]
fn upward_threshold_crossings_are_reported_once() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    let mut crossed = Vec::new();
    for (i, p) in [70.0, 80.0, 79.0, 85.0, 95.0, 101.0]
        .into_iter()
        .enumerate()
    {
        let at = i64::try_from(i).expect("small") * 60 * SEC;
        for ev in e.ingest(&fx.session(p, Source::Statusline, at), t(at)) {
            if let LimitEvent::ThresholdCrossed { threshold, .. } = ev {
                crossed.push(threshold);
            }
        }
    }
    // 79 → 85 does not cross 75 again: it came from below 75 only once.
    assert_eq!(crossed, vec![75, 90, 100]);
}

#[test]
fn projection_uses_the_observed_slope_and_signals_changes() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    // 1 %/min, reset in 60 min from 50 % → full before the reset.
    let mut events = Vec::new();
    for i in 0..5 {
        let at = i * 60 * SEC;
        #[allow(clippy::cast_precision_loss)]
        let p = 50.0 + i as f64;
        e.ingest(
            &fx.obs(LimitKind::Session, p, 64 * 60 * SEC, Source::Statusline, at),
            t(at),
        );
    }
    let (snap, ev) = e.snapshot(t(4 * 60 * SEC));
    events.extend(ev);
    let p = snap.limits[0].projection.expect("projection");
    assert!((p.rate_per_min - 1.0).abs() < 1e-9);
    assert_eq!(p.confidence, crate::limits::Confidence::High);
    assert!(p.will_hit_before_reset);
    assert_eq!(p.eta_100_at, Some(t(4 * 60 * SEC + 46 * 60 * SEC)));
    assert!(matches!(
        events.as_slice(),
        [LimitEvent::ProjectionChanged {
            will_hit_before_reset: true,
            ..
        }]
    ));
    // Unchanged: no repeat.
    assert_eq!(e.snapshot(t(4 * 60 * SEC)).1, Vec::new());
}

#[test]
fn session_shares_split_the_window_by_cost_and_add_up() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    // Window: resets in 3 h, so it started 2 h ago.
    fx.spend(1, 0, 3_000_000, -3600 * SEC);
    fx.spend(2, 1, 1_000_000, -60 * SEC);
    fx.spend(3, 1, 9_000_000, -3 * 3600 * SEC); // before the window
    e.ingest(&fx.session(40.0, Source::Statusline, 0), t(0));
    e.set_live_sessions(vec![LiveSession {
        account: fx.account,
        session_uuid: "bbbb-1".into(),
        session_id: Some(fx.sessions[1]),
        pid: 42,
        cwd: "C:/work".into(),
        name: Some("refactor".into()),
        status: LiveStatus::Busy,
        started_at: None,
        status_updated_at: None,
    }]);
    let snap = e.snapshot(t(0)).0;

    let top: Vec<_> = snap
        .top_sessions
        .iter()
        .map(|s| (s.session_id, s.share_percent, s.cost.0, s.status))
        .collect();
    assert_eq!(
        top,
        vec![
            (Some(fx.sessions[0]), Some(30.0), 3_000_000, None),
            (
                Some(fx.sessions[1]),
                Some(10.0),
                1_000_000,
                Some(LiveStatus::Busy)
            ),
        ]
    );
    let total: f64 = snap
        .top_sessions
        .iter()
        .filter_map(|s| s.share_percent)
        .sum();
    assert!((total - 40.0).abs() < 1e-9, "shares add up to p_now");

    let live = &snap.live_sessions[0];
    assert_eq!(live.name.as_deref(), Some("refactor"));
    assert_eq!(live.share_percent, Some(10.0));
    // 1000 non-cache-read tokens in the last 5 minutes.
    assert!((live.tokens_per_min - 200.0).abs() < 1e-9);
}

#[test]
fn without_a_measured_window_only_cost_is_shown() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    fx.spend(1, 0, 2_000_000, -60 * SEC);
    let snap = e.snapshot(t(0)).0;
    assert_eq!(snap.limits, Vec::new());
    assert_eq!(snap.top_sessions.len(), 1);
    assert_eq!(snap.top_sessions[0].share_percent, None);
    assert_eq!(snap.top_sessions[0].cost.0, 2_000_000);

    // After a reset the estimate is not a measurement either.
    e.ingest(
        &fx.obs(
            LimitKind::Session,
            50.0,
            -3600 * SEC,
            Source::Statusline,
            -4 * 3600 * SEC,
        ),
        t(-4 * 3600 * SEC),
    );
    e.tick(t(0));
    let snap = e.snapshot(t(0)).0;
    assert_eq!(snap.limits[0].source, Source::Estimate);
    assert!(snap.top_sessions.iter().all(|s| s.share_percent.is_none()));
}

#[test]
fn poll_inputs_summarise_activity_and_resets() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    assert_eq!(e.poll_inputs(fx.account).activity, Activity::None);
    e.ingest(&fx.session(5.0, Source::Statusline, 0), t(0));
    let live = |status| LiveSession {
        account: fx.account,
        session_uuid: "x".into(),
        session_id: None,
        pid: 1,
        cwd: String::new(),
        name: None,
        status,
        started_at: None,
        status_updated_at: None,
    };
    e.set_live_sessions(vec![live(LiveStatus::Idle), live(LiveStatus::Busy)]);
    let inputs = e.poll_inputs(fx.account);
    assert_eq!(inputs.activity, Activity::Busy);
    assert_eq!(inputs.statusline_at, Some(t(0)));
    assert_eq!(inputs.resets, vec![t(3 * 3600 * SEC)]);
    e.set_live_sessions(vec![live(LiveStatus::Idle)]);
    assert_eq!(e.poll_inputs(fx.account).activity, Activity::Idle);
}

#[test]
fn visible_key_ignores_invisible_changes() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    e.ingest(&fx.session(40.2, Source::Statusline, 0), t(0));
    let a = e.snapshot(t(0)).0;
    e.ingest(&fx.session(40.4, Source::Statusline, SEC), t(SEC));
    let b = e.snapshot(t(2 * SEC)).0;
    assert_ne!(a, b);
    assert_eq!(
        a.visible_key(),
        b.visible_key(),
        "40.2 → 40.4 rounds the same"
    );
    e.ingest(&fx.session(40.6, Source::Statusline, 2 * SEC), t(2 * SEC));
    let c = e.snapshot(t(3 * SEC)).0;
    assert_ne!(b.visible_key(), c.visible_key());
}
