//! The periodic summaries (Step 17b) with a fake clock: when they are due, that they go out
//! once (also across a restart), late delivery, quiet hours and mute, and their texts.

use std::sync::Arc;

use super::*;
use crate::attribution::Method;
use crate::clock::{LocalTime, UtcClock};
use crate::domain::{MicroUsd, ProjectId};
use crate::queries::{
    Basis, CycleProject, DailyDigest, FiveHourHits, FixedZone, ProjectRef, ScopedPeak, WeekBounds,
    WeeklyDigest, WindowRow,
};
use tokenradar_i18n::HourCycle;

const HOUR: i64 = 60 * MINUTE_MS;
/// 2026-10-28, a Wednesday, 00:00 UTC.
const DAY0: i64 = 1_793_145_600_000;

fn t(hours: i64) -> Timestamp {
    Timestamp::from_millis(DAY0 + hours * HOUR)
}

/// The weekly window that reset on Wednesday at 08:00 UTC (11:00 in Istanbul).
fn week() -> WeekBounds {
    WeekBounds {
        starts_at: t(8 - 7 * 24),
        resets_at: t(8),
    }
}

fn istanbul() -> FixedZone {
    FixedZone::offset(3 * HOUR)
}

/// Local time in Istanbul (UTC+3, no DST), for quiet hours and weekday texts.
struct IstanbulClock;

impl LocalClock for IstanbulClock {
    fn local(&self, t: Timestamp) -> Option<LocalTime> {
        UtcClock.local(t.saturating_add_millis(3 * HOUR))
    }

    fn hour_cycle(&self) -> HourCycle {
        HourCycle::H23
    }
}

fn with(prefs: NotifyPrefs, history: Vec<Fired>, muted: Option<Timestamp>) -> Notifier {
    Notifier::new(prefs, Locale::En, Arc::new(IstanbulClock), history, muted)
}

fn plain() -> Notifier {
    with(NotifyPrefs::default(), Vec::new(), None)
}

fn weekly() -> Slot {
    weekly_slot(&DigestPrefs::default(), week(), &istanbul())
}

#[test]
fn the_weekly_summary_is_due_an_hour_after_the_reset() {
    let slot = weekly();
    assert_eq!(slot.due_at, t(9));
    assert_eq!((slot.from, slot.to), (week().starts_at, week().resets_at));
}

#[test]
fn a_custom_day_and_time_is_the_first_one_after_the_reset() {
    let zone = istanbul();
    let monday_nine = DigestPrefs {
        weekly_at: WeeklyAt::Custom,
        weekly_day: 0,
        weekly_time: "09:00".into(),
        ..DigestPrefs::default()
    };
    // Wednesday 11:00 local reset → Monday 09:00 local = 06:00 UTC, five days later.
    assert_eq!(
        weekly_slot(&monday_nine, week(), &zone).due_at,
        t(5 * 24 + 6)
    );

    // The same day, later than the reset: that day.
    let wednesday_noon = DigestPrefs {
        weekly_day: 2,
        weekly_time: "12:00".into(),
        ..monday_nine.clone()
    };
    assert_eq!(weekly_slot(&wednesday_noon, week(), &zone).due_at, t(9));

    // The same day, before the reset: a week later.
    let wednesday_ten = DigestPrefs {
        weekly_day: 2,
        weekly_time: "10:00".into(),
        ..monday_nine
    };
    assert_eq!(
        weekly_slot(&wednesday_ten, week(), &zone).due_at,
        t(7 * 24 + 7)
    );
}

#[test]
fn a_summary_goes_out_once_and_not_again_after_a_restart() {
    let mut n = plain();
    let slot = weekly();
    assert_eq!(n.digest_state(&slot, t(8)), DigestState::Wait(t(9)));
    assert_eq!(n.digest_state(&slot, t(9)), DigestState::Due);

    let out = n.send_digest(&slot, None, t(9));
    assert_eq!(out.record.len(), 1);
    assert_eq!(n.digest_state(&slot, t(10)), DigestState::Done);

    // The stored row reads back, and a restarted notifier knows it was sent.
    let history: Vec<Fired> = out
        .record
        .iter()
        .map(|f| Fired::parse(f.rule.as_str(), &f.dedupe_key(), f.at).expect("parse"))
        .collect();
    assert_eq!(history, out.record);
    let restarted = with(NotifyPrefs::default(), history, None);
    assert_eq!(restarted.digest_state(&slot, t(12)), DigestState::Done);
}

