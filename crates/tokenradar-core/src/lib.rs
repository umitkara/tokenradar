//! Core of TokenRadar: domain model, storage, indexing, providers and the limit engine.
//!
//! This crate has no Tauri dependency so that a headless daemon or another front end can reuse it.

pub mod clock;
pub mod config;
pub mod domain;
pub mod events;
mod fsutil;
pub mod indexer;
pub mod ipc;
pub mod limits;
pub mod live_sessions;
pub mod paths;
pub mod pricing;
pub mod providers;
pub mod runtime;
pub mod secret;
pub mod statusline;
pub mod store;
pub mod watcher;
