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

use serde::{Deserialize, Serialize};
use specta::Type;

use super::auto_compact::AutoCompact;
use super::payload::{RateWindow, StatuslinePayload};
use crate::domain::{Scope, Source, Timestamp};
use crate::ipc::{CompactCount, LimitInfo, StatusReply};
use crate::limits::{FRESH_MS, SESSION_WINDOW_MS, Severity, WEEK_MS};

mod prefs;
mod presets;
mod render;

pub use prefs::{
    AnsiColor, BAR_WIDTHS, BarStyle, ColorMode, DEFAULT_SEGMENTS, DEFAULT_SEPARATOR, Format, Group,
    Imported, Item, Layout, MAX_ITEMS, MAX_LABEL, MAX_MARGIN, MAX_SEPARATOR, MAX_SPACE, MAX_TEXT,
    PREFS_VERSION, ResetFormat, SegmentKind, StatuslinePrefs, Theme, clean,
};
pub use presets::Preset;
pub use render::{
    CLAUDE_CODE_RESERVED, Paint, Span, Style, bar_sample, default_label, display_width, render,
    spans, to_ansi,
};

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
    /// The rest of the payload's metadata, for the items added in Step 12i. All of it comes
    /// from the payload Claude Code just sent, so it is always current.
    pub facts: Facts,
    /// Where the local time of a reset comes from (`reset_format = "clock"`).
    pub zone: Zone,
    /// The session's compactions, from the app (Step 12n); `None` when unknown.
    pub compacts: Option<CompactCount>,
    /// Claude Code's auto-compact setting, read by the caller from the settings files (I/O);
    /// `None` when it is on with no window set.
    pub auto_compact: Option<AutoCompact>,
}

/// The time zone a reset's wall-clock time is shown in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Zone {
    /// The system's, with its daylight saving rules for the reset's date. Only on Windows for
    /// now; elsewhere the reset stays a countdown rather than show UTC as local time.
    #[default]
    System,
    /// A fixed offset east of UTC, in minutes: for tests.
    Fixed(i32),
}

/// A wall-clock time; `weekday` 0 is Monday.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    pub weekday: u8,
    pub hour: u8,
    pub minute: u8,
}

/// The weekly window's room left per day (`daily_budget`): it keeps the window's source.
#[derive(Debug, Clone, PartialEq)]
pub struct Budget {
    /// Percentage points per day.
    pub per_day: f64,
    /// Critical with nothing left; past the window's thresholds, or running ahead of the time
    /// line (ADR-044, ADR-045), what the window has.
    pub severity: Severity,
    pub source: Source,
    pub observed_at: Timestamp,
}

/// A burn rate is noise in a session's first minutes.
pub const BURN_RATE_MIN_MS: u64 = 5 * 60_000;
const DAY_MS: i64 = 24 * 60 * 60_000;

/// Session and project metadata from the payload.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts {
    /// `model.id`; [`Line::model`] holds the display name when there is one.
    pub model_id: Option<String>,
    /// Tokens in the context window now, and its size.
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
    pub context_size: Option<u64>,
    pub duration_ms: Option<u64>,
    pub api_duration_ms: Option<u64>,
    /// Lines added and removed.
    pub lines: Option<(u64, u64)>,
    pub vim_mode: Option<String>,
    pub version: Option<String>,
    pub agent_name: Option<String>,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub project_dir: Option<String>,
    /// The worktree branch from the payload, else filled by the caller (reading `.git` is
    /// I/O); see [`super::git::branch`].
    pub git_branch: Option<String>,
    /// The home folder, for paths shown with `~`.
    pub home: Option<String>,
    /// Step 12m: `session_name` and `output_style.name`.
    pub session_name: Option<String>,
    pub output_style: Option<String>,
    /// The share of the last request's input read from the cache, 0–100 %.
    pub cache_ratio: Option<f64>,
    /// The last request's input tokens with the cache: what the context window holds now.
    pub context_used: Option<u64>,
}