#[test]
fn a_missed_summary_is_sent_late_within_two_days_then_skipped() {
    let n = plain();
    let slot = weekly();
    assert_eq!(n.digest_state(&slot, t(9 + 47)), DigestState::Due);
    assert_eq!(n.digest_state(&slot, t(9 + 48)), DigestState::Missed);
}

#[test]
fn a_sent_summary_is_remembered_past_the_other_rules_history() {
    let mut n = plain();
    let monday = DigestPrefs {
        weekly_at: WeeklyAt::Custom,
        weekly_day: 1,
        ..DigestPrefs::default()
    };
    // Due almost a week after the reset; evaluating limits nine days on must not forget it.
    let slot = weekly_slot(&monday, week(), &istanbul());
    n.send_digest(&slot, None, slot.due_at);
    n.evaluate(&[], t(9 * 24));
    assert!(n.digest_sent(&slot));
}

#[test]
fn quiet_hours_and_a_mute_hold_a_summary_until_they_end() {
    let quiet = NotifyPrefs {
        quiet_hours: QuietHours {
            enabled: true,
            start: "11:30".into(),
            end: "13:00".into(),
        },
        ..NotifyPrefs::default()
    };
    let n = with(quiet, Vec::new(), None);
    // Due 12:00 local (09:00 UTC), quiet until 13:00 local (10:00 UTC).
    assert_eq!(n.digest_state(&weekly(), t(9)), DigestState::Wait(t(10)));
    assert_eq!(n.digest_state(&weekly(), t(10)), DigestState::Due);

    let muted = with(NotifyPrefs::default(), Vec::new(), Some(t(15)));
    assert_eq!(
        muted.digest_state(&weekly(), t(9)),
        DigestState::Wait(t(15))
    );
    assert_eq!(muted.digest_state(&weekly(), t(15)), DigestState::Due);
}

#[test]
fn summaries_follow_the_switches() {
    let n = plain();
    let (daily, _) = daily_slot(&DigestPrefs::default(), t(12), &istanbul());
    assert_eq!(
        n.digest_state(&daily, t(12)),
        DigestState::Off,
        "daily is off by default"
    );

    let off = with(
        NotifyPrefs {
            enabled: false,
            ..NotifyPrefs::default()
        },
        Vec::new(),
        None,
    );
    assert_eq!(off.digest_state(&weekly(), t(9)), DigestState::Off);

    let no_weekly = NotifyPrefs {
        digest: DigestPrefs {
            weekly: false,
            ..DigestPrefs::default()
        },
        ..NotifyPrefs::default()
    };
    let n = with(no_weekly, Vec::new(), None);
    assert_eq!(n.digest_state(&weekly(), t(9)), DigestState::Off);
}

#[test]
fn the_daily_summary_covers_yesterday_from_its_time_on() {
    let zone = istanbul();
    let prefs = DigestPrefs {
        daily: true,
        daily_time: "09:00".into(),
        ..DigestPrefs::default()
    };
    // Local midnight of Wednesday is Tuesday 21:00 UTC.
    let wed = t(-3);
    let tue = t(-27);
    let mon = t(-51);

    // Wednesday 10:00 local: Tuesday's summary, due 09:00 local; the next one Thursday.
    let (slot, next) = daily_slot(&prefs, t(7), &zone);
    assert_eq!((slot.from, slot.to, slot.due_at), (tue, wed, t(6)));
    assert_eq!(next, t(24 + 6));

    // Wednesday 08:00 local: still Monday's summary (due Tuesday 09:00); next is today's.
    let (slot, next) = daily_slot(&prefs, t(5), &zone);
    assert_eq!((slot.from, slot.to, slot.due_at), (mon, tue, t(6 - 24)));
    assert_eq!(next, t(6));

    // Still sendable until the next one is due, then skipped.
    let n = with(
        NotifyPrefs {
            digest: prefs,
            ..NotifyPrefs::default()
        },
        Vec::new(),
        None,
    );
    assert_eq!(n.digest_state(&slot, t(5)), DigestState::Due);
    assert_eq!(n.digest_state(&slot, t(6)), DigestState::Missed);
}

