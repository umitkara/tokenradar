//! What the widget shows, without drawing it.
//!
//! [`WidgetModel`] is fed from the tray's model (the same snapshot, the same rules for stale
//! and error), so the widget never disagrees with the tray icon. [`WidgetModel::visual`] turns
//! it into [`WidgetVisual`], the redraw key: texts are final, positions are quantized, and two
//! equal visuals draw the same pixels.

use tokenradar_core::domain::{Source, Timestamp};
use tokenradar_core::limits::{Freshness, LimitView, Severity};
use tokenradar_i18n::{Arg, Localizer, Weekday};

use crate::tray::system::LocalClock;
use crate::tray::visual::TrayModel;

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 24 * 60 * MINUTE_MS;
/// The time mark moves in steps of a quarter point, so a minute's drift does not redraw.
const MARK_STEPS: f64 = 400.0;

/// One limit, reduced to what the widget needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub percent: f64,
    pub severity: Severity,
    pub estimate: bool,
    pub stale: bool,
    pub resets_at: Option<Timestamp>,
    pub window_start: Option<Timestamp>,
}

impl Bar {
    fn of(l: &LimitView) -> Self {
        Self {
            percent: l.percent,
            severity: l.severity,
            estimate: l.source == Source::Estimate,
            stale: l.freshness == Freshness::Stale,
            resets_at: l.resets_at,
            window_start: l.window_start,
        }
    }
}

/// The widget's input. Top bar = weekly, bottom bar = 5-hour window (design-language §1).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WidgetModel {
    pub weekly: Option<Bar>,
    /// `None` hides the second row: the account has no 5-hour window (ADR-041) or no reading
    /// of it yet. The gap is never filled with a guess.
    pub five_hour: Option<Bar>,
    /// Limits cannot be read (the tray's error dot).
    pub error: bool,
}

impl WidgetModel {
    pub fn of(tray: &TrayModel) -> Self {
        Self {
            weekly: tray.weekly().map(Bar::of),
            five_hour: tray.five_hour().map(Bar::of),
            error: tray.is_error(),
        }
    }

    pub fn visual(
        &self,
        l10n: &Localizer,
        clock: &dyn LocalClock,
        now: Timestamp,
        pace_mark: bool,
    ) -> WidgetVisual {
        let row = |bar: Option<&Bar>, label: &str, with_day: bool| {
            Row::of(bar, l10n.msg(label), l10n, clock, now, pace_mark, with_day)
        };
        let mut rows = vec![row(self.weekly.as_ref(), "widget-label-weekly", true)];
        if let Some(bar) = &self.five_hour {
            rows.push(row(Some(bar), "widget-label-five-hour", false));
        }
        let widest_value = l10n.fmt(
            "tray-value",
            &[("pct", Arg::Num(100.0)), ("estimate", Arg::Str("yes"))],
        );
        let reset_samples = std::iter::once("00:00".to_owned())
            .chain((0..7).map(|d| Weekday::from_sunday0(d).abbr(l10n.locale()).to_owned()))
            .collect();
        WidgetVisual {
            rows,
            error: self.error,
            widest_value,
            reset_samples,
        }
    }

    /// When the visual next changes by itself: the next minute of a countdown, the moment a
    /// weekday turns into a countdown, a reset, or the next minute while a time mark moves.
    pub fn next_change(&self, now: Timestamp, pace_mark: bool) -> Option<Timestamp> {
        let now_ms = now.as_millis();
        let mut wait: Option<i64> = None;
        let mut take = |ms: i64| {
            if ms > 0 {
                wait = Some(wait.map_or(ms, |w| w.min(ms)));
            }
        };
        for bar in [&self.weekly, &self.five_hour].into_iter().flatten() {
            let Some(reset) = bar.resets_at else { continue };
            let left = reset.as_millis() - now_ms;
            if left <= 0 {
                continue;
            }
            if left > DAY_MS {
                take(left - DAY_MS);
            } else {
                take(match left % MINUTE_MS {
                    0 => MINUTE_MS,
                    part => part,
                });
            }
            if pace_mark && bar.window_start.is_some() {
                take(MINUTE_MS - now_ms.rem_euclid(MINUTE_MS));
            }
        }
        wait.map(|w| Timestamp::from_millis(now_ms + w))
    }
}

