//! Drawing a [`Line`]: segments → painted spans → ANSI text.
//!
//! Each segment is drawn on its own. When the terminal is narrow the line gives up detail in
//! this order, until it fits:
//!
//! 1. session segments (context, share, cost, model), from the end of the user's list;
//! 2. the bar;
//! 3. the projection (`▲38m`);
//! 4. the reset countdowns;
//! 5. limit segments from the end, keeping the first.
//!
//! The double layout fits each of its two lines on its own.

use serde::Serialize;
use specta::Type;
use tokenradar_i18n::{Arg, Localizer};

use super::{Layout, Line, Reading, SegmentKind, StatuslinePrefs, Window};
use crate::limits::Severity;

/// Cells in the bar; each stands for 12.5 %.
const BAR_CELLS: usize = 8;
const BAR_FULL: char = '█';
const BAR_EMPTY: char = '░';

/// How a span is drawn. The CLI maps it to ANSI codes; the settings preview to theme colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Paint {
    Plain,
    /// Separators, estimates and stale values.
    Dim,
    Ok,
    Warn,
    Crit,
}

impl Paint {
    fn of(severity: Severity) -> Self {
        match severity {
            Severity::Ok => Self::Ok,
            Severity::Warn => Self::Warn,
            Severity::Crit => Self::Crit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct Span {
    pub text: String,
    pub paint: Paint,
}

fn span(text: impl Into<String>, paint: Paint) -> Span {
    Span {
        text: text.into(),
        paint,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub color: bool,
    /// Terminal columns; `None` is unlimited.
    pub width: Option<usize>,
}

/// What a segment may show; reduced step by step on a narrow terminal.
#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // one switch per part of a segment
struct Detail {
    bar: bool,
    eta: bool,
    resets: bool,
}

/// The line as text: ANSI colors if `style.color`, lines joined by `\n`.
pub fn render(l: &Localizer, line: &Line, prefs: &StatuslinePrefs, style: Style) -> String {
    to_ansi(&spans(l, line, prefs, style), style.color)
}

/// The line as painted spans, one list per output line. Never empty.
pub fn spans(l: &Localizer, line: &Line, prefs: &StatuslinePrefs, style: Style) -> Vec<Vec<Span>> {
    // The bar needs color to read: without it, it is noise.
    let bar = prefs.bar && style.color;
    let groups: Vec<Vec<SegmentKind>> = match prefs.layout {
        Layout::Single => vec![prefs.segments.clone()],
        Layout::Double => {
            let (limits, session) = prefs.segments.iter().partition(|k| k.is_limit());
            vec![limits, session]
        }
    };
    let lines: Vec<Vec<Span>> = groups
        .iter()
        .map(|kinds| fit(l, line, kinds, bar, style.width))
        .filter(|spans| !spans.is_empty())
        .collect();
    if lines.is_empty() {
        vec![vec![span(l.msg("sl-no-limits"), Paint::Dim)]]
    } else {
        lines
    }
}

/// The richest form of `kinds` that fits `width`.
fn fit(
    l: &Localizer,
    line: &Line,
    kinds: &[SegmentKind],
    bar: bool,
    width: Option<usize>,
) -> Vec<Span> {
    let mut detail = Detail {
        bar,
        eta: true,
        resets: true,
    };
    // Only segments that have something to show take part.
    let mut kinds: Vec<SegmentKind> = kinds
        .iter()
        .copied()
        .filter(|k| segment(l, line, *k, detail).is_some())
        .collect();
    loop {
        let out = compose(l, line, &kinds, detail);
        let len: usize = out.iter().map(|s| s.text.chars().count()).sum();
        if width.is_none_or(|w| len <= w) {
            return out;
        }
        if kinds.len() > 1
            && let Some(i) = kinds.iter().rposition(|k| !k.is_limit())
        {
            kinds.remove(i);
        } else if detail.bar {
            detail.bar = false;
        } else if detail.eta {
            detail.eta = false;
        } else if detail.resets {
            detail.resets = false;
        } else if kinds.len() > 1 {
            kinds.pop();
        } else {
            return out;
        }
    }
}

fn compose(l: &Localizer, line: &Line, kinds: &[SegmentKind], d: Detail) -> Vec<Span> {
    let mut out = Vec::new();
    for spans in kinds.iter().filter_map(|k| segment(l, line, *k, d)) {
        if !out.is_empty() {
            out.push(sep());
        }
        out.extend(spans);
    }
    out
}

fn sep() -> Span {
    span(" │ ", Paint::Dim)
}

fn segment(l: &Localizer, line: &Line, kind: SegmentKind, d: Detail) -> Option<Vec<Span>> {
    match kind {
        SegmentKind::FiveHour => line
            .five_hour
            .as_ref()
            .map(|w| window(l, line, &l.msg("sl-label-five-hour"), w, d)),
        SegmentKind::SevenDay => line
            .seven_day
            .as_ref()
            .map(|w| window(l, line, &l.msg("sl-label-seven-day"), w, d)),
        SegmentKind::Scoped => {
            let mut out = Vec::new();
            for s in &line.scoped {
                if !out.is_empty() {
                    out.push(sep());
                }
                // Compact: the label and the percentage, no bar, no countdown.
                let compact = Detail {
                    bar: false,
                    eta: false,
                    resets: false,
                };
                out.extend(window(l, line, &s.label, &s.window, compact));
            }
            (!out.is_empty()).then_some(out)
        }
        SegmentKind::Context => line.context.as_ref().map(|r| {
            let paint = value_paint(line, r, Paint::of(Severity::of(r.value)));
            vec![span(
                l.fmt("sl-context", &[("pct", Arg::Num(r.value))]),
                paint,
            )]
        }),
        SegmentKind::SessionShare => line.session_share.as_ref().map(|r| {
            // Always an estimate: the message carries the `≈`.
            vec![span(
                l.fmt("sl-session-share", &[("pct", Arg::Num(r.value))]),
                value_paint(line, r, Paint::Plain),
            )]
        }),
        SegmentKind::Cost => line.cost.as_ref().map(|r| {
            vec![span(
                l.fmt("sl-cost", &[("usd", Arg::Num(r.value))]),
                value_paint(line, r, Paint::Plain),
            )]
        }),
        SegmentKind::Model => line
            .model
            .as_ref()
            .map(|r| vec![span(r.value.clone(), value_paint(line, r, Paint::Plain))]),
    }
}

/// `paint`, or dim when the value is stale.
fn value_paint<T>(line: &Line, r: &Reading<T>, paint: Paint) -> Paint {
    if line.is_stale(r.observed_at) {
        Paint::Dim
    } else {
        paint
    }
}

/// `7d ████░░░░ 41% · 3d4h ▲38m`.
fn window(l: &Localizer, line: &Line, label: &str, w: &Window, d: Detail) -> Vec<Span> {
    let dim = w.estimated() || line.is_stale(w.observed_at);
    let paint = if dim {
        Paint::Dim
    } else {
        Paint::of(Severity::of(w.percent))
    };
    let mut out = Vec::new();
    let mark = if w.estimated() { "≈" } else { "" };
    out.push(span(format!("{mark}{label} "), paint));
    if d.bar {
        let full = bar_cells(w.percent);
        out.push(span(BAR_FULL.to_string().repeat(full), paint));
        out.push(span(
            BAR_EMPTY.to_string().repeat(BAR_CELLS - full),
            Paint::Dim,
        ));
        out.push(span(" ", Paint::Plain));
    }
    out.push(span(
        l.fmt("sl-percent", &[("pct", Arg::Num(w.percent))]),
        paint,
    ));
    if let Some(ms) = w.resets_in.filter(|_| d.resets) {
        out.push(span(" · ", Paint::Dim));
        out.push(span(
            l.fmt("sl-resets-in", &[("ms", ms_arg(ms))]),
            if dim { Paint::Dim } else { Paint::Plain },
        ));
    }
    if let Some(ms) = w.eta.filter(|_| d.eta) {
        out.push(span(" ", Paint::Plain));
        out.push(span(
            l.fmt("sl-eta", &[("ms", ms_arg(ms))]),
            if dim { Paint::Dim } else { Paint::Warn },
        ));
    }
    out
}

/// Filled cells: any use shows at least one, only 100 % fills all.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub(super) fn bar_cells(percent: f64) -> usize {
    let cells = (percent.clamp(0.0, 100.0) / 100.0 * BAR_CELLS as f64).ceil();
    (cells as usize).min(BAR_CELLS)
}

#[allow(clippy::cast_precision_loss)] // durations are far below 2^52 ms
fn ms_arg(ms: i64) -> Arg<'static> {
    Arg::Num(ms as f64)
}

/// Spans as terminal text; `color` false leaves the escape codes out.
pub fn to_ansi(lines: &[Vec<Span>], color: bool) -> String {
    let mut out = String::new();
    for (i, spans) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        // An empty span (the bar's empty part at 100 %) would only add escape codes.
        for s in spans.iter().filter(|s| !s.text.is_empty()) {
            let code = match s.paint {
                Paint::Plain => None,
                Paint::Dim => Some("2"),
                Paint::Ok => Some("36"),
                Paint::Warn => Some("33"),
                Paint::Crit => Some("31"),
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
    }
    out
}
