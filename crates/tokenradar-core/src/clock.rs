//! Wall-clock time behind a trait, so schedulers and the limit engine can be tested with a fake
//! clock instead of sleeping.
//!
//! [`LocalClock`] turns instants into the user's wall-clock time (quiet hours, "resets Wed
//! 10:59"). The core has no time-zone database; the app supplies the operating system's
//! conversion and tests a fixed one.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use tokenradar_i18n::{HourCycle, Weekday};

use crate::domain::Timestamp;

pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

/// The real clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// A clock that only moves when told to. Clones share the same time.
#[derive(Debug, Clone, Default)]
pub struct ManualClock(Arc<AtomicI64>);

impl ManualClock {
    pub fn at(t: Timestamp) -> Self {
        Self(Arc::new(AtomicI64::new(t.as_millis())))
    }

    pub fn set(&self, t: Timestamp) {
        self.0.store(t.as_millis(), Ordering::SeqCst);
    }

    pub fn advance_ms(&self, ms: i64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_millis(self.0.load(Ordering::SeqCst))
    }
}

/// A wall-clock reading in the user's time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    /// Year, month, day: only compared, to tell "today" from other days.
    pub date: (u16, u16, u16),
    pub weekday: Weekday,
    pub hour: u8,
    pub minute: u8,
}

impl LocalTime {
    /// Minutes since local midnight.
    pub fn minute_of_day(self) -> u16 {
        u16::from(self.hour) * 60 + u16::from(self.minute)
    }
}

/// Local time conversion, swappable in tests.
pub trait LocalClock: Send + Sync {
    fn local(&self, t: Timestamp) -> Option<LocalTime>;
    /// The user's 12/24-hour preference (English only; Turkish is always 24 hours).
    fn hour_cycle(&self) -> HourCycle;
}

/// UTC as local time with a 24-hour clock: the fallback without an operating-system clock
/// (tests, other platforms until Step 21). The date is a day count, which is enough to compare.
#[derive(Debug, Clone, Copy, Default)]
pub struct UtcClock;

impl LocalClock for UtcClock {
    fn local(&self, t: Timestamp) -> Option<LocalTime> {
        let secs = t.as_millis().div_euclid(1000);
        let days = secs.div_euclid(86_400);
        let in_day = secs.rem_euclid(86_400);
        Some(LocalTime {
            date: (0, 0, u16::try_from(days.rem_euclid(65_536)).ok()?),
            // 1970-01-01 was a Thursday.
            weekday: Weekday::from_sunday0(u16::try_from((days + 4).rem_euclid(7)).ok()?),
            hour: u8::try_from(in_day / 3600).ok()?,
            minute: u8::try_from(in_day / 60 % 60).ok()?,
        })
    }

    fn hour_cycle(&self) -> HourCycle {
        HourCycle::H23
    }
}
