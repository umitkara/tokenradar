//! Accounts, projects, sessions and models.
//!
//! `*Spec` types are what a writer knows before a row exists; the plain types are stored rows.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::{AccountId, ModelId, ParseEnumError, ProjectId, SessionId, Timestamp};

/// Defines a fieldless enum with a stable storage string per variant.
macro_rules! stored_enum {
    (
        $(#[$doc:meta])* $name:ident as $kind:literal {
            $($(#[$vdoc:meta])* $variant:ident = $text:literal,)+
        }
    ) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($(#[$vdoc])* $variant,)+
        }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)+
                }
            }
        }

        impl std::str::FromStr for $name {
            type Err = ParseEnumError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    _ => Err(ParseEnumError { kind: $kind, value: s.to_owned() }),
                }
            }
        }
    };
}
pub(crate) use stored_enum;

stored_enum! {
    /// The agent whose limits an account tracks.
    Provider as "provider" {
        Claude = "claude",
        Codex = "codex",
    }
}

stored_enum! {
    /// Main conversation or a subagent spawned by one.
    SessionKind as "session kind" {
        Main = "main",
        Subagent = "subagent",
    }
}

stored_enum! {
    /// Model family, used for grouping and per-family calibration weights.
    /// Prices are per model, not per family (ADR-025).
    ModelFamily as "model family" {
        Fable = "fable",
        Mythos = "mythos",
        Opus = "opus",
        Sonnet = "sonnet",
        Haiku = "haiku",
        /// Placeholder records Claude Code writes without an API call (`<synthetic>`).
        Synthetic = "synthetic",
        Unknown = "unknown",
    }
}

/// A provider configuration directory being tracked, e.g. `~/.claude`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct AccountSpec {
    pub provider: Provider,
    pub label: String,
    /// Normalized path; UNC for WSL. Unique per provider.
    pub config_dir: String,
    pub plan: Option<String>,
    pub tier: Option<String>,
    /// `native` or `wsl:<distro>`.
    pub environment: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Account {
    pub id: AccountId,
    #[serde(flatten)]
    pub spec: AccountSpec,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ProjectSpec {
    pub account_id: AccountId,
    /// Normalized `cwd`: case-folded on Windows. Unique per account.
    pub path_norm: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Project {
    pub id: ProjectId,
    #[serde(flatten)]
    pub spec: ProjectSpec,
}

/// What the indexer knows about a session. Upserts merge: `None` keeps the stored value,
/// `first_ts` only moves earlier and `last_ts` only moves later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SessionSpec {
    pub account_id: AccountId,
    /// Session uuid; subagents use `agent-<id>`. Unique per account.
    pub uuid: String,
    pub kind: SessionKind,
    pub parent_id: Option<SessionId>,
    /// Top-level main session of a subagent tree.
    pub root_id: Option<SessionId>,
    pub project_id: Option<ProjectId>,
    pub title: Option<String>,
    pub git_branch: Option<String>,
    pub entrypoint: Option<String>,
    pub cc_version: Option<String>,
    pub first_ts: Option<Timestamp>,
    pub last_ts: Option<Timestamp>,
}

impl SessionSpec {
    /// A spec that carries only the identity fields.
    pub fn new(account_id: AccountId, uuid: impl Into<String>, kind: SessionKind) -> Self {
        Self {
            account_id,
            uuid: uuid.into(),
            kind,
            parent_id: None,
            root_id: None,
            project_id: None,
            title: None,
            git_branch: None,
            entrypoint: None,
            cc_version: None,
            first_ts: None,
            last_ts: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Session {
    pub id: SessionId,
    #[serde(flatten)]
    pub spec: SessionSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Model {
    pub id: ModelId,
    /// Model id exactly as the provider reports it, e.g. `claude-opus-5-5`.
    pub name: String,
    pub family: ModelFamily,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_enums_round_trip() {
        for family in [
            ModelFamily::Fable,
            ModelFamily::Mythos,
            ModelFamily::Opus,
            ModelFamily::Sonnet,
            ModelFamily::Haiku,
            ModelFamily::Synthetic,
            ModelFamily::Unknown,
        ] {
            assert_eq!(family.as_str().parse::<ModelFamily>(), Ok(family));
        }
        assert_eq!("subagent".parse::<SessionKind>(), Ok(SessionKind::Subagent));
        assert!("gemini".parse::<Provider>().is_err());
    }
}
