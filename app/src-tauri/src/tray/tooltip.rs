//! The tray tooltip: localized, at most 127 UTF-16 units (the `szTip` limit of Windows).
//!
//! ```text
//! TokenRadar
//! 5s: %72 · 1 sa 48 dk sonra yenilenir
//! Haftalık: %34 · Çar 10:59'da yenilenir
//! ```
//!
//! A model's own weekly limit (`Fable · haftalık`) gets a line from 75 % on; the rings never
//! show it.
//!
//! When the full text does not fit, the title is dropped first; then every line falls back to
//! its short form
//! (`5s: %72 (1 sa 48 dk)`, `Sessiz: 10:00'a kadar`), and only then is the text cut. The
//! longest combination (estimate, stale, error and mute together) fits in the short form.

use tokenradar_core::domain::{Scope, Source, Timestamp};
use tokenradar_core::limits::LimitView;
use tokenradar_i18n::format::{clock, weekday_clock};
use tokenradar_i18n::{Arg, Localizer};

use super::system::{LocalClock, LocalTime};
use super::visual::TrayModel;

/// `NOTIFYICONDATAW::szTip` holds 128 UTF-16 units including the terminating NUL.
pub const MAX_UTF16: usize = 127;

const MINUTE_MS: i64 = 60_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Full,
    /// Full lines without the "TokenRadar" title.
    Untitled,
    Short,
}

impl Form {
    /// The message key for this form.
    const fn pick(self, full: &'static str, short: &'static str) -> &'static str {
        match self {
            Self::Full | Self::Untitled => full,
            Self::Short => short,
        }
    }
}

pub fn tooltip(
    model: &TrayModel,
    l10n: &Localizer,
    clock: &dyn LocalClock,
    now: Timestamp,
) -> String {
    for form in [Form::Full, Form::Untitled] {
        let text = compose(model, l10n, clock, now, form);
        if fits(&text) {
            return text;
        }
    }
    truncate(compose(model, l10n, clock, now, Form::Short))
}

fn compose(
    model: &TrayModel,
    l10n: &Localizer,
    clock: &dyn LocalClock,
    now: Timestamp,
    form: Form,
) -> String {
    let mut lines = Vec::new();
    // Long texts give the title's room to the data.
    if form == Form::Full || model.core_failed {
        lines.push(l10n.msg("tray-title"));
    }
    if model.core_failed {
        lines.push(l10n.msg("tray-error"));
        return lines.join("\n");
    }
    let today = clock.local(now).map(|t| t.date);
    let at = |t: Timestamp, with_day: bool| -> Option<String> {
        let local = clock.local(t)?;
        Some(if with_day || Some(local.date) != today {
            weekday_clock(
                l10n.locale(),
                local.weekday,
                local.hour,
                local.minute,
                clock.hour_cycle(),
            )
        } else {
            short_clock(l10n, clock, local)
        })
    };

    let five_hour = model.five_hour();
    let weekly = model.weekly();
    if let Some(l) = five_hour {
        let value = value(l10n, l);
        let remaining = l.resets_at.map(|r| r.as_millis() - now.as_millis());
        lines.push(match remaining {
            Some(ms) if ms > 0 => {
                let key = form.pick("tray-five-hour-resets", "tray-five-hour-resets-short");
                l10n.fmt(
                    key,
                    &[
                        ("value", Arg::Str(&value)),
                        ("ms", Arg::from(ceil_minute(ms))),
                    ],
                )
            }
            _ => l10n.fmt("tray-five-hour", &[("value", Arg::Str(&value))]),
        });
    }
    if let Some(l) = weekly {
        let value = value(l10n, l);
        let reset = l.resets_at.filter(|&r| r > now).and_then(|r| at(r, true));
        lines.push(match reset {
            Some(when) => {
                let key = form.pick("tray-weekly-resets", "tray-weekly-resets-short");
                l10n.fmt(key, &[("value", Arg::Str(&value)), ("at", Arg::Str(&when))])
            }
            None => l10n.fmt("tray-weekly", &[("value", Arg::Str(&value))]),
        });
    }
    for l in model.scoped_alerts() {
        let value = value(l10n, l);
        let name = scope_name(l.scope.as_ref());
        let reset = l.resets_at.filter(|&r| r > now).and_then(|r| at(r, true));
        let mut args = vec![("name", Arg::Str(&name)), ("value", Arg::Str(&value))];
        let key = match (form, &reset) {
            (Form::Short, _) => "tray-weekly-scoped-short",
            (Form::Full | Form::Untitled, Some(when)) => {
                args.push(("at", Arg::Str(when)));
                "tray-weekly-scoped-resets"
            }
            (Form::Full | Form::Untitled, None) => "tray-weekly-scoped",
        };
        lines.push(l10n.fmt(key, &args));
    }
    if five_hour.is_none() && weekly.is_none() {
        lines.push(l10n.msg("tray-no-data"));
    }
    if model.is_stale() {
        let last = [five_hour, weekly]
            .into_iter()
            .flatten()
            .map(|l| l.observed_at)
            .max();
        if let Some(when) = last.and_then(|t| at(t, false)) {
            lines.push(l10n.fmt("tray-stale", &[("at", Arg::Str(&when))]));
        }
    }
    if model.is_error() {
        lines.push(l10n.msg(form.pick("tray-error", "tray-error-short")));
    }
    if model.is_muted(now)
        && let Some(until) = model.muted_until.and_then(|t| clock.local(t))
    {
        let until = short_clock(l10n, clock, until);
        let key = form.pick("tray-muted", "tray-muted-short");
        lines.push(l10n.fmt(key, &[("until", Arg::Str(&until))]));
    }
    lines.join("\n")
}

