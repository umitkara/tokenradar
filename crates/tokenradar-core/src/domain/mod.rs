//! Provider-agnostic domain types: accounts, sessions, projects, usage events and limit windows.
//!
//! Types that reach the UI derive `specta::Type`. Every 64-bit integer here (ids, timestamps,
//! micro-dollars) stays below 2^53, so it is safe to export as a TypeScript `number`.

mod entities;
mod ids;
mod limit;
mod money;
mod time;
mod usage;

pub use entities::{
    Account, AccountSpec, Model, ModelFamily, Project, ProjectSpec, Provider, Session, SessionKind,
    SessionSpec, SessionTitle, TitleSource,
};
pub use ids::{AccountId, ModelId, ProjectId, SessionId};
pub use limit::{LimitKind, LimitObservation, LimitWindow, Scope, Source};
pub use money::MicroUsd;
pub use time::{BUCKET_MS, Timestamp};
pub use usage::{MsgKey, Speed, TokenCounts, Usage, UsageEvent};

/// A stored string did not match any known variant of a domain enum.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown {kind} value {value:?}")]
pub struct ParseEnumError {
    pub kind: &'static str,
    pub value: String,
}
