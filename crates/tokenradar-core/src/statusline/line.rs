//! The statusline's line (Step 12c): which segments, in what order, from which source.
//!
//! ```text
//! 7d ████░░░░ 41% · 3d4h │ Fable 22% │ ctx 38% │ ≈2.1% this session │ API $4.12
//! ```
//!
//! The payload Claude Code just sent is the freshest measurement, so its values win. The app
//! adds what only it knows: scoped limits, the projection (`▲`) and the session's share. A
//! window the payload lacks falls back to the app's value only if that is a measurement: the
//! app's estimates are not shown here, and a window the account does not have (ADR-041) gets
//! no segment at all. Every value keeps its source and the time it was observed; an app value
//! older than [`FRESH_MS`] is drawn dim.
//!
//! The CLI renders the line for Claude Code; the settings window renders a sample of it as a
//! preview, through the same code.

use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;

use super::payload::{RateWindow, StatuslinePayload};
use crate::domain::{Scope, Source, Timestamp};
use crate::ipc::{LimitInfo, StatusReply};
use crate::limits::{FRESH_MS, SESSION_WINDOW_MS, WEEK_MS};

mod render;

pub use render::{Paint, Span, Style, render, spans, to_ansi};

/// One part of the line. The names are the `segments` values in `config.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    FiveHour,
    SevenDay,
    /// Weekly limits of one model or surface, e.g. Fable's.
    Scoped,
    Context,
    SessionShare,
    Cost,
    Model,
}

impl SegmentKind {
    pub const ALL: [Self; 7] = [
        Self::FiveHour,
        Self::SevenDay,
        Self::Scoped,
        Self::Context,
        Self::SessionShare,
        Self::Cost,
        Self::Model,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FiveHour => "five_hour",
            Self::SevenDay => "seven_day",
            Self::Scoped => "scoped",
            Self::Context => "context",
            Self::SessionShare => "session_share",
            Self::Cost => "cost",
            Self::Model => "model",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }

    /// A limit window. Limits go on the first line of the double layout and are dropped last
    /// when the terminal is narrow.
    pub const fn is_limit(self) -> bool {
        matches!(self, Self::FiveHour | Self::SevenDay | Self::Scoped)
    }
}

/// One line, or limits on the first and the session on the second.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    #[default]
    Single,
    Double,
}

impl Layout {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Double => "double",
        }
    }
}

/// The default segments. `five_hour` shows only on accounts that have the window.
pub const DEFAULT_SEGMENTS: [SegmentKind; 6] = [
    SegmentKind::FiveHour,
    SegmentKind::SevenDay,
    SegmentKind::Scoped,
    SegmentKind::Context,
    SegmentKind::SessionShare,
    SegmentKind::Cost,
];

/// `[statusline]` in `config.toml`: what the line shows.
///
/// ```toml
/// [statusline]
/// segments = ["five_hour", "seven_day", "scoped", "context", "session_share", "cost"]
/// layout = "single"   # single | double
/// bar = true
/// pace = true
/// ```
///
/// Reading never fails: unknown segment names are skipped, an unknown layout is `single`, and
/// a list left empty (or holding only unknown names) is the default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct StatuslinePrefs {
    pub segments: Vec<SegmentKind>,
    pub layout: Layout,
    /// The 8-cell bar on the 5-hour and weekly windows. Dropped without color and when narrow.
    pub bar: bool,
    /// How much of the window's time has passed: a `│` mark on the bar and `⏱43%` after the
    /// value (ADR-044). Added in 0.1.1; absent from older files, so it defaults to on.
    pub pace: bool,
}

impl Default for StatuslinePrefs {
    fn default() -> Self {
        Self {
            segments: DEFAULT_SEGMENTS.to_vec(),
            layout: Layout::Single,
            bar: true,
            pace: true,
        }
    }
}

impl StatuslinePrefs {
    /// Without duplicates; an empty list becomes the default.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        let mut seen = Vec::with_capacity(self.segments.len());
        self.segments.retain(|k| {
            let new = !seen.contains(k);
            seen.push(*k);
            new
        });
        if self.segments.is_empty() {
            self.segments = DEFAULT_SEGMENTS.to_vec();
        }
        self
    }
}

