//! Query tests (Step 14b): keyset paging, session trees, impact, the overview, the heat map in
//! local time, the series and the background catch-up of stale attributions.

use super::*;
use crate::attribution::{self, Method};
use crate::calibration::Calibration;
use crate::domain::{
    AccountId, AccountSpec, LimitKind, LimitObservation, LimitRef, MicroUsd, ModelFamily, ModelId,
    MsgKey, ProjectId, ProjectSpec, Provider, SessionId, SessionKind, SessionSpec, SessionTitle,
    Source, Timestamp, TitleSource, Usage, UsageEvent,
};
use crate::store::Store;

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;
/// 2026-11-04T10:00:00Z, a Wednesday.
const T0: i64 = 1_793_786_400_000;

fn ts(ms: i64) -> Timestamp {
    Timestamp::from_millis(ms)
}

fn cal(k: f64, confidence: f64, version: i64) -> Calibration {
    Calibration {
        kind: LimitKind::WeeklyAll,
        scope: None,
        source: Source::Statusline,
        k,
        alpha: std::collections::BTreeMap::new(),
        fitted: vec![ModelFamily::Opus],
        confidence,
        n_obs: 40,
        mad_rel: 0.04,
        drift: None,
        computed_at: ts(T0),
        version,
    }
}

struct Env {
    _dir: tempfile::TempDir,
    store: Store,
    account: AccountId,
    proj_a: ProjectId,
    proj_b: ProjectId,
    opus: ModelId,
    sonnet: ModelId,
    key: std::cell::Cell<u32>,
}

impl Env {
    fn new() -> Self {
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
        let project = |path: &str, name: &str| {
            w.upsert_project(ProjectSpec {
                account_id: account,
                path_norm: path.into(),
                display_name: name.into(),
                display_path: None,
            })
            .expect("project")
        };
        let proj_a = project("d:/work/alpha", "alpha");
        let proj_b = project("d:/work/beta", "beta");
        let opus = w
            .upsert_model("claude-opus-5-5".into(), ModelFamily::Opus)
            .expect("model");
        let sonnet = w
            .upsert_model("claude-sonnet-5-5".into(), ModelFamily::Sonnet)
            .expect("model");
        Self {
            _dir: dir,
            store,
            account,
            proj_a,
            proj_b,
            opus,
            sonnet,
            key: std::cell::Cell::new(0),
        }
    }

    fn session(&self, uuid: &str, project: ProjectId, title: Option<&str>) -> SessionId {
        let mut spec = SessionSpec::new(self.account, uuid, SessionKind::Main);
        spec.project_id = Some(project);
        spec.title = title.map(|t| SessionTitle {
            text: t.into(),
            source: TitleSource::Ai,
        });
        self.store.writer().upsert_session(spec).expect("session")
    }

    fn subagent(&self, uuid: &str, root: SessionId) -> SessionId {
        let mut spec = SessionSpec::new(self.account, uuid, SessionKind::Subagent);
        spec.parent_id = Some(root);
        spec.root_id = Some(root);
        self.store.writer().upsert_session(spec).expect("agent")
    }

    /// One event; `cache_read` tokens let the cache stats see something.
    fn use_(&self, session: SessionId, model: ModelId, at: i64, usd: f64, cache_read: u64) {
        let k = self.key.get() + 1;
        self.key.set(k);
        let mut usage = Usage::default();
        usage.tokens.input = 100;
        usage.tokens.output = 10;
        usage.tokens.cache_read = cache_read;
        #[allow(clippy::cast_possible_truncation)]
        let cost = MicroUsd((usd * 1e6).round() as i64);
        self.store
            .writer()
            .upsert_events(vec![UsageEvent {
                account_id: self.account,
                session_id: session,
                model_id: model,
                ts: ts(at),
                msg_key: MsgKey::from_ids(&format!("m{k}"), &format!("r{k}")),
                usage,
                cost,
            }])
            .expect("event");
    }

    fn reading(&self, at: i64, percent: f64, resets: i64) {
        self.store
            .writer()
            .record_snapshot(
                LimitObservation {
                    account: self.account,
                    kind: LimitKind::WeeklyAll,
                    scope: None,
                    percent,
                    resets_at: Some(ts(resets)),
                    severity: None,
                    source: Source::Statusline,
                    session_uuid: None,
                    observed_at: ts(at),
                },
                Some(WEEK),
            )
            .expect("snapshot");
    }

