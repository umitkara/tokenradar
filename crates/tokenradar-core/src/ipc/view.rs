//! What the app tells the CLI, and what `tokenradar … --json` prints (`"schema": 1`).
//!
//! These types are a stable, versioned contract: they are built from the internal view model
//! (`PanelSnapshot`) or, with the app closed, from the database, and never change shape
//! without a new schema number. Enumerations travel as `snake_case` strings so that a newer
//! app does not break an older CLI.

use serde::{Deserialize, Serialize};

use crate::domain::{LimitObservation, Source, Timestamp};
use crate::limits::{Freshness, LimitView, PanelSnapshot, SessionShareView};
use crate::live_sessions::LiveStatus;

/// Version of the `--json` output and of these replies.
pub const JSON_SCHEMA: u8 = 1;

/// Where a reply's data came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// The running app's live state.
    App,
    /// The database, read while the app was not running.
    Database,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountInfo {
    pub label: String,
    pub plan: Option<String>,
    pub tier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimitInfo {
    /// `session` (5-hour), `weekly_all`, `weekly_scoped`, …
    pub kind: String,
    pub scope: Option<String>,
    pub percent: f64,
    pub resets_at: Option<Timestamp>,
    /// `statusline`, `api`, `estimate`, `codex_log`.
    pub source: String,
    /// True for estimates; shown with `≈`.
    pub estimated: bool,
    pub observed_at: Timestamp,
    /// `fresh`, `aging`, `stale`.
    pub freshness: String,
    /// At the current rate: when 100 % is reached, if before the reset.
    pub eta_100_at: Option<Timestamp>,
    pub will_hit_before_reset: bool,
    /// `low`, `medium`, `high`; `None` without a projection.
    pub projection_confidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_uuid: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub project: Option<String>,
    /// `busy`, `idle`, `unknown`; `None` when not running.
    pub status: Option<String>,
    /// `≈ %` of the 5-hour window (an estimate); `None` without a measured window.
    pub share_percent: Option<f64>,
    /// API-equivalent cost in the window, USD.
    pub cost_usd: f64,
    pub tokens_per_min: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusReply {
    pub schema: u8,
    pub origin: Origin,
    pub account: Option<AccountInfo>,
    pub limits: Vec<LimitInfo>,
    /// The session asked about, when the app knows it.
    pub session: Option<SessionInfo>,
    pub live_sessions: Vec<SessionInfo>,
    pub statusline_last_seen: Option<Timestamp>,
    /// `off`, `active`, `token_stale`, `backoff`, `offline`, `error`; `None` from the database.
    pub api_state: Option<String>,
    pub generated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimitsReply {
    pub schema: u8,
    pub origin: Origin,
    pub limits: Vec<LimitInfo>,
    pub generated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionsReply {
    pub schema: u8,
    pub origin: Origin,
    pub sessions: Vec<SessionInfo>,
    pub generated_at: Timestamp,
}

impl StatusReply {
    pub fn from_snapshot(s: &PanelSnapshot, session_uuid: Option<&str>) -> Self {
        let session = session_uuid.and_then(|uuid| {
            s.live_sessions
                .iter()
                .chain(&s.top_sessions)
                .find(|x| x.session_uuid.as_deref() == Some(uuid))
                .map(SessionInfo::from_view)
        });
        Self {
            schema: JSON_SCHEMA,
            origin: Origin::App,
            account: s.account.as_ref().map(|a| AccountInfo {
                label: a.label.clone(),
                plan: a.plan.clone(),
                tier: a.tier.clone(),
            }),
            limits: s.limits.iter().map(LimitInfo::from_view).collect(),
            session,
            live_sessions: s.live_sessions.iter().map(SessionInfo::from_view).collect(),
            statusline_last_seen: s.statusline.last_seen,
            api_state: Some(enum_name(&s.api.state, "state")),
            generated_at: s.generated_at,
        }
    }

    /// The app is running but has not built a view yet.
    pub fn empty(now: Timestamp) -> Self {
        Self {
            schema: JSON_SCHEMA,
            origin: Origin::App,
            account: None,
            limits: Vec::new(),
            session: None,
            live_sessions: Vec::new(),
            statusline_last_seen: None,
            api_state: None,
            generated_at: now,
        }
    }

    pub fn limits_reply(self) -> LimitsReply {
        LimitsReply {
            schema: self.schema,
            origin: self.origin,
            limits: self.limits,
            generated_at: self.generated_at,
        }
    }

    pub fn sessions_reply(self) -> SessionsReply {
        SessionsReply {
            schema: self.schema,
            origin: self.origin,
            sessions: self.live_sessions,
            generated_at: self.generated_at,
        }
    }
}

impl LimitInfo {
    pub fn from_view(l: &LimitView) -> Self {
        let projection = l.projection.as_ref();
        Self {
            kind: l.kind.as_str().to_owned(),
            scope: l.scope.as_ref().map(ToString::to_string),
            percent: l.percent,
            resets_at: l.resets_at,
            source: l.source.as_str().to_owned(),
            estimated: l.source == Source::Estimate,
            observed_at: l.observed_at,
            freshness: enum_name(&l.freshness, ""),
            eta_100_at: projection
                .filter(|p| p.will_hit_before_reset)
                .and_then(|p| p.eta_100_at),
            will_hit_before_reset: projection.is_some_and(|p| p.will_hit_before_reset),
            projection_confidence: projection.map(|p| enum_name(&p.confidence, "")),
        }
    }

    /// A stored reading, as the app would show it at `now`: a window whose reset passed is an
    /// estimated 0 % (the same rule as the limit engine).
    pub fn from_stored(o: &LimitObservation, now: Timestamp) -> Self {
        let reset_passed = o.resets_at.is_some_and(|r| r <= now);
        let age = now.as_millis() - o.observed_at.as_millis();
        Self {
            kind: o.kind.as_str().to_owned(),
            scope: o.scope.as_ref().map(ToString::to_string),
            percent: if reset_passed { 0.0 } else { o.percent },
            resets_at: if reset_passed { None } else { o.resets_at },
            source: if reset_passed {
                Source::Estimate.as_str().to_owned()
            } else {
                o.source.as_str().to_owned()
            },
            estimated: reset_passed,
            observed_at: o.observed_at,
            freshness: enum_name(&Freshness::of(age), ""),
            eta_100_at: None,
            will_hit_before_reset: false,
            projection_confidence: None,
        }
    }
}

impl SessionInfo {
    pub fn from_view(s: &SessionShareView) -> Self {
        #[allow(clippy::cast_precision_loss)]
        let cost_usd = s.cost.0 as f64 / 1_000_000.0;
        Self {
            session_uuid: s.session_uuid.clone(),
            name: s.name.clone(),
            title: s.title.clone(),
            project: s.project.clone(),
            status: s.status.map(|st| enum_name(&st, "")),
            share_percent: s.share_percent,
            cost_usd,
            tokens_per_min: s.tokens_per_min,
        }
    }

    /// A running session seen directly in Claude Code's session files (app not running).
    pub fn from_live_file(uuid: &str, name: Option<&str>, cwd: &str, status: LiveStatus) -> Self {
        Self {
            session_uuid: Some(uuid.to_owned()),
            name: name.map(str::to_owned),
            title: None,
            project: Some(cwd.to_owned()),
            status: Some(enum_name(&status, "")),
            share_percent: None,
            cost_usd: 0.0,
            tokens_per_min: 0.0,
        }
    }
}

/// The serde name of a unit-like enum value; `tag` names the field of internally tagged ones.
fn enum_name(v: &impl Serialize, tag: &str) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(serde_json::Value::Object(m)) => m
            .get(tag)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AccountId, LimitKind};
    use crate::providers::claude::usage_api::ApiState;

    #[test]
    fn enum_names_follow_serde() {
        assert_eq!(enum_name(&Freshness::Aging, ""), "aging");
        assert_eq!(enum_name(&LiveStatus::Busy, ""), "busy");
        assert_eq!(
            enum_name(
                &ApiState::Backoff {
                    until: Timestamp::from_millis(0)
                },
                "state"
            ),
            "backoff"
        );
    }

    #[test]
    fn a_stored_reading_past_its_reset_is_an_estimated_zero() {
        let now = Timestamp::from_millis(10_000_000);
        let obs = LimitObservation {
            account: AccountId(1),
            kind: LimitKind::Session,
            scope: None,
            percent: 80.0,
            resets_at: Some(Timestamp::from_millis(9_000_000)),
            severity: None,
            source: Source::Statusline,
            session_uuid: None,
            observed_at: Timestamp::from_millis(8_000_000),
        };
        let past = LimitInfo::from_stored(&obs, now);
        assert_eq!((past.percent, past.estimated), (0.0, true));
        assert_eq!(past.source, "estimate");
        let live = LimitInfo::from_stored(&obs, Timestamp::from_millis(8_500_000));
        assert_eq!(
            (live.percent, live.estimated, live.source.as_str()),
            (80.0, false, "statusline")
        );
        assert_eq!(live.freshness, "fresh");
    }
}
