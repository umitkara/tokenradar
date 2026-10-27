//! Fake panel states for looking at and measuring the panel (debug builds or the `spike`
//! feature). `TOKENRADAR_FAKE_LIMITS=panel:<scene>` replaces the core like the tray scenarios
//! do, so these runs never touch real data. Every state in design-language.md §9 has a scene.
//!
//! Switching scenes at run time: `invoke('fake_panel_scene', { scene })` from the page (over
//! CDP), which also updates the tray.

use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tokenradar_core::domain::{
    AccountId, LimitKind, MicroUsd, Scope, SessionId, Source, Timestamp,
};
use tokenradar_core::events::CoreEvent;
use tokenradar_core::indexer::{IndexPhase, IndexProgress};
use tokenradar_core::limits::{
    AccountView, ApiStatusView, Confidence, EstimateReason, Freshness, IndexState, LimitView,
    PanelSnapshot, Projection, RateMethod, SchemaDriftView, SessionShareView, Severity,
    StatuslineState,
};
use tokenradar_core::live_sessions::LiveStatus;
use tokenradar_core::providers::claude::usage_api::{ApiState, PollDiagnostics};

use crate::ipc;
use crate::tray::{self, TrayMsg};

pub const SCENES: [&str; 14] = [
    "ok",
    "crit",
    "many",
    "quiet",
    "loading",
    "indexing",
    "no-limits",
    "estimate",
    "token-stale",
    "offline",
    "stale",
    "drift",
    "no-claude",
    "five-hour-only",
];

/// The scene the panel shows; `None` until a scene was set, or for `loading`.
#[derive(Default)]
pub struct FakePanel(Mutex<Option<PanelSnapshot>>);

pub fn start(app: &AppHandle, scene: &str) {
    app.manage(FakePanel::default());
    set_scene(app, scene);
}

/// `Some` when fake panel mode is on, holding the scene's snapshot (`loading` has none).
#[allow(clippy::option_option)]
pub fn panel_snapshot(app: &AppHandle) -> Option<Option<PanelSnapshot>> {
    let fake = app.try_state::<FakePanel>()?;
    let snapshot = fake.0.lock().ok()?.clone();
    Some(snapshot)
}

pub fn set_scene(app: &AppHandle, scene: &str) {
    let Some(fake) = app.try_state::<FakePanel>() else {
        return;
    };
    let snapshot = build(scene);
    if let Ok(mut current) = fake.0.lock() {
        current.clone_from(&snapshot);
    }
    if let Some(snapshot) = snapshot {
        tray::send(app, TrayMsg::Snapshot(Box::new(snapshot.clone())));
        ipc::emit_core_event(app, CoreEvent::LimitsUpdated(Box::new(snapshot)));
    }
}

// Command arguments are deserialized into owned values.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn fake_panel_scene(app: AppHandle, scene: String) -> Result<(), String> {
    if !SCENES.contains(&scene.as_str()) {
        return Err(format!("unknown scene; one of {SCENES:?}"));
    }
    set_scene(&app, &scene);
    Ok(())
}

const MIN: i64 = 60_000;

fn at(offset_min: i64) -> Timestamp {
    Timestamp::from_millis(Timestamp::now().as_millis() + offset_min * MIN)
}

struct L {
    kind: LimitKind,
    scope: Option<&'static str>,
    percent: f64,
    resets_in_min: i64,
    age_min: i64,
    source: Source,
    projection: Option<Projection>,
}

impl L {
    fn new(kind: LimitKind, percent: f64, resets_in_min: i64) -> Self {
        Self {
            kind,
            scope: None,
            percent,
            resets_in_min,
            age_min: 0,
            source: Source::Statusline,
            projection: None,
        }
    }

    fn view(self) -> LimitView {
        let freshness = match self.age_min {
            ..10 => Freshness::Fresh,
            10..30 => Freshness::Aging,
            _ => Freshness::Stale,
        };
        LimitView {
            kind: self.kind,
            scope: self.scope.map(|s| Scope::Model(s.to_owned())),
            percent: self.percent,
            resets_at: Some(at(self.resets_in_min)),
            severity: Severity::of(self.percent),
            provider_severity: None,
            source: self.source,
            estimate_reason: None,
            observed_at: at(-self.age_min),
            freshness,
            projection: self.projection,
        }
    }
}