    fn ctx<'a>(
        &self,
        cals: &'a [Calibration],
        zone: &'a dyn LocalZone,
        now: i64,
    ) -> QueryContext<'a> {
        QueryContext {
            account: self.account,
            now: ts(now),
            calibrations: cals,
            zone,
        }
    }
}

#[test]
fn window_pages_are_stable_while_windows_arrive() {
    let e = Env::new();
    for i in 0..7 {
        e.store
            .writer()
            .insert_estimated_window(
                e.account,
                LimitKind::WeeklyAll,
                ts(T0 + i * WEEK - WEEK),
                ts(T0 + i * WEEK),
            )
            .expect("window");
    }
    let zone = FixedZone::utc();
    let ctx = e.ctx(&[], &zone, T0 + 10 * WEEK);
    let filter = WindowFilter::default();
    let first = list_windows(&e.store, &ctx, &filter, None, Some(3)).expect("page");
    assert_eq!(first.items.len(), 3);
    // A newer window lands between pages; the next page continues where the first stopped.
    e.store
        .writer()
        .insert_estimated_window(
            e.account,
            LimitKind::WeeklyAll,
            ts(T0 + 7 * WEEK),
            ts(T0 + 8 * WEEK),
        )
        .expect("window");
    let mut seen: Vec<i64> = first
        .items
        .iter()
        .map(|w| w.resets_at.as_millis())
        .collect();
    let mut next = first.next;
    while let Some(cursor) = next {
        let page = list_windows(&e.store, &ctx, &filter, Some(&cursor), Some(3)).expect("page");
        seen.extend(page.items.iter().map(|w| w.resets_at.as_millis()));
        next = page.next;
    }
    let expected: Vec<i64> = (0..7).rev().map(|i| T0 + i * WEEK).collect();
    assert_eq!(
        seen, expected,
        "every window once, newest first, none skipped"
    );
    let row = &list_windows(&e.store, &ctx, &filter, None, Some(1))
        .expect("page")
        .items[0];
    assert!(row.estimated);
    assert_eq!(
        row.hit_limit, None,
        "an estimated window's limit hit is unknown"
    );
    assert_eq!(row.basis, None, "not attributed yet");
    let none = list_windows(
        &e.store,
        &ctx,
        &WindowFilter {
            limit: Some(LimitRef::new(LimitKind::Session, None)),
            range: None,
        },
        None,
        None,
    )
    .expect("page");
    assert_eq!(none.items, Vec::new());
}

/// Two sessions in alpha (one with a subagent), one in beta, all inside an open week with
/// readings, attributed with a calibration.
fn week_fixture(e: &Env) -> (SessionId, SessionId, SessionId, SessionId) {
    let main = e.session("aaaaaaaa-1111", e.proj_a, Some("Refactor parser"));
    let agent = e.subagent("agent-x", main);
    let second = e.session("bbbbbbbb-2222", e.proj_a, None);
    let third = e.session("cccccccc-3333", e.proj_b, Some("Docs"));
    e.use_(main, e.opus, T0 + HOUR, 4.0, 1_000);
    e.use_(agent, e.sonnet, T0 + HOUR + 5 * MIN, 2.0, 0);
    e.use_(second, e.opus, T0 + 2 * DAY, 1.0, 0);
    e.use_(third, e.opus, T0 + 3 * DAY, 3.0, 0);
    let resets = T0 + WEEK;
    for (i, p) in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0].into_iter().enumerate() {
        let i = i64::try_from(i).expect("small");
        e.reading(T0 + i * 14 * HOUR, p, resets);
    }
    (main, agent, second, third)
}