impl<'de> Deserialize<'de> for StatuslinePrefs {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct Raw {
            segments: Option<Vec<String>>,
            layout: Option<String>,
            bar: Option<bool>,
            pace: Option<bool>,
        }
        let raw = Raw::deserialize(de)?;
        let default = Self::default();
        Ok(Self {
            segments: raw.segments.map_or(default.segments, |names| {
                names.iter().filter_map(|n| SegmentKind::parse(n)).collect()
            }),
            layout: match raw.layout.as_deref() {
                Some("double") => Layout::Double,
                _ => Layout::Single,
            },
            bar: raw.bar.unwrap_or(default.bar),
            pace: raw.pace.unwrap_or(default.pace),
        }
        .normalized())
    }
}

/// A value with where it came from and when.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading<T> {
    pub value: T,
    pub source: Source,
    pub observed_at: Timestamp,
}

impl<T> Reading<T> {
    fn now(value: T, source: Source, now: Timestamp) -> Self {
        Self {
            value,
            source,
            observed_at: now,
        }
    }
}

/// One limit window as the line shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub percent: f64,
    /// Until the reset, ms.
    pub resets_in: Option<i64>,
    /// Until 100 % at the current rate, ms; only when it comes before the reset.
    pub eta: Option<i64>,
    /// How much of the window's time has passed, 0–100 %: a measurement of the clock, not
    /// an estimate (ADR-044). `None` without a reset time.
    pub elapsed: Option<f64>,
    /// `Estimate` after a passed reset; the app's own estimates never get here.
    pub source: Source,
    pub observed_at: Timestamp,
}

impl Window {
    pub fn estimated(&self) -> bool {
        self.source == Source::Estimate
    }
}

/// A weekly limit of one model or surface: `Fable 22%`.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopedWindow {
    /// The model or surface name, as the provider spells it.
    pub label: String,
    pub window: Window,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// When the line is drawn; ages are measured against it.
    pub at: Timestamp,
    pub five_hour: Option<Window>,
    pub seven_day: Option<Window>,
    pub scoped: Vec<ScopedWindow>,
    /// `context_window.used_percentage`.
    pub context: Option<Reading<f64>>,
    /// This session's share of the 5-hour window, %; always an estimate.
    pub session_share: Option<Reading<f64>>,
    /// `cost.total_cost_usd`: what the session would cost at API prices, not a bill.
    pub cost: Option<Reading<f64>>,
    pub model: Option<Reading<String>>,
}

impl Line {
    /// A line with nothing on it.
    pub fn empty(at: Timestamp) -> Self {
        Self {
            at,
            five_hour: None,
            seven_day: None,
            scoped: Vec::new(),
            context: None,
            session_share: None,
            cost: None,
            model: None,
        }
    }

    pub fn from_sources(
        payload: Option<&StatuslinePayload>,
        reply: Option<&StatusReply>,
        now: Timestamp,
    ) -> Self {
        let app = |kind: &str| {
            reply.and_then(|r| {
                r.limits
                    .iter()
                    .find(|l| l.kind == kind && l.scope.is_none())
            })
        };
        let absent = |kind: &str| reply.is_some_and(|r| r.is_absent(kind));
        let usable = |v: Option<f64>| v.filter(|v| v.is_finite() && *v >= 0.0);
        Self {
            at: now,
            five_hour: window(
                payload.and_then(StatuslinePayload::five_hour),
                app("session"),
                absent("session"),
                SESSION_WINDOW_MS,
                now,
            ),
            seven_day: window(
                payload.and_then(StatuslinePayload::seven_day),
                app("weekly_all"),
                absent("weekly_all"),
                WEEK_MS,
                now,
            ),
            scoped: reply.map_or_else(Vec::new, |r| scoped(&r.limits, now)),
            context: usable(
                payload
                    .and_then(|p| p.context_window.as_ref())
                    .and_then(|c| c.used_percentage),
            )
            .map(|v| Reading::now(v, Source::Statusline, now)),
            session_share: reply.and_then(|r| {
                let share = r.session.as_ref()?.share_percent?;
                Some(Reading {
                    value: share,
                    source: Source::Estimate,
                    observed_at: r.generated_at,
                })
            }),
            cost: usable(
                payload
                    .and_then(|p| p.cost.as_ref())
                    .and_then(|c| c.total_cost_usd),
            )
            .map(|v| Reading::now(v, Source::Statusline, now)),
            model: payload
                .and_then(|p| p.model.as_ref())
                .and_then(|m| m.display_name.as_ref().or(m.id.as_ref()))
                .map(|name| name.trim())
                .filter(|name| !name.is_empty())
                .map(|name| Reading::now(name.to_owned(), Source::Statusline, now)),
        }
    }

