//! Limit engine: source fusion (statusline > API > estimate), snapshots and windows, reset
//! detection, burn rate and projection, the v0.1 session shares and the panel view model.

mod engine;
pub mod rate;
mod share;
mod view;

pub use engine::{
    AccountMeta, FUSION_WINDOW_MS, HEARTBEAT_MS, LimitEngine, SESSION_WINDOW_MS, WEEK_MS,
    window_length_ms,
};
pub use rate::{Confidence, Projection, RateMethod};
pub use share::{RootUsage, shares};
pub use view::{
    AccountView, ApiStatusView, CRIT_AT, EstimateReason, Freshness, IndexState, LimitEvent,
    LimitView, PanelSnapshot, SchemaDriftView, SessionShareView, Severity, StatuslineState,
    THRESHOLDS, VisibleKey, WARN_AT,
};
