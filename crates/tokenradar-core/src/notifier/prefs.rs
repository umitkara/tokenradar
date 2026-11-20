//! The `[notifications]` section of `config.toml`.
//!
//! ```toml
//! [notifications]
//! enabled = true
//! five_hour_thresholds = [75, 90, 100]
//! weekly_thresholds = [75, 90, 100]
//! projection = true
//! final_warning_min = 15      # 0 turns the final warning off
//! reset_notify_min = 80
//!
//! [notifications.quiet_hours]
//! enabled = false
//! start = "23:00"
//! end = "08:00"
//!
//! [notifications.digest]
//! weekly = true
//! weekly_at = "after_reset"   # or "custom": weekly_day and weekly_time
//! weekly_day = 0              # Monday-zero
//! weekly_time = "09:00"
//! daily = false
//! daily_time = "09:00"
//! ```

use serde::{Deserialize, Serialize};
use specta::Type;

/// At most this many thresholds per limit type.
pub const MAX_THRESHOLDS: usize = 5;
/// The projection rule looks this far ahead; the final warning must be shorter.
pub const PROJECTION_HORIZON_MIN: u32 = 60;
/// Weekly limits look a day ahead, at a high-confidence rate only (ADR-063): at a few points
/// per hour, an hour ahead is past the 90 % threshold already.
pub const WEEKLY_PROJECTION_HORIZON_MIN: u32 = 24 * 60;

/// When and how the notifier speaks. [`NotifyPrefs::normalized`] makes any value safe to use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct NotifyPrefs {
    /// Off: nothing is shown or recorded.
    pub enabled: bool,
    /// Percentages of the 5-hour window that raise a notification, ascending.
    pub five_hour_thresholds: Vec<u8>,
    /// The same for weekly limits, model-scoped ones included.
    pub weekly_thresholds: Vec<u8>,
    /// "At this rate it will be full in N min" (at most once per window).
    pub projection: bool,
    /// A second, last projection warning below this many minutes; `0` is off.
    pub final_warning_min: u32,
    /// A reset is announced only if the window peaked at or above this percentage.
    pub reset_notify_min: u8,
    pub quiet_hours: QuietHours,
    /// The periodic summaries (Step 17b).
    pub digest: DigestPrefs,
}

impl Default for NotifyPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            five_hour_thresholds: vec![75, 90, 100],
            weekly_thresholds: vec![75, 90, 100],
            projection: true,
            final_warning_min: 15,
            reset_notify_min: 80,
            quiet_hours: QuietHours::default(),
            digest: DigestPrefs::default(),
        }
    }
}

/// When the weekly summary is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum WeeklyAt {
    /// An hour after the weekly limit resets.
    #[default]
    AfterReset,
    /// On `weekly_day` at `weekly_time`, local time.
    Custom,
}

/// The periodic summaries: the past weekly limit window and, optionally, the past day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct DigestPrefs {
    pub weekly: bool,
    pub weekly_at: WeeklyAt,
    /// Monday-zero day for [`WeeklyAt::Custom`].
    pub weekly_day: u8,
    /// `HH:MM` for [`WeeklyAt::Custom`].
    pub weekly_time: String,
    pub daily: bool,
    /// `HH:MM`: when yesterday's summary is sent.
    pub daily_time: String,
}

impl Default for DigestPrefs {
    fn default() -> Self {
        Self {
            weekly: true,
            weekly_at: WeeklyAt::AfterReset,
            weekly_day: 0,
            weekly_time: "09:00".into(),
            daily: false,
            daily_time: "09:00".into(),
        }
    }
}

/// A daily span, in local time, in which notifications are recorded but not shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct QuietHours {
    pub enabled: bool,
    /// `HH:MM`, 24 hours.
    pub start: String,
    /// `HH:MM`; earlier than `start` means the span crosses midnight.
    pub end: String,
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: false,
            start: "23:00".into(),
            end: "08:00".into(),
        }
    }
}

impl QuietHours {
    /// `(start, end)` in minutes after midnight; `None` when off, empty or malformed.
    pub fn span(&self) -> Option<(u16, u16)> {
        if !self.enabled {
            return None;
        }
        let span = (parse_hhmm(&self.start)?, parse_hhmm(&self.end)?);
        (span.0 != span.1).then_some(span)
    }

    /// Whether `minute` (after local midnight) falls inside the span.
    pub fn contains(&self, minute: u16) -> bool {
        match self.span() {
            Some((start, end)) if start < end => (start..end).contains(&minute),
            Some((start, end)) => minute >= start || minute < end,
            None => false,
        }
    }
}