#[test]
#[allow(clippy::too_many_lines)] // one fixture, every list option against it
fn sessions_count_subagents_sort_page_and_filter() {
    let e = Env::new();
    let (main, agent, second, third) = week_fixture(&e);
    let cals = [cal(0.5, 0.9, 3)];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + 4 * DAY);
    attribution::refresh(&e.store, e.account, &cals, ctx.now, true, true).expect("refresh");

    let by_cost = list_sessions(
        &e.store,
        &ctx,
        &SessionFilter::default(),
        SessionSort::Cost,
        None,
        None,
        &[],
    )
    .expect("list");
    let ids: Vec<SessionId> = by_cost.items.iter().map(|s| s.id).collect();
    assert_eq!(
        ids,
        vec![main, third, second],
        "subagents are not rows of their own"
    );
    let top = &by_cost.items[0];
    assert_eq!(
        top.cost,
        MicroUsd(6_000_000),
        "the subagent's cost counts for its root"
    );
    assert_eq!(top.subagents, 1);
    assert_eq!(top.models, vec!["claude-opus-5-5", "claude-sonnet-5-5"]);
    assert_eq!(top.name.text, "Refactor parser");
    let untitled = &by_cost.items[2];
    assert!(untitled.name.fallback);
    assert_eq!(untitled.name.text, "alpha · bbbbbbbb");
    assert!(top.weekly_percent.is_some_and(|p| p > 0.0));
    assert!(
        top.impact_basis
            .is_some_and(|b| b.method == Method::Calibrated && !b.approx)
    );

    // Page of one, three times.
    let mut cursor = None;
    let mut paged = Vec::new();
    loop {
        let p = list_sessions(
            &e.store,
            &ctx,
            &SessionFilter::default(),
            SessionSort::Cost,
            cursor.as_deref(),
            Some(1),
            &[],
        )
        .expect("page");
        paged.extend(p.items.iter().map(|s| s.id));
        match p.next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    assert_eq!(paged, ids);

    let filtered = |f: SessionFilter| -> Vec<SessionId> {
        list_sessions(
            &e.store,
            &ctx,
            &f,
            SessionSort::LastActive,
            None,
            None,
            &["cccccccc-3333".to_owned()],
        )
        .expect("list")
        .items
        .iter()
        .map(|s| s.id)
        .collect()
    };
    assert_eq!(
        filtered(SessionFilter {
            project: Some(e.proj_b),
            ..SessionFilter::default()
        }),
        vec![third]
    );
    assert_eq!(
        filtered(SessionFilter {
            family: Some(ModelFamily::Sonnet),
            ..SessionFilter::default()
        }),
        vec![main],
        "the subagent's model counts for the tree"
    );
    assert_eq!(
        filtered(SessionFilter {
            search: Some("pars".into()),
            ..SessionFilter::default()
        }),
        vec![main]
    );
    assert_eq!(
        filtered(SessionFilter {
            search: Some("100%_".into()),
            ..SessionFilter::default()
        }),
        Vec::<SessionId>::new(),
        "LIKE wildcards are literal"
    );
    assert_eq!(
        filtered(SessionFilter {
            live_only: true,
            ..SessionFilter::default()
        }),
        vec![third]
    );
    let range = TimeRange::new(ts(T0 + DAY), ts(T0 + 4 * DAY));
    let mut in_range = filtered(SessionFilter {
        range: Some(range),
        ..SessionFilter::default()
    });
    in_range.sort();
    let mut want = vec![second, third];
    want.sort();
    assert_eq!(in_range, want);

    let detail = get_session(&e.store, &ctx, agent, &[])
        .expect("get")
        .expect("found");
    assert_eq!(
        detail.session.id, main,
        "a subagent id opens its main session"
    );
    assert_eq!(detail.subagents.len(), 1);
    assert_eq!(detail.subagents[0].cost, MicroUsd(2_000_000));
    assert_eq!(detail.path.as_deref(), Some("d:/work/alpha"));
    let timeline_total: i64 = detail.timeline.iter().map(|b| b.cost.0).sum();
    assert_eq!(timeline_total, 6_000_000);
    assert!(detail.cache.hit_ratio.is_some_and(|r| r > 0.0));
    assert!(detail.cache.savings.0 > 0);
    assert_eq!(detail.impact.len(), 1);
    assert_eq!(detail.impact[0].hit_limit, Some(false));
    assert!(
        get_session(&e.store, &ctx, SessionId(9_999), &[])
            .expect("get")
            .is_none()
    );
}

#[test]
fn window_detail_ensures_and_shares_add_up() {
    let e = Env::new();
    week_fixture(&e);
    let cals = [cal(0.5, 0.9, 3)];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + 4 * DAY);
    let id = e.store.limit_windows(e.account).expect("windows")[0].id;
    let d = get_window(&e.store, &ctx, id).expect("get").expect("found");
    let shares: f64 = d.shares.iter().filter_map(|s| s.percent).sum();
    let other = d.other.map_or(0.0, |o| o.percent);
    assert!((shares + other - 5.0).abs() < 0.01, "shares + Other = peak");
    assert!(d.window.open);
    assert_eq!(d.window.sessions, 3);
    assert_ne!(d.points, Vec::new());
    assert!(d.points.iter().all(|p| p.source == Source::Statusline));
    assert!(get_window(&e.store, &ctx, 9_999).expect("get").is_none());
}

