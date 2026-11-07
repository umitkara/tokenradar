//! Which limits an account has (ADR-041).
//!
//! A limit is **present** while readings of it arrive, **unknown** before the first one, and
//! **absent** once the sources evidently report the account without it. Verified Ultra
//! accounts, for example, have no 5-hour window. An absent limit gets no estimate, no
//! notification and no place on any surface. Its first reading makes it present again.
//!
//! The evidence:
//!
//! - **Usage API (S2):** a response is a full listing. A limit it lacks is absent at once.
//! - **Statusline (S1):** Claude Code drops a window from the payload once its reset passed,
//!   and it calls the statusline without a new API response too (cache expiry, permission
//!   mode, `refreshInterval`). A missing window only counts in a payload that carries a
//!   *changed* reading of another limit, because that proves a fresh API response, and a fresh
//!   response always carries every window the account has. [`ABSENT_AFTER`] such payloads in
//!   a row make the limit absent.
//!
//! The absent set is kept in `kv` so that a restart, and the CLI with the app closed, know it.
//! The miss counters are not: after a restart the count starts over.

use std::collections::{HashMap, HashSet};

use super::engine::{PERCENT_EPSILON, same_window};
use crate::domain::{AccountId, LimitKind, LimitObservation, LimitRef, Timestamp};

/// Statusline payloads in a row that must lack a limit before it counts as absent.
pub const ABSENT_AFTER: u8 = 3;

/// Limits every Claude account was expected to have: they can be absent before any reading.
pub const EXPECTED: [LimitKind; 2] = [LimitKind::Session, LimitKind::WeeklyAll];

/// `kv` key of an account's absent limits.
pub fn kv_key(account: AccountId) -> String {
    format!("limits.absent.{}", account.0)
}

/// The stored form: a JSON array of [`LimitRef::encode`] strings, sorted.
pub fn encode(absent: &[LimitRef]) -> String {
    let mut items: Vec<String> = absent.iter().map(LimitRef::encode).collect();
    items.sort();
    serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_owned())
}

/// Reads [`encode`]'s output. Unreadable entries are skipped, never fatal.
pub fn decode(value: &str) -> Vec<LimitRef> {
    serde_json::from_str::<Vec<String>>(value)
        .unwrap_or_default()
        .iter()
        .filter_map(|s| LimitRef::decode(s))
        .collect()
}

/// One account's presence state.
#[derive(Debug, Default)]
pub(super) struct Presence {
    absent: HashSet<LimitRef>,
    /// Fresh statusline payloads in a row that lacked the limit.
    misses: HashMap<LimitRef, u8>,
    /// The latest statusline reading of each limit: `(percent, resets_at)`.
    last: HashMap<LimitRef, (f64, Option<Timestamp>)>,
}

