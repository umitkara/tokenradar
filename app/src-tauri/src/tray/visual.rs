//! What the tray icon shows, reduced to the visually meaningful fields.
//!
//! [`TrayVisual`] is the redraw key: percentages are rounded to whole points, everything else
//! is a flag. Two snapshots that map to the same visual draw the same pixels, so the icon is
//! not replaced (step 08, update throttle).

use tokenradar_core::domain::{LimitKind, Timestamp};
use tokenradar_core::limits::{Freshness, LimitView, PanelSnapshot, Severity, WARN_AT};
use tokenradar_core::providers::claude::usage_api::ApiState;

/// One ring: a limit's rounded percentage and its severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ring {
    /// `0..=100`; readings above 100 draw a full ring.
    pub percent: u8,
    pub severity: Severity,
}

impl Ring {
    fn of(limit: &LimitView) -> Self {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let percent = limit.percent.round().clamp(0.0, 100.0) as u8;
        Self {
            percent,
            severity: limit.severity,
        }
    }
}

/// The icon's content. Outer ring = weekly, inner ring = 5-hour window, on every surface
/// (design-language §1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrayVisual {
    pub weekly: Option<Ring>,
    pub five_hour: Option<Ring>,
    /// Every shown reading is older than 30 minutes: drawn at 50 % opacity.
    pub stale: bool,
    /// Limits cannot be read: a dot in the top-right corner.
    pub error: bool,
    /// Notifications are muted: a pause mark in the bottom-right corner.
    pub paused: bool,
}

/// The named states of step 08, derived from a [`TrayVisual`] (golden-test coverage).
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    NoData,
    Ok,
    Warn,
    Crit,
    Stale,
    Error,
    Paused,
}

#[cfg(test)]
impl TrayVisual {
    /// The most important state; markers combine freely in the drawing itself.
    pub fn state(self) -> TrayState {
        if self.error {
            return TrayState::Error;
        }
        if self.weekly.is_none() && self.five_hour.is_none() {
            return TrayState::NoData;
        }
        if self.stale {
            return TrayState::Stale;
        }
        if self.paused {
            return TrayState::Paused;
        }
        let worst = [self.weekly, self.five_hour]
            .into_iter()
            .flatten()
            .map(|r| r.severity)
            .fold(Severity::Ok, |a, b| match (a, b) {
                (Severity::Crit, _) | (_, Severity::Crit) => Severity::Crit,
                (Severity::Warn, _) | (_, Severity::Warn) => Severity::Warn,
                _ => Severity::Ok,
            });
        match worst {
            Severity::Ok => TrayState::Ok,
            Severity::Warn => TrayState::Warn,
            Severity::Crit => TrayState::Crit,
        }
    }
}

/// Everything the tray reads besides the clock.
#[derive(Debug, Clone, Default)]
pub struct TrayModel {
    /// The latest panel snapshot; `None` until the limit engine built one.
    pub snapshot: Option<PanelSnapshot>,
    /// The core did not start, so nothing will ever arrive.
    pub core_failed: bool,
    /// Notifications are muted until then (`kv.notify_snoozed_until`).
    pub muted_until: Option<Timestamp>,
}

impl TrayModel {
    /// The 5-hour window shown on the inner ring.
    pub fn five_hour(&self) -> Option<&LimitView> {
        self.limit(&LimitKind::Session)
    }

    /// The all-models weekly limit shown on the outer ring. Scoped weekly limits are panel-only.
    pub fn weekly(&self) -> Option<&LimitView> {
        self.limit(&LimitKind::WeeklyAll)
    }

    /// Weekly limits of one model or surface at or above the warning threshold, highest first.
    /// They stay off the rings (outer = all models, on every surface) but reach the tooltip.
    pub fn scoped_alerts(&self) -> Vec<&LimitView> {
        let mut out: Vec<_> = self
            .snapshot
            .iter()
            .flat_map(|s| &s.limits)
            .filter(|l| l.kind == LimitKind::WeeklyScoped && l.percent >= WARN_AT)
            .collect();
        out.sort_by(|a, b| b.percent.total_cmp(&a.percent));
        out
    }

    fn limit(&self, kind: &LimitKind) -> Option<&LimitView> {
        self.snapshot
            .as_ref()?
            .limits
            .iter()
            .find(|l| &l.kind == kind && l.scope.is_none())
    }

    pub fn is_muted(&self, now: Timestamp) -> bool {
        self.muted_until.is_some_and(|until| until > now)
    }

    /// Every shown reading is stale (> 30 min). Nothing shown is not "stale", it is "no data".
    pub fn is_stale(&self) -> bool {
        let shown: Vec<_> = [self.weekly(), self.five_hour()]
            .into_iter()
            .flatten()
            .collect();
        !shown.is_empty() && shown.iter().all(|l| l.freshness == Freshness::Stale)
    }

    /// Limits cannot be read and nothing fresh is on screen. A failing API next to a live
    /// statusline is not an error the user needs to see.
    pub fn is_error(&self) -> bool {
        if self.core_failed {
            return true;
        }
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        let failing = matches!(
            snapshot.api.state,
            ApiState::Error { .. } | ApiState::TokenStale
        );
        let nothing_fresh =
            (self.weekly().is_none() && self.five_hour().is_none()) || self.is_stale();
        failing && nothing_fresh
    }