#[test]
fn closed_windows_with_an_older_calibration_are_caught_up_and_flagged_meanwhile() {
    let e = Env::new();
    week_fixture(&e);
    let zone = FixedZone::utc();
    let old = [cal(0.5, 0.9, 3)];
    let after = T0 + WEEK + HOUR;
    attribution::refresh(&e.store, e.account, &old, ts(after), false, false).expect("refresh");
    let newer = [cal(0.4, 0.9, 4)];
    let ctx = e.ctx(&newer, &zone, after + HOUR);
    let row = |ctx: &QueryContext<'_>| {
        list_windows(&e.store, ctx, &WindowFilter::default(), None, None)
            .expect("list")
            .items
            .remove(0)
    };
    assert!(row(&ctx).basis.expect("attributed").stale);
    let live =
        attribution::refresh(&e.store, e.account, &newer, ctx.now, true, false).expect("refresh");
    assert_eq!(
        live.recomputed, 0,
        "outside the calibration pass closed windows wait"
    );
    let pass =
        attribution::refresh(&e.store, e.account, &newer, ctx.now, false, true).expect("refresh");
    assert_eq!(pass.recomputed, 1);
    assert!(!row(&ctx).basis.expect("attributed").stale);
}

#[test]
fn projects_list_trend_and_detail() {
    let e = Env::new();
    week_fixture(&e);
    let cals = [cal(0.5, 0.9, 3)];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + 4 * DAY);
    attribution::refresh(&e.store, e.account, &cals, ctx.now, true, true).expect("refresh");
    let page = list_projects(
        &e.store,
        &ctx,
        &ProjectFilter::default(),
        ProjectSort::Cost,
        None,
        None,
        &[],
    )
    .expect("list");
    let names: Vec<&str> = page.items.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "beta"]);
    let alpha = &page.items[0];
    assert_eq!(alpha.sessions, 2);
    assert_eq!(alpha.cost, MicroUsd(7_000_000));
    assert_eq!(alpha.trend.len(), projects::TREND_WEEKS);
    assert_eq!(
        alpha.trend.last(),
        Some(&MicroUsd(7_000_000)),
        "this week is the last slot"
    );
    let searched = list_projects(
        &e.store,
        &ctx,
        &ProjectFilter {
            search: Some("work/be".into()),
            range: None,
            hide_temp: false,
        },
        ProjectSort::LastActive,
        None,
        None,
        &[],
    )
    .expect("list");
    assert_eq!(searched.items.len(), 1);
    let one = list_projects(
        &e.store,
        &ctx,
        &ProjectFilter::default(),
        ProjectSort::Cost,
        None,
        Some(1),
        &[],
    )
    .expect("list");
    let rest = list_projects(
        &e.store,
        &ctx,
        &ProjectFilter::default(),
        ProjectSort::Cost,
        one.next.as_deref(),
        Some(1),
        &[],
    )
    .expect("list");
    assert_eq!(rest.items[0].name, "beta");
    assert!(rest.next.is_none());
    assert_eq!(page.hidden, None, "nothing hidden unless asked");

    let d = get_project(&e.store, &ctx, e.proj_a)
        .expect("get")
        .expect("found");
    assert_eq!(d.top_sessions.len(), 2);
    assert_eq!(d.models.len(), 2);
    assert_eq!(d.weeks.len(), 1);
    assert!(d.weeks[0].percent.is_some());
}