fn projection(
    rate: f64,
    at_reset: f64,
    eta_min: Option<i64>,
    confidence: Confidence,
) -> Projection {
    Projection {
        rate_per_min: rate,
        eta_100_at: eta_min.map(at),
        will_hit_before_reset: at_reset > 100.0,
        projected_at_reset: Some(at_reset),
        confidence,
        method: RateMethod::Observed,
    }
}

fn session(
    n: i64,
    name: Option<&str>,
    title: Option<&str>,
    project: &str,
    status: Option<LiveStatus>,
    share: f64,
    rate: f64,
) -> SessionShareView {
    SessionShareView {
        session_id: Some(SessionId(n)),
        session_uuid: Some(format!("00000000-0000-4000-8000-{n:012}")),
        name: name.map(str::to_owned),
        title: title.map(str::to_owned),
        project: Some(project.to_owned()),
        status,
        share_percent: Some(share),
        cost: MicroUsd(0),
        tokens_per_min: rate,
    }
}

fn live() -> Vec<SessionShareView> {
    vec![
        session(
            1,
            Some("panel-layout"),
            None,
            "tokenradar",
            Some(LiveStatus::Busy),
            18.4,
            12_400.0,
        ),
        session(
            2,
            None,
            Some("Fix flaky upload test"),
            "web-shop",
            Some(LiveStatus::Busy),
            9.2,
            6_100.0,
        ),
        session(3, None, None, "infra", Some(LiveStatus::Idle), 3.1, 0.0),
    ]
}

fn many() -> Vec<SessionShareView> {
    let mut list = live();
    for (i, project) in ["docs", "api", "mobile", "cli", "scripts"]
        .iter()
        .enumerate()
    {
        let n = i64::try_from(i).unwrap_or_default() + 4;
        #[allow(clippy::cast_precision_loss)]
        let share = 2.5 - n as f64 * 0.2;
        list.push(session(
            n,
            None,
            Some("Refactor"),
            project,
            Some(LiveStatus::Idle),
            share,
            0.0,
        ));
    }
    list
}

fn base(limits: Vec<LimitView>, live_sessions: Vec<SessionShareView>) -> PanelSnapshot {
    PanelSnapshot {
        account: Some(AccountView {
            id: AccountId(1),
            label: "Claude".to_owned(),
            plan: Some("max".to_owned()),
            tier: Some("default_claude_max_20x".to_owned()),
            extra_usage_enabled: Some(false),
        }),
        limits,
        live_sessions,
        top_sessions: Vec::new(),
        index_state: IndexState {
            initial_scan_done: true,
            last_indexed_at: Some(at(0)),
            recent_ready: true,
            ..IndexState::default()
        },
        api: ApiStatusView {
            state: ApiState::Active,
            diagnostics: PollDiagnostics {
                last_attempt: Some(at(-3)),
                last_success: Some(at(-3)),
                ..PollDiagnostics::default()
            },
        },
        statusline: StatuslineState {
            last_seen: Some(at(0)),
        },
        generated_at: at(0),
    }
}

fn normal_limits() -> Vec<LimitView> {
    vec![
        L {
            projection: Some(projection(0.21, 68.0, Some(276), Confidence::High)),
            ..L::new(LimitKind::Session, 42.0, 108)
        }
        .view(),
        L {
            projection: Some(projection(0.004, 51.0, Some(16_500), Confidence::Medium)),
            ..L::new(LimitKind::WeeklyAll, 34.0, 3 * 24 * 60 + 17)
        }
        .view(),
        L {
            scope: Some("Fable"),
            source: Source::Api,
            age_min: 3,
            ..L::new(LimitKind::WeeklyScoped, 12.0, 3 * 24 * 60 + 17)
        }
        .view(),
    ]
}

