use std::sync::Arc;

use super::*;
use crate::clock::UtcClock;
use crate::domain::AccountId;
use crate::limits::{Freshness, Projection, RateMethod, Severity};

/// 2026-10-28 (a Wednesday) 09:11 UTC; [`UtcClock`] makes UTC the local time.
const NOW: Timestamp = Timestamp::from_millis(1_793_145_600_000 + (9 * 60 + 11) * MINUTE_MS);

fn at(minutes: i64) -> Timestamp {
    NOW.saturating_add_millis(minutes * MINUTE_MS)
}

fn notifier(prefs: NotifyPrefs, locale: Locale) -> Notifier {
    Notifier::new(prefs, locale, Arc::new(UtcClock), Vec::new(), None)
}

fn en() -> Notifier {
    notifier(NotifyPrefs::default(), Locale::En)
}

/// A measured limit resetting `resets_in` minutes after [`NOW`].
fn limit(kind: LimitKind, percent: f64, resets_in: i64) -> LimitView {
    LimitView {
        scope: (kind == LimitKind::WeeklyScoped).then(|| Scope::Model("Fable".into())),
        kind,
        percent,
        resets_at: Some(at(resets_in)),
        severity: Severity::of(percent),
        provider_severity: None,
        source: Source::Statusline,
        estimate_reason: None,
        observed_at: NOW,
        freshness: Freshness::Fresh,
        projection: None,
    }
}

fn five(percent: f64) -> LimitView {
    limit(LimitKind::Session, percent, 108)
}

/// Full in `eta` minutes after [`NOW`], before the reset.
fn projected(mut l: LimitView, eta: i64, confidence: Confidence) -> LimitView {
    l.projection = Some(Projection {
        rate_per_min: 0.5,
        eta_100_at: Some(at(eta)),
        will_hit_before_reset: true,
        projected_at_reset: Some(120.0),
        confidence,
        method: RateMethod::Observed,
    });
    l
}

fn titles(out: &Outcome) -> Vec<&str> {
    out.show.iter().map(|n| n.title.as_str()).collect()
}

fn reset_event(kind: LimitKind, previous: f64) -> LimitEvent {
    LimitEvent::Reset {
        account: AccountId(1),
        kind,
        scope: None,
        previous_percent: previous,
        reset_at: at(-1),
    }
}

#[test]
fn a_threshold_fires_once_per_window() {
    let mut n = en();
    assert_eq!(n.evaluate(&[five(60.0)], NOW), Outcome::default());

    let out = n.evaluate(&[five(76.0)], NOW);
    assert_eq!(titles(&out), ["5-hour window reached 75%"]);
    assert_eq!(out.show[0].body, "Resets in 1h 48m.");
    assert_eq!(out.record.len(), 1);
    assert_eq!(
        out.record[0].dedupe_key(),
        format!("threshold:session:-:{}:75", at(108).as_millis())
    );

    assert_eq!(n.evaluate(&[five(80.0)], at(1)), Outcome::default());
    let out = n.evaluate(&[five(91.5)], at(2));
    assert_eq!(titles(&out), ["5-hour window reached 90%"]);
    let out = n.evaluate(&[five(100.0)], at(3));
    assert_eq!(titles(&out), ["5-hour window is full"]);
}

#[test]
fn a_jump_announces_only_the_highest_threshold() {
    let mut n = en();
    let out = n.evaluate(&[five(92.0)], NOW);
    assert_eq!(titles(&out), ["5-hour window reached 90%"]);
    let recorded: Vec<_> = out.record.iter().map(|f| f.threshold).collect();
    assert_eq!(recorded, [Some(75), Some(90)]);
    assert_eq!(n.evaluate(&[five(92.0)], at(1)), Outcome::default());
}

#[test]
fn a_new_window_rearms_the_rules_but_drift_does_not() {
    let mut n = en();
    assert_eq!(n.evaluate(&[five(91.0)], NOW).show.len(), 1);
    // The usage API's reset time drifts a little: still the same window.
    let drifted = limit(LimitKind::Session, 91.0, 110);
    assert_eq!(n.evaluate(&[drifted], NOW), Outcome::default());
    // Five hours later, a new window.
    let next = limit(LimitKind::Session, 77.0, 108 + 300);
    let out = n.evaluate(&[next], at(200));
    assert_eq!(titles(&out), ["5-hour window reached 75%"]);
}