#[test]
fn projects_temp_filter_and_tree() {
    let e = Env::new();
    week_fixture(&e);
    let cals = [cal(0.5, 0.9, 3)];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + 4 * DAY);
    attribution::refresh(&e.store, e.account, &cals, ctx.now, true, true).expect("refresh");

    // Temporary folders: the patterns hide `beta`, and the first page says how many it hid.
    let temp = crate::queries::temp_like_patterns(&["work/BE*".to_owned()], |_| None);
    let hide = ProjectFilter {
        hide_temp: true,
        ..ProjectFilter::default()
    };
    let kept =
        list_projects(&e.store, &ctx, &hide, ProjectSort::Cost, None, None, &temp).expect("list");
    let names: Vec<&str> = kept.items.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["alpha"]);
    assert_eq!(kept.hidden, Some(1));
    let none =
        list_projects(&e.store, &ctx, &hide, ProjectSort::Cost, None, None, &[]).expect("list");
    assert_eq!((none.items.len(), none.hidden), (2, Some(0)));

    // The tree: both projects under one merged folder, totals on it.
    let tree = project_tree(
        &e.store,
        &ctx,
        &ProjectFilter::default(),
        ProjectSort::Cost,
        &[],
    )
    .expect("tree");
    let labels: Vec<&str> = tree.nodes.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(labels, ["d:/work", "alpha", "beta"]);
    assert_eq!(tree.nodes[0].projects, 2);
    assert_eq!(
        tree.nodes[0].cost,
        MicroUsd(tree.nodes[1].cost.0 + tree.nodes[2].cost.0)
    );
    assert_eq!(tree.hidden, None);
    let hidden_tree = project_tree(&e.store, &ctx, &hide, ProjectSort::Cost, &temp).expect("tree");
    assert_eq!(
        hidden_tree.nodes.len(),
        1,
        "one project left, no folder above it"
    );
    assert_eq!(hidden_tree.hidden, Some(1));
}

#[test]
fn heatmap_uses_local_hours_and_counts_active_days() {
    let e = Env::new();
    let s = e.session("dddddddd-4444", e.proj_a, None);
    // Wednesday 10:00 UTC twice a week apart, and Thursday 23:30 UTC.
    e.use_(s, e.opus, T0, 1.0, 0);
    e.use_(s, e.opus, T0 + WEEK, 2.0, 0);
    e.use_(s, e.opus, T0 + DAY + 13 * HOUR + 30 * MIN, 4.0, 0);
    let ist = FixedZone::offset(3 * HOUR);
    let ctx = e.ctx(&[], &ist, T0 + 2 * WEEK);
    let h = get_heatmap(
        &e.store,
        &ctx,
        TimeRange::new(ts(T0 - DAY), ts(T0 + 2 * WEEK)),
        HeatMetric::Cost,
    )
    .expect("heatmap");
    assert_eq!(h.cells.len(), 168);
    let cell = |d: u8, hr: u8| h.cells[usize::from(d) * 24 + usize::from(hr)];
    // 10:00 UTC is 13:00 in Istanbul (Wednesday = 2).
    assert_eq!(cell(2, 13).total, 3_000_000.0);
    assert_eq!(cell(2, 13).active_days, 2);
    assert_eq!(cell(2, 13).mean, 1_500_000.0);
    // Thursday 23:30 UTC is Friday 02:30 local.
    assert_eq!(cell(4, 2).total, 4_000_000.0);
    assert_eq!(h.max_total, 4_000_000.0);
    assert_eq!(h.basis, None);

    let pct = get_heatmap(&e.store, &ctx, TimeRange::ALL, HeatMetric::Percent).expect("heatmap");
    assert!(
        pct.cells.iter().all(|c| c.total == 0.0),
        "no calibration, no percent"
    );
    assert_eq!(pct.basis.map(|b| b.method), Some(Method::CostOnly));
    let cals = [cal(0.5, 0.9, 3)];
    let ctx = e.ctx(&cals, &ist, T0 + 2 * WEEK);
    let pct = get_heatmap(&e.store, &ctx, TimeRange::ALL, HeatMetric::Percent).expect("heatmap");
    assert!(
        (pct.cells[2 * 24 + 13].total - 1.5).abs() < 1e-9,
        "k · cost: 0.5 × $3"
    );
    assert!(pct.basis.is_some_and(|b| b.approx));
}

#[test]
fn heatmap_follows_dst_changes() {
    let e = Env::new();
    let s = e.session("eeeeeeee-5555", e.proj_a, None);
    let before = Timestamp::parse_iso("2026-03-28T12:00:00Z")
        .expect("ts")
        .as_millis();
    let after = Timestamp::parse_iso("2026-03-30T12:00:00Z")
        .expect("ts")
        .as_millis();
    e.use_(s, e.opus, before, 1.0, 0);
    e.use_(s, e.opus, after, 1.0, 0);
    let cet = FixedZone {
        base_ms: HOUR,
        dst_ms: HOUR,
        dst: vec![(
            Timestamp::parse_iso("2026-03-29T01:00:00Z").expect("ts"),
            Timestamp::parse_iso("2026-10-25T01:00:00Z").expect("ts"),
        )],
    };
    let ctx = e.ctx(&[], &cet, after + DAY);
    let h = get_heatmap(
        &e.store,
        &ctx,
        TimeRange::new(ts(before - DAY), ts(after + DAY)),
        HeatMetric::Cost,
    )
    .expect("heatmap");
    // Saturday 12:00 UTC = 13:00 CET; Monday 12:00 UTC = 14:00 CEST.
    assert_eq!(h.cells[5 * 24 + 13].total, 1_000_000.0);
    assert_eq!(h.cells[14].total, 1_000_000.0);
}

