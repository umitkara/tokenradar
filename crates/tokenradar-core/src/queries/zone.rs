//! Local time for day, week-day and hour boundaries (architecture §6: storage is UTC).
//!
//! A [`LocalZone`] only answers "what is the UTC offset at this instant". Ranges are split
//! into spans of constant offset ([`spans`]), so a query can group by
//! `(bucket_ts + offset) / hour` inside each span: half-hour zones and DST changes land in
//! the right local hour. The system zone on Windows asks `SystemTimeToTzSpecificLocalTime`,
//! which applies the rules of the date in question; jiff's `TimeZone::system()` would need a
//! bundled tz database on Windows.

use crate::domain::Timestamp;

const HOUR_MS: i64 = 3_600_000;
const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;

pub trait LocalZone: Send + Sync {
    /// Local time minus UTC at `at`, in milliseconds.
    fn offset_ms(&self, at: Timestamp) -> i64;
}

/// The computer's zone. Outside Windows it is UTC until Step 21.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemZone;

impl LocalZone for SystemZone {
    fn offset_ms(&self, at: Timestamp) -> i64 {
        system_offset(at).unwrap_or(0)
    }
}

/// A zone with fixed rules, for tests and synthetic data: `base` plus `dst` inside each
/// `[from, to)` period.
#[derive(Debug, Clone, Default)]
pub struct FixedZone {
    pub base_ms: i64,
    pub dst_ms: i64,
    pub dst: Vec<(Timestamp, Timestamp)>,
}

impl FixedZone {
    pub fn utc() -> Self {
        Self::default()
    }

    pub fn offset(base_ms: i64) -> Self {
        Self {
            base_ms,
            ..Self::default()
        }
    }
}

impl LocalZone for FixedZone {
    fn offset_ms(&self, at: Timestamp) -> i64 {
        let in_dst = self.dst.iter().any(|&(f, t)| at >= f && at < t);
        self.base_ms + if in_dst { self.dst_ms } else { 0 }
    }
}