#[test]
fn the_log_survives_a_restart() {
    let mut first = en();
    let out = first.evaluate(&[five(91.0)], NOW);
    let history: Vec<Fired> = out
        .record
        .iter()
        .map(|f| Fired::parse(f.rule.as_str(), &f.dedupe_key(), f.at).expect("parse"))
        .collect();
    assert_eq!(history, out.record);

    let mut restarted = Notifier::new(
        NotifyPrefs::default(),
        Locale::En,
        Arc::new(UtcClock),
        history,
        None,
    );
    assert_eq!(restarted.evaluate(&[five(95.0)], at(5)), Outcome::default());
}

#[test]
fn dedupe_keys_round_trip_with_scopes() {
    let fired = Fired {
        rule: Rule::Threshold,
        kind: "weekly_scoped".into(),
        scope: Some("model:Fable".into()),
        resets_at: at(3000),
        threshold: Some(90),
        at: NOW,
    };
    let key = fired.dedupe_key();
    assert_eq!(Fired::parse("threshold", &key, NOW), Some(fired));
    assert_eq!(Fired::parse("projection", &key, NOW), None);
    assert_eq!(Fired::parse("summary", "summary:x", NOW), None);
    assert_eq!(Fired::parse("reset", "reset:session:-:abc:-", NOW), None);
}

#[test]
fn weekly_and_scoped_limits_have_their_own_texts_and_thresholds() {
    let mut n = notifier(
        NotifyPrefs {
            weekly_thresholds: vec![50],
            ..NotifyPrefs::default()
        },
        Locale::En,
    );
    // Friday 10:59 UTC.
    let resets = (2 * 24 + 1) * 60 + 48;
    let out = n.evaluate(
        &[
            limit(LimitKind::WeeklyAll, 55.0, resets),
            limit(LimitKind::WeeklyScoped, 51.0, resets),
            five(60.0),
        ],
        NOW,
    );
    assert_eq!(
        titles(&out),
        ["Weekly limit reached 50%", "Fable weekly limit reached 50%"]
    );
    assert_eq!(out.show[0].body, "Resets Fri 10:59.");
}

#[test]
fn turkish_texts_use_suffixes() {
    let mut n = notifier(NotifyPrefs::default(), Locale::Tr);
    let resets = (2 * 24 + 1) * 60 + 48;
    let out = n.evaluate(
        &[five(91.0), limit(LimitKind::WeeklyAll, 76.0, resets)],
        NOW,
    );
    assert_eq!(
        titles(&out),
        [
            "5 saatlik pencere %90'a ulaştı",
            "Haftalık limit %75'e ulaştı"
        ]
    );
    assert_eq!(out.show[0].body, "1 sa 48 dk sonra yenilenir.");
    assert_eq!(out.show[1].body, "Cum 10:59'da yenilenir.");

    let out = n.evaluate(&[projected(five(95.0), 12, Confidence::High)], at(1));
    assert_eq!(titles(&out), ["5 saatlik pencere 11 dk içinde dolacak"]);
    let out = n.on_reset(&reset_event(LimitKind::Session, 97.0), at(2));
    assert_eq!(titles(&out), ["5 saatlik pencere yenilendi"]);
    assert_eq!(out.show[0].body, "Tam kapasite.");
}

#[test]
fn projection_needs_a_trusted_rate_and_fires_once_per_window() {
    let mut n = en();
    // Too uncertain, then too far away: nothing.
    assert_eq!(
        n.evaluate(&[projected(five(50.0), 38, Confidence::Low)], NOW),
        Outcome::default()
    );
    assert_eq!(
        n.evaluate(&[projected(five(50.0), 90, Confidence::High)], NOW),
        Outcome::default()
    );

    let out = n.evaluate(&[projected(five(52.0), 38, Confidence::Medium)], NOW);
    assert_eq!(
        titles(&out),
        ["At this rate the 5-hour window fills in 38m"]
    );
    assert_eq!(out.show[0].body, "Resets in 1h 48m.");

    // The rate drops and rises again within the window: no second projection notice.
    assert_eq!(n.evaluate(&[five(55.0)], at(5)), Outcome::default());
    assert_eq!(
        n.evaluate(&[projected(five(58.0), 30, Confidence::High)], at(6)),
        Outcome::default()
    );

    // Close to full: the final warning, once.
    let out = n.evaluate(&[projected(five(70.0), 20, Confidence::High)], at(8));
    assert_eq!(titles(&out), ["5-hour window fills in 12m"]);
    assert_eq!(out.show[0].rule, Rule::Final);
    assert_eq!(
        n.evaluate(&[projected(five(72.0), 22, Confidence::High)], at(10)),
        Outcome::default()
    );
}