#[test]
fn series_bands_and_markers() {
    let e = Env::new();
    let (main, ..) = week_fixture(&e);
    let cals = [cal(0.5, 0.9, 3)];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + 4 * DAY);
    attribution::refresh(&e.store, e.account, &cals, ctx.now, true, true).expect("refresh");
    let limit = LimitRef::new(LimitKind::WeeklyAll, None);
    let s = get_timeseries(
        &e.store,
        &ctx,
        &limit,
        TimeRange::new(ts(T0 - DAY), ts(T0 + 5 * DAY)),
        Some(4),
    )
    .expect("series");
    assert_eq!(s.points.len(), 4, "thinned to the requested size");
    assert!(s.points.windows(2).all(|w| w[0].at <= w[1].at));
    assert_eq!(s.windows.len(), 1);
    assert_eq!(s.windows[0].hit_limit, Some(false));
    assert_eq!(s.markers.len(), 3);
    let m = s
        .markers
        .iter()
        .find(|m| m.session_id == main)
        .expect("marker");
    assert!(m.percent.is_some());
    assert!(m.end > m.start);
}

#[test]
fn overview_uses_the_open_week_and_the_plan_fee() {
    let e = Env::new();
    let (main, _, _, third) = week_fixture(&e);
    // Last week's usage, for the comparison (no window: rolling fallback is not used).
    let old = e.session("ffffffff-6666", e.proj_b, None);
    e.use_(old, e.opus, T0 - 2 * DAY, 2.0, 0);
    let cals = [cal(0.5, 0.9, 3)];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + 4 * DAY);
    let plan = PlanInput {
        subscription: Some("max"),
        tier: Some("default_claude_max_20x"),
        user_fee_usd: None,
    };
    let o = get_overview(&e.store, &ctx, Vec::new(), Vec::new(), &plan).expect("overview");
    assert!(o.week.window_id.is_some());
    assert_eq!(o.week.starts_at, ts(T0));
    assert_eq!(o.cost, MicroUsd(10_000_000), "the old session is last week");
    assert_eq!(o.previous_cost, None, "no earlier window to compare with");
    assert_eq!(o.tokens.tokens.input, 400);
    assert_eq!(o.models.len(), 2);
    assert_eq!(o.top_sessions[0].session_id, main);
    assert_eq!(o.top_sessions[0].daily.len(), 5);
    assert!(o.top_sessions.iter().any(|s| s.session_id == third));
    assert_eq!(o.top_projects[0].project.name, "alpha");
    assert!(o.basis.is_some_and(|b| b.method == Method::Calibrated));
    assert_eq!(
        o.series.len(),
        1,
        "no 5-hour readings, only the weekly series"
    );
    assert!(o.calibration.is_some());
    assert_eq!(o.plan.plan.as_deref(), Some("Ultra"));
    assert_eq!(o.plan.fee, Some(MicroUsd(200_000_000)));
    assert_eq!(o.plan.fee_source, Some(PlanFeeSource::Table));
    assert_eq!(
        o.plan.month_start,
        Timestamp::parse_iso("2026-11-01T00:00:00Z").expect("ts")
    );
    assert_eq!(o.plan.month_cost, MicroUsd(12_000_000));
    assert!((o.plan.ratio.expect("ratio") - 0.06).abs() < 1e-9);
    // T0 + 4 days is 2026-11-08 10:00 UTC: 7 days 10 hours of a 30-day month.
    assert!((o.plan.month_elapsed - (7.0 + 10.0 / 24.0) / 30.0).abs() < 1e-9);
    assert_eq!(o.plan.previous_month_cost, None, "no usage in October");
    assert_eq!(o.plan.previous_ratio, None);

    let october = e.session("eeeeeeee-5555", e.proj_b, None);
    e.use_(october, e.opus, T0 - 10 * DAY, 50.0, 0);
    let o = get_overview(&e.store, &ctx, Vec::new(), Vec::new(), &plan).expect("overview");
    assert_eq!(o.plan.previous_month_cost, Some(MicroUsd(50_000_000)));
    assert!((o.plan.previous_ratio.expect("previous ratio") - 0.25).abs() < 1e-9);

    let user = PlanInput {
        user_fee_usd: Some(120.0),
        ..plan
    };
    let o = get_overview(&e.store, &ctx, Vec::new(), Vec::new(), &user).expect("overview");
    assert_eq!(o.plan.fee, Some(MicroUsd(120_000_000)));
    assert_eq!(o.plan.fee_source, Some(PlanFeeSource::User));
}