/// `Scope::Model("fable")` → `Fable`; a surface keeps its name.
fn scope_name(scope: Option<&Scope>) -> String {
    let raw = match scope {
        Some(Scope::Model(m) | Scope::Surface(m)) => m.as_str(),
        None => "?",
    };
    let mut chars = raw.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn short_clock(l10n: &Localizer, clock: &dyn LocalClock, t: LocalTime) -> String {
    self::clock(l10n.locale(), t.hour, t.minute, clock.hour_cycle())
}

/// `%72`, or `≈%0` for an estimate. Whole points, like the ring: a 0.1-point change must not
/// rewrite the tooltip.
fn value(l10n: &Localizer, l: &LimitView) -> String {
    let estimate = if l.source == Source::Estimate {
        "yes"
    } else {
        "no"
    };
    l10n.fmt(
        "tray-value",
        &[
            ("pct", Arg::Num(l.percent.round())),
            ("estimate", Arg::Str(estimate)),
        ],
    )
}

/// A countdown rounded **up** to whole minutes: "resets in 1 h 49 m" never claims an earlier
/// reset than the real one, and it only changes on minute boundaries.
fn ceil_minute(ms: i64) -> i64 {
    (ms + MINUTE_MS - 1) / MINUTE_MS * MINUTE_MS
}

/// When the tooltip text next changes by itself: the next minute boundary of the 5-hour
/// countdown, or the end of the mute. `None`: nothing to wait for.
pub fn next_change(model: &TrayModel, now: Timestamp) -> Option<Timestamp> {
    let countdown = model
        .five_hour()
        .and_then(|l| l.resets_at)
        .map(|r| r.as_millis() - now.as_millis())
        .filter(|&ms| ms > 0)
        .map(|ms| match ms % MINUTE_MS {
            0 => MINUTE_MS,
            rest => rest,
        });
    let mute = model
        .muted_until
        .map(|u| u.as_millis() - now.as_millis())
        .filter(|&ms| ms > 0);
    let wait = [countdown, mute].into_iter().flatten().min()?;
    Some(Timestamp::from_millis(now.as_millis() + wait))
}

fn fits(text: &str) -> bool {
    text.encode_utf16().count() <= MAX_UTF16
}

fn truncate(text: String) -> String {
    if fits(&text) {
        return text;
    }
    let mut out = String::new();
    let mut units = 0;
    for c in text.chars() {
        // Room for the ellipsis (one UTF-16 unit).
        if units + c.len_utf16() > MAX_UTF16 - 1 {
            break;
        }
        units += c.len_utf16();
        out.push(c);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use tokenradar_core::domain::LimitKind;
    use tokenradar_core::limits::Freshness;
    use tokenradar_core::providers::claude::usage_api::ApiState;
    use tokenradar_i18n::{Domain, HourCycle, Locale, Weekday};

    use super::*;
    use crate::tray::visual::tests::{limit, snapshot};

    /// UTC clock with a fixed epoch: 2026-10-28 (a Wednesday) 09:11 is `NOW`.
    struct FakeClock(HourCycle);

    const MIDNIGHT: i64 = 1_793_145_600_000; // 2026-10-28T00:00:00Z, a Wednesday.
    const NOW: Timestamp = Timestamp::from_millis(MIDNIGHT + (9 * 60 + 11) * MINUTE_MS);

    impl LocalClock for FakeClock {
        fn local(&self, t: Timestamp) -> Option<LocalTime> {
            let ms = t.as_millis() - MIDNIGHT;
            let day = ms.div_euclid(86_400_000);
            let in_day = ms.rem_euclid(86_400_000) / MINUTE_MS;
            Some(LocalTime {
                date: (2026, 10, u16::try_from(28 + day).ok()?),
                weekday: Weekday::from_sunday0(u16::try_from((3 + day).rem_euclid(7)).ok()?),
                hour: u8::try_from(in_day / 60).ok()?,
                minute: u8::try_from(in_day % 60).ok()?,
            })
        }

        fn hour_cycle(&self) -> HourCycle {
            self.0
        }
    }

    fn at(minutes_from_now: i64) -> Timestamp {
        Timestamp::from_millis(NOW.as_millis() + minutes_from_now * MINUTE_MS)
    }

    fn model() -> TrayModel {
        let mut five = limit(LimitKind::Session, 72.0, Freshness::Fresh);
        five.resets_at = Some(at(108));
        let mut week = limit(LimitKind::WeeklyAll, 34.0, Freshness::Fresh);
        // Friday 10:59.
        week.resets_at = Some(Timestamp::from_millis(
            MIDNIGHT + 2 * 86_400_000 + (10 * 60 + 59) * MINUTE_MS,
        ));
        TrayModel {
            snapshot: Some(snapshot(vec![five, week], ApiState::Active)),
            ..TrayModel::default()
        }
    }

    fn text(m: &TrayModel, locale: Locale, cycle: HourCycle) -> String {
        let l10n = Localizer::new(locale, &[Domain::Tray]);
        tooltip(m, &l10n, &FakeClock(cycle), NOW)
    }

    #[test]
    fn tooltip_full_form_in_both_languages() {
        assert_eq!(
            text(&model(), Locale::Tr, HourCycle::H12),
            "TokenRadar\n5s: %72 · 1 sa 48 dk sonra yenilenir\nHaftalık: %34 · Cum 10:59'da yenilenir"
        );
        assert_eq!(
            text(&model(), Locale::En, HourCycle::H12),
            "TokenRadar\n5h: 72% · resets in 1h 48m\nWeekly: 34% · resets Fri 10:59 AM"
        );
    }

    #[test]
    fn tooltip_countdown_rounds_up_to_minutes() {
        let mut m = model();
        if let Some(s) = m.snapshot.as_mut() {
            s.limits[0].resets_at = Some(Timestamp::from_millis(at(107).as_millis() + 1));
        }
        assert!(text(&m, Locale::Tr, HourCycle::H23).contains("1 sa 48 dk sonra"));
    }

    #[test]
    fn tooltip_marks_estimates_staleness_and_errors() {
        let mut m = model();
        if let Some(s) = m.snapshot.as_mut() {
            s.limits[0].source = Source::Estimate;
            s.limits[0].percent = 0.0;
            for l in &mut s.limits {
                l.freshness = Freshness::Stale;
                l.observed_at = at(-45);
            }
            s.api.state = ApiState::TokenStale;
        }
        assert_eq!(
            text(&m, Locale::Tr, HourCycle::H23),
            "TokenRadar
5s: ≈%0 · 1 sa 48 dk sonra yenilenir
Haftalık: %34 · Cum 10:59'da yenilenir
Son veri: 08:26
Limit verisi alınamıyor"
        );
    }

    #[test]
    fn tooltip_shows_the_end_of_a_mute() {
        let mut m = model();
        m.muted_until = Some(at(49));
        let tr = text(&m, Locale::Tr, HourCycle::H23);
        assert!(
            tr.ends_with(
                "
Bildirimler 10:00'a kadar susturuldu"
            ),
            "{tr}"
        );
        let en = text(&m, Locale::En, HourCycle::H12);
        assert!(
            en.ends_with(
                "
Notifications muted until 10:00 AM"
            ),
            "{en}"
        );
    }

    #[test]
    fn tooltip_percentages_are_whole_points() {
        let mut m = model();
        if let Some(s) = m.snapshot.as_mut() {
            s.limits[0].percent = 72.4;
        }
        assert!(text(&m, Locale::Tr, HourCycle::H23).contains("5s: %72 ·"));
    }

    #[test]
    fn tooltip_falls_back_to_short_forms() {
        let mut m = model();
        if let Some(s) = m.snapshot.as_mut() {
            for l in &mut s.limits {
                l.freshness = Freshness::Stale;
                l.observed_at = at(-3000);
                l.source = Source::Estimate;
                l.percent = 100.0;
            }
            s.api.state = ApiState::Error {
                retry_at: Timestamp::from_millis(0),
            };
        }
        m.muted_until = Some(at(60));
        for locale in [Locale::Tr, Locale::En] {
            let t = text(&m, locale, HourCycle::H12);
            assert!(t.encode_utf16().count() <= MAX_UTF16, "{t}");
            assert!(!t.ends_with('…'), "{t}");
            assert!(t.contains(" ("), "short form expected: {t}");
            assert!(t.contains("Sessiz: ") || t.contains("Muted until"), "{t}");
        }
    }

    fn with_scoped(mut m: TrayModel, percent: f64) -> TrayModel {
        let mut scoped = limit(LimitKind::WeeklyScoped, percent, Freshness::Fresh);
        scoped.scope = Some(Scope::Model("fable".into()));
        scoped.resets_at = Some(Timestamp::from_millis(
            MIDNIGHT + 2 * 86_400_000 + (10 * 60 + 59) * MINUTE_MS,
        ));
        if let Some(s) = m.snapshot.as_mut() {
            s.limits.push(scoped);
        }
        m
    }

    #[test]
    fn tooltip_names_a_model_limit_from_the_warning_threshold() {
        assert!(!text(&with_scoped(model(), 74.6), Locale::Tr, HourCycle::H23).contains("Fable"));
        let m = with_scoped(model(), 95.0);
        assert!(text(&m, Locale::Tr, HourCycle::H23).ends_with(
            "
Fable · haftalık: %95 · Cum 10:59'da yenilenir"
        ),);
        assert!(text(&m, Locale::En, HourCycle::H23).ends_with(
            "
Fable · weekly: 95% · resets Fri 10:59"
        ),);
    }

    #[test]
    fn tooltip_worst_case_fits_without_cutting() {
        let mut m = with_scoped(model(), 100.0);
        if let Some(s) = m.snapshot.as_mut() {
            for l in &mut s.limits {
                l.freshness = Freshness::Stale;
                l.observed_at = at(-3000);
                l.source = Source::Estimate;
                l.percent = 100.0;
            }
            s.api.state = ApiState::Error {
                retry_at: Timestamp::from_millis(0),
            };
        }
        m.muted_until = Some(at(60));
        for locale in [Locale::Tr, Locale::En] {
            let t = text(&m, locale, HourCycle::H12);
            assert!(t.encode_utf16().count() <= MAX_UTF16, "{t}");
            assert!(!t.ends_with('…'), "{t}");
            assert!(t.contains("Fable · 7"), "{t}");
        }
    }

    #[test]
    fn tooltip_without_data_or_core() {
        assert_eq!(
            text(&TrayModel::default(), Locale::Tr, HourCycle::H23),
            "TokenRadar\nHenüz limit verisi yok"
        );
        let failed = TrayModel {
            core_failed: true,
            ..TrayModel::default()
        };
        assert_eq!(
            text(&failed, Locale::En, HourCycle::H23),
            "TokenRadar\nCan't read limit data"
        );
    }

    #[test]
    fn truncation_respects_utf16_units() {
        let long = "ş".repeat(200);
        let cut = truncate(long);
        assert_eq!(cut.encode_utf16().count(), MAX_UTF16);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn next_change_follows_the_countdown_and_mute() {
        let mut m = model();
        // 108 minutes exactly: the text changes one minute later.
        assert_eq!(next_change(&m, NOW), Some(at(1)));
        if let Some(s) = m.snapshot.as_mut() {
            s.limits[0].resets_at = Some(Timestamp::from_millis(at(10).as_millis() + 1500));
        }
        assert_eq!(
            next_change(&m, NOW),
            Some(Timestamp::from_millis(NOW.as_millis() + 1500))
        );
        m.snapshot = None;
        assert_eq!(next_change(&m, NOW), None);
        m.muted_until = Some(at(30));
        assert_eq!(next_change(&m, NOW), Some(at(30)));
    }
}