#[test]
fn a_late_first_projection_goes_straight_to_the_final_warning() {
    let mut n = en();
    let out = n.evaluate(&[projected(five(60.0), 10, Confidence::High)], NOW);
    assert_eq!(out.show.len(), 1);
    assert_eq!(out.show[0].rule, Rule::Final);
    let rules: Vec<_> = out.record.iter().map(|f| f.rule).collect();
    assert_eq!(rules, [Rule::Projection, Rule::Final]);
}

#[test]
fn projection_settings_are_respected() {
    let mut off = notifier(
        NotifyPrefs {
            projection: false,
            ..NotifyPrefs::default()
        },
        Locale::En,
    );
    assert_eq!(
        off.evaluate(&[projected(five(60.0), 10, Confidence::High)], NOW),
        Outcome::default()
    );
    let mut no_final = notifier(
        NotifyPrefs {
            final_warning_min: 0,
            ..NotifyPrefs::default()
        },
        Locale::En,
    );
    let out = no_final.evaluate(&[projected(five(60.0), 10, Confidence::High)], NOW);
    assert_eq!(out.show[0].rule, Rule::Projection);
}

#[test]
fn rules_firing_together_show_the_most_important_one() {
    let mut n = en();
    // Already over 90 and about to be full: the projection says more than "reached 90 %".
    let out = n.evaluate(&[projected(five(93.0), 30, Confidence::High)], NOW);
    assert_eq!(out.show.len(), 1);
    assert_eq!(out.show[0].rule, Rule::Projection);
    assert_eq!(out.record.len(), 3);

    // Full beats everything.
    let mut n = en();
    let out = n.evaluate(&[five(100.0)], NOW);
    assert_eq!(titles(&out), ["5-hour window is full"]);
}

#[test]
fn estimates_and_unknown_kinds_are_never_announced() {
    let mut n = en();
    let mut estimate = five(95.0);
    estimate.source = Source::Estimate;
    let spend = limit(LimitKind::Spend, 99.0, 100);
    let mut no_reset = five(95.0);
    no_reset.resets_at = None;
    assert_eq!(
        n.evaluate(&[estimate, spend, no_reset], NOW),
        Outcome::default()
    );
}

#[test]
fn resets_are_announced_only_after_a_busy_window() {
    let mut n = en();
    assert_eq!(
        n.on_reset(&reset_event(LimitKind::Session, 50.0), NOW),
        Outcome::default()
    );
    let out = n.on_reset(&reset_event(LimitKind::Session, 85.0), NOW);
    assert_eq!(titles(&out), ["5-hour window has reset"]);
    assert_eq!(out.show[0].body, "Full capacity again.");
    assert_eq!(
        n.on_reset(&reset_event(LimitKind::Session, 85.0), at(1)),
        Outcome::default()
    );

    let mut strict = notifier(
        NotifyPrefs {
            reset_notify_min: 100,
            ..NotifyPrefs::default()
        },
        Locale::En,
    );
    assert_eq!(
        strict
            .on_reset(&reset_event(LimitKind::WeeklyAll, 99.0), NOW)
            .show
            .len(),
        0
    );
    let out = strict.on_reset(&reset_event(LimitKind::WeeklyAll, 100.0), NOW);
    assert_eq!(titles(&out), ["Weekly limit has reset"]);
}

#[test]
fn a_mute_holds_notices_and_releases_the_true_ones_as_a_summary() {
    let mut n = en();
    n.set_muted_until(Some(at(60)));
    let out = n.evaluate(&[five(76.0)], NOW);
    assert_eq!(out.show.len(), 0);
    assert_eq!(out.record.len(), 1, "recorded while muted");
    let out = n.evaluate(&[five(91.0)], at(10));
    assert_eq!(out.show.len(), 0);
    let out = n.on_reset(&reset_event(LimitKind::WeeklyAll, 95.0), at(20));
    assert_eq!(out.show.len(), 0);
    assert_eq!(n.next_wake(at(20)), Some(at(60)));

    // Still muted: nothing yet.
    assert_eq!(n.evaluate(&[five(92.0)], at(59)), Outcome::default());
    // The mute ended: one notice per limit, both in one summary.
    let weekly = limit(LimitKind::WeeklyAll, 1.0, 7 * 24 * 60);
    let out = n.evaluate(&[five(92.0), weekly.clone()], at(60));
    assert_eq!(out.show.len(), 1);
    assert_eq!(out.show[0].rule, Rule::Summary);
    assert_eq!(out.show[0].title, "2 notifications while paused");
    assert_eq!(
        out.show[0].body,
        "5-hour window reached 90%\nWeekly limit has reset"
    );
    assert_eq!(out.record.len(), 0);
    assert_eq!(n.next_wake(at(60)), None);
    assert_eq!(
        n.evaluate(&[five(92.0), weekly], at(61)),
        Outcome::default()
    );
}