#[test]
fn overview_compares_with_the_same_time_into_last_week() {
    let e = Env::new();
    week_fixture(&e);
    // Last week's window, $1 a day.
    let old = e.session("ffffffff-6666", e.proj_b, None);
    for d in 0..7 {
        e.use_(old, e.opus, T0 - WEEK + d * DAY + HOUR, 1.0, 0);
    }
    e.reading(T0 - WEEK + HOUR, 1.0, T0);
    let zone = FixedZone::utc();
    let ctx = e.ctx(&[], &zone, T0 + 4 * DAY);
    let o = get_overview(
        &e.store,
        &ctx,
        Vec::new(),
        Vec::new(),
        &PlanInput::default(),
    )
    .expect("overview");
    assert_eq!(o.previous_cost, Some(MicroUsd(7_000_000)));
    assert_eq!(
        o.previous_cost_to_date,
        Some(MicroUsd(4_000_000)),
        "four days into this week: the first four days of last week"
    );
    // This week: $10 against $4 → +150 %.
    assert!((o.change.expect("change") - 1.5).abs() < 1e-9);
}

#[test]
fn overview_without_a_weekly_window_uses_the_last_seven_days() {
    let e = Env::new();
    let s = e.session("gggggggg-7777", e.proj_a, None);
    e.use_(s, e.opus, T0 - 10 * DAY, 5.0, 0);
    e.use_(s, e.opus, T0 - DAY, 1.0, 0);
    let zone = FixedZone::utc();
    let ctx = e.ctx(&[], &zone, T0);
    let o = get_overview(
        &e.store,
        &ctx,
        Vec::new(),
        Vec::new(),
        &PlanInput::default(),
    )
    .expect("overview");
    assert_eq!(o.week.window_id, None);
    assert_eq!(o.cost, MicroUsd(1_000_000));
    assert_eq!(o.previous_cost, Some(MicroUsd(5_000_000)));
    assert_eq!(
        o.previous_cost_to_date,
        Some(MicroUsd(5_000_000)),
        "the rolling week is whole: to date is the whole previous week"
    );
    assert_eq!(o.top_sessions[0].percent, None, "no window, no percentage");
    assert_eq!(o.plan.fee, None);
    assert_eq!(o.series, Vec::new());
}

#[test]
fn basis_flags_follow_method_and_confidence() {
    let b = Basis::new(Method::Calibrated, Some(0.8), false);
    assert!(!b.approx && !b.dim);
    let b = Basis::new(Method::Calibrated, Some(0.2), false);
    assert!(!b.approx && b.dim);
    let b = Basis::new(Method::Proportional, Some(0.8), true);
    assert!(b.approx && b.dim && b.stale);
    let w = Basis::weakest([
        Basis::new(Method::Calibrated, Some(0.9), false),
        Basis::new(Method::Estimated, Some(0.7), false),
    ])
    .expect("some");
    assert_eq!(w.method, Method::Estimated);
    assert_eq!(w.confidence, Some(0.7));
}

#[test]
fn cursors_round_trip_and_bad_ones_start_over() {
    let c = Cursor {
        key: 1_793_786_400_000.0,
        id: 42,
    };
    assert_eq!(Cursor::decode(Some(&c.encode())), Some(c));
    let f = Cursor { key: 3.25, id: 7 };
    assert_eq!(Cursor::decode(Some(&f.encode())), Some(f));
    assert_eq!(Cursor::decode(Some("garbage")), None);
    assert_eq!(Cursor::decode(None), None);
    assert_eq!(like_pattern("a%b_c\\"), "%a\\%b\\_c\\\\%");
}

