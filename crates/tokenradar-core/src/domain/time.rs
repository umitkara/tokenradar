//! UTC timestamps with millisecond precision, the only time representation in storage.

use std::fmt;

use serde::{Deserialize, Serialize};
use specta::Type;

/// Width of a `usage_5m` rollup bucket.
pub const BUCKET_MS: i64 = 5 * 60 * 1000;

/// Milliseconds since the Unix epoch, UTC.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    pub const fn from_millis(ms: i64) -> Self {
        Self(ms)
    }

    pub const fn as_millis(self) -> i64 {
        self.0
    }

    pub fn now() -> Self {
        Self(jiff::Timestamp::now().as_millisecond())
    }

    /// Parses an RFC 3339 / ISO 8601 instant such as `2026-10-26T20:41:12.345Z`.
    /// Sub-millisecond digits are truncated.
    pub fn parse_iso(s: &str) -> Result<Self, jiff::Error> {
        s.parse::<jiff::Timestamp>()
            .map(|t| Self(t.as_millisecond()))
    }

    /// Start of the five-minute bucket that contains this instant.
    #[must_use]
    pub const fn bucket_5m(self) -> Self {
        Self(self.0 - self.0.rem_euclid(BUCKET_MS))
    }

    #[must_use]
    pub const fn saturating_add_millis(self, ms: i64) -> Self {
        Self(self.0.saturating_add(ms))
    }

    #[must_use]
    pub const fn saturating_sub_millis(self, ms: i64) -> Self {
        Self(self.0.saturating_sub(ms))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match jiff::Timestamp::from_millisecond(self.0) {
            Ok(t) => t.fmt(f),
            Err(_) => write!(f, "{}ms", self.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_claude_code_timestamps() {
        let ts = Timestamp::parse_iso("2026-10-26T20:41:12.345Z").expect("valid");
        assert_eq!(ts.as_millis(), 1_793_047_272_345);
        assert_eq!(ts.to_string(), "2026-10-26T20:41:12.345Z");
    }

    #[test]
    fn rejects_malformed_timestamps() {
        assert!(Timestamp::parse_iso("2026-13-01T00:00:00Z").is_err());
        assert!(Timestamp::parse_iso("yesterday").is_err());
    }

    #[test]
    fn buckets_floor_to_five_minutes() {
        let ts = Timestamp::from_millis(BUCKET_MS * 7 + 1234);
        assert_eq!(ts.bucket_5m().as_millis(), BUCKET_MS * 7);
        assert_eq!(ts.bucket_5m().bucket_5m(), ts.bucket_5m());
        // Pre-epoch instants floor downwards too.
        assert_eq!(
            Timestamp::from_millis(-1).bucket_5m().as_millis(),
            -BUCKET_MS
        );
    }
}