fn window(peak: Option<f64>, hit: Option<bool>, cost_usd: i64) -> WindowRow {
    WindowRow {
        id: 1,
        kind: LimitKind::WeeklyAll,
        scope: None,
        starts_at: week().starts_at,
        resets_at: week().resets_at,
        peak_percent: peak,
        hit_limit: hit,
        estimated: hit.is_none(),
        open: false,
        cost: MicroUsd(cost_usd * 1_000_000),
        sessions: 12,
        other_percent: None,
        basis: Some(Basis {
            method: Method::Calibrated,
            confidence: Some(0.9),
            approx: false,
            dim: false,
            stale: false,
        }),
    }
}

fn digest() -> WeeklyDigest {
    WeeklyDigest {
        window: window(Some(64.0), Some(false), 123),
        hit_at: None,
        scoped: vec![ScopedPeak {
            window_id: 2,
            scope: Scope::Model("Fable".into()),
            peak_percent: Some(40.0),
            hit_limit: Some(false),
            estimated: false,
        }],
        five_hour: None,
        top_project: Some(CycleProject {
            project: Some(ProjectRef {
                id: ProjectId(7),
                name: "radar".into(),
            }),
            percent: Some(30.0),
            cost: MicroUsd(50_000_000),
        }),
        previous: Some(window(Some(52.0), Some(false), 150)),
    }
}

#[test]
fn the_weekly_text_says_peak_hits_cost_project_and_change() {
    let n = plain();
    let notice = n.weekly_notice(&digest());
    assert_eq!(notice.rule, Rule::Digest);
    assert_eq!(notice.target, NoticeTarget::Overview);
    assert_eq!(notice.title, "Last week: peak 64%");
    assert_eq!(
        notice.body,
        "Weekly limit not hit · Fable 40%\n\
         $123.00 API-equivalent · Top project: radar\n\
         vs previous week: peak +12 pts, cost \u{2212}18%"
    );
}

#[test]
fn the_weekly_text_names_the_hit_and_counts_five_hour_windows() {
    let mut d = digest();
    d.window.hit_limit = Some(true);
    d.window.peak_percent = Some(100.0);
    // Wednesday 14:20 local.
    d.hit_at = Some(t(-7 * 24 + 11).saturating_add_millis(20 * MINUTE_MS));
    d.five_hour = Some(FiveHourHits {
        windows: 14,
        hit: 2,
    });
    d.scoped[0].hit_limit = Some(true);
    d.previous = None;
    let tr = Notifier::new(
        NotifyPrefs::default(),
        Locale::Tr,
        Arc::new(IstanbulClock),
        Vec::new(),
        None,
    );
    let notice = tr.weekly_notice(&d);
    assert_eq!(notice.title, "Geçen hafta: tepe %100");
    assert_eq!(
        notice.body,
        "14 adet 5 saatlik pencereden 2 tanesi limite takıldı · \
         Haftalık limite Çar 14:20'de takıldı · Fable doldu\n\
         API karşılığı $123,00 · En çok: radar"
    );
}

#[test]
fn an_estimated_week_is_marked_and_its_hit_unknown() {
    let mut d = digest();
    d.window = window(Some(71.4), None, 80);
    d.scoped.clear();
    d.top_project = None;
    d.previous = None;
    let notice = plain().weekly_notice(&d);
    assert_eq!(notice.title, "Last week: peak ≈71%");
    assert_eq!(
        notice.body,
        "Limit hit unknown (estimated week)\n$80.00 API-equivalent"
    );
}

#[test]
fn the_daily_text_uses_points_when_calibrated() {
    let d = DailyDigest {
        from: t(-27),
        to: t(-3),
        cost: MicroUsd(12_400_000),
        percent: Some(9.04),
        basis: Basis {
            method: Method::Estimated,
            confidence: Some(0.8),
            approx: true,
            dim: true,
            stale: false,
        },
        sessions: 5,
        top_project: Some(ProjectRef {
            id: ProjectId(7),
            name: "radar".into(),
        }),
    };
    let notice = plain().daily_notice(&d);
    assert_eq!(notice.title, "Yesterday: ≈9.0 pts of the weekly limit");
    assert_eq!(
        notice.body,
        "$12.40 API-equivalent · 5 sessions · Top project: radar"
    );

    let uncalibrated = DailyDigest {
        percent: None,
        sessions: 1,
        top_project: None,
        ..d
    };
    let notice = plain().daily_notice(&uncalibrated);
    assert_eq!(notice.title, "Yesterday: $12.40 API-equivalent");
    assert_eq!(notice.body, "1 session");
}
