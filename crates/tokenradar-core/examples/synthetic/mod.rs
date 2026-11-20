//! Synthetic year of usage for query benchmarks (Step 14b): sessions with subagents in
//! projects, ~150k events a month, weekly windows with statusline and API readings that follow
//! `k · Σ α · cost` plus a little outside use, a calibration, and stored attributions.
//!
//! Shared by `examples/gen_synthetic.rs` and `benches/queries.rs` through `#[path]`.
//! Deterministic: the same parameters give the same database. Nothing here is real data.

// Synthetic counts and indices stay far below the cast limits; `generate` is one linear script
// and compares exact synthetic values.
#![allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    dead_code
)]

use std::collections::BTreeMap;
use std::path::Path;

use tokenradar_core::attribution;
use tokenradar_core::calibration::Calibration;
use tokenradar_core::domain::{
    AccountId, AccountSpec, LimitKind, LimitObservation, ModelFamily, ModelId, MsgKey, ProjectId,
    ProjectSpec, Provider, Scope, SessionKind, SessionSpec, SessionTitle, Source, Timestamp,
    TitleSource, TokenCounts, Usage, UsageEvent,
};
use tokenradar_core::pricing::Pricing;
use tokenradar_core::store::Store;

pub const MIN: i64 = 60_000;
pub const HOUR: i64 = 60 * MIN;
pub const DAY: i64 = 24 * HOUR;
pub const WEEK: i64 = 7 * DAY;
/// Local time of the synthetic user (Istanbul, no DST).
pub const LOCAL_OFFSET: i64 = 3 * HOUR;
/// Weekly usage outside Claude Code on top of the logged cost (the Other bucket's truth).
const OUTSIDE: f64 = 0.03;

#[derive(Debug, Clone)]
pub struct Params {
    /// The generated data ends here; queries use it as "now".
    pub end: Timestamp,
    pub days: i64,
    pub sessions: usize,
    pub events_per_month: usize,
    pub projects: usize,
    /// Readings start this many days after the first usage (earlier weeks are estimated).
    pub readings_after_days: i64,
    pub raw_retention_days: i64,
    pub seed: u64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            // 2026-11-18T12:00:00Z: a Wednesday, four hours into a weekly window.
            end: Timestamp::from_millis(1_795_003_200_000),
            days: 365,
            sessions: 3_000,
            events_per_month: 150_000,
            projects: 40,
            readings_after_days: 28,
            raw_retention_days: 90,
            seed: 0x7e57_da7a,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Summary {
    pub account: i64,
    pub roots: usize,
    pub subagents: usize,
    pub events: usize,
    pub raw_kept: u64,
    pub snapshots: usize,
    pub windows: usize,
    pub attributed: usize,
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.unit() * (hi - lo) as f64) as i64
    }

    fn pick<'a, T>(&mut self, weighted: &'a [(T, f64)]) -> &'a T {
        let total: f64 = weighted.iter().map(|w| w.1).sum();
        let mut x = self.unit() * total;
        for (item, w) in weighted {
            if x < *w {
                return item;
            }
            x -= w;
        }
        &weighted[weighted.len() - 1].0
    }
}

const MODELS: [(&str, ModelFamily, f64); 4] = [
    ("claude-opus-5-5", ModelFamily::Opus, 1.0),
    ("claude-fable-5-1", ModelFamily::Fable, 2.0),
    ("claude-sonnet-5-5", ModelFamily::Sonnet, 0.8),
    ("claude-haiku-5-5", ModelFamily::Haiku, 0.5),
];

/// The first weekly reset (Wednesdays 08:00 UTC) at or after `t`.
fn next_reset(t: i64) -> i64 {
    // 2026-11-18T08:00:00Z, a Wednesday.
    const ANCHOR: i64 = 1_794_988_800_000;
    let r = (t - ANCHOR).rem_euclid(WEEK);
    if r == 0 { t } else { t - r + WEEK }
}

