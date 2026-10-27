//! Usage-limit providers (Claude first, Codex later).
//!
//! Everything specific to one agent's files lives in its module; the indexer and the store
//! only see domain types. The common `Provider` trait (architecture §16) is introduced with
//! the second provider (Step 20), when there is something to abstract over.

pub mod claude;
