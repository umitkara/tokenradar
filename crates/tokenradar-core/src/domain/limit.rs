//! Limit observations and windows (architecture §7–§8).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use specta::Type;

use super::entities::stored_enum;
use super::{AccountId, ParseEnumError, Timestamp};

/// Which limit an observation describes. Unknown kinds from a provider are kept verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum LimitKind {
    /// The 5-hour window.
    Session,
    WeeklyAll,
    /// A weekly limit restricted to a scope, e.g. one model.
    WeeklyScoped,
    Spend,
    Other(String),
}

impl LimitKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Session => "session",
            Self::WeeklyAll => "weekly_all",
            Self::WeeklyScoped => "weekly_scoped",
            Self::Spend => "spend",
            Self::Other(s) => s,
        }
    }
}

impl From<&str> for LimitKind {
    fn from(s: &str) -> Self {
        match s {
            "session" => Self::Session,
            "weekly_all" => Self::WeeklyAll,
            "weekly_scoped" => Self::WeeklyScoped,
            "spend" => Self::Spend,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// What a scoped limit applies to. Stored as `model:<name>` or `surface:<name>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Model(String),
    Surface(String),
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(m) => write!(f, "model:{m}"),
            Self::Surface(s) => write!(f, "surface:{s}"),
        }
    }
}

impl FromStr for Scope {
    type Err = ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once(':') {
            Some(("model", m)) => Ok(Self::Model(m.to_owned())),
            Some(("surface", v)) => Ok(Self::Surface(v.to_owned())),
            _ => Err(ParseEnumError {
                kind: "scope",
                value: s.to_owned(),
            }),
        }
    }
}

stored_enum! {
    /// Where a number came from. The UI always shows it (honest numbers).
    Source as "source" {
        Statusline = "statusline",
        Api = "api",
        /// Derived locally; never persisted to `limit_snapshots`.
        Estimate = "estimate",
        CodexLog = "codex_log",
    }
}

/// One reading of a limit's utilization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct LimitObservation {
    pub account: AccountId,
    pub kind: LimitKind,
    pub scope: Option<Scope>,
    /// 0–100, may exceed 100.
    pub percent: f64,
    pub resets_at: Option<Timestamp>,
    pub severity: Option<String>,
    pub source: Source,
    pub session_uuid: Option<String>,
    pub observed_at: Timestamp,
}

/// A 5-hour or weekly window, observed or estimated from logs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct LimitWindow {
    pub id: i64,
    pub account: AccountId,
    pub kind: LimitKind,
    pub scope: Option<Scope>,
    pub starts_at: Timestamp,
    pub resets_at: Timestamp,
    pub peak_percent: Option<f64>,
    pub hit_limit: bool,
    /// Derived from logs because no snapshot covered it.
    pub estimated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_round_trips() {
        for scope in [
            Scope::Model("Fable".into()),
            Scope::Surface("claude_code".into()),
        ] {
            assert_eq!(scope.to_string().parse::<Scope>(), Ok(scope));
        }
        assert!("Fable".parse::<Scope>().is_err());
    }

    #[test]
    fn unknown_limit_kinds_are_kept() {
        let kind = LimitKind::from("monthly_opus");
        assert_eq!(kind, LimitKind::Other("monthly_opus".into()));
        assert_eq!(kind.as_str(), "monthly_opus");
        assert_eq!(LimitKind::from("weekly_all"), LimitKind::WeeklyAll);
    }
}