/// `[from, to)` cut into `(start, end, offset)` spans of constant offset. Offsets are probed
/// hourly; a change is located to the minute.
pub(crate) fn spans(zone: &dyn LocalZone, from: i64, to: i64) -> Vec<(i64, i64, i64)> {
    let mut out = Vec::new();
    if from >= to {
        return out;
    }
    let mut start = from;
    let mut offset = zone.offset_ms(Timestamp::from_millis(from));
    let mut t = from;
    while t < to {
        let next = (t + HOUR_MS).min(to);
        let o = zone.offset_ms(Timestamp::from_millis(next));
        if o != offset && next < to {
            // The change is in (t, next]: find its first minute.
            let (mut lo, mut hi) = (t, next);
            while hi - lo > MINUTE_MS {
                let mid = lo + (hi - lo) / 2 / MINUTE_MS * MINUTE_MS;
                let mid = if mid <= lo { lo + MINUTE_MS } else { mid };
                if zone.offset_ms(Timestamp::from_millis(mid)) == offset {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            out.push((start, hi, offset));
            start = hi;
            offset = zone.offset_ms(Timestamp::from_millis(hi));
        }
        t = next;
    }
    out.push((start, to, offset));
    out
}

/// Monday-zero week day and hour of a local instant given as "local milliseconds".
pub(crate) fn weekday_hour(local_ms: i64) -> (u8, u8) {
    let days = local_ms.div_euclid(DAY_MS);
    // 1970-01-01 was a Thursday (Monday-zero 3).
    let weekday = u8::try_from((days + 3).rem_euclid(7)).unwrap_or(0);
    let hour = u8::try_from(local_ms.rem_euclid(DAY_MS) / HOUR_MS).unwrap_or(0);
    (weekday, hour)
}

/// UTC instant of local midnight on the day containing `at`.
pub fn local_day_start(zone: &dyn LocalZone, at: Timestamp) -> Timestamp {
    let local = at.as_millis() + zone.offset_ms(at);
    to_utc(zone, local - local.rem_euclid(DAY_MS))
}

/// UTC instant of local midnight on the first day of the month containing `at`.
pub fn local_month_start(zone: &dyn LocalZone, at: Timestamp) -> Timestamp {
    let local = at.as_millis() + zone.offset_ms(at);
    let Ok(ts) = jiff::Timestamp::from_millisecond(local) else {
        return at;
    };
    let date = ts.to_zoned(jiff::tz::TimeZone::UTC).date();
    let first = date.first_of_month();
    let Ok(midnight) = first
        .to_datetime(jiff::civil::Time::midnight())
        .to_zoned(jiff::tz::TimeZone::UTC)
    else {
        return at;
    };
    to_utc(zone, midnight.timestamp().as_millisecond())
}

/// The UTC instant shown as `local` ms: offset taken where it applies (one correction step
/// covers a DST change between the guess and the answer).
fn to_utc(zone: &dyn LocalZone, local: i64) -> Timestamp {
    let guess = local - zone.offset_ms(Timestamp::from_millis(local));
    let utc = local - zone.offset_ms(Timestamp::from_millis(guess));
    Timestamp::from_millis(utc)
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn system_offset(at: Timestamp) -> Option<i64> {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::Time::SystemTimeToTzSpecificLocalTime;
    // Whole minutes: zones are minute-aligned and SYSTEMTIME seconds would only add noise.
    let ms = at.as_millis() - at.as_millis().rem_euclid(MINUTE_MS);
    let utc = jiff::Timestamp::from_millisecond(ms)
        .ok()?
        .to_zoned(jiff::tz::TimeZone::UTC)
        .datetime();
    let part = |n: i8| u16::try_from(n).ok();
    let st = SYSTEMTIME {
        wYear: u16::try_from(utc.year()).ok()?,
        wMonth: part(utc.month())?,
        wDayOfWeek: 0,
        wDay: part(utc.day())?,
        wHour: part(utc.hour())?,
        wMinute: part(utc.minute())?,
        wSecond: 0,
        wMilliseconds: 0,
    };
    let mut local = st;
    // SAFETY: a null zone is the current one; both pointers are to valid SYSTEMTIMEs.
    let ok =
        unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &raw const st, &raw mut local) };
    if ok == 0 {
        return None;
    }
    let local_dt = jiff::civil::DateTime::new(
        i16::try_from(local.wYear).ok()?,
        i8::try_from(local.wMonth).ok()?,
        i8::try_from(local.wDay).ok()?,
        i8::try_from(local.wHour).ok()?,
        i8::try_from(local.wMinute).ok()?,
        0,
        0,
    )
    .ok()?;
    let local_ms = local_dt
        .to_zoned(jiff::tz::TimeZone::UTC)
        .ok()?
        .timestamp()
        .as_millisecond();
    Some(local_ms - ms)
}

#[cfg(not(windows))]
fn system_offset(_at: Timestamp) -> Option<i64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> Timestamp {
        Timestamp::parse_iso(s).expect("timestamp")
    }

    /// Central European rules for 2026: UTC+1, +2 from 29 March 01:00 UTC to 25 October
    /// 01:00 UTC.
    pub(crate) fn cet() -> FixedZone {
        FixedZone {
            base_ms: HOUR_MS,
            dst_ms: HOUR_MS,
            dst: vec![(ts("2026-03-29T01:00:00Z"), ts("2026-10-25T01:00:00Z"))],
        }
    }

    #[test]
    fn spans_split_at_dst_changes() {
        let z = cet();
        let s = spans(
            &z,
            ts("2026-03-28T00:00:00Z").as_millis(),
            ts("2026-03-30T00:00:00Z").as_millis(),
        );
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].1, ts("2026-03-29T01:00:00Z").as_millis());
        assert_eq!((s[0].2, s[1].2), (HOUR_MS, 2 * HOUR_MS));
        let whole = spans(
            &FixedZone::offset(19_800_000),
            0,
            ts("2026-01-01T00:00:00Z").as_millis(),
        );
        assert_eq!(whole.len(), 1, "a fixed zone is one span");
    }

    #[test]
    fn weekday_and_hour_of_local_time() {
        // 2026-11-16 is a Monday.
        assert_eq!(
            weekday_hour(ts("2026-11-16T13:30:00Z").as_millis()),
            (0, 13)
        );
        assert_eq!(
            weekday_hour(ts("2026-11-22T23:59:00Z").as_millis()),
            (6, 23)
        );
    }

    #[test]
    fn local_day_and_month_starts() {
        let z = FixedZone::offset(3 * HOUR_MS); // Istanbul
        assert_eq!(
            local_day_start(&z, ts("2026-11-20T22:30:00Z")),
            ts("2026-11-20T21:00:00Z"),
            "01:30 local on the 21st belongs to the day starting 21:00 UTC on the 20th"
        );
        assert_eq!(
            local_month_start(&z, ts("2026-11-20T10:00:00Z")),
            ts("2026-10-31T21:00:00Z")
        );
        let c = cet();
        assert_eq!(
            local_day_start(&c, ts("2026-03-29T12:00:00Z")),
            ts("2026-03-28T23:00:00Z"),
            "midnight before the change is still +1"
        );
    }

    #[cfg(windows)]
    #[test]
    fn system_offset_is_whole_minutes() {
        let o = SystemZone.offset_ms(Timestamp::now());
        assert_eq!(o % MINUTE_MS, 0);
        assert!(o.abs() <= 14 * HOUR_MS);
    }
}
