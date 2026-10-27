//! Notification rules (architecture §5): thresholds, projection and reset, once per window,
//! with mute and quiet hours.
//!
//! The notifier is state-based (ADR-033): every snapshot the limit engine builds is checked
//! against the user's rules, and what already fired is remembered in `notification_log`. So a
//! changed threshold list, a restart or a jump past two thresholds at once all behave the same
//! way: the highest threshold that has not fired in this window is announced once, the lower
//! ones are recorded silently. Resets come from the engine's [`LimitEvent::Reset`], since a
//! reset is an edge, not a state.
//!
//! The notifier is pure: it returns what to show and what to record, and the runtime writes the
//! records (`store::notifications`) and publishes the notices.
//!
//! **Suppression.** While muted or in quiet hours, notices are recorded but held back. When
//! both end, the held ones that are still true are announced together: one notice as is,
//! several as one summary.

mod prefs;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use serde::Serialize;
use specta::Type;
use tokenradar_i18n::format::weekday_clock;
use tokenradar_i18n::{Arg, Domain, Locale, Localizer};

pub use prefs::{MAX_THRESHOLDS, NotifyPrefs, PROJECTION_HORIZON_MIN, QuietHours, parse_hhmm};

use crate::clock::LocalClock;
use crate::domain::{LimitKind, Scope, Source, Timestamp};
use crate::limits::{Confidence, LimitEvent, LimitView};

const MINUTE_MS: i64 = 60_000;
const DAY_MIN: i64 = 24 * 60;
/// Two reset times this close belong to one window: the usage API's drifts by microseconds,
/// the statusline's is in whole seconds (ADR-028).
pub const SAME_WINDOW_MS: i64 = 5 * MINUTE_MS;
/// `kv` key holding the end of the notification mute in Unix milliseconds. The tray writes it
/// (ADR-030); the runtime reads it when the notifier starts.
pub const KV_MUTED_UNTIL: &str = "notify_snoozed_until";
/// Records older than this cannot match a current window (a week plus slack).
pub const HISTORY_MS: i64 = 8 * DAY_MIN * MINUTE_MS;

/// Which rule produced a notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    Threshold,
    Projection,
    /// The last projection warning, shortly before the limit is full.
    Final,
    Reset,
    /// Several held notices announced together after a mute or quiet hours. Never recorded.
    Summary,
    /// "Send test notification" in the settings. Never recorded.
    Test,
}

impl Rule {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Threshold => "threshold",
            Self::Projection => "projection",
            Self::Final => "final",
            Self::Reset => "reset",
            Self::Summary => "summary",
            Self::Test => "test",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "threshold" => Self::Threshold,
            "projection" => Self::Projection,
            "final" => Self::Final,
            "reset" => Self::Reset,
            _ => return None,
        })
    }
}

/// Something to show the user, already localized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct Notice {
    pub rule: Rule,
    pub title: String,
    pub body: String,
}

/// One rule that fired for one window: a `notification_log` row.
///
/// The stored key is `rule:kind:scope:resets_at:threshold` (`-` for no scope or threshold);
/// matching tolerates [`SAME_WINDOW_MS`] of drift in `resets_at`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fired {
    pub rule: Rule,
    pub kind: String,
    pub scope: Option<String>,
    pub resets_at: Timestamp,
    pub threshold: Option<u8>,
    pub at: Timestamp,
}

impl Fired {
    pub fn dedupe_key(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.rule.as_str(),
            self.kind,
            self.scope.as_deref().unwrap_or("-"),
            self.resets_at.as_millis(),
            self.threshold
                .map_or_else(|| "-".to_owned(), |t| t.to_string()),
        )
    }

    /// Reads a stored row back; `None` for keys this build does not understand.
    pub fn parse(rule: &str, key: &str, at: Timestamp) -> Option<Self> {
        let rule = Rule::parse(rule)?;
        // The scope may contain ':' (`model:Fable`): split the fixed parts off both ends.
        let tail = key.strip_prefix(rule.as_str())?.strip_prefix(':')?;
        let (kind, rest) = tail.split_once(':')?;
        let mut back = rest.rsplitn(3, ':');
        let threshold = back.next()?;
        let resets_at = back.next()?.parse().ok()?;
        let scope = back.next()?;
        Some(Self {
            rule,
            kind: kind.to_owned(),
            scope: (scope != "-").then(|| scope.to_owned()),
            resets_at: Timestamp::from_millis(resets_at),
            threshold: if threshold == "-" {
                None
            } else {
                Some(threshold.parse().ok()?)
            },
            at,
        })
    }

    fn matches(
        &self,
        rule: Rule,
        id: &LimitId,
        resets_at: Timestamp,
        threshold: Option<u8>,
    ) -> bool {
        self.rule == rule
            && self.kind == id.kind
            && self.scope == id.scope
            && self.threshold == threshold
            && (self.resets_at.as_millis() - resets_at.as_millis()).abs() <= SAME_WINDOW_MS
    }
}

