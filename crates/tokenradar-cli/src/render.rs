//! The statusline line: `◔ 5h 72% · 1h48m ▲38m │ 7d 34% │ ≈9% this session`.
//!
//! The payload Claude Code just sent is the freshest measurement, so its percentages and reset
//! times win. The app adds what only it knows: the projection (`▲`) and the session's share.
//! Windows the payload lacks fall back to the app's values.

use tokenradar_core::domain::Timestamp;
use tokenradar_core::ipc::{LimitInfo, StatusReply};
use tokenradar_core::limits::Severity;
use tokenradar_core::statusline::StatuslinePayload;
use tokenradar_core::statusline::payload::RateWindow;
use tokenradar_i18n::{Arg, Localizer};

/// One limit window as the line shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub percent: f64,
    /// Until the reset, ms.
    pub resets_in: Option<i64>,
    /// Until 100 % at the current rate, ms; only when it comes before the reset.
    pub eta: Option<i64>,
    /// Not a measurement (a passed reset, or the app's estimate).
    pub estimated: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Line {
    pub five_hour: Option<Window>,
    pub seven_day: Option<Window>,
    /// This session's share of the 5-hour window, %.
    pub session_share: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub color: bool,
    /// Terminal columns; `None` is unlimited.
    pub width: Option<usize>,
}

impl Style {
    /// ANSI colors unless `NO_COLOR` is set; the width from `COLUMNS`.
    pub fn from_env() -> Self {
        Self {
            color: std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty()),
            width: std::env::var("COLUMNS")
                .ok()
                .and_then(|c| c.trim().parse().ok())
                .filter(|w| *w > 0),
        }
    }
}

impl Line {
    pub fn from_sources(
        payload: Option<&StatuslinePayload>,
        reply: Option<&StatusReply>,
        now: Timestamp,
    ) -> Self {
        let app = |kind: &str| reply.and_then(|r| r.limits.iter().find(|l| l.kind == kind));
        Self {
            five_hour: window(
                payload.and_then(StatuslinePayload::five_hour),
                app("session"),
                now,
            ),
            seven_day: window(
                payload.and_then(StatuslinePayload::seven_day),
                app("weekly_all"),
                now,
            ),
            session_share: reply
                .and_then(|r| r.session.as_ref())
                .and_then(|s| s.share_percent),
        }
    }
}

fn until(at: Option<Timestamp>, now: Timestamp) -> Option<i64> {
    at.map(|t| t.as_millis() - now.as_millis())
}

