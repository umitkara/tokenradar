//! Periodic summaries (Step 17b, ADR-071): the past weekly limit window and, optionally, the
//! past local day.
//!
//! **When.** The weekly summary covers the last weekly window that reset. By default it is due
//! an hour after the reset; with a custom day and time, at the first such local time at or
//! after the reset. The daily one covers yesterday and is due today at the chosen time.
//!
//! **Once.** A sent summary is a `notification_log` row (`digest:<period>:-:<period end>:-`),
//! so a restart does not repeat it. A summary missed while the app was closed is sent late,
//! once, up to [`Period::late_ms`] after it was due; after that it is skipped. While muted or
//! in quiet hours it waits for the end, like the other notices.
//!
//! **Content** comes from [`crate::queries::weekly_digest`] and
//! [`crate::queries::daily_digest`]; a summary of a period without any usage is recorded but
//! not shown.

use tokenradar_i18n::format::weekday_clock;
use tokenradar_i18n::{Arg, Domain, Locale, Localizer};

use super::{DigestPrefs, Fired, Notice, NoticeTarget, Notifier, Rule, WeeklyAt, parse_hhmm};
use crate::clock::LocalClock;
use crate::domain::{MicroUsd, Timestamp};
use crate::queries::{
    DailyDigest, LocalZone, WeekBounds, WeeklyDigest, at_local_minute, local_day_start,
    local_weekday, next_local_day_start,
};

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 60 * MINUTE_MS;
const DAY_MS: i64 = 24 * HOUR_MS;
/// The default weekly summary waits this long after the reset, so the closed window's last
/// readings and attribution are in.
pub const AFTER_RESET_MS: i64 = HOUR_MS;
/// A due weekly summary whose window is not attributed yet waits at most this long for it;
/// then it goes out without the project line.
pub const ATTRIBUTION_WAIT_MS: i64 = 2 * HOUR_MS;
/// Digest rows stay in memory this long after they were sent: longer than any summary can be
/// due after its period ends (a custom day up to a week later, plus [`Period::late_ms`]).
pub const DIGEST_HISTORY_MS: i64 = 10 * DAY_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Weekly,
    Daily,
}

impl Period {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Weekly => "weekly",
            Self::Daily => "daily",
        }
    }

    /// How long after it was due a summary may still be sent: two days for the weekly one
    /// (user decision, 2026-11-21), until the next one for the daily one.
    pub const fn late_ms(self) -> i64 {
        match self {
            Self::Weekly => 2 * DAY_MS,
            Self::Daily => DAY_MS,
        }
    }
}

/// One summary: the period it covers and when it is due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub period: Period,
    pub from: Timestamp,
    pub to: Timestamp,
    pub due_at: Timestamp,
}

/// What to do about a slot now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestState {
    /// Notifications or this summary are off.
    Off,
    /// Already sent (or recorded as empty).
    Done,
    /// Due too long ago; skipped.
    Missed,
    /// Not yet: due later, or held back until a mute or quiet hours end.
    Wait(Timestamp),
    Due,
}

/// The weekly summary of `week`.
pub fn weekly_slot(prefs: &DigestPrefs, week: WeekBounds, zone: &dyn LocalZone) -> Slot {
    let due_at = match prefs.weekly_at {
        WeeklyAt::AfterReset => week.resets_at.saturating_add_millis(AFTER_RESET_MS),
        WeeklyAt::Custom => {
            let minute = parse_hhmm(&prefs.weekly_time).unwrap_or(9 * 60);
            next_local(zone, week.resets_at, Some(prefs.weekly_day), minute)
        }
    };
    Slot {
        period: Period::Weekly,
        from: week.starts_at,
        to: week.resets_at,
        due_at,
    }
}

/// The newest daily summary that is due by `now` (yesterday's after today's time, otherwise
/// the day before's), and when the next one will be due.
pub fn daily_slot(prefs: &DigestPrefs, now: Timestamp, zone: &dyn LocalZone) -> (Slot, Timestamp) {
    let minute = parse_hhmm(&prefs.daily_time).unwrap_or(9 * 60);
    let today = local_day_start(zone, now);
    let due_today = at_local_minute(zone, today, minute);
    let slot_for = |day: Timestamp| {
        let yesterday = local_day_start(zone, day.saturating_sub_millis(1));
        Slot {
            period: Period::Daily,
            from: yesterday,
            to: day,
            due_at: at_local_minute(zone, day, minute),
        }
    };
    if now >= due_today {
        let tomorrow = next_local_day_start(zone, now);
        (slot_for(today), at_local_minute(zone, tomorrow, minute))
    } else {
        let yesterday = local_day_start(zone, today.saturating_sub_millis(1));
        (slot_for(yesterday), due_today)
    }
}