    pub fn visual(&self, now: Timestamp) -> TrayVisual {
        TrayVisual {
            weekly: self.weekly().map(Ring::of),
            five_hour: self.five_hour().map(Ring::of),
            stale: self.is_stale(),
            error: self.is_error(),
            paused: self.is_muted(now),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use tokenradar_core::domain::Source;
    use tokenradar_core::limits::{ApiStatusView, IndexState, StatuslineState};
    use tokenradar_core::providers::claude::usage_api::PollDiagnostics;

    use super::*;

    pub(crate) fn limit(kind: LimitKind, percent: f64, freshness: Freshness) -> LimitView {
        LimitView {
            kind,
            scope: None,
            percent,
            resets_at: None,
            severity: Severity::of(percent),
            provider_severity: None,
            source: Source::Api,
            estimate_reason: None,
            observed_at: Timestamp::from_millis(0),
            freshness,
            projection: None,
        }
    }

    pub(crate) fn snapshot(limits: Vec<LimitView>, api: ApiState) -> PanelSnapshot {
        PanelSnapshot {
            account: None,
            limits,
            live_sessions: Vec::new(),
            top_sessions: Vec::new(),
            index_state: IndexState::default(),
            api: ApiStatusView {
                state: api,
                diagnostics: PollDiagnostics::default(),
            },
            statusline: StatuslineState::default(),
            generated_at: Timestamp::from_millis(0),
        }
    }

    fn model(limits: Vec<LimitView>, api: ApiState) -> TrayModel {
        TrayModel {
            snapshot: Some(snapshot(limits, api)),
            ..TrayModel::default()
        }
    }

    const NOW: Timestamp = Timestamp::from_millis(1_000_000);

    #[test]
    fn rings_map_weekly_outside_and_five_hour_inside() {
        let m = model(
            vec![
                limit(LimitKind::Session, 72.4, Freshness::Fresh),
                limit(LimitKind::WeeklyAll, 91.6, Freshness::Fresh),
            ],
            ApiState::Active,
        );
        let v = m.visual(NOW);
        assert_eq!(
            v.five_hour,
            Some(Ring {
                percent: 72,
                severity: Severity::Ok
            })
        );
        assert_eq!(
            v.weekly,
            Some(Ring {
                percent: 92,
                severity: Severity::Crit
            })
        );
        assert_eq!(v.state(), TrayState::Crit);
    }

    #[test]
    fn scoped_weekly_limits_stay_off_the_icon() {
        let mut scoped = limit(LimitKind::WeeklyScoped, 95.0, Freshness::Fresh);
        scoped.scope = Some(tokenradar_core::domain::Scope::Model("fable".into()));
        let v = model(vec![scoped], ApiState::Active).visual(NOW);
        assert_eq!(v.state(), TrayState::NoData);
    }

    #[test]
    fn percentages_are_clamped_and_rounded() {
        let m = model(
            vec![limit(LimitKind::Session, 104.0, Freshness::Fresh)],
            ApiState::Active,
        );
        assert_eq!(m.visual(NOW).five_hour.map(|r| r.percent), Some(100));
    }

    #[test]
    fn stale_needs_every_shown_reading_stale() {
        let both = |a, b| {
            model(
                vec![
                    limit(LimitKind::Session, 10.0, a),
                    limit(LimitKind::WeeklyAll, 10.0, b),
                ],
                ApiState::Active,
            )
        };
        assert!(both(Freshness::Stale, Freshness::Stale).visual(NOW).stale);
        assert!(!both(Freshness::Stale, Freshness::Fresh).visual(NOW).stale);
        assert!(!model(Vec::new(), ApiState::Active).visual(NOW).stale);
    }

    fn error() -> ApiState {
        ApiState::Error {
            retry_at: Timestamp::from_millis(0),
        }
    }

    #[test]
    fn errors_show_only_without_fresh_data() {
        let fresh = limit(LimitKind::Session, 10.0, Freshness::Fresh);
        let stale = limit(LimitKind::Session, 10.0, Freshness::Stale);
        assert!(!model(vec![fresh], error()).visual(NOW).error);
        assert!(
            model(vec![stale.clone()], ApiState::TokenStale)
                .visual(NOW)
                .error
        );
        assert!(model(Vec::new(), error()).visual(NOW).error);
        // Offline is a network state, not a failure; staleness already shows it.
        let offline = ApiState::Offline {
            retry_at: Timestamp::from_millis(0),
        };
        assert!(!model(vec![stale], offline).visual(NOW).error);
        let failed = TrayModel {
            core_failed: true,
            ..TrayModel::default()
        };
        assert_eq!(failed.visual(NOW).state(), TrayState::Error);
    }

    #[test]
    fn mute_expires() {
        let m = TrayModel {
            muted_until: Some(Timestamp::from_millis(NOW.as_millis() + 1)),
            ..TrayModel::default()
        };
        assert!(m.visual(NOW).paused);
        assert!(!m.visual(Timestamp::from_millis(NOW.as_millis() + 1)).paused);
    }
}
