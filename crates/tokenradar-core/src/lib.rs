//! Core of TokenRadar: domain model, storage, indexing, providers and the limit engine.
//!
//! This crate has no Tauri dependency so that a headless daemon or another front end can reuse it.

pub mod attribution;
pub mod calibration;
pub mod clock;
pub mod config;
pub mod deeplink;
pub mod doctor;
pub mod domain;
pub mod events;
mod fsutil;
pub mod indexer;
pub mod ipc;
mod jsonc;
pub mod limits;
pub mod live_sessions;
pub mod notifier;
pub mod paths;
pub mod pricing;
pub mod providers;
pub mod queries;
pub mod runtime;
pub mod secret;
pub mod statusline;
pub mod store;
pub mod terminal_font;
pub mod watcher;