/// The first local `minute` at or after `after`, on `weekday` (Monday-zero) when given.
fn next_local(
    zone: &dyn LocalZone,
    after: Timestamp,
    weekday: Option<u8>,
    minute: u16,
) -> Timestamp {
    let mut day = local_day_start(zone, after);
    for _ in 0..9 {
        let at = at_local_minute(zone, day, minute);
        if at >= after && weekday.is_none_or(|w| local_weekday(zone, day) == w) {
            return at;
        }
        day = next_local_day_start(zone, day);
    }
    after
}

impl Notifier {
    pub fn digest_prefs(&self) -> &DigestPrefs {
        &self.prefs.digest
    }

    /// Whether the summaries of `period` are on (and notifications at all).
    pub fn digest_enabled(&self, period: Period) -> bool {
        self.prefs.enabled
            && match period {
                Period::Weekly => self.prefs.digest.weekly,
                Period::Daily => self.prefs.digest.daily,
            }
    }

    pub fn digest_state(&self, slot: &Slot, now: Timestamp) -> DigestState {
        if !self.digest_enabled(slot.period) {
            return DigestState::Off;
        }
        if self.digest_sent(slot) {
            return DigestState::Done;
        }
        if now < slot.due_at {
            return DigestState::Wait(slot.due_at);
        }
        if now.as_millis() >= slot.due_at.as_millis() + slot.period.late_ms() {
            return DigestState::Missed;
        }
        if self.suppressed(now) {
            // Re-checked at the end; past the late limit by then, it is skipped.
            return DigestState::Wait(
                self.suppression_end(now)
                    .unwrap_or_else(|| now.saturating_add_millis(MINUTE_MS)),
            );
        }
        DigestState::Due
    }

    pub(super) fn digest_sent(&self, slot: &Slot) -> bool {
        self.fired.iter().any(|f| {
            f.rule == Rule::Digest
                && f.kind == slot.period.key()
                && (f.resets_at.as_millis() - slot.to.as_millis()).abs() <= super::SAME_WINDOW_MS
        })
    }

    /// Records `slot` as sent and shows `notice` (`None`: nothing worth saying).
    pub fn send_digest(
        &mut self,
        slot: &Slot,
        notice: Option<Notice>,
        now: Timestamp,
    ) -> super::Outcome {
        let fired = Fired {
            rule: Rule::Digest,
            kind: slot.period.key().to_owned(),
            scope: None,
            resets_at: slot.to,
            threshold: None,
            at: now,
        };
        self.fired.push(fired.clone());
        super::Outcome {
            show: notice.into_iter().collect(),
            record: vec![fired],
        }
    }

    pub fn weekly_notice(&self, d: &WeeklyDigest) -> Notice {
        weekly_text(&self.l10n, self.clock.as_ref(), d)
    }

    pub fn daily_notice(&self, d: &DailyDigest) -> Notice {
        daily_text(&self.l10n, d)
    }
}

/// The weekly summary as the settings' "Show now" shows it (not recorded).
pub fn weekly_preview(locale: Locale, clock: &dyn LocalClock, d: &WeeklyDigest) -> Notice {
    weekly_text(&Localizer::new(locale, &[Domain::Notify]), clock, d)
}

// Amounts far below 2^52 micro-dollars: no precision is lost in practice.
#[allow(clippy::cast_precision_loss)]
fn usd(m: MicroUsd) -> f64 {
    m.0 as f64 / 1_000_000.0
}

/// A whole number with its sign, `+12` / `−8` (a real minus sign), `0` unsigned; with
/// `percent` in the locale's percent form (`+%12`, `+12%`).
fn signed(locale: Locale, n: f64, percent: bool) -> String {
    let n = n.round();
    let abs = if percent {
        tokenradar_i18n::format::pct(locale, n.abs())
    } else {
        tokenradar_i18n::format::num(locale, n.abs(), 0)
    };
    if n > 0.0 {
        format!("+{abs}")
    } else if n < 0.0 {
        format!("\u{2212}{abs}")
    } else {
        abs
    }
}