/// Writes the synthetic year into a new database at `path` (which must not exist).
pub fn generate(path: &Path, p: &Params) -> Summary {
    let store = Store::open(path).expect("open");
    let w = store.writer();
    let account = w
        .upsert_account(AccountSpec {
            provider: Provider::Claude,
            label: "Synthetic".into(),
            config_dir: "C:/synthetic/.claude".into(),
            plan: Some("max".into()),
            tier: Some("default_claude_max_20x".into()),
            environment: "native".into(),
        })
        .expect("account");
    let mut rng = Rng(p.seed | 1);
    let projects: Vec<ProjectId> = (0..p.projects)
        .map(|i| {
            w.upsert_project(ProjectSpec {
                account_id: account,
                path_norm: format!("d:/synthetic/group-{}/project-{i:02}", i % 5),
                display_name: format!("project-{i:02}"),
            })
            .expect("project")
        })
        .collect();
    let models: Vec<ModelId> = MODELS
        .iter()
        .map(|(name, family, _)| w.upsert_model((*name).into(), *family).expect("model"))
        .collect();

    let end = p.end.as_millis();
    let start = end - p.days * DAY;
    let total_events = (p.events_per_month as f64 * p.days as f64 / 30.44) as usize;
    // A third of main sessions add subagents worth a quarter of their events.
    let mean = total_events as f64 / p.sessions as f64 / 1.09;
    let pricing = Pricing::claude();
    let mut summary = Summary {
        account: account.0,
        ..Summary::default()
    };
    let mut batch: Vec<UsageEvent> = Vec::with_capacity(5_000);
    let mut key: u64 = 0;
    let flush = |batch: &mut Vec<UsageEvent>| {
        if !batch.is_empty() {
            w.upsert_events(std::mem::take(batch)).expect("events");
        }
    };

    // Session starts, oldest first, so events arrive roughly in time order.
    let mut starts: Vec<i64> = (0..p.sessions)
        .map(|_| {
            loop {
                let day = rng.range(0, p.days);
                let local_midnight = start - (start + LOCAL_OFFSET).rem_euclid(DAY) + day * DAY;
                let weekday = ((local_midnight + LOCAL_OFFSET).div_euclid(DAY) + 3).rem_euclid(7);
                if weekday >= 5 && rng.unit() < 0.7 {
                    continue;
                }
                let hour =
                    (14.0 + (rng.unit() + rng.unit() + rng.unit() - 1.5) * 6.0).clamp(7.0, 23.5);
                let t = local_midnight - LOCAL_OFFSET + (hour * HOUR as f64) as i64;
                if t >= start && t < end - HOUR {
                    break t;
                }
            }
        })
        .collect();
    starts.sort_unstable();

    for (i, &t0) in starts.iter().enumerate() {
        let duration = rng.range(15 * MIN, 4 * HOUR).min(end - t0 - MIN);
        let n = (mean * (0.2 + 1.6 * rng.unit())).max(20.0) as usize;
        let main_model = *rng.pick(&[(0usize, 0.6), (1, 0.2), (2, 0.15), (3, 0.05)]);
        let mut spec = SessionSpec::new(
            account,
            format!("{:08x}-synthetic-{i:05}", rng.next() as u32),
            SessionKind::Main,
        );
        spec.project_id = Some(projects[(rng.unit().powi(2) * p.projects as f64) as usize]);
        spec.title = (rng.unit() < 0.8).then(|| SessionTitle {
            text: format!("Synthetic task {i}"),
            source: TitleSource::Ai,
        });
        spec.first_ts = Some(Timestamp::from_millis(t0));
        spec.last_ts = Some(Timestamp::from_millis(t0 + duration));
        spec.git_branch = Some("main".into());
        spec.cc_version = Some("2.5.0".into());
        spec.entrypoint = Some("cli".into());
        let root = w.upsert_session(spec).expect("session");
        summary.roots += 1;
        let mut trees = vec![(root, main_model, n)];
        if rng.unit() < 0.35 {
            let k = rng.range(1, 4) as usize;
            let share = n / 4 / k;
            for j in 0..k {
                let mut s =
                    SessionSpec::new(account, format!("agent-{i:05}-{j}"), SessionKind::Subagent);
                s.parent_id = Some(root);
                s.root_id = Some(root);
                s.title = Some(SessionTitle {
                    text: "Explore".into(),
                    source: TitleSource::Agent,
                });
                let sub = w.upsert_session(s).expect("agent");
                summary.subagents += 1;
                trees.push((sub, *rng.pick(&[(2usize, 0.7), (3, 0.3)]), share.max(5)));
            }
        }
        for (session, model, count) in trees {
            for e in 0..count {
                let at = t0 + (e as i64 * duration) / count as i64 + rng.range(0, 20_000);
                let usage = Usage {
                    tokens: TokenCounts {
                        input: rng.range(1, 50) as u64,
                        output: rng.range(100, 3_000) as u64,
                        cache_read: rng.range(20_000, 150_000) as u64,
                        cache_w5m: rng.range(0, 5_000) as u64,
                        cache_w1h: 0,
                        thinking: 0,
                    },
                    ..Usage::default()
                };
                key += 1;
                batch.push(UsageEvent {
                    account_id: account,
                    session_id: session,
                    model_id: models[model],
                    ts: Timestamp::from_millis(at.min(end - 1)),
                    msg_key: MsgKey::from_ids(&format!("msg_{key:010}"), &format!("req_{key:010}")),
                    usage,
                    cost: pricing.cost(MODELS[model].0, &usage).cost,
                });
                summary.events += 1;
                if batch.len() >= 5_000 {
                    flush(&mut batch);
                }
            }
        }
    }
    flush(&mut batch);

    // Readings: weighted cost per bucket from the rollup, accumulated per weekly window.
    let buckets = store
        .bucket_costs(
            account,
            Timestamp::from_millis(start),
            Timestamp::from_millis(end),
        )
        .expect("buckets");
    let alpha = |f: ModelFamily| MODELS.iter().find(|m| m.1 == f).map_or(1.0, |m| m.2);
    let mut weighted: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for b in &buckets {
        let e = weighted.entry(b.bucket.as_millis()).or_default();
        e.0 += alpha(b.family) * b.cost.0 as f64 / 1e6;
        if b.family == ModelFamily::Fable {
            e.1 += b.cost.0 as f64 / 1e6;
        }
    }
    // k so a median busy week reaches about 60 %.
    let mut weekly: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for (&t, &(all, fable)) in &weighted {
        let e = weekly.entry(next_reset(t + 1)).or_default();
        e.0 += all;
        e.1 += fable;
    }
    let mut totals: Vec<f64> = weekly.values().map(|v| v.0).filter(|v| *v > 0.0).collect();
    totals.sort_by(f64::total_cmp);
    let k = 60.0 / totals.get(totals.len() / 2).copied().unwrap_or(1.0);
    let mut fable: Vec<f64> = weekly.values().map(|v| v.1).filter(|v| *v > 0.0).collect();
    fable.sort_by(f64::total_cmp);
    let k_fable = 40.0 / fable.get(fable.len() / 2).copied().unwrap_or(1.0);

    let readings_from = start + p.readings_after_days * DAY;
    let mut snapshot =
        |kind: LimitKind, scope: Option<Scope>, source: Source, at: i64, pct: f64, resets: i64| {
            w.record_snapshot(
                LimitObservation {
                    account,
                    kind,
                    scope,
                    percent: pct.min(100.0),
                    resets_at: Some(Timestamp::from_millis(resets)),
                    severity: None,
                    source,
                    session_uuid: None,
                    observed_at: Timestamp::from_millis(at),
                },
                Some(WEEK),
            )
            .expect("snapshot");
            summary.snapshots += 1;
        };
    let fable_scope = || Some(Scope::Model("Fable".into()));
    let mut reset = next_reset(readings_from + 1);
    let (mut cum, mut cum_f, mut last_sl, mut last_sl_at) = (0.0, 0.0, -1.0, 0i64);
    let mut t = readings_from - readings_from.rem_euclid(5 * MIN);
    let mut next_api = t;
    while t < end {
        if t >= reset {
            reset += WEEK;
            (cum, cum_f, last_sl) = (0.0, 0.0, -1.0);
        }
        let (all, fab) = weighted.get(&t).copied().unwrap_or_default();
        cum += k * all * (1.0 + OUTSIDE);
        cum_f += k_fable * fab;
        if all > 0.0 {
            // Statusline: on activity, when the rounded value changes or every 30 minutes.
            let shown = cum.round();
            if shown != last_sl || t - last_sl_at >= 30 * MIN {
                snapshot(
                    LimitKind::WeeklyAll,
                    None,
                    Source::Statusline,
                    t + 4 * MIN,
                    shown,
                    reset,
                );
                (last_sl, last_sl_at) = (shown, t);
            }
        }
        if t >= next_api {
            snapshot(
                LimitKind::WeeklyAll,
                None,
                Source::Api,
                t + 2 * MIN,
                cum.round(),
                reset,
            );
            snapshot(
                LimitKind::WeeklyScoped,
                fable_scope(),
                Source::Api,
                t + 2 * MIN,
                cum_f.round(),
                reset,
            );
            next_api = t + 15 * MIN;
            if (t - (reset - WEEK)).rem_euclid(DAY) < 15 * MIN {
                w.insert_breakdown(
                    account,
                    Timestamp::from_millis(t + 2 * MIN),
                    Some(Timestamp::from_millis(reset - WEEK - 1_000)),
                    vec![
                        ("claude_code".into(), Some(97.0)),
                        ("chat".into(), Some(3.0)),
                    ],
                )
                .expect("breakdown");
            }
        }
        t += 5 * MIN;
    }

    let fitted = vec![
        ModelFamily::Opus,
        ModelFamily::Fable,
        ModelFamily::Sonnet,
        ModelFamily::Haiku,
    ];
    let mut cals = vec![
        Calibration {
            kind: LimitKind::WeeklyAll,
            scope: None,
            source: Source::Statusline,
            k: k * (1.0 + OUTSIDE),
            alpha: MODELS
                .iter()
                .filter(|m| m.2 != 1.0)
                .map(|m| (m.1, m.2))
                .collect(),
            fitted,
            confidence: 0.85,
            n_obs: 120,
            mad_rel: 0.04,
            drift: None,
            computed_at: p.end,
            version: 0,
        },
        Calibration {
            kind: LimitKind::WeeklyScoped,
            scope: fable_scope(),
            source: Source::Api,
            k: k_fable,
            alpha: BTreeMap::new(),
            fitted: vec![ModelFamily::Fable],
            confidence: 0.55,
            n_obs: 60,
            mad_rel: 0.06,
            drift: None,
            computed_at: p.end,
            version: 0,
        },
    ];
    for c in &mut cals {
        c.version = w
            .insert_calibration(account, c.clone())
            .expect("calibration");
    }
    w.prune_raw_events(Timestamp::from_millis(end - p.raw_retention_days * DAY))
        .expect("prune");

    attribution::fill_estimated(&store, account).expect("estimated");
    loop {
        let done =
            attribution::refresh(&store, account, &cals, p.end, true, true).expect("refresh");
        summary.attributed += done.recomputed;
        if !done.more {
            break;
        }
    }
    summary.windows = store.limit_windows(account).expect("windows").len();
    summary.raw_kept = store
        .table_counts()
        .expect("counts")
        .into_iter()
        .find(|(t, _)| t == "usage_events")
        .map_or(0, |(_, n)| n);
    w.shutdown();
    summary
}

/// The account and calibrations of a generated database, for queries.
pub fn load(store: &Store) -> (AccountId, Vec<Calibration>) {
    let account = store.accounts().expect("accounts")[0].id;
    let cals = store.latest_calibrations(account).expect("calibrations");
    (account, cals)
}