/// `"7:05"` or `"07:05"` → 425.
pub fn parse_hhmm(s: &str) -> Option<u16> {
    let (h, m) = s.trim().split_once(':')?;
    if !(1..=2).contains(&h.len()) || !(1..=2).contains(&m.len()) {
        return None;
    }
    let (h, m): (u16, u16) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

impl NotifyPrefs {
    /// Thresholds in 1–100, sorted, unique, at most [`MAX_THRESHOLDS`]; the final warning
    /// below the projection horizon; the reset filter in 0–100; quiet hours as `HH:MM`.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        for list in [&mut self.five_hour_thresholds, &mut self.weekly_thresholds] {
            list.retain(|t| (1..=100).contains(t));
            list.sort_unstable();
            list.dedup();
            list.truncate(MAX_THRESHOLDS);
        }
        self.final_warning_min = self.final_warning_min.min(PROJECTION_HORIZON_MIN - 1);
        self.reset_notify_min = self.reset_notify_min.min(100);
        let defaults = QuietHours::default();
        for (field, fallback) in [
            (&mut self.quiet_hours.start, defaults.start),
            (&mut self.quiet_hours.end, defaults.end),
        ] {
            *field = parse_hhmm(field).map_or(fallback, |m| format!("{:02}:{:02}", m / 60, m % 60));
        }
        let digest = DigestPrefs::default();
        for (field, fallback) in [
            (&mut self.digest.weekly_time, digest.weekly_time),
            (&mut self.digest.daily_time, digest.daily_time),
        ] {
            *field = parse_hhmm(field).map_or(fallback, |m| format!("{:02}:{:02}", m / 60, m % 60));
        }
        if self.digest.weekly_day > 6 {
            self.digest.weekly_day = 0;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizing_makes_any_value_safe() {
        let prefs = NotifyPrefs {
            five_hour_thresholds: vec![100, 0, 90, 90, 150, 50, 10, 20, 30],
            weekly_thresholds: vec![],
            final_warning_min: 90,
            reset_notify_min: 200,
            quiet_hours: QuietHours {
                enabled: true,
                start: "7:5".into(),
                end: "25:00".into(),
            },
            ..NotifyPrefs::default()
        }
        .normalized();
        assert_eq!(prefs.five_hour_thresholds, [10, 20, 30, 50, 90]);
        assert_eq!(prefs.weekly_thresholds.len(), 0);
        assert_eq!(prefs.final_warning_min, 59);
        assert_eq!(prefs.reset_notify_min, 100);
        assert_eq!(prefs.quiet_hours.start, "07:05");
        assert_eq!(prefs.quiet_hours.end, "08:00");
    }

    #[test]
    fn digest_times_and_day_are_normalized() {
        let prefs = NotifyPrefs {
            digest: DigestPrefs {
                weekly_day: 9,
                weekly_time: "7:5".into(),
                daily_time: "noon".into(),
                ..DigestPrefs::default()
            },
            ..NotifyPrefs::default()
        }
        .normalized();
        assert_eq!(prefs.digest.weekly_day, 0);
        assert_eq!(prefs.digest.weekly_time, "07:05");
        assert_eq!(prefs.digest.daily_time, "09:00");
    }

    #[test]
    fn a_config_without_a_digest_section_gets_the_defaults() {
        let prefs: NotifyPrefs = toml::from_str("enabled = true").expect("toml");
        assert_eq!(prefs.digest, DigestPrefs::default());
        assert!(prefs.digest.weekly && !prefs.digest.daily);
    }

    #[test]
    fn quiet_hours_may_cross_midnight() {
        let overnight = QuietHours {
            enabled: true,
            start: "23:00".into(),
            end: "08:00".into(),
        };
        assert!(overnight.contains(23 * 60));
        assert!(overnight.contains(2 * 60));
        assert!(!overnight.contains(8 * 60));
        assert!(!overnight.contains(12 * 60));

        let lunch = QuietHours {
            start: "12:00".into(),
            end: "13:30".into(),
            ..overnight.clone()
        };
        assert!(lunch.contains(12 * 60 + 59));
        assert!(!lunch.contains(13 * 60 + 30));

        let off = QuietHours {
            enabled: false,
            ..overnight
        };
        assert!(!off.contains(23 * 60));
    }

    #[test]
    fn times_are_strict() {
        assert_eq!(parse_hhmm("00:00"), Some(0));
        assert_eq!(parse_hhmm("23:59"), Some(1439));
        for bad in ["24:00", "12:60", "12", "12:5x", "-1:00", "123:00", "12:005"] {
            assert_eq!(parse_hhmm(bad), None, "{bad}");
        }
    }
}