    /// Whether a value observed at `observed_at` is too old to be drawn as current.
    pub fn is_stale(&self, observed_at: Timestamp) -> bool {
        self.at.as_millis() - observed_at.as_millis() >= FRESH_MS
    }

    /// A sample for the settings preview: every segment filled with typical values. The
    /// 5-hour window is left out when `five_hour` is false (an account without it).
    pub fn sample(at: Timestamp, five_hour: bool) -> Self {
        const MIN: i64 = 60_000;
        let w = |percent: f64, resets_in: i64, length: i64| Window {
            percent,
            resets_in: Some(resets_in),
            eta: None,
            elapsed: elapsed(Some(resets_in), length),
            source: Source::Statusline,
            observed_at: at,
        };
        Self {
            at,
            five_hour: five_hour.then(|| Window {
                eta: Some(38 * MIN),
                ..w(72.0, 108 * MIN, SESSION_WINDOW_MS)
            }),
            seven_day: Some(w(41.0, (3 * 24 + 4) * 60 * MIN, WEEK_MS)),
            scoped: vec![ScopedWindow {
                label: "Fable".into(),
                window: Window {
                    source: Source::Api,
                    ..w(22.0, (3 * 24 + 4) * 60 * MIN, WEEK_MS)
                },
            }],
            context: Some(Reading::now(38.0, Source::Statusline, at)),
            session_share: Some(Reading::now(2.1, Source::Estimate, at)),
            cost: Some(Reading::now(4.12, Source::Statusline, at)),
            model: Some(Reading::now("Opus 5.5".into(), Source::Statusline, at)),
        }
    }
}

fn until(at: Option<Timestamp>, now: Timestamp) -> Option<i64> {
    at.map(|t| t.as_millis() - now.as_millis())
}

pub use crate::limits::pace::elapsed;

fn eta(app: Option<&LimitInfo>, now: Timestamp) -> Option<i64> {
    app.filter(|a| a.will_hit_before_reset)
        .and_then(|a| until(a.eta_100_at, now))
        .filter(|ms| *ms > 0)
}

/// The app's measurement as a window; `None` for its estimates.
fn from_app(a: &LimitInfo, length: i64, now: Timestamp) -> Option<Window> {
    if a.estimated {
        return None;
    }
    let resets_in = until(a.resets_at, now);
    Some(Window {
        percent: a.percent,
        resets_in,
        eta: eta(Some(a), now),
        elapsed: elapsed(resets_in, length),
        source: a.source.parse().unwrap_or(Source::Api),
        observed_at: a.observed_at,
    })
}

fn window(
    measured: Option<&RateWindow>,
    app: Option<&LimitInfo>,
    absent: bool,
    length: i64,
    now: Timestamp,
) -> Option<Window> {
    let measured = measured.and_then(|m| Some((m.percent()?, m.resets_at())));
    // The payload's own reading wins even over "absent": the app flips back on this push.
    let w = match measured {
        Some((percent, resets_at)) => Window {
            percent,
            resets_in: until(resets_at, now),
            eta: eta(app, now),
            elapsed: elapsed(until(resets_at, now), length),
            source: Source::Statusline,
            observed_at: now,
        },
        None if absent => return None,
        None => from_app(app?, length, now)?,
    };
    Some(settle(w, now))
}

/// A passed reset means the window started over: an estimated 0 % (the engine's rule).
fn settle(w: Window, now: Timestamp) -> Window {
    if w.resets_in.is_some_and(|ms| ms <= 0) {
        Window {
            percent: 0.0,
            resets_in: None,
            eta: None,
            elapsed: None,
            source: Source::Estimate,
            observed_at: now,
        }
    } else {
        w
    }
}

/// The app's scoped weekly limits that are measurements, in its order.
fn scoped(limits: &[LimitInfo], now: Timestamp) -> Vec<ScopedWindow> {
    limits
        .iter()
        .filter(|l| l.kind == "weekly_scoped")
        .filter_map(|l| {
            let label = scope_label(l.scope.as_deref()?);
            let window = settle(from_app(l, WEEK_MS, now)?, now);
            Some(ScopedWindow { label, window })
        })
        .collect()
}

/// `model:Fable` → `Fable`; a scope this build does not know is shown as it is.
fn scope_label(scope: &str) -> String {
    match scope.parse::<Scope>() {
        Ok(Scope::Model(name) | Scope::Surface(name)) => name,
        Err(_) => scope.to_owned(),
    }
}

#[cfg(test)]
mod tests;