fn weekly_text(l10n: &Localizer, clock: &dyn LocalClock, d: &WeeklyDigest) -> Notice {
    let locale = l10n.locale();
    let w = &d.window;
    let approx = if w.estimated { "yes" } else { "no" };
    let title = match w.peak_percent {
        Some(p) => l10n.fmt(
            "digest-weekly-title",
            &[("pct", Arg::from(p.round())), ("approx", Arg::Str(approx))],
        ),
        None => l10n.msg("digest-weekly-title-nopct"),
    };

    let mut lines = Vec::new();
    // Line 1: did a limit fill up.
    let mut hits = Vec::new();
    if let Some(f) = d.five_hour {
        hits.push(l10n.fmt(
            "digest-five-hour-hits",
            &[
                ("hit", Arg::from(i64::from(f.hit))),
                ("windows", Arg::from(i64::from(f.windows))),
            ],
        ));
    }
    let hit_at = d
        .hit_at
        .and_then(|t| clock.local(t))
        .map(|t| weekday_clock(locale, t.weekday, t.hour, t.minute, clock.hour_cycle()));
    hits.push(match (w.hit_limit, hit_at) {
        (Some(true), Some(at)) => l10n.fmt("digest-weekly-hit-at", &[("at", Arg::Str(&at))]),
        (Some(true), None) => l10n.msg("digest-weekly-hit"),
        (Some(false), _) => l10n.msg("digest-weekly-not-hit"),
        (None, _) => l10n.msg("digest-weekly-hit-unknown"),
    });
    for s in &d.scoped {
        let name = match &s.scope {
            crate::domain::Scope::Model(n) | crate::domain::Scope::Surface(n) => n.as_str(),
        };
        let text = match (s.hit_limit, s.peak_percent) {
            (Some(true), _) => l10n.fmt("digest-scoped-hit", &[("name", Arg::Str(name))]),
            (_, Some(p)) => l10n.fmt(
                "digest-scoped-peak",
                &[
                    ("name", Arg::Str(name)),
                    ("pct", Arg::from(p.round())),
                    ("approx", Arg::Str(if s.estimated { "yes" } else { "no" })),
                ],
            ),
            (_, None) => continue,
        };
        hits.push(text);
    }
    lines.push(hits.join(" · "));

    // Line 2: cost and the top project.
    let mut spend = vec![l10n.fmt("digest-cost", &[("usd", Arg::from(usd(w.cost)))])];
    if let Some(name) = d
        .top_project
        .as_ref()
        .and_then(|p| p.project.as_ref())
        .map(|p| p.name.as_str())
    {
        spend.push(l10n.fmt("digest-top-project", &[("name", Arg::Str(name))]));
    }
    lines.push(spend.join(" · "));

    // Line 3: against the week before.
    if let Some(prev) = &d.previous {
        let mut parts = Vec::new();
        if let (Some(now), Some(before)) = (w.peak_percent, prev.peak_percent) {
            let delta = now.round() - before.round();
            parts.push(l10n.fmt(
                "digest-delta-peak",
                &[("delta", Arg::Str(&signed(locale, delta, false)))],
            ));
        }
        if prev.cost.0 > 0 {
            let change = (usd(w.cost) / usd(prev.cost) - 1.0) * 100.0;
            parts.push(l10n.fmt(
                "digest-delta-cost",
                &[("delta", Arg::Str(&signed(locale, change, true)))],
            ));
        }
        if !parts.is_empty() {
            lines.push(l10n.fmt("digest-delta", &[("parts", Arg::Str(&parts.join(", ")))]));
        }
    }

    Notice {
        rule: Rule::Digest,
        title,
        body: lines.join("\n"),
        target: NoticeTarget::Overview,
    }
}

fn daily_text(l10n: &Localizer, d: &DailyDigest) -> Notice {
    let cost = l10n.fmt("digest-cost", &[("usd", Arg::from(usd(d.cost)))]);
    let title = match d.percent {
        Some(p) => l10n.fmt(
            "digest-daily-title",
            &[("pts", Arg::from((p * 10.0).round() / 10.0))],
        ),
        None => l10n.fmt("digest-daily-title-cost", &[("cost", Arg::Str(&cost))]),
    };
    let mut parts = Vec::new();
    if d.percent.is_some() {
        parts.push(cost);
    }
    parts.push(l10n.fmt(
        "digest-sessions",
        &[("count", Arg::from(i64::from(d.sessions)))],
    ));
    if let Some(p) = &d.top_project {
        parts.push(l10n.fmt("digest-top-project", &[("name", Arg::Str(&p.name))]));
    }
    Notice {
        rule: Rule::Digest,
        title,
        body: parts.join(" · "),
        target: NoticeTarget::Overview,
    }
}
