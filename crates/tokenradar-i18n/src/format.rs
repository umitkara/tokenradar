//! Number, percentage, duration and clock formatting shared by every Fluent bundle
//! (architecture §12).
//!
//! These are the Rust halves of the `NUM`, `PCT` and `DUR` Fluent functions, plus the small
//! hand-written clock formatter (no ICU data is embedded). The TypeScript side (Step 09) must
//! produce the same strings; `testdata/i18n-vectors.json` pins them for both.

use crate::Locale;

/// Layout of a [`dur`] string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurStyle {
    /// `1 sa 48 dk` / `1h 48m`.
    Short,
    /// No spaces, for tight places such as the statusline: `1sa48dk` / `1h48m`.
    Compact,
    /// Words, for screen readers: `1 saat 48 dakika` / `1 hour 48 minutes`.
    Long,
}

impl DurStyle {
    /// Fluent argument value; anything unknown falls back to [`DurStyle::Short`].
    pub fn parse(s: &str) -> Self {
        match s {
            "compact" => Self::Compact,
            "long" => Self::Long,
            _ => Self::Short,
        }
    }
}

/// `n` with `decimals` fraction digits and locale grouping: TR `1.234,5`, EN `1,234.5`.
pub fn num(locale: Locale, n: f64, decimals: usize) -> String {
    let (group, point) = match locale {
        Locale::En => (',', '.'),
        Locale::Tr => ('.', ','),
    };
    let fixed = format!("{:.*}", decimals, n.abs());
    let (int, frac) = fixed.split_once('.').unwrap_or((&fixed, ""));
    let mut out = String::with_capacity(fixed.len() + int.len() / 3 + 1);
    // `-0` after rounding is printed as `0`.
    if n < 0.0 && fixed.bytes().any(|b| b.is_ascii_digit() && b != b'0') {
        out.push('-');
    }
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(group);
        }
        out.push(c);
    }
    if !frac.is_empty() {
        out.push(point);
        out.push_str(frac);
    }
    out
}

/// A percentage with at most one decimal, trailing `,0` dropped: TR `%72`, `%23,5`;
/// EN `72%`, `23.5%`.
pub fn pct(locale: Locale, n: f64) -> String {
    let rounded = (n * 10.0).round() / 10.0;
    let decimals = usize::from(rounded.fract() != 0.0);
    let digits = num(locale, rounded, decimals);
    match locale {
        Locale::En => format!("{digits}%"),
        Locale::Tr => format!("%{digits}"),
    }
}

/// A duration in its two largest units, truncated: `59 sn`, `1 dk`, `1 sa 48 dk`, `2 g 3 sa`
/// (EN `59s`, `1m`, `1h 48m`, `2d 3h`). Negative durations print as zero.
pub fn dur(locale: Locale, ms: i64, style: DurStyle) -> String {
    const SEC: i64 = 1000;
    const MIN: i64 = 60 * SEC;
    const HOUR: i64 = 60 * MIN;
    const DAY: i64 = 24 * HOUR;

    let ms = ms.max(0);
    let (big, small) = if ms >= DAY {
        ((ms / DAY, Unit::Day), Some((ms % DAY / HOUR, Unit::Hour)))
    } else if ms >= HOUR {
        ((ms / HOUR, Unit::Hour), Some((ms % HOUR / MIN, Unit::Min)))
    } else if ms >= MIN {
        ((ms / MIN, Unit::Min), None)
    } else {
        ((ms / SEC, Unit::Sec), None)
    };

    let mut out = part(locale, big.0, big.1, style);
    if let Some((n, unit)) = small
        && n > 0
    {
        if style != DurStyle::Compact {
            out.push(' ');
        }
        out.push_str(&part(locale, n, unit, style));
    }
    out
}

#[derive(Clone, Copy)]
enum Unit {
    Day,
    Hour,
    Min,
    Sec,
}

fn part(locale: Locale, n: i64, unit: Unit, style: DurStyle) -> String {
    if style == DurStyle::Long {
        let word = match (locale, unit) {
            (Locale::En, Unit::Day) => "day",
            (Locale::En, Unit::Hour) => "hour",
            (Locale::En, Unit::Min) => "minute",
            (Locale::En, Unit::Sec) => "second",
            (Locale::Tr, Unit::Day) => "gün",
            (Locale::Tr, Unit::Hour) => "saat",
            (Locale::Tr, Unit::Min) => "dakika",
            (Locale::Tr, Unit::Sec) => "saniye",
        };
        // Turkish nouns stay singular after a number.
        let plural = if locale == Locale::En && n != 1 {
            "s"
        } else {
            ""
        };
        return format!("{n} {word}{plural}");
    }
    let suffix = match (locale, unit) {
        (Locale::En, Unit::Day) => "d",
        (Locale::En, Unit::Hour) => "h",
        (Locale::En, Unit::Min) => "m",
        (Locale::En, Unit::Sec) => "s",
        (Locale::Tr, Unit::Day) => "g",
        (Locale::Tr, Unit::Hour) => "sa",
        (Locale::Tr, Unit::Min) => "dk",
        (Locale::Tr, Unit::Sec) => "sn",
    };
    // Turkish separates number and unit in the long form; English never does.
    if locale == Locale::Tr && style == DurStyle::Short {
        format!("{n} {suffix}")
    } else {
        format!("{n}{suffix}")
    }
}