impl Presence {
    pub fn with_absent(absent: impl IntoIterator<Item = LimitRef>) -> Self {
        Self {
            absent: absent.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn is_absent(&self, r: &LimitRef) -> bool {
        self.absent.contains(r)
    }

    /// The absent limits in their stored order.
    pub fn absent(&self) -> Vec<LimitRef> {
        let mut out: Vec<LimitRef> = self.absent.iter().cloned().collect();
        out.sort_by_key(LimitRef::encode);
        out
    }

    /// A real reading of `r` arrived. Returns `true` if it was absent until now.
    pub fn observed(&mut self, r: &LimitRef) -> bool {
        self.misses.remove(r);
        self.absent.remove(r)
    }

    /// One statusline payload's readings; `covered` are the limits a payload can carry.
    /// Returns the limits that became absent.
    pub fn statusline(
        &mut self,
        readings: &[LimitObservation],
        covered: &[LimitRef],
    ) -> Vec<LimitRef> {
        let mut fresh = false;
        for obs in readings {
            let prev = self
                .last
                .insert(LimitRef::of(obs), (obs.percent, obs.resets_at));
            // The first reading after a start proves nothing: there is nothing to compare.
            fresh |= prev.is_some_and(|(percent, resets_at)| {
                (percent - obs.percent).abs() > PERCENT_EPSILON
                    || !same_window(resets_at, obs.resets_at)
            });
        }
        if !fresh {
            return Vec::new();
        }
        let mut became = Vec::new();
        for r in covered {
            if readings
                .iter()
                .any(|o| o.kind == r.kind && o.scope == r.scope)
            {
                continue;
            }
            let misses = self.misses.entry(r.clone()).or_default();
            *misses = misses.saturating_add(1);
            if *misses >= ABSENT_AFTER && self.absent.insert(r.clone()) {
                became.push(r.clone());
            }
        }
        became
    }

    /// A full listing of the account's limits (a usage API response): each of `candidates`
    /// not in `listed` is absent. Returns the limits that became absent.
    pub fn listing(
        &mut self,
        listed: &[LimitRef],
        candidates: impl IntoIterator<Item = LimitRef>,
    ) -> Vec<LimitRef> {
        let mut became = Vec::new();
        for r in candidates {
            if !listed.contains(&r) && self.absent.insert(r.clone()) {
                self.misses.remove(&r);
                became.push(r);
            }
        }
        became
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Scope, Source};

    fn reading(kind: LimitKind, percent: f64, resets_at: i64) -> LimitObservation {
        LimitObservation {
            account: AccountId(1),
            kind,
            scope: None,
            percent,
            resets_at: Some(Timestamp::from_millis(resets_at)),
            severity: None,
            source: Source::Statusline,
            session_uuid: None,
            observed_at: Timestamp::from_millis(0),
        }
    }

    fn covered() -> Vec<LimitRef> {
        EXPECTED.map(|k| LimitRef::new(k, None)).to_vec()
    }

    fn session() -> LimitRef {
        LimitRef::new(LimitKind::Session, None)
    }

    #[test]
    fn only_payloads_with_a_changed_reading_count() {
        let mut p = Presence::default();
        let weekly = |pct| [reading(LimitKind::WeeklyAll, pct, 9_000_000)];
        // The first payload has nothing to compare with; repeats prove no fresh response.
        for _ in 0..10 {
            assert_eq!(p.statusline(&weekly(40.0), &covered()).len(), 0);
        }
        assert!(!p.is_absent(&session()));
        assert_eq!(p.statusline(&weekly(40.1), &covered()).len(), 0);
        assert_eq!(p.statusline(&weekly(40.2), &covered()).len(), 0);
        assert_eq!(p.statusline(&weekly(40.3), &covered()), vec![session()]);
        assert!(p.is_absent(&session()));
        // Already absent: not reported again.
        assert_eq!(p.statusline(&weekly(40.4), &covered()).len(), 0);
    }

    #[test]
    fn a_reading_resets_the_count_and_the_decision() {
        let mut p = Presence::default();
        let weekly = |pct| [reading(LimitKind::WeeklyAll, pct, 9_000_000)];
        p.statusline(&weekly(1.0), &covered());
        p.statusline(&weekly(2.0), &covered());
        p.statusline(&weekly(3.0), &covered());
        assert!(!p.observed(&session()), "was not absent yet");
        p.statusline(&weekly(4.0), &covered());
        p.statusline(&weekly(5.0), &covered());
        assert!(!p.is_absent(&session()), "the count started over");
        p.statusline(&weekly(6.0), &covered());
        assert!(p.is_absent(&session()));
        assert!(p.observed(&session()));
        assert!(!p.is_absent(&session()));
    }

    #[test]
    fn a_new_window_counts_as_a_change() {
        let mut p = Presence::default();
        p.statusline(&[reading(LimitKind::WeeklyAll, 0.0, 1_000)], &covered());
        for (i, at) in [2_000_000, 4_000_000, 6_000_000].into_iter().enumerate() {
            let became = p.statusline(&[reading(LimitKind::WeeklyAll, 0.0, at)], &covered());
            assert_eq!(became.is_empty(), i < 2);
        }
    }

    #[test]
    fn a_listing_is_authoritative() {
        let mut p = Presence::default();
        let fable = LimitRef::new(LimitKind::WeeklyScoped, Some(Scope::Model("Fable".into())));
        let weekly = LimitRef::new(LimitKind::WeeklyAll, None);
        let became = p.listing(
            std::slice::from_ref(&weekly),
            [session(), weekly.clone(), fable.clone()],
        );
        assert_eq!(became, vec![session(), fable]);
        assert_eq!(p.listing(&[weekly], [session()]).len(), 0);
    }

    #[test]
    fn the_stored_form_round_trips_and_tolerates_garbage() {
        let fable = LimitRef::new(LimitKind::WeeklyScoped, Some(Scope::Model("Fable".into())));
        let stored = encode(&[fable.clone(), session()]);
        assert_eq!(stored, r#"["session","weekly_scoped|model:Fable"]"#);
        assert_eq!(decode(&stored), vec![session(), fable]);
        assert_eq!(decode("not json").len(), 0);
        assert_eq!(decode(r#"["", "session"]"#), vec![session()]);
    }
}