/// The state the settings preview draws its sample in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SampleState {
    #[default]
    Normal,
    Warn,
    Crit,
    /// The app's values are older than [`FRESH_MS`].
    Stale,
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
            facts: Facts::default(),
            zone: Zone::System,
            compacts: None,
            auto_compact: None,
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
            facts: payload.map(facts).unwrap_or_default(),
            zone: Zone::System,
            compacts: reply.and_then(|r| r.session.as_ref()?.compacts),
            auto_compact: None,
        }
    }

    /// The weekly window's room left, shared out over the days to its reset: at least one day,
    /// a started day counting whole. `None` without a weekly window or its reset time.
    pub fn daily_budget(&self) -> Option<Budget> {
        let w = self.seven_day.as_ref()?;
        let resets_in = w.resets_in.filter(|ms| *ms > 0)?;
        let left = (100.0 - w.percent).max(0.0);
        let days = (resets_in + DAY_MS - 1) / DAY_MS;
        #[allow(clippy::cast_precision_loss)] // at most a few days
        let per_day = left / days.max(1) as f64;
        let ahead = w
            .elapsed
            .is_some_and(|e| crate::limits::pace::warns(w.percent, e));
        let severity = if left <= 0.0 {
            Severity::Crit
        } else {
            match Severity::of(w.percent) {
                Severity::Ok if ahead => Severity::Warn,
                s => s,
            }
        };
        Some(Budget {
            per_day,
            severity,
            source: w.source,
            observed_at: w.observed_at,
        })
    }

    /// The session's API-priced cost per hour: two measurements, so not an estimate. `None`
    /// in the first [`BURN_RATE_MIN_MS`].
    pub fn burn_rate(&self) -> Option<Reading<f64>> {
        let cost = self.cost.as_ref()?;
        let ms = self
            .facts
            .duration_ms
            .filter(|ms| *ms >= BURN_RATE_MIN_MS)?;
        #[allow(clippy::cast_precision_loss)] // durations are far below 2^52 ms
        let hours = ms as f64 / 3_600_000.0;
        Some(Reading {
            value: cost.value / hours,
            source: cost.source,
            observed_at: cost.observed_at,
        })
    }

    /// The tokens the context window has left: from the last request's counts, else from the
    /// used percentage.
    pub fn tokens_left(&self) -> Option<u64> {
        let size = self.facts.context_size?;
        if let Some(used) = self.facts.context_used {
            return Some(size.saturating_sub(used));
        }
        let used = self.context.as_ref()?.value.clamp(0.0, 100.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        Some((size as f64 * (100.0 - used) / 100.0).round() as u64)
    }

    /// The local time `resets_in` ms from now; `None` where the zone is unknown.
    pub fn reset_clock(&self, resets_in: i64) -> Option<Clock> {
        let at = self.at.as_millis().checked_add(resets_in)?;
        match self.zone {
            Zone::Fixed(minutes) => utc_clock(at + i64::from(minutes) * 60_000),
            Zone::System => system_clock(at),
        }
    }

    /// Whether a value observed at `observed_at` is too old to be drawn as current.
    pub fn is_stale(&self, observed_at: Timestamp) -> bool {
        self.at.as_millis() - observed_at.as_millis() >= FRESH_MS
    }

    /// A sample for the settings preview: every segment filled with typical values. The
    /// 5-hour window is left out when `five_hour` is false (an account without it).
    pub fn sample(at: Timestamp, five_hour: bool) -> Self {
        Self::sample_in(at, five_hour, SampleState::Normal)
    }

    /// [`Line::sample`] in a given state: past the warning or the critical threshold, or
    /// with the app's values gone stale.
    pub fn sample_in(at: Timestamp, five_hour: bool, state: SampleState) -> Self {
        const MIN: i64 = 60_000;
        let (five, seven, context) = match state {
            SampleState::Normal | SampleState::Stale => (72.0, 41.0, 38.0),
            SampleState::Warn => (84.0, 78.0, 81.0),
            SampleState::Crit => (96.0, 93.0, 92.0),
        };
        let app_at = match state {
            SampleState::Stale => Timestamp::from_millis(at.as_millis() - 15 * MIN),
            _ => at,
        };
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
                ..w(five, 108 * MIN, SESSION_WINDOW_MS)
            }),
            seven_day: Some(Window {
                // Stale: the payload had no weekly reading, the app's API value stands in.
                source: if state == SampleState::Stale {
                    Source::Api
                } else {
                    Source::Statusline
                },
                observed_at: app_at,
                ..w(seven, (3 * 24 + 4) * 60 * MIN, WEEK_MS)
            }),
            scoped: vec![ScopedWindow {
                label: "Fable".into(),
                window: Window {
                    source: Source::Api,
                    observed_at: app_at,
                    ..w(22.0, (3 * 24 + 4) * 60 * MIN, WEEK_MS)
                },
            }],
            context: Some(Reading::now(context, Source::Statusline, at)),
            session_share: Some(Reading {
                value: 2.1,
                source: Source::Estimate,
                observed_at: app_at,
            }),
            cost: Some(Reading::now(4.12, Source::Statusline, at)),
            model: Some(Reading::now("Opus 5.5".into(), Source::Statusline, at)),
            facts: Facts {
                model_id: Some("claude-opus-5-5".into()),
                tokens_in: Some(76_000),
                tokens_out: Some(1_200),
                context_size: Some(200_000),
                duration_ms: Some(1_234_567),
                api_duration_ms: Some(345_678),
                lines: Some((156, 23)),
                vim_mode: Some("NORMAL".into()),
                version: Some("2.1.310".into()),
                agent_name: Some("reviewer".into()),
                session_id: Some("a813d09f-4c1e-4b7a-9d2f-6e5b0c3a1f88".into()),
                cwd: Some("/home/ada/projects/alpha/src".into()),
                project_dir: Some("/home/ada/projects/alpha".into()),
                git_branch: Some("main".into()),
                home: Some("/home/ada".into()),
                session_name: Some("refactor parser".into()),
                output_style: Some("Explanatory".into()),
                cache_ratio: Some(88.8),
                context_used: Some(76_000),
            },
            zone: Zone::System,
            compacts: Some(CompactCount {
                auto: 2,
                manual: 1,
                other: 0,
            }),
            auto_compact: Some(AutoCompact::Window(300_000)),
        }
    }
}