fn window(
    measured: Option<&RateWindow>,
    app: Option<&LimitInfo>,
    now: Timestamp,
) -> Option<Window> {
    let eta = app
        .filter(|a| a.will_hit_before_reset)
        .and_then(|a| until(a.eta_100_at, now))
        .filter(|ms| *ms > 0);
    let w = if let Some(percent) = measured.and_then(RateWindow::percent) {
        Window {
            percent,
            resets_in: until(measured.and_then(RateWindow::resets_at), now),
            eta,
            estimated: false,
        }
    } else {
        let a = app?;
        Window {
            percent: a.percent,
            resets_in: until(a.resets_at, now),
            eta,
            estimated: a.estimated,
        }
    };
    // A passed reset means the window started over: an estimated 0 % (the engine's rule).
    if w.resets_in.is_some_and(|ms| ms <= 0) {
        return Some(Window {
            percent: 0.0,
            resets_in: None,
            eta: None,
            estimated: true,
        });
    }
    Some(w)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Paint {
    Plain,
    Dim,
    Level(Severity),
}

struct Seg {
    text: String,
    paint: Paint,
}

fn seg(text: impl Into<String>, paint: Paint) -> Seg {
    Seg {
        text: text.into(),
        paint,
    }
}

/// How much of the line to show; the first that fits the width wins.
#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // one switch per part of the line
struct Detail {
    share: bool,
    eta: bool,
    resets: bool,
    seven_day: bool,
}

const DETAILS: [Detail; 5] = [
    Detail {
        share: true,
        eta: true,
        resets: true,
        seven_day: true,
    },
    Detail {
        share: false,
        eta: true,
        resets: true,
        seven_day: true,
    },
    Detail {
        share: false,
        eta: false,
        resets: true,
        seven_day: true,
    },
    Detail {
        share: false,
        eta: false,
        resets: false,
        seven_day: true,
    },
    Detail {
        share: false,
        eta: false,
        resets: false,
        seven_day: false,
    },
];

pub fn render(l: &Localizer, line: &Line, style: Style) -> String {
    let mut chosen = None;
    for d in DETAILS {
        let segs = segments(l, line, d);
        let width: usize = segs.iter().map(|s| s.text.chars().count()).sum();
        let fits = style.width.is_none_or(|w| width <= w);
        chosen = Some(segs);
        if fits {
            break;
        }
    }
    paint(&chosen.unwrap_or_default(), style.color)
}

fn segments(l: &Localizer, line: &Line, d: Detail) -> Vec<Seg> {
    let mut out = Vec::new();
    let Some(lead) = line.five_hour.as_ref().or(line.seven_day.as_ref()) else {
        out.push(seg(l.msg("sl-no-limits"), Paint::Dim));
        return out;
    };
    out.push(seg(pie(lead.percent), level(lead)));
    out.push(seg(" ", Paint::Plain));
    let sep = || seg(" │ ", Paint::Dim);
    let mut first = true;
    if let Some(w) = &line.five_hour {
        push_window(&mut out, l, "sl-label-five-hour", w, d);
        first = false;
    }
    if let Some(w) = line.seven_day.as_ref().filter(|_| d.seven_day || first) {
        if !first {
            out.push(sep());
        }
        push_window(&mut out, l, "sl-label-seven-day", w, d);
    }
    if let Some(share) = line.session_share.filter(|_| d.share) {
        out.push(sep());
        out.push(seg(
            l.fmt("sl-session-share", &[("pct", Arg::Num(share))]),
            Paint::Plain,
        ));
    }
    out
}

fn push_window(out: &mut Vec<Seg>, l: &Localizer, label: &str, w: &Window, d: Detail) {
    let text = l.fmt(
        "sl-window",
        &[
            ("label", Arg::Str(&l.msg(label))),
            ("pct", Arg::Num(w.percent)),
        ],
    );
    let text = if w.estimated {
        format!("≈{text}")
    } else {
        text
    };
    out.push(seg(text, level(w)));
    if let Some(ms) = w.resets_in.filter(|_| d.resets) {
        out.push(seg(" · ", Paint::Dim));
        out.push(seg(
            l.fmt("sl-resets-in", &[("ms", ms_arg(ms))]),
            Paint::Plain,
        ));
    }
    if let Some(ms) = w.eta.filter(|_| d.eta) {
        out.push(seg(" ", Paint::Plain));
        out.push(seg(
            l.fmt("sl-eta", &[("ms", ms_arg(ms))]),
            Paint::Level(Severity::Warn),
        ));
    }
}

#[allow(clippy::cast_precision_loss)] // durations are far below 2^52 ms
fn ms_arg(ms: i64) -> Arg<'static> {
    Arg::Num(ms as f64)
}

fn level(w: &Window) -> Paint {
    if w.estimated {
        Paint::Dim
    } else {
        Paint::Level(Severity::of(w.percent))
    }
}

/// A pie that fills in quarters.
fn pie(percent: f64) -> &'static str {
    match percent {
        p if p < 12.5 => "○",
        p if p < 37.5 => "◔",
        p if p < 62.5 => "◑",
        p if p < 87.5 => "◕",
        _ => "●",
    }
}

fn paint(segs: &[Seg], color: bool) -> String {
    let mut out = String::new();
    for s in segs {
        let code = match s.paint {
            Paint::Plain => None,
            Paint::Dim => Some("2"),
            Paint::Level(Severity::Ok) => Some("36"),
            Paint::Level(Severity::Warn) => Some("33"),
            Paint::Level(Severity::Crit) => Some("31"),
        };
        match code.filter(|_| color) {
            Some(code) => {
                out.push_str("\x1b[");
                out.push_str(code);
                out.push('m');
                out.push_str(&s.text);
                out.push_str("\x1b[0m");
            }
            None => out.push_str(&s.text),
        }
    }
    out
}

#[cfg(test)]
mod tests;
