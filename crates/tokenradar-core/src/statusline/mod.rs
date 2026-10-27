//! The statusline bridge (S1, ADR-005, ADR-009): payload, spool, `settings.json` install.
//!
//! ```text
//!  Claude Code ──stdin──► tokenradar statusline ──push──► app (IPC) ──► LimitEngine
//!                                │ app not running
//!                                └──► spool/*.jsonl ──(app start, every 5 min)──► LimitEngine
//! ```

pub mod install;
pub mod payload;
pub mod settings;
pub mod shell;
pub mod spool;

pub use payload::{
    StatuslinePayload, StatuslinePush, account_for, default_config_dir, path_in_dir, same_dir,
};