#[test]
fn held_notices_that_are_no_longer_true_are_dropped() {
    let mut n = en();
    n.set_muted_until(Some(at(30)));
    assert_eq!(
        n.evaluate(&[projected(five(60.0), 40, Confidence::High)], NOW)
            .show
            .len(),
        0
    );
    // The rate fell: the projection is no longer true when the mute ends.
    assert_eq!(n.evaluate(&[five(62.0)], at(30)), Outcome::default());

    // A single held notice that is still true is shown as itself, with current numbers.
    n.set_muted_until(Some(at(40)));
    assert_eq!(n.evaluate(&[five(80.0)], at(31)).show.len(), 0);
    let out = n.evaluate(&[five(81.0)], at(40));
    assert_eq!(titles(&out), ["5-hour window reached 75%"]);
    assert_eq!(out.show[0].body, "Resets in 1h 8m.");
}

#[test]
fn quiet_hours_hold_notices_until_they_end() {
    let mut n = notifier(
        NotifyPrefs {
            quiet_hours: QuietHours {
                enabled: true,
                start: "22:00".into(),
                end: "09:30".into(),
            },
            ..NotifyPrefs::default()
        },
        Locale::En,
    );
    assert!(n.suppressed(NOW));
    assert_eq!(n.evaluate(&[five(80.0)], NOW).show.len(), 0);
    // 09:30 is 19 minutes after NOW.
    assert_eq!(n.next_wake(NOW), Some(at(19)));
    let out = n.evaluate(&[five(80.0)], at(19));
    assert_eq!(titles(&out), ["5-hour window reached 75%"]);
}

#[test]
fn a_mute_and_quiet_hours_both_have_to_end() {
    let mut n = notifier(
        NotifyPrefs {
            quiet_hours: QuietHours {
                enabled: true,
                start: "09:00".into(),
                end: "09:20".into(),
            },
            ..NotifyPrefs::default()
        },
        Locale::En,
    );
    n.set_muted_until(Some(at(30)));
    assert_eq!(n.evaluate(&[five(80.0)], NOW).show.len(), 0);
    // Quiet hours end first; the runtime wakes, finds the mute, waits for it.
    assert_eq!(n.next_wake(NOW), Some(at(9)));
    assert_eq!(n.evaluate(&[five(80.0)], at(9)).show.len(), 0);
    assert_eq!(n.next_wake(at(9)), Some(at(30)));
    assert_eq!(n.evaluate(&[five(80.0)], at(30)).show.len(), 1);
}

#[test]
fn disabled_means_silent() {
    let mut n = notifier(
        NotifyPrefs {
            enabled: false,
            ..NotifyPrefs::default()
        },
        Locale::En,
    );
    assert_eq!(n.evaluate(&[five(100.0)], NOW), Outcome::default());
    assert_eq!(
        n.on_reset(&reset_event(LimitKind::Session, 100.0), NOW),
        Outcome::default()
    );
}

#[test]
fn changed_thresholds_apply_at_once() {
    let mut n = en();
    assert_eq!(n.evaluate(&[five(60.0)], NOW).show.len(), 0);
    n.set_prefs(NotifyPrefs {
        five_hour_thresholds: vec![50, 90],
        ..NotifyPrefs::default()
    });
    let out = n.evaluate(&[five(60.0)], at(1));
    assert_eq!(titles(&out), ["5-hour window reached 50%"]);
}

#[test]
fn the_locale_can_change() {
    let mut n = en();
    n.set_locale(Locale::Tr);
    let out = n.evaluate(&[five(76.0)], NOW);
    assert_eq!(titles(&out), ["5 saatlik pencere %75'e ulaştı"]);
}

#[test]
fn test_notices_are_localized() {
    assert_eq!(
        test_notice(Locale::En).title,
        "TokenRadar test notification"
    );
    assert_eq!(test_notice(Locale::Tr).title, "TokenRadar test bildirimi");
}
