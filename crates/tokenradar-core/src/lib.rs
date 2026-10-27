//! Core of TokenRadar: domain model, storage, indexing, providers and the limit engine.
//!
//! This crate has no Tauri dependency so that a headless daemon or another front end can reuse it.

pub mod domain;
pub mod indexer;
pub mod ipc;
pub mod limits;
pub mod paths;
pub mod pricing;
pub mod providers;
pub mod secret;
pub mod store;
