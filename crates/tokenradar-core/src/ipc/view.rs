//! What the app tells the CLI, and what `tokenradar … --json` prints (`"schema": 1`).
//!
//! These types are a stable, versioned contract: they are built from the internal view model
//! (`PanelSnapshot`) or, with the app closed, from the database, and never change shape
//! without a new schema number. Enumerations travel as `snake_case` strings so that a newer
//! app does not break an older CLI. A field added later carries `#[serde(default)]`, so that
//! both directions keep working: an older CLI ignores it, a newer one reads an older app.

use serde::{Deserialize, Serialize};

use crate::domain::{LimitObservation, LimitRef, Source, Timestamp};
use crate::limits::{Freshness, LimitView, PanelSnapshot, SessionShareView};
use crate::live_sessions::LiveStatus;
use crate::statusline::git_status::GitStatus;

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

/// A limit this account does not have (ADR-041), e.g. the 5-hour window of a verified Ultra
/// account. It is never in `limits`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsentLimit {
    /// Same names as [`LimitInfo::kind`].
    pub kind: String,
    pub scope: Option<String>,
}

impl AbsentLimit {
    pub fn from_ref(r: &LimitRef) -> Self {
        Self {
            kind: r.kind.as_str().to_owned(),
            scope: r.scope.as_ref().map(ToString::to_string),
        }
    }
}

/// How many times a session was compacted (Step 12n), from its transcript.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactCount {
    pub auto: u32,
    pub manual: u32,
    /// A trigger this build does not know; part of [`CompactCount::total`].
    #[serde(default)]
    pub other: u32,
}

impl CompactCount {
    pub fn total(&self) -> u32 {
        self.auto
            .saturating_add(self.manual)
            .saturating_add(self.other)
    }
}

impl From<crate::store::CompactCounts> for CompactCount {
    fn from(c: crate::store::CompactCounts) -> Self {
        Self {
            auto: c.auto,
            manual: c.manual,
            other: c.other,
        }
    }
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
    /// Compactions so far. Added in Step 12n; `None` from older apps and while the session is
    /// not indexed: unknown, never shown as 0.
    #[serde(default)]
    pub compacts: Option<CompactCount>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusReply {
    pub schema: u8,
    pub origin: Origin,
    pub account: Option<AccountInfo>,
    pub limits: Vec<LimitInfo>,
    /// Limits the account does not have. Added in 0.1.1; empty from older apps.
    #[serde(default)]
    pub absent: Vec<AbsentLimit>,
    /// The session asked about, when the app knows it.
    pub session: Option<SessionInfo>,
    pub live_sessions: Vec<SessionInfo>,
    pub statusline_last_seen: Option<Timestamp>,
    /// `off`, `active`, `token_stale`, `backoff`, `offline`, `error`; `None` from the database.
    pub api_state: Option<String>,
    pub generated_at: Timestamp,
    /// The working tree of the query's `cwd` (Step 12r, ADR-059), when the app has a current
    /// result. Absent from older apps, from the database and while the first run goes on:
    /// the item is hidden then, never shown with a guess.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<GitStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimitsReply {
    pub schema: u8,
    pub origin: Origin,
    pub limits: Vec<LimitInfo>,
    /// Limits the account does not have. Added in 0.1.1; empty from older apps.
    #[serde(default)]
    pub absent: Vec<AbsentLimit>,
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
            absent: s.absent.iter().map(AbsentLimit::from_ref).collect(),
            session,
            live_sessions: s.live_sessions.iter().map(SessionInfo::from_view).collect(),
            statusline_last_seen: s.statusline.last_seen,
            api_state: Some(enum_name(&s.api.state, "state")),
            generated_at: s.generated_at,
            git: None,
        }
    }

    /// The app is running but has not built a view yet.
    pub fn empty(now: Timestamp) -> Self {
        Self {
            schema: JSON_SCHEMA,
            origin: Origin::App,
            account: None,
            limits: Vec::new(),
            absent: Vec::new(),
            session: None,
            live_sessions: Vec::new(),
            statusline_last_seen: None,
            api_state: None,
            generated_at: now,
            git: None,
        }
    }

    pub fn limits_reply(self) -> LimitsReply {
        LimitsReply {
            schema: self.schema,
            origin: self.origin,
            limits: self.limits,
            absent: self.absent,
            generated_at: self.generated_at,
        }
    }

    /// Whether the account lacks the limit of this kind, unscoped.
    pub fn is_absent(&self, kind: &str) -> bool {
        self.absent
            .iter()
            .any(|a| a.kind == kind && a.scope.is_none())
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
            compacts: None,
        }
    }

    /// A session the view does not list but the database knows: only its identity and
    /// compactions, nothing measured.
    pub fn compacts_only(uuid: &str, compacts: CompactCount) -> Self {
        Self {
            session_uuid: Some(uuid.to_owned()),
            name: None,
            title: None,
            project: None,
            status: None,
            share_percent: None,
            cost_usd: 0.0,
            tokens_per_min: 0.0,
            compacts: Some(compacts),
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
            compacts: None,
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
    fn replies_without_the_absent_field_still_parse() {
        let now = Timestamp::from_millis(1_000);
        let mut reply = StatusReply::empty(now);
        reply.absent = vec![AbsentLimit {
            kind: "session".into(),
            scope: None,
        }];
        let mut v = serde_json::to_value(&reply).expect("json");
        assert_eq!(v["absent"][0]["kind"], "session");
        assert!(reply.is_absent("session") && !reply.is_absent("weekly_all"));
        // An app from before 0.1.1 does not send the field.
        v.as_object_mut().expect("object").remove("absent");
        let old: StatusReply = serde_json::from_value(v).expect("old reply");
        assert_eq!(old.absent.len(), 0);
    }

    #[test]
    fn sessions_without_compacts_from_older_apps_still_parse() {
        let mut info = SessionInfo::compacts_only(
            "s1",
            CompactCount {
                auto: 2,
                manual: 1,
                other: 0,
            },
        );
        let mut v = serde_json::to_value(&info).expect("json");
        assert_eq!(v["compacts"]["auto"], 2);
        assert_eq!(info.compacts.map(|c| c.total()), Some(3));
        // An app from before Step 12n sends neither field.
        v.as_object_mut().expect("object").remove("compacts");
        let old: SessionInfo = serde_json::from_value(v).expect("old session");
        assert_eq!(old.compacts, None);
        info.compacts = None;
        assert_eq!(old, info);
        let no_other: CompactCount =
            serde_json::from_str(r#"{"auto":1,"manual":0}"#).expect("count");
        assert_eq!(no_other.total(), 1);
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