/// What one evaluation decided.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub show: Vec<Notice>,
    /// New `notification_log` rows, shown or held.
    pub record: Vec<Fired>,
}

/// Which family of texts and thresholds a limit uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    FiveHour,
    Weekly,
    /// A weekly limit of one model or surface.
    Scoped,
}

impl Class {
    fn of(kind: &LimitKind) -> Option<Self> {
        match kind {
            LimitKind::Session => Some(Self::FiveHour),
            LimitKind::WeeklyAll => Some(Self::Weekly),
            LimitKind::WeeklyScoped => Some(Self::Scoped),
            LimitKind::Spend | LimitKind::Other(_) => None,
        }
    }

    const fn key(self) -> &'static str {
        match self {
            Self::FiveHour => "five_hour",
            Self::Weekly => "weekly",
            Self::Scoped => "scoped",
        }
    }
}

/// A limit as the notifier tells limits apart.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LimitId {
    kind: String,
    scope: Option<String>,
}

impl LimitId {
    fn of(kind: &LimitKind, scope: Option<&Scope>) -> Self {
        Self {
            kind: kind.as_str().to_owned(),
            scope: scope.map(ToString::to_string),
        }
    }
}

/// A notice held back during suppression. Re-checked and re-rendered when it ends, so the
/// numbers in it are current.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Held {
    rule: Rule,
    id: LimitId,
    resets_at: Timestamp,
    threshold: Option<u8>,
}

/// Rules that fire together for one limit: only the most important one is shown. Full beats
/// the final warning, which beats the projection, which beats a lower threshold; between
/// thresholds the higher one wins.
fn priority(rule: Rule, threshold: Option<u8>) -> (u8, u8) {
    let class = match (rule, threshold) {
        (Rule::Threshold, Some(t)) if t >= 100 => 4,
        (Rule::Final, _) => 3,
        (Rule::Projection, _) => 2,
        _ => 1,
    };
    (class, threshold.unwrap_or(0))
}

pub struct Notifier {
    prefs: NotifyPrefs,
    l10n: Localizer,
    clock: Arc<dyn LocalClock>,
    fired: Vec<Fired>,
    muted_until: Option<Timestamp>,
    held: Vec<Held>,
}

impl std::fmt::Debug for Notifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Notifier")
            .field("prefs", &self.prefs)
            .field("fired", &self.fired.len())
            .field("muted_until", &self.muted_until)
            .field("held", &self.held)
            .finish_non_exhaustive()
    }
}

impl Notifier {
    /// `history` is what `notification_log` holds from the last [`HISTORY_MS`].
    pub fn new(
        prefs: NotifyPrefs,
        locale: Locale,
        clock: Arc<dyn LocalClock>,
        history: Vec<Fired>,
        muted_until: Option<Timestamp>,
    ) -> Self {
        Self {
            prefs: prefs.normalized(),
            l10n: Localizer::new(locale, &[Domain::Notify]),
            clock,
            fired: history,
            muted_until,
            held: Vec::new(),
        }
    }

    pub fn set_prefs(&mut self, prefs: NotifyPrefs) {
        self.prefs = prefs.normalized();
        if !self.prefs.enabled {
            self.held.clear();
        }
    }

    pub fn set_locale(&mut self, locale: Locale) {
        if self.l10n.locale() != locale {
            self.l10n = Localizer::new(locale, &[Domain::Notify]);
        }
    }

    pub fn set_muted_until(&mut self, until: Option<Timestamp>) {
        self.muted_until = until;
    }

    /// Whether notices are held back at `now`: muted, or inside quiet hours.
    pub fn suppressed(&self, now: Timestamp) -> bool {
        self.muted_until.is_some_and(|u| u > now) || self.in_quiet_hours(now)
    }