#[allow(clippy::too_many_lines)]
fn build(scene: &str) -> Option<PanelSnapshot> {
    let snapshot = match scene {
        "loading" => return None,
        "crit" => {
            let limits = vec![
                L {
                    projection: Some(projection(0.18, 113.0, Some(39), Confidence::High)),
                    ..L::new(LimitKind::Session, 93.0, 112)
                }
                .view(),
                L {
                    projection: Some(projection(0.02, 148.0, Some(450), Confidence::Low)),
                    ..L::new(LimitKind::WeeklyAll, 91.0, 2 * 24 * 60)
                }
                .view(),
            ];
            base(limits, live())
        }
        "many" => base(normal_limits(), many()),
        "quiet" => {
            let mut s = base(
                vec![
                    L {
                        projection: Some(projection(0.0, 42.0, None, Confidence::High)),
                        ..L::new(LimitKind::Session, 42.0, 108)
                    }
                    .view(),
                    L::new(LimitKind::WeeklyAll, 34.0, 3 * 24 * 60 + 17).view(),
                ],
                Vec::new(),
            );
            s.top_sessions = live()
                .into_iter()
                .map(|mut x| {
                    x.status = None;
                    x.tokens_per_min = 0.0;
                    x
                })
                .collect();
            s
        }
        "indexing" => {
            let mut s = base(normal_limits(), live());
            s.index_state = IndexState {
                initial_scan_done: false,
                last_indexed_at: None,
                progress: Some(IndexProgress {
                    files_done: 2210,
                    files_total: 5200,
                    bytes_done: 4_900_000_000,
                    bytes_total: 11_700_000_000,
                    phase: IndexPhase::Indexing,
                }),
                recent_ready: true,
                drift: None,
            };
            s
        }
        "no-limits" => {
            let mut s = base(Vec::new(), live());
            s.api.state = ApiState::Off;
            s.api.diagnostics = PollDiagnostics::default();
            s.statusline.last_seen = None;
            s
        }
        "estimate" => {
            let estimate = |kind, percent, resets| {
                let mut v = L {
                    source: Source::Estimate,
                    projection: None,
                    ..L::new(kind, percent, resets)
                }
                .view();
                v.estimate_reason = Some(EstimateReason::Reset);
                v
            };
            let mut s = base(
                vec![
                    estimate(LimitKind::Session, 0.0, 300),
                    L {
                        source: Source::Api,
                        age_min: 40,
                        ..L::new(LimitKind::WeeklyAll, 34.0, 3 * 24 * 60)
                    }
                    .view(),
                ],
                live(),
            );
            s.statusline.last_seen = None;
            s
        }
        "token-stale" => {
            let api = |kind, percent, resets| {
                L {
                    source: Source::Api,
                    age_min: 45,
                    ..L::new(kind, percent, resets)
                }
                .view()
            };
            let mut s = base(
                vec![
                    api(LimitKind::Session, 42.0, 60),
                    api(LimitKind::WeeklyAll, 34.0, 3 * 24 * 60),
                ],
                live(),
            );
            s.api.state = ApiState::TokenStale;
            s.statusline.last_seen = None;
            s
        }
        "offline" => {
            let mut s = base(
                vec![
                    L {
                        source: Source::Api,
                        age_min: 14,
                        ..L::new(LimitKind::Session, 42.0, 90)
                    }
                    .view(),
                    L {
                        source: Source::Api,
                        age_min: 14,
                        ..L::new(LimitKind::WeeklyAll, 34.0, 3 * 24 * 60)
                    }
                    .view(),
                ],
                Vec::new(),
            );
            s.api.state = ApiState::Offline { retry_at: at(1) };
            s.statusline.last_seen = None;
            s
        }
        "stale" => {
            let stale = |kind, percent, resets| {
                L {
                    age_min: 45,
                    ..L::new(kind, percent, resets)
                }
                .view()
            };
            let mut s = base(
                vec![
                    stale(LimitKind::Session, 42.0, 60),
                    stale(LimitKind::WeeklyAll, 34.0, 3 * 24 * 60),
                ],
                Vec::new(),
            );
            s.statusline.last_seen = Some(at(-45));
            s
        }
        "drift" => {
            let mut s = base(normal_limits(), live());
            s.index_state.drift = Some(SchemaDriftView {
                unknown_count: 37,
                last_at: at(-2),
            });
            s
        }
        "no-claude" => {
            let mut s = base(Vec::new(), Vec::new());
            s.account = None;
            s.api.state = ApiState::Off;
            s.statusline.last_seen = None;
            s
        }
        "five-hour-only" => base(
            vec![
                L {
                    projection: Some(projection(0.3, 64.0, Some(150), Confidence::Low)),
                    ..L::new(LimitKind::Session, 55.0, 30)
                }
                .view(),
            ],
            live(),
        ),
        _ => base(normal_limits(), live()),
    };
    Some(snapshot)
}