#[test]
fn session_detail_lists_cwds_efforts_fast_mode_and_a_deleted_transcript() {
    let e = Env::new();
    let main = e.session("s-attrs", e.proj_a, Some("attrs"));
    let agent = e.subagent("agent-attrs", main);
    e.use_(main, e.opus, T0, 1.0, 0);
    e.use_(agent, e.opus, T0 + HOUR, 1.0, 0);
    let attr = |sid: SessionId, kind: &str, value: &str, first: i64, last: i64| {
        let (kind, value) = (kind.to_owned(), value.to_owned());
        e.store
            .writer()
            .call(move |c| {
                c.execute(
                    "INSERT INTO session_attrs VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![sid.0, kind, value, first, last],
                )?;
                Ok(())
            })
            .expect("attr");
    };
    attr(main, "cwd", "D:/work/alpha", T0, T0 + 2 * HOUR);
    attr(main, "cwd", "D:/work/alpha/sub", T0 + HOUR, T0 + HOUR);
    attr(main, "effort", "high", T0, T0 + HOUR);
    attr(agent, "effort", "xhigh", T0 + HOUR, T0 + 3 * HOUR);
    attr(main, "unknown", "ignored", T0, T0);
    let file = |path: &str, deleted: Option<i64>| {
        let (path, account) = (path.to_owned(), e.account.0);
        e.store
            .writer()
            .call(move |c| {
                c.execute(
                    "INSERT INTO source_files (account_id, path, kind, size, mtime, offset,
                                               session_id, deleted_at)
                     VALUES (?1, ?2, 'main', 1, 1, 1, ?3, ?4)",
                    rusqlite::params![account, path, main.0, deleted],
                )?;
                Ok(())
            })
            .expect("file");
    };
    let cals = [];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0 + DAY);
    let detail = || {
        get_session(&e.store, &ctx, main, &[])
            .expect("get")
            .expect("found")
    };

    let d = detail();
    let paths: Vec<&str> = d.cwds.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(
        paths,
        ["D:/work/alpha", "D:/work/alpha/sub"],
        "first used first"
    );
    assert_eq!(
        d.efforts,
        ["xhigh", "high"],
        "latest first, the subagent's included"
    );
    assert!(!d.fast);
    assert!(
        !d.session.transcript_deleted,
        "never indexed is not deleted"
    );

    attr(agent, "speed", "fast", T0 + HOUR, T0 + HOUR);
    file("a.jsonl", None);
    file("b.jsonl", Some(T0 + DAY));
    let d = detail();
    assert!(d.fast);
    assert!(!d.session.transcript_deleted, "one file is still there");

    e.store
        .writer()
        .call(|c| {
            c.execute("UPDATE source_files SET deleted_at = 1", [])?;
            Ok(())
        })
        .expect("delete");
    assert!(detail().session.transcript_deleted);
    let page = list_sessions(
        &e.store,
        &ctx,
        &SessionFilter::default(),
        SessionSort::LastActive,
        None,
        None,
        &[],
    )
    .expect("list");
    assert!(
        page.items
            .iter()
            .any(|r| r.id == main && r.transcript_deleted)
    );
}

#[test]
fn session_dirs_are_the_tree_cwds_and_a_resolved_project_path() {
    let e = Env::new();
    let main = e.session("s-dirs", e.proj_a, None);
    let agent = e.subagent("agent-dirs", main);
    e.store
        .writer()
        .call(move |c| {
            c.execute(
                "INSERT INTO session_attrs VALUES (?1, 'cwd', 'D:/work/alpha/sub', 1, 2)",
                [agent.0],
            )?;
            Ok(())
        })
        .expect("attr");
    let cals = [];
    let zone = FixedZone::utc();
    let ctx = e.ctx(&cals, &zone, T0);
    assert_eq!(
        session_dirs(&e.store, &ctx, agent).expect("dirs"),
        ["D:/work/alpha/sub", "d:/work/alpha"]
    );
    assert_eq!(
        session_dirs(&e.store, &ctx, SessionId(9_999)).expect("dirs"),
        Vec::<String>::new()
    );
    let unresolved = e
        .store
        .writer()
        .upsert_project(ProjectSpec {
            account_id: e.account,
            path_norm: "unresolved:C--x".into(),
            display_name: "x".into(),
            display_path: None,
        })
        .expect("project");
    let lone = e.session("s-lone", unresolved, None);
    assert_eq!(
        session_dirs(&e.store, &ctx, lone).expect("dirs"),
        Vec::<String>::new()
    );
}