/// One row as drawn: `7g ███░░░░░░░ %34 · Çar`.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub label: String,
    /// `%34`, `≈%72`, or `–` without a reading.
    pub value: String,
    /// The weekday of the reset (`Çar`) or the time left (`1:48`).
    pub reset: Option<String>,
    /// `0..=1`.
    pub fill: f32,
    /// `None` without a reading: an empty track.
    pub severity: Option<Severity>,
    pub estimate: bool,
    pub stale: bool,
    /// Where the time mark stands, `0..=1`.
    pub mark: Option<f32>,
}

impl Row {
    fn of(
        bar: Option<&Bar>,
        label: String,
        l10n: &Localizer,
        clock: &dyn LocalClock,
        now: Timestamp,
        pace_mark: bool,
        with_day: bool,
    ) -> Self {
        let Some(bar) = bar else {
            return Self {
                label,
                value: l10n.msg("widget-value-none"),
                reset: None,
                fill: 0.0,
                severity: None,
                estimate: false,
                stale: false,
                mark: None,
            };
        };
        let value = l10n.fmt(
            "tray-value",
            &[
                ("pct", Arg::Num(bar.percent.round())),
                (
                    "estimate",
                    Arg::Str(if bar.estimate { "yes" } else { "no" }),
                ),
            ],
        );
        #[allow(clippy::cast_possible_truncation)]
        let fill = (bar.percent.round() / 100.0).clamp(0.0, 1.0) as f32;
        Self {
            label,
            value,
            reset: bar
                .resets_at
                .and_then(|r| reset_text(l10n, clock, r, now, with_day)),
            fill,
            severity: Some(bar.severity),
            estimate: bar.estimate,
            stale: bar.stale,
            mark: if pace_mark {
                time_mark(bar.window_start, bar.resets_at, now)
            } else {
                None
            },
        }
    }
}

/// The weekday of a reset more than a day away (`Çar`), else the time left rounded up to the
/// minute (`1:48`), so it never claims an earlier reset than the real one.
fn reset_text(
    l10n: &Localizer,
    clock: &dyn LocalClock,
    reset: Timestamp,
    now: Timestamp,
    with_day: bool,
) -> Option<String> {
    let left = reset.as_millis() - now.as_millis();
    if left <= 0 {
        return None;
    }
    if with_day && left > DAY_MS {
        return clock
            .local(reset)
            .map(|t| t.weekday.abbr(l10n.locale()).to_owned());
    }
    let minutes = (left + MINUTE_MS - 1) / MINUTE_MS;
    Some(format!("{}:{:02}", minutes / 60, minutes % 60))
}

/// The share of the window gone at `now` (ADR-044), as the panel's bars show it.
fn time_mark(start: Option<Timestamp>, end: Option<Timestamp>, now: Timestamp) -> Option<f32> {
    let (start, end, now) = (start?.as_millis(), end?.as_millis(), now.as_millis());
    if end <= start || now < start || now >= end {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let share = (now - start) as f64 / (end - start) as f64;
    #[allow(clippy::cast_possible_truncation)]
    Some(((share * MARK_STEPS).floor() / MARK_STEPS) as f32)
}

/// Everything that decides the widget's pixels, besides the environment.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetVisual {
    /// One row (weekly) or two (weekly, 5-hour).
    pub rows: Vec<Row>,
    pub error: bool,
    /// The widest value text (`≈%100`): the value column is this wide, so bars do not jump.
    pub widest_value: String,
    /// Every reset text a row can show, for the reset column's width.
    pub reset_samples: Vec<String>,
}