    fn in_quiet_hours(&self, now: Timestamp) -> bool {
        self.clock
            .local(now)
            .is_some_and(|t| self.prefs.quiet_hours.contains(t.minute_of_day()))
    }

    /// When held notices could next be released: the end of the mute or of quiet hours,
    /// whichever comes first (the caller evaluates again then). `None` with nothing held.
    pub fn next_wake(&self, now: Timestamp) -> Option<Timestamp> {
        if self.held.is_empty() {
            return None;
        }
        let mute_end = self.muted_until.filter(|&u| u > now);
        let quiet_end = self.prefs.quiet_hours.span().and_then(|(_, end)| {
            let minute = i64::from(self.clock.local(now)?.minute_of_day());
            let delta = (i64::from(end) - minute).rem_euclid(DAY_MIN);
            let this_minute = now.as_millis() - now.as_millis().rem_euclid(MINUTE_MS);
            Some(Timestamp::from_millis(this_minute + delta * MINUTE_MS)).filter(|&t| t > now)
        });
        match (mute_end, quiet_end) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Checks every limit of a freshly built snapshot.
    pub fn evaluate(&mut self, limits: &[LimitView], now: Timestamp) -> Outcome {
        let mut out = Outcome::default();
        if !self.prefs.enabled {
            return out;
        }
        self.forget_old(now);
        let suppressed = self.suppressed(now);
        for limit in limits {
            self.evaluate_limit(limit, now, suppressed, &mut out);
        }
        if !suppressed {
            self.release(limits, now, &mut out);
        }
        out
    }

    fn evaluate_limit(
        &mut self,
        limit: &LimitView,
        now: Timestamp,
        suppressed: bool,
        out: &mut Outcome,
    ) {
        let Some(class) = Class::of(&limit.kind) else {
            return;
        };
        // Estimates are never announced as facts; without a reset time there is no window.
        let Some(resets_at) = limit.resets_at.filter(|&r| r > now) else {
            return;
        };
        if limit.source == Source::Estimate {
            return;
        }
        let id = LimitId::of(&limit.kind, limit.scope.as_ref());
        let mut due: Vec<(Rule, Option<u8>)> = Vec::new();

        let thresholds = match class {
            Class::FiveHour => &self.prefs.five_hour_thresholds,
            Class::Weekly | Class::Scoped => &self.prefs.weekly_thresholds,
        };
        for &t in thresholds {
            if limit.percent >= f64::from(t) && !self.has(Rule::Threshold, &id, resets_at, Some(t))
            {
                due.push((Rule::Threshold, Some(t)));
            }
        }
        if let Some(rule) = self.projection_rule(limit, now) {
            // The final warning makes the earlier one pointless.
            for r in [Rule::Projection, Rule::Final] {
                if !self.has(r, &id, resets_at, None) && (r == rule || r == Rule::Projection) {
                    due.push((r, None));
                }
            }
        }
        let Some(&top) = due.iter().max_by_key(|(r, t)| priority(*r, *t)) else {
            return;
        };
        for (rule, threshold) in &due {
            let fired = Fired {
                rule: *rule,
                kind: id.kind.clone(),
                scope: id.scope.clone(),
                resets_at,
                threshold: *threshold,
                at: now,
            };
            self.fired.push(fired.clone());
            out.record.push(fired);
        }
        let (rule, threshold) = top;
        if suppressed {
            self.held.push(Held {
                rule,
                id,
                resets_at,
                threshold,
            });
        } else if let Some(notice) = self.render(rule, threshold, limit, now) {
            out.show.push(notice);
        }
    }

    /// `Final` or `Projection` when the limit will be full within the horizon at a rate the
    /// engine trusts; `None` otherwise or when projection notices are off.
    fn projection_rule(&self, limit: &LimitView, now: Timestamp) -> Option<Rule> {
        if !self.prefs.projection || limit.percent >= 100.0 {
            return None;
        }
        let p = limit.projection?;
        if !p.will_hit_before_reset || p.confidence == Confidence::Low {
            return None;
        }
        let left = p.eta_100_at?.as_millis() - now.as_millis();
        if left <= 0 {
            return None;
        }
        let final_ms = i64::from(self.prefs.final_warning_min) * MINUTE_MS;
        if final_ms > 0 && left <= final_ms {
            Some(Rule::Final)
        } else if left <= i64::from(PROJECTION_HORIZON_MIN) * MINUTE_MS {
            Some(Rule::Projection)
        } else {
            None
        }
    }

    /// Handles the engine's reset signal.
    pub fn on_reset(&mut self, event: &LimitEvent, now: Timestamp) -> Outcome {
        let mut out = Outcome::default();
        let LimitEvent::Reset {
            kind,
            scope,
            previous_percent,
            reset_at,
            ..
        } = event
        else {
            return out;
        };
        if !self.prefs.enabled || Class::of(kind).is_none() {
            return out;
        }
        // A window that never got close is not worth a notification every five hours.
        if *previous_percent < f64::from(self.prefs.reset_notify_min) {
            return out;
        }
        let id = LimitId::of(kind, scope.as_ref());
        if self.has(Rule::Reset, &id, *reset_at, None) {
            return out;
        }
        let fired = Fired {
            rule: Rule::Reset,
            kind: id.kind.clone(),
            scope: id.scope.clone(),
            resets_at: *reset_at,
            threshold: None,
            at: now,
        };
        self.fired.push(fired.clone());
        out.record.push(fired);
        if self.suppressed(now) {
            self.held.push(Held {
                rule: Rule::Reset,
                id,
                resets_at: *reset_at,
                threshold: None,
            });
        } else {
            out.show.push(self.render_reset(kind, scope.as_ref()));
        }
        out
    }

    /// Announces the held notices that are still true, then forgets them all.
    fn release(&mut self, limits: &[LimitView], now: Timestamp, out: &mut Outcome) {
        if self.held.is_empty() {
            return;
        }
        let held = std::mem::take(&mut self.held);
        // One notice per limit: the most important one still true.
        let mut best: Vec<(&Held, Option<&LimitView>)> = Vec::new();
        for h in &held {
            let current = limits
                .iter()
                .find(|l| LimitId::of(&l.kind, l.scope.as_ref()) == h.id);
            if !self.still_true(h, current, now) {
                continue;
            }
            match best.iter_mut().find(|(b, _)| b.id == h.id) {
                Some(slot)
                    if priority(h.rule, h.threshold) > priority(slot.0.rule, slot.0.threshold) =>
                {
                    *slot = (h, current);
                }
                Some(_) => {}
                None => best.push((h, current)),
            }
        }
        let notices: Vec<Notice> = best
            .into_iter()
            .filter_map(|(h, current)| match (h.rule, current) {
                (Rule::Reset, _) => {
                    let kind = LimitKind::from(h.id.kind.as_str());
                    let scope = h.id.scope.as_deref().and_then(|s| s.parse().ok());
                    Some(self.render_reset(&kind, scope.as_ref()))
                }
                (rule, Some(limit)) => self.render(rule, h.threshold, limit, now),
                (_, None) => None,
            })
            .collect();
        match notices.len() {
            0 => {}
            1 => out.show.extend(notices),
            n => out.show.push(self.summary(&notices, n)),
        }
    }

    fn still_true(&self, h: &Held, current: Option<&LimitView>, now: Timestamp) -> bool {
        let same_window = |l: &LimitView| {
            l.resets_at
                .is_some_and(|r| (r.as_millis() - h.resets_at.as_millis()).abs() <= SAME_WINDOW_MS)
        };
        match h.rule {
            Rule::Threshold => current.is_some_and(|l| {
                same_window(l) && h.threshold.is_some_and(|t| l.percent >= f64::from(t))
            }),
            Rule::Projection | Rule::Final => {
                current.is_some_and(|l| same_window(l) && self.projection_rule(l, now).is_some())
            }
            // Still news while the new window has not climbed to the first threshold.
            Rule::Reset => current.is_none_or(|l| {
                let first = match Class::of(&l.kind) {
                    Some(Class::FiveHour) => self.prefs.five_hour_thresholds.first(),
                    _ => self.prefs.weekly_thresholds.first(),
                };
                l.percent < f64::from(first.copied().unwrap_or(100))
            }),
            Rule::Summary | Rule::Test => false,
        }
    }

    fn has(&self, rule: Rule, id: &LimitId, resets_at: Timestamp, threshold: Option<u8>) -> bool {
        self.fired
            .iter()
            .any(|f| f.matches(rule, id, resets_at, threshold))
    }

    fn forget_old(&mut self, now: Timestamp) {
        let horizon = now.as_millis() - HISTORY_MS;
        self.fired.retain(|f| f.resets_at.as_millis() >= horizon);
    }

    // -----------------------------------------------------------------------------------------
    // Texts
    // -----------------------------------------------------------------------------------------

    fn render(
        &self,
        rule: Rule,
        threshold: Option<u8>,
        limit: &LimitView,
        now: Timestamp,
    ) -> Option<Notice> {
        let class = Class::of(&limit.kind)?;
        let name = scope_name(limit.scope.as_ref());
        let what = self.limit_name(class, &name);
        let base = [
            ("limit", Arg::Str(class.key())),
            ("name", Arg::Str(&name)),
            ("what", Arg::Str(&what)),
        ];
        let resets = self.resets_text(class, limit.resets_at?, now);
        let (title, body) = match rule {
            Rule::Threshold => {
                let t = threshold?;
                let title = self.l10n.fmt(
                    "notify-threshold-title",
                    &[base[0], base[1], base[2], ("pct", Arg::from(i64::from(t)))],
                );
                (title, resets)
            }
            Rule::Projection | Rule::Final => {
                let eta = ceil_minute(limit.projection?.eta_100_at?.as_millis() - now.as_millis());
                let key = if rule == Rule::Final {
                    "notify-final-title"
                } else {
                    "notify-projection-title"
                };
                let title = self
                    .l10n
                    .fmt(key, &[base[0], base[1], base[2], ("eta", Arg::from(eta))]);
                (title, resets)
            }
            Rule::Reset | Rule::Summary | Rule::Test => return None,
        };
        Some(Notice { rule, title, body })
    }

    /// "5-hour window", "Weekly limit", "Fable weekly limit".
    fn limit_name(&self, class: Class, name: &str) -> String {
        self.l10n.fmt(
            "notify-limit",
            &[("limit", Arg::Str(class.key())), ("name", Arg::Str(name))],
        )
    }

    /// "Resets in 1h 12m" for the 5-hour window, "Resets Wed 10:59" for weekly limits.
    fn resets_text(&self, class: Class, resets_at: Timestamp, now: Timestamp) -> String {
        let local = (class != Class::FiveHour)
            .then(|| self.clock.local(resets_at))
            .flatten();
        if let Some(t) = local {
            let at = weekday_clock(
                self.l10n.locale(),
                t.weekday,
                t.hour,
                t.minute,
                self.clock.hour_cycle(),
            );
            self.l10n.fmt("notify-resets-at", &[("at", Arg::Str(&at))])
        } else {
            let ms = ceil_minute(resets_at.as_millis() - now.as_millis());
            self.l10n.fmt("notify-resets-in", &[("ms", Arg::from(ms))])
        }
    }

    fn render_reset(&self, kind: &LimitKind, scope: Option<&Scope>) -> Notice {
        let class = Class::of(kind).unwrap_or(Class::FiveHour);
        let what = self.limit_name(class, &scope_name(scope));
        Notice {
            rule: Rule::Reset,
            title: self
                .l10n
                .fmt("notify-reset-title", &[("what", Arg::Str(&what))]),
            body: self.l10n.msg("notify-reset-body"),
        }
    }

    fn summary(&self, notices: &[Notice], count: usize) -> Notice {
        Notice {
            rule: Rule::Summary,
            title: self.l10n.fmt(
                "notify-summary-title",
                &[("count", Arg::from(i64::try_from(count).unwrap_or(i64::MAX)))],
            ),
            body: notices
                .iter()
                .map(|n| n.title.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

/// The notice "Send test notification" shows.
pub fn test_notice(locale: Locale) -> Notice {
    let l10n = Localizer::new(locale, &[Domain::Notify]);
    Notice {
        rule: Rule::Test,
        title: l10n.msg("notify-test-title"),
        body: l10n.msg("notify-test-body"),
    }
}

/// `Fable` for `model:Fable`; empty without a scope.
fn scope_name(scope: Option<&Scope>) -> String {
    match scope {
        Some(Scope::Model(name) | Scope::Surface(name)) => name.clone(),
        None => String::new(),
    }
}

/// Rounded up to whole minutes, so "in 1 min" never shows for 10 seconds left as "0 min".
fn ceil_minute(ms: i64) -> i64 {
    (ms.max(0) + MINUTE_MS - 1) / MINUTE_MS * MINUTE_MS
}
