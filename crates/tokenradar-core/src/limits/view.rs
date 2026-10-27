//! What the panel and the tray draw (`PanelSnapshot`) and the signals the notifier will use
//! (`LimitEvent`). Computed in the core; the UI only renders (CLAUDE.md conventions).

use serde::Serialize;
use specta::Type;

use super::rate::Projection;
use crate::domain::{AccountId, LimitKind, MicroUsd, Scope, SessionId, Source, Timestamp};
use crate::live_sessions::LiveStatus;
use crate::providers::claude::usage_api::{ApiState, PollDiagnostics};

/// Default status thresholds (design-language.md): `< 75` ok, `75–90` warn, `≥ 90` crit.
pub const WARN_AT: f64 = 75.0;
pub const CRIT_AT: f64 = 90.0;
/// Upward crossings of these raise [`LimitEvent::ThresholdCrossed`].
pub const THRESHOLDS: [u8; 3] = [75, 90, 100];

/// A reading is fresh below this age …
pub const FRESH_MS: i64 = 10 * 60 * 1000;
/// … aging below this, stale above.
pub const STALE_MS: i64 = 30 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Ok,
    Warn,
    Crit,
}

impl Severity {
    pub fn of(percent: f64) -> Self {
        if percent >= CRIT_AT {
            Self::Crit
        } else if percent >= WARN_AT {
            Self::Warn
        } else {
            Self::Ok
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// Under 10 minutes old.
    Fresh,
    /// 10–30 minutes: "updated X min ago".
    Aging,
    /// Over 30 minutes.
    Stale,
}

impl Freshness {
    pub fn of(age_ms: i64) -> Self {
        if age_ms < FRESH_MS {
            Self::Fresh
        } else if age_ms < STALE_MS {
            Self::Aging
        } else {
            Self::Stale
        }
    }
}

/// Why a value is an estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum EstimateReason {
    /// `resets_at` passed without a new reading: assumed 0 %.
    Reset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct LimitView {
    pub kind: LimitKind,
    pub scope: Option<Scope>,
    pub percent: f64,
    pub resets_at: Option<Timestamp>,
    /// From our thresholds; the provider's own label is `provider_severity`.
    pub severity: Severity,
    pub provider_severity: Option<String>,
    pub source: Source,
    /// Set when `source` is [`Source::Estimate`].
    pub estimate_reason: Option<EstimateReason>,
    pub observed_at: Timestamp,
    pub freshness: Freshness,
    /// `None` for estimates and when no rate can be derived.
    pub projection: Option<Projection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct AccountView {
    pub id: AccountId,
    pub label: String,
    /// `subscriptionType`, e.g. `max`.
    pub plan: Option<String>,
    /// `rateLimitTier`, e.g. `default_claude_max_20x`.
    pub tier: Option<String>,
    /// From the usage API; `None` until it answered.
    pub extra_usage_enabled: Option<bool>,
}

/// One session tree's part in the current 5-hour window.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct SessionShareView {
    pub session_id: Option<SessionId>,
    pub session_uuid: Option<String>,
    /// Name from the live-session file (`/rename` or Claude Code's choice).
    pub name: Option<String>,
    /// Title from the transcript.
    pub title: Option<String>,
    pub project: Option<String>,
    /// `None` when the session is not running.
    pub status: Option<LiveStatus>,
    /// `≈ %` of the window; `None` without a measured window percentage (no invented numbers).
    pub share_percent: Option<f64>,
    /// API-equivalent cost in the window.
    pub cost: MicroUsd,
    /// Input, output and cache-write tokens per minute over the last 5 minutes.
    pub tokens_per_min: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Type)]
pub struct IndexState {
    pub initial_scan_done: bool,
    pub last_indexed_at: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct ApiStatusView {
    pub state: ApiState,
    pub diagnostics: PollDiagnostics,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Type)]
pub struct StatuslineState {
    /// `None`: never seen, the bridge is probably not installed.
    pub last_seen: Option<Timestamp>,
}

/// Everything the tray panel needs (Step 10).
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct PanelSnapshot {
    /// The primary account; `None` when no Claude Code directory exists.
    pub account: Option<AccountView>,
    pub limits: Vec<LimitView>,
    /// Running sessions, largest share first.
    pub live_sessions: Vec<SessionShareView>,
    /// The window's largest consumers, live or not (shown when nothing is live).
    pub top_sessions: Vec<SessionShareView>,
    pub index_state: IndexState,
    pub api: ApiStatusView,
    pub statusline: StatuslineState,
    pub generated_at: Timestamp,
}

/// kind, scope, rounded percent, severity, source, freshness, will hit, rounded ETA (min).
type VisibleLimit = (
    String,
    Option<String>,
    i64,
    Severity,
    Source,
    Freshness,
    bool,
    Option<i64>,
);
/// uuid, status, rounded share.
type VisibleSession = (Option<String>, Option<LiveStatus>, Option<i64>);

/// The parts of a snapshot a user can see change; `LimitsUpdated` is only sent when these do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleKey {
    account: Option<AccountView>,
    limits: Vec<VisibleLimit>,
    sessions: Vec<VisibleSession>,
    index: IndexState,
    api: std::mem::Discriminant<ApiState>,
    statusline_seen: bool,
}

impl PanelSnapshot {
    pub fn visible_key(&self) -> VisibleKey {
        #[allow(clippy::cast_possible_truncation)]
        let round = |p: f64| p.round() as i64;
        VisibleKey {
            account: self.account.clone(),
            limits: self
                .limits
                .iter()
                .map(|l| {
                    (
                        l.kind.as_str().to_owned(),
                        l.scope.as_ref().map(ToString::to_string),
                        round(l.percent),
                        l.severity,
                        l.source,
                        l.freshness,
                        l.projection.is_some_and(|p| p.will_hit_before_reset),
                        // Minutes, so the countdown does not count as a change.
                        l.resets_at.map(|r| r.as_millis() / 60_000),
                    )
                })
                .collect(),
            sessions: self
                .live_sessions
                .iter()
                .map(|s| (s.session_uuid.clone(), s.status, s.share_percent.map(round)))
                .collect(),
            index: self.index_state,
            api: std::mem::discriminant(&self.api.state),
            statusline_seen: self.statusline.last_seen.is_some(),
        }
    }
}

/// Signals for the notifier (Step 11), which applies its own rules, hysteresis and quiet hours.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LimitEvent {
    /// A limit rose past one of [`THRESHOLDS`] within one window.
    ThresholdCrossed {
        account: AccountId,
        kind: LimitKind,
        scope: Option<Scope>,
        threshold: u8,
        percent: f64,
        resets_at: Option<Timestamp>,
    },
    /// "Will be full before the reset at this rate" switched on or off.
    ProjectionChanged {
        account: AccountId,
        kind: LimitKind,
        scope: Option<Scope>,
        will_hit_before_reset: bool,
        eta_100_at: Option<Timestamp>,
        resets_at: Option<Timestamp>,
    },
    /// `resets_at` passed; the limit is assumed back at 0 %.
    Reset {
        account: AccountId,
        kind: LimitKind,
        scope: Option<Scope>,
        previous_percent: f64,
        reset_at: Timestamp,
    },
}