impl WidgetVisual {
    /// What a screen reader says for the strip (the window's title): the limits in words,
    /// with the same values and reset texts the strip shows.
    pub fn spoken(&self, l10n: &Localizer) -> String {
        let rows: Vec<String> = self
            .rows
            .iter()
            .zip(["widget-a11y-weekly", "widget-a11y-five-hour"])
            .map(|(row, key)| {
                let limit = l10n.fmt(key, &[("value", Arg::Str(&row.value))]);
                match &row.reset {
                    Some(reset) => l10n.fmt(
                        "widget-a11y-resets",
                        &[("limit", Arg::Str(&limit)), ("reset", Arg::Str(reset))],
                    ),
                    None => limit,
                }
            })
            .collect();
        l10n.fmt("widget-a11y", &[("rows", Arg::Str(&rows.join("; ")))])
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use tokenradar_core::domain::LimitKind;
    use tokenradar_core::providers::claude::usage_api::ApiState;
    use tokenradar_i18n::{Domain, HourCycle, Locale};

    use super::*;
    use crate::tray::system::LocalTime;
    use crate::tray::visual::tests::{limit, snapshot};

    /// UTC clock; day 0 of the epoch was a Thursday.
    pub(crate) struct UtcFake;

    impl LocalClock for UtcFake {
        fn local(&self, t: Timestamp) -> Option<LocalTime> {
            let ms = t.as_millis();
            let days = ms.div_euclid(DAY_MS);
            let in_day = ms.rem_euclid(DAY_MS);
            Some(LocalTime {
                date: (1970, 1, 1),
                weekday: Weekday::from_sunday0(u16::try_from((days + 4).rem_euclid(7)).ok()?),
                hour: u8::try_from(in_day / 3_600_000).ok()?,
                minute: u8::try_from(in_day / MINUTE_MS % 60).ok()?,
            })
        }

        fn hour_cycle(&self) -> HourCycle {
            HourCycle::H23
        }
    }

    /// A Monday, 09:00 UTC.
    pub(crate) const NOW: Timestamp = Timestamp::from_millis(4 * DAY_MS + 9 * 3_600_000);

    fn l10n(locale: Locale) -> Localizer {
        Localizer::new(locale, &[Domain::Tray])
    }

    fn tray(limits: Vec<LimitView>) -> TrayModel {
        TrayModel {
            snapshot: Some(snapshot(limits, ApiState::Active)),
            ..TrayModel::default()
        }
    }

    #[allow(clippy::unnecessary_wraps)] // the type of `resets_at`
    fn at(ms: i64) -> Option<Timestamp> {
        Some(Timestamp::from_millis(NOW.as_millis() + ms))
    }

    #[test]
    fn rows_are_weekly_on_top_and_five_hour_below() {
        let mut weekly = limit(LimitKind::WeeklyAll, 34.4, Freshness::Fresh);
        weekly.resets_at = at(2 * DAY_MS);
        let mut five = limit(LimitKind::Session, 71.6, Freshness::Fresh);
        five.resets_at = at(108 * MINUTE_MS - 30_000);
        let v = WidgetModel::of(&tray(vec![five, weekly])).visual(
            &l10n(Locale::Tr),
            &UtcFake,
            NOW,
            false,
        );
        assert_eq!(v.rows.len(), 2);
        assert_eq!(
            (v.rows[0].label.as_str(), v.rows[0].value.as_str()),
            ("7g", "%34")
        );
        // Monday + 2 days.
        assert_eq!(v.rows[0].reset.as_deref(), Some("Çar"));
        assert_eq!(
            (v.rows[1].label.as_str(), v.rows[1].value.as_str()),
            ("5s", "%72")
        );
        assert_eq!(v.rows[1].reset.as_deref(), Some("1:48"));
        assert!((v.rows[1].fill - 0.72).abs() < 1e-6);
    }

    #[test]
    fn without_a_five_hour_reading_there_is_one_row() {
        let weekly = limit(LimitKind::WeeklyAll, 10.0, Freshness::Fresh);
        let v =
            WidgetModel::of(&tray(vec![weekly])).visual(&l10n(Locale::En), &UtcFake, NOW, false);
        assert_eq!(v.rows.len(), 1);
        assert_eq!(v.rows[0].label, "7d");
        assert_eq!(v.rows[0].value, "10%");
        // Nothing at all yet: one empty weekly row.
        let empty = WidgetModel::default().visual(&l10n(Locale::Tr), &UtcFake, NOW, false);
        assert_eq!(empty.rows.len(), 1);
        assert_eq!(empty.rows[0].severity, None);
        assert_eq!(empty.rows[0].value, "–");
    }

    #[test]
    fn estimates_carry_the_sign_and_stale_rows_are_marked() {
        let mut five = limit(LimitKind::Session, 40.0, Freshness::Stale);
        five.source = Source::Estimate;
        let weekly = limit(LimitKind::WeeklyAll, 10.0, Freshness::Fresh);
        let v = WidgetModel::of(&tray(vec![weekly, five])).visual(
            &l10n(Locale::Tr),
            &UtcFake,
            NOW,
            false,
        );
        assert_eq!(v.rows[1].value, "≈%40");
        assert!(v.rows[1].estimate && v.rows[1].stale);
        assert!(!v.rows[0].stale);
    }

    #[test]
    fn a_weekly_reset_within_a_day_becomes_a_countdown() {
        let mut weekly = limit(LimitKind::WeeklyAll, 80.0, Freshness::Fresh);
        weekly.resets_at = at(DAY_MS - MINUTE_MS);
        let m = WidgetModel::of(&tray(vec![weekly]));
        let v = m.visual(&l10n(Locale::Tr), &UtcFake, NOW, false);
        assert_eq!(v.rows[0].reset.as_deref(), Some("23:59"));
    }

    #[test]
    fn the_time_mark_follows_the_window_and_is_optional() {
        let mut weekly = limit(LimitKind::WeeklyAll, 50.0, Freshness::Fresh);
        weekly.window_start = at(-DAY_MS);
        weekly.resets_at = at(DAY_MS);
        let m = WidgetModel::of(&tray(vec![weekly]));
        let on = m.visual(&l10n(Locale::Tr), &UtcFake, NOW, true);
        assert_eq!(on.rows[0].mark, Some(0.5));
        let off = m.visual(&l10n(Locale::Tr), &UtcFake, NOW, false);
        assert_eq!(off.rows[0].mark, None);
    }

    #[test]
    fn redraws_wait_for_the_next_visible_change() {
        let mut weekly = limit(LimitKind::WeeklyAll, 50.0, Freshness::Fresh);
        weekly.resets_at = at(3 * DAY_MS);
        let mut m = WidgetModel::of(&tray(vec![weekly.clone()]));
        // A weekday only changes when the reset comes within a day.
        assert_eq!(m.next_change(NOW, false), at(2 * DAY_MS));
        // A countdown changes at its next minute boundary.
        let mut five = limit(LimitKind::Session, 10.0, Freshness::Fresh);
        five.resets_at = at(90 * MINUTE_MS + 15_000);
        m = WidgetModel::of(&tray(vec![weekly, five]));
        assert_eq!(m.next_change(NOW, false), at(15_000));
        // Without resets nothing moves by itself.
        assert_eq!(WidgetModel::default().next_change(NOW, true), None);
    }

    #[test]
    fn the_spoken_name_reads_the_rows_in_words() {
        let mut weekly = limit(LimitKind::WeeklyAll, 34.4, Freshness::Fresh);
        weekly.resets_at = at(2 * DAY_MS);
        let five = limit(LimitKind::Session, 71.6, Freshness::Fresh);
        let tr = l10n(Locale::Tr);
        let v =
            WidgetModel::of(&tray(vec![five, weekly.clone()])).visual(&tr, &UtcFake, NOW, false);
        assert_eq!(
            v.spoken(&tr),
            "TokenRadar: Haftalık limit %34, yenilenme: Çar; 5 saatlik limit %72"
        );
        let en = l10n(Locale::En);
        let v = WidgetModel::of(&tray(vec![weekly])).visual(&en, &UtcFake, NOW, false);
        assert_eq!(v.spoken(&en), "TokenRadar: Weekly limit 34%, resets: Wed");
    }

    #[test]
    fn errors_come_from_the_tray_rules() {
        let failed = TrayModel {
            core_failed: true,
            ..TrayModel::default()
        };
        assert!(WidgetModel::of(&failed).error);
    }
}
