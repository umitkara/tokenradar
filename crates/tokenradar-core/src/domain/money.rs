//! Money as integer micro-dollars, so sums never drift.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub};

use serde::{Deserialize, Serialize};
use specta::Type;

/// US dollars × 10⁶. A price of $1 per million tokens is exactly one micro-dollar per token.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[serde(transparent)]
pub struct MicroUsd(pub i64);

impl MicroUsd {
    pub const ZERO: Self = Self(0);

    /// Rounds a fractional micro-dollar amount half away from zero.
    #[allow(clippy::cast_possible_truncation)] // saturating `as` is the intent for absurd inputs
    pub fn from_f64_rounded(micro_usd: f64) -> Self {
        Self(micro_usd.round() as i64)
    }
}

impl fmt::Display for MicroUsd {
    /// `$12.345678`. Display only: the UI formats money through Fluent.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        write!(f, "{sign}${}.{:06}", abs / 1_000_000, abs % 1_000_000)
    }
}

impl Add for MicroUsd {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for MicroUsd {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for MicroUsd {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Neg for MicroUsd {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Sum for MicroUsd {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, Add::add)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_dollars() {
        assert_eq!(MicroUsd(12_345_678).to_string(), "$12.345678");
        assert_eq!(MicroUsd(-5).to_string(), "-$0.000005");
    }

    #[test]
    fn rounds_half_away_from_zero() {
        assert_eq!(MicroUsd::from_f64_rounded(2.5), MicroUsd(3));
        assert_eq!(MicroUsd::from_f64_rounded(2.4999), MicroUsd(2));
    }
}