/// A day of the week, for [`weekday_clock`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    /// From a Sunday-based index (`0` = Sunday), as Windows' `SYSTEMTIME` and JS `getDay()`
    /// count; out-of-range values wrap.
    pub const fn from_sunday0(day: u16) -> Self {
        match day % 7 {
            0 => Self::Sun,
            1 => Self::Mon,
            2 => Self::Tue,
            3 => Self::Wed,
            4 => Self::Thu,
            5 => Self::Fri,
            _ => Self::Sat,
        }
    }

    const fn abbr(self, locale: Locale) -> &'static str {
        match (locale, self) {
            (Locale::En, Self::Mon) => "Mon",
            (Locale::En, Self::Tue) => "Tue",
            (Locale::En, Self::Wed) => "Wed",
            (Locale::En, Self::Thu) => "Thu",
            (Locale::En, Self::Fri) => "Fri",
            (Locale::En, Self::Sat) => "Sat",
            (Locale::En, Self::Sun) => "Sun",
            (Locale::Tr, Self::Mon) => "Pzt",
            (Locale::Tr, Self::Tue) => "Sal",
            (Locale::Tr, Self::Wed) => "Çar",
            (Locale::Tr, Self::Thu) => "Per",
            (Locale::Tr, Self::Fri) => "Cum",
            (Locale::Tr, Self::Sat) => "Cmt",
            (Locale::Tr, Self::Sun) => "Paz",
        }
    }
}

/// 12- or 24-hour clock. Turkish always uses 24 hours; English follows the OS region
/// (architecture §12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HourCycle {
    #[default]
    H23,
    H12,
}

/// A wall-clock time: TR `09:05`; EN `09:05` or `9:05 AM`.
pub fn clock(locale: Locale, hour: u8, minute: u8, cycle: HourCycle) -> String {
    if locale == Locale::En && cycle == HourCycle::H12 {
        let h12 = match hour % 12 {
            0 => 12,
            h => h,
        };
        let meridiem = if hour < 12 { "AM" } else { "PM" };
        format!("{h12}:{minute:02} {meridiem}")
    } else {
        format!("{hour:02}:{minute:02}")
    }
}

/// Weekday and time: TR `Çar 10:59`, EN `Wed 10:59` or `Wed 10:59 AM`.
pub fn weekday_clock(
    locale: Locale,
    weekday: Weekday,
    hour: u8,
    minute: u8,
    cycle: HourCycle,
) -> String {
    format!(
        "{} {}",
        weekday.abbr(locale),
        clock(locale, hour, minute, cycle)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clocks_follow_the_locale_and_hour_cycle() {
        assert_eq!(clock(Locale::Tr, 9, 5, HourCycle::H12), "09:05");
        assert_eq!(clock(Locale::En, 9, 5, HourCycle::H23), "09:05");
        assert_eq!(clock(Locale::En, 0, 0, HourCycle::H12), "12:00 AM");
        assert_eq!(clock(Locale::En, 12, 30, HourCycle::H12), "12:30 PM");
        assert_eq!(clock(Locale::En, 23, 59, HourCycle::H12), "11:59 PM");
        assert_eq!(
            weekday_clock(Locale::Tr, Weekday::from_sunday0(3), 10, 59, HourCycle::H23),
            "Çar 10:59"
        );
        assert_eq!(
            weekday_clock(Locale::En, Weekday::from_sunday0(0), 7, 0, HourCycle::H12),
            "Sun 7:00 AM"
        );
    }

    #[test]
    fn numbers_group_by_locale() {
        assert_eq!(num(Locale::Tr, 1234.5, 1), "1.234,5");
        assert_eq!(num(Locale::En, 1234.5, 1), "1,234.5");
        assert_eq!(num(Locale::En, 1_234_567.0, 0), "1,234,567");
        assert_eq!(num(Locale::En, 999.0, 0), "999");
        assert_eq!(num(Locale::Tr, -0.04, 1), "0,0");
        assert_eq!(num(Locale::Tr, -12.0, 0), "-12");
    }

    #[test]
    fn percentages_follow_turkish_and_english_order() {
        let cases = [
            (0.0, "%0", "0%"),
            (5.0, "%5", "5%"),
            (72.0, "%72", "72%"),
            (100.0, "%100", "100%"),
            (23.5, "%23,5", "23.5%"),
            (23.04, "%23", "23%"),
        ];
        for (n, tr, en) in cases {
            assert_eq!(pct(Locale::Tr, n), tr, "{n}");
            assert_eq!(pct(Locale::En, n), en, "{n}");
        }
    }

    #[test]
    fn durations_use_the_two_largest_units() {
        let min = 60_000;
        let hour = 60 * min;
        let cases = [
            (59_000, "59 sn", "59s", "59sn"),
            (min, "1 dk", "1m", "1dk"),
            (hour + 48 * min, "1 sa 48 dk", "1h 48m", "1sa48dk"),
            (2 * hour, "2 sa", "2h", "2sa"),
            (
                2 * 24 * hour + 3 * hour + 59 * min,
                "2 g 3 sa",
                "2d 3h",
                "2g3sa",
            ),
            (-5, "0 sn", "0s", "0sn"),
        ];
        for (ms, tr, en, tr_compact) in cases {
            assert_eq!(dur(Locale::Tr, ms, DurStyle::Short), tr);
            assert_eq!(dur(Locale::En, ms, DurStyle::Short), en);
            assert_eq!(dur(Locale::Tr, ms, DurStyle::Compact), tr_compact);
        }
        assert_eq!(dur(Locale::En, hour + 48 * min, DurStyle::Compact), "1h48m");
    }
}
