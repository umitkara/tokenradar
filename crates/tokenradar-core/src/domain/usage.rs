//! Token usage of a single model response.

use std::fmt;

use super::{AccountId, MicroUsd, ModelId, SessionId, Timestamp};

/// Deduplication key of a response: xxh3-128 of `message.id` and `requestId` (ADR-007).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MsgKey(pub [u8; 16]);

impl MsgKey {
    /// Key for a response identified by its message id and request id.
    pub fn from_ids(message_id: &str, request_id: &str) -> Self {
        // The separator keeps ("ab", "c") and ("a", "bc") apart.
        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
        hasher.update(message_id.as_bytes());
        hasher.update(&[0]);
        hasher.update(request_id.as_bytes());
        Self(hasher.digest128().to_le_bytes())
    }

    /// Fallback key when the ids are missing: a hash of some other stable identity, such as
    /// the line's `uuid` or the raw line. The indexer counts how often it falls back.
    pub fn from_fallback(bytes: &[u8]) -> Self {
        Self(xxhash_rust::xxh3::xxh3_128(bytes).to_le_bytes())
    }
}

impl fmt::Debug for MsgKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MsgKey(")?;
        for b in &self.0[..4] {
            write!(f, "{b:02x}")?;
        }
        write!(f, "…)")
    }
}

/// Service speed. Fast mode is billed at a premium.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Speed {
    #[default]
    Standard,
    Fast,
}

impl Speed {
    /// Value stored in `usage_events.speed`.
    pub const fn as_i64(self) -> i64 {
        match self {
            Self::Standard => 0,
            Self::Fast => 1,
        }
    }

    pub const fn from_i64(v: i64) -> Self {
        if v == 1 { Self::Fast } else { Self::Standard }
    }
}

/// Token counts of one response.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TokenCounts {
    pub input: u64,
    /// Includes thinking tokens.
    pub output: u64,
    pub cache_read: u64,
    pub cache_w5m: u64,
    pub cache_w1h: u64,
    /// Subset of `output`; informational, not billed separately.
    pub thinking: u64,
}

impl TokenCounts {
    /// Splits cache writes into 5-minute and 1-hour parts. Without a breakdown, the whole
    /// `cache_creation_input_tokens` value counts as 5-minute writes (architecture §9.1).
    pub fn split_cache_creation(total: u64, breakdown: Option<(u64, u64)>) -> (u64, u64) {
        breakdown.unwrap_or((total, 0))
    }

    /// Total prompt size: every input token, cached or not.
    pub const fn prompt_tokens(&self) -> u64 {
        self.input + self.cache_read + self.cache_w5m + self.cache_w1h
    }
}

/// Everything that determines what a response cost.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Usage {
    pub tokens: TokenCounts,
    pub web_search: u32,
    pub web_fetch: u32,
    pub speed: Speed,
}

/// One deduplicated model response, ready to store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEvent {
    pub account_id: AccountId,
    pub session_id: SessionId,
    pub model_id: ModelId,
    pub ts: Timestamp,
    pub msg_key: MsgKey,
    pub usage: Usage,
    /// API-equivalent cost from `Pricing` (architecture §9.1).
    pub cost: MicroUsd,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msg_key_separates_fields() {
        assert_ne!(MsgKey::from_ids("ab", "c"), MsgKey::from_ids("a", "bc"));
        assert_eq!(
            MsgKey::from_ids("msg_1", "req_1"),
            MsgKey::from_ids("msg_1", "req_1")
        );
    }

    #[test]
    fn cache_creation_without_breakdown_counts_as_5m() {
        assert_eq!(TokenCounts::split_cache_creation(900, None), (900, 0));
        assert_eq!(
            TokenCounts::split_cache_creation(900, Some((100, 800))),
            (100, 800)
        );
    }
}
