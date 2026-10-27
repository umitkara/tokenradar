//! v0.1 attribution (architecture §9.3): a session's share of the current 5-hour window is
//! `p_now × C_session / C_window`, subagents counted with their main session.
//!
//! It assumes the whole window was used on this machine, so the UI marks it with `≈`. Without
//! a measured `p_now` there is no percentage at all, only cost and tokens: no invented
//! numbers (vision, principle 1).

use crate::domain::{MicroUsd, SessionId};

/// Usage of one session tree (main session plus its subagents) in a time range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootUsage {
    pub root: SessionId,
    pub cost: MicroUsd,
    /// Input, output and cache-write tokens. Cache reads are left out: they dwarf the rest
    /// and say little about activity.
    pub tokens: u64,
}

/// Each root's `≈ %` of the window. `None` per root when no measured percentage exists or the
/// window has no cost to divide by.
pub fn shares(p_now: Option<f64>, usage: &[RootUsage]) -> Vec<(SessionId, Option<f64>)> {
    let total: i64 = usage.iter().map(|u| u.cost.0.max(0)).sum();
    usage
        .iter()
        .map(|u| {
            let share = match p_now {
                Some(p) if total > 0 => {
                    #[allow(clippy::cast_precision_loss)]
                    let frac = u.cost.0.max(0) as f64 / total as f64;
                    Some(p * frac)
                }
                _ => None,
            };
            (u.root, share)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn usage(costs: &[i64]) -> Vec<RootUsage> {
        costs
            .iter()
            .enumerate()
            .map(|(i, &c)| RootUsage {
                root: SessionId(i64::try_from(i).expect("small")),
                cost: MicroUsd(c),
                tokens: 0,
            })
            .collect()
    }

    #[test]
    fn shares_are_proportional_to_cost() {
        let s = shares(Some(60.0), &usage(&[3_000_000, 1_000_000]));
        assert_eq!(
            s,
            vec![(SessionId(0), Some(45.0)), (SessionId(1), Some(15.0))]
        );
    }

    #[test]
    fn no_percentage_without_a_measurement_or_cost() {
        assert!(
            shares(None, &usage(&[5, 7]))
                .iter()
                .all(|(_, s)| s.is_none())
        );
        assert!(
            shares(Some(30.0), &usage(&[0, 0]))
                .iter()
                .all(|(_, s)| s.is_none())
        );
    }

    proptest! {
        /// Conservation: the shares add up to the window's percentage.
        #[test]
        fn shares_sum_to_p_now(
            p in 0.0f64..150.0,
            costs in proptest::collection::vec(0i64..50_000_000, 1..20),
        ) {
            prop_assume!(costs.iter().any(|&c| c > 0));
            let total: f64 = shares(Some(p), &usage(&costs))
                .iter()
                .map(|(_, s)| s.expect("share"))
                .sum();
            prop_assert!((total - p).abs() < 1e-9 * p.max(1.0));
        }
    }
}