/// The payload's metadata for [`Facts`]; numbers that are not usable counts are left out.
fn facts(p: &StatuslinePayload) -> Facts {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = |v: Option<f64>| v.filter(|v| v.is_finite() && *v >= 0.0).map(|v| v as u64);
    let text = |v: Option<&String>| Some(clean(v?.trim(), 256)).filter(|s| !s.is_empty());
    let cost = p.cost.as_ref();
    let ctx = p.context_window.as_ref();
    let workspace = p.workspace.as_ref();
    // The last request's input: fresh, written to the cache, read from it.
    let usage = ctx.and_then(|c| c.current_usage.as_ref()).and_then(|u| {
        let (fresh, written, read) = (
            count(u.input_tokens),
            count(u.cache_creation_input_tokens),
            count(u.cache_read_input_tokens),
        );
        if fresh.is_none() && written.is_none() && read.is_none() {
            return None;
        }
        Some((fresh.unwrap_or(0), written.unwrap_or(0), read.unwrap_or(0)))
    });
    Facts {
        model_id: text(p.model.as_ref().and_then(|m| m.id.as_ref())),
        tokens_in: count(ctx.and_then(|c| c.total_input_tokens)),
        tokens_out: count(ctx.and_then(|c| c.total_output_tokens)),
        context_size: count(ctx.and_then(|c| c.context_window_size)).filter(|n| *n > 0),
        duration_ms: count(cost.and_then(|c| c.total_duration_ms)),
        api_duration_ms: count(cost.and_then(|c| c.total_api_duration_ms)),
        lines: match (
            count(cost.and_then(|c| c.total_lines_added)),
            count(cost.and_then(|c| c.total_lines_removed)),
        ) {
            (None, None) => None,
            (added, removed) => Some((added.unwrap_or(0), removed.unwrap_or(0))),
        },
        vim_mode: text(p.vim.as_ref().and_then(|v| v.mode.as_ref())),
        version: text(p.version.as_ref()),
        agent_name: text(p.agent.as_ref().and_then(|a| a.name.as_ref())),
        session_id: text(p.session_id.as_ref()),
        cwd: text(
            workspace
                .and_then(|w| w.current_dir.as_ref())
                .or(p.cwd.as_ref()),
        ),
        project_dir: text(workspace.and_then(|w| w.project_dir.as_ref())),
        git_branch: text(p.worktree.as_ref().and_then(|w| w.branch.as_ref())),
        home: std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .filter(|h| !h.is_empty()),
        session_name: text(p.session_name.as_ref()),
        output_style: text(p.output_style.as_ref().and_then(|o| o.name.as_ref())),
        cache_ratio: usage.and_then(|(fresh, written, read)| {
            let all = fresh + written + read;
            #[allow(clippy::cast_precision_loss)] // token counts are far below 2^52
            (all > 0).then(|| read as f64 / all as f64 * 100.0)
        }),
        context_used: usage.map(|(fresh, written, read)| fresh + written + read),
    }
}

/// The wall-clock time of `ms` since the epoch, read as UTC.
fn utc_clock(ms: i64) -> Option<Clock> {
    let dt = jiff::Timestamp::from_millisecond(ms)
        .ok()?
        .to_zoned(jiff::tz::TimeZone::UTC)
        .datetime();
    Some(Clock {
        weekday: u8::try_from(dt.weekday().to_monday_zero_offset()).ok()?,
        hour: u8::try_from(dt.hour()).ok()?,
        minute: u8::try_from(dt.minute()).ok()?,
    })
}

/// The local time of `ms` since the epoch, with the zone's rules for that date.
#[cfg(windows)]
#[allow(unsafe_code)]
fn system_clock(ms: i64) -> Option<Clock> {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::Time::SystemTimeToTzSpecificLocalTime;
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
        wSecond: part(utc.second())?,
        wMilliseconds: 0,
    };
    let mut local = st;
    // SAFETY: a null zone is the current one; both pointers are to valid SYSTEMTIMEs.
    let ok =
        unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &raw const st, &raw mut local) };
    if ok == 0 {
        return None;
    }
    let date = jiff::civil::Date::new(
        i16::try_from(local.wYear).ok()?,
        i8::try_from(local.wMonth).ok()?,
        i8::try_from(local.wDay).ok()?,
    )
    .ok()?;
    Some(Clock {
        weekday: u8::try_from(date.weekday().to_monday_zero_offset()).ok()?,
        hour: u8::try_from(local.wHour).ok()?,
        minute: u8::try_from(local.wMinute).ok()?,
    })
}

#[cfg(not(windows))]
fn system_clock(_ms: i64) -> Option<Clock> {
    None
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
