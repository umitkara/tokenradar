//! Drawing a [`Line`]: items → painted spans → ANSI text.
//!
//! Each item is drawn on its own; `line_break` items split the output into lines, and each
//! line is fitted to the terminal on its own. When a line is too wide it gives up detail in
//! this order, until it fits:
//!
//! 1. session and project items, from the end of the user's list (with the separators and
//!    texts right before them);
//! 2. the bar (with its time mark);
//! 3. the elapsed time (`⏱43%`);
//! 4. the projection (`▲38m`);
//! 5. the reset countdowns;
//! 6. limit items from the end, keeping the first; then the remaining texts.
//!
//! Colors (ADR-049): an item's own color replaces the automatic one, but on limits and the
//! context window only below the warning threshold: the warning and critical colors, and the
//! dim drawing of estimates and stale values, always win. Without color the Powerline themes
//! fall back to the plain one and the bar is left out. The Powerline blocks leave the bar out
//! too: drawn in the block's text color, its full and empty cells look alike.

use serde::Serialize;
use specta::Type;
use tokenradar_i18n::{Arg, Localizer};

use super::{AnsiColor, Format, Item, Line, Reading, SegmentKind, StatuslinePrefs, Theme, Window};
use crate::limits::Severity;

/// Cells in the bar; each stands for 12.5 %.
const BAR_CELLS: usize = 8;
const BAR_FULL: char = '█';
const BAR_EMPTY: char = '░';
/// Where the window's time stands, between two cells (ADR-044).
const BAR_MARK: char = '│';

/// How a span is drawn. The CLI maps it to ANSI codes; the settings preview to colors.
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
    /// A color of the user's (or a Powerline block's text); replaces the paint's color.
    pub fg: Option<AnsiColor>,
    /// A Powerline block.
    pub bg: Option<AnsiColor>,
    /// The index of the item in `StatuslinePrefs::items` the span belongs to; `None` for
    /// separators and joints between items.
    pub item: Option<u16>,
}

fn span(text: impl Into<String>, paint: Paint) -> Span {
    Span {
        text: text.into(),
        paint,
        fg: None,
        bg: None,
        item: None,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub color: bool,
    /// Terminal columns; `None` is unlimited.
    pub width: Option<usize>,
}

/// What an item may show; reduced step by step on a narrow terminal.
#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // one switch per part of a segment
struct Detail {
    bar: bool,
    pace: bool,
    eta: bool,
    resets: bool,
}

const COMPACT: Detail = Detail {
    bar: false,
    pace: false,
    eta: false,
    resets: false,
};

struct Ctx<'a> {
    l: &'a Localizer,
    line: &'a Line,
    items: &'a [Item],
    separator: &'a str,
    color: bool,
    theme: Theme,
}

impl Ctx<'_> {
    fn kind(&self, i: usize) -> SegmentKind {
        self.items[i].kind
    }

    /// Whether bars are drawn: only in color, and only on the plain theme.
    fn bars(&self) -> bool {
        self.color && self.theme == Theme::Plain
    }
}

/// The line as text: ANSI colors if `style.color`, lines joined by `\n`.
pub fn render(l: &Localizer, line: &Line, prefs: &StatuslinePrefs, style: Style) -> String {
    to_ansi(&spans(l, line, prefs, style), style.color)
}

/// The line as painted spans, one list per output line. Never empty.
pub fn spans(l: &Localizer, line: &Line, prefs: &StatuslinePrefs, style: Style) -> Vec<Vec<Span>> {
    let ctx = Ctx {
        l,
        line,
        items: &prefs.items,
        separator: &prefs.separator,
        color: style.color,
        theme: if style.color {
            prefs.theme
        } else {
            Theme::Plain
        },
    };
    let mut groups: Vec<Vec<usize>> = vec![Vec::new()];
    for (i, item) in prefs.items.iter().enumerate().filter(|(_, i)| i.enabled) {
        match item.kind {
            SegmentKind::LineBreak => groups.push(Vec::new()),
            _ => groups.last_mut().expect("never empty").push(i),
        }
    }
    let lines: Vec<Vec<Span>> = groups
        .iter()
        .map(|items| fit(&ctx, items, style.width))
        .filter(|spans| !spans.is_empty())
        .collect();
    if lines.is_empty() {
        vec![vec![span(l.msg("sl-no-limits"), Paint::Dim)]]
    } else {
        lines
    }
}

/// The richest form of the items at `indices` that fits `width`. Empty when none of them
/// has a value to show.
fn fit(ctx: &Ctx<'_>, indices: &[usize], width: Option<usize>) -> Vec<Span> {
    let mut d = Detail {
        bar: true,
        pace: true,
        eta: true,
        resets: true,
    };
    // Only items that have something to show take part.
    let mut idx: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|&i| piece(ctx, i, d).is_some())
        .collect();
    if idx.iter().all(|&i| ctx.kind(i).is_layout()) {
        return Vec::new();
    }
    loop {
        let out = compose(ctx, &idx, d);
        let len: usize = out.iter().map(|s| s.text.chars().count()).sum();
        if width.is_none_or(|w| len <= w) {
            return out;
        }
        let values = idx.iter().filter(|&&i| !ctx.kind(i).is_layout()).count();
        if values > 1
            && let Some(p) = idx.iter().rposition(|&i| {
                let k = ctx.kind(i);
                !k.is_layout() && !k.is_limit()
            })
        {
            drop_at(ctx, &mut idx, p);
        } else if d.bar {
            d.bar = false;
        } else if d.pace {
            d.pace = false;
        } else if d.eta {
            d.eta = false;
        } else if d.resets {
            d.resets = false;
        } else if values > 1
            && let Some(p) = idx.iter().rposition(|&i| !ctx.kind(i).is_layout())
        {
            drop_at(ctx, &mut idx, p);
        } else if let Some(p) = idx.iter().rposition(|&i| ctx.kind(i).is_layout()) {
            idx.remove(p);
        } else {
            return out;
        }
    }
}

/// Removes the value at `p` and the separators and texts right before it.
fn drop_at(ctx: &Ctx<'_>, idx: &mut Vec<usize>, p: usize) {
    idx.remove(p);
    let mut q = p;
    while q > 0 && ctx.kind(idx[q - 1]).is_layout() {
        idx.remove(q - 1);
        q -= 1;
    }
}

fn compose(ctx: &Ctx<'_>, idx: &[usize], d: Detail) -> Vec<Span> {
    match ctx.theme {
        Theme::Plain => compose_plain(ctx, idx, d),
        Theme::Powerline => compose_blocks(ctx, idx, d, '\u{E0B0}', '\u{E0B1}'),
        Theme::PowerlineRound => compose_blocks(ctx, idx, d, '\u{E0B4}', '\u{E0B5}'),
    }
}

fn compose_plain(ctx: &Ctx<'_>, idx: &[usize], d: Detail) -> Vec<Span> {
    let mut out = Vec::new();
    let mut prev: Option<SegmentKind> = None;
    for &i in idx {
        let Some(spans) = piece(ctx, i, d) else {
            continue;
        };
        let item = &ctx.items[i];
        if let Some(p) = prev {
            // The separator goes between two values; next to a text or a separator of the
            // user's, a space.
            out.push(if p.is_layout() || item.kind.is_layout() {
                span(" ", Paint::Plain)
            } else {
                value_sep(ctx)
            });
        }
        out.extend(spans.into_iter().map(|mut s| {
            if let Some(c) = item.color
                && matches!(s.paint, Paint::Ok | Paint::Plain)
                && !s.text.trim().is_empty()
            {
                s.fg = Some(c);
            }
            s.item = Some(tag(i));
            s
        }));
        prev = Some(item.kind);
    }
    out
}

/// Powerline: each item a colored block, joined by `full`; two blocks of the same color by
/// the thin `thin`. Separator items have no place here and are skipped.
fn compose_blocks(ctx: &Ctx<'_>, idx: &[usize], d: Detail, full: char, thin: char) -> Vec<Span> {
    let mut out = Vec::new();
    let mut prev: Option<AnsiColor> = None;
    let block = |text: &str, paint: Paint, fg: AnsiColor, bg: AnsiColor, item: Option<u16>| Span {
        text: text.to_owned(),
        paint,
        fg: Some(fg),
        bg: Some(bg),
        item,
    };
    for &i in idx {
        let item = &ctx.items[i];
        if item.kind == SegmentKind::Separator {
            continue;
        }
        let Some(spans) = piece(ctx, i, d) else {
            continue;
        };
        let bg = block_color(ctx, item);
        let fg = bg.contrast();
        match prev {
            Some(p) if p == bg => out.push(block(&thin.to_string(), Paint::Plain, fg, bg, None)),
            Some(p) => out.push(block(&full.to_string(), Paint::Plain, p, bg, None)),
            None => {}
        }
        out.push(block(" ", Paint::Plain, fg, bg, Some(tag(i))));
        for s in spans {
            // The block's color carries the severity; inside it only dim stays, in a grey
            // that shows (ADR-049: estimates and stale values stay told apart).
            if s.paint == Paint::Dim {
                out.push(block(&s.text, Paint::Dim, bg.muted(), bg, Some(tag(i))));
            } else {
                out.push(block(&s.text, Paint::Plain, fg, bg, Some(tag(i))));
            }
        }
        out.push(block(" ", Paint::Plain, fg, bg, Some(tag(i))));
        prev = Some(bg);
    }
    if let Some(p) = prev {
        out.push(Span {
            text: full.to_string(),
            paint: Paint::Plain,
            fg: Some(p),
            bg: None,
            item: None,
        });
    }
    out
}

#[allow(clippy::cast_possible_truncation)] // at most MAX_ITEMS items
fn tag(i: usize) -> u16 {
    i as u16
}

/// The block's color: red or yellow past the thresholds, else the user's or the kind's.
fn block_color(ctx: &Ctx<'_>, item: &Item) -> AnsiColor {
    match severity(ctx, item.kind) {
        Some(Severity::Crit) => AnsiColor::Red,
        Some(Severity::Warn) => AnsiColor::Yellow,
        _ => item.color.unwrap_or_else(|| item.kind.block_color()),
    }
}

/// The severity of a limit or the context window, unless it is drawn dim.
fn severity(ctx: &Ctx<'_>, kind: SegmentKind) -> Option<Severity> {
    let line = ctx.line;
    let of_window = |w: &Window| {
        (!w.estimated() && !line.is_stale(w.observed_at)).then(|| Severity::of(w.percent))
    };
    match kind {
        SegmentKind::FiveHour => line.five_hour.as_ref().and_then(of_window),
        SegmentKind::SevenDay => line.seven_day.as_ref().and_then(of_window),
        SegmentKind::Scoped => line
            .scoped
            .iter()
            .filter_map(|s| of_window(&s.window))
            .max_by_key(|s| match s {
                Severity::Ok => 0,
                Severity::Warn => 1,
                Severity::Crit => 2,
            }),
        SegmentKind::Context => line
            .context
            .as_ref()
            .filter(|r| !line.is_stale(r.observed_at))
            .map(|r| Severity::of(r.value)),
        _ => None,
    }
}

fn value_sep(ctx: &Ctx<'_>) -> Span {
    if ctx.separator.is_empty() {
        span(" ", Paint::Plain)
    } else {
        span(format!(" {} ", ctx.separator), Paint::Dim)
    }
}

/// The label an item shows when the user gave none; empty for items without one.
pub fn default_label(l: &Localizer, kind: SegmentKind) -> String {
    let key = match kind {
        SegmentKind::FiveHour => "sl-label-five-hour",
        SegmentKind::SevenDay => "sl-label-seven-day",
        SegmentKind::Context => "sl-label-context",
        SegmentKind::TokensIn => "sl-label-tokens-in",
        SegmentKind::TokensOut => "sl-label-tokens-out",
        SegmentKind::Duration => "sl-label-duration",
        SegmentKind::ApiDuration => "sl-label-api-duration",
        SegmentKind::GitBranch => "sl-label-git",
        _ => return String::new(),
    };
    l.msg(key)
}

/// The item's label: the user's, or the default. Only spaces: no label.
fn label(l: &Localizer, item: &Item) -> String {
    if item.label.is_empty() {
        default_label(l, item.kind)
    } else {
        item.label.trim().to_owned()
    }
}

/// `label value`, or the value alone without a label.
fn labeled(label: &str, value: &str) -> String {
    if label.is_empty() {
        value.to_owned()
    } else {
        format!("{label} {value}")
    }
}

/// The spans of item `i`, without its color; `None` when it has nothing to show.
#[allow(clippy::too_many_lines)] // one arm per kind
fn piece(ctx: &Ctx<'_>, i: usize, d: Detail) -> Option<Vec<Span>> {
    let (l, line, item) = (ctx.l, ctx.line, &ctx.items[i]);
    let facts = &line.facts;
    let plain = |text: String| Some(vec![span(text, Paint::Plain)]);
    let lbl = || label(l, item);
    match item.kind {
        SegmentKind::FiveHour | SegmentKind::SevenDay => {
            let w = match item.kind {
                SegmentKind::FiveHour => line.five_hour.as_ref(),
                _ => line.seven_day.as_ref(),
            }?;
            let d = Detail {
                bar: d.bar && ctx.bars() && item.format == Some(Format::Bar),
                pace: d.pace && item.pace,
                eta: d.eta,
                resets: d.resets && item.reset,
            };
            Some(window(l, line, &lbl(), w, d))
        }
        SegmentKind::Scoped => {
            let mut out = Vec::new();
            let custom = item.label.trim();
            if !custom.is_empty() && !line.scoped.is_empty() {
                out.push(span(format!("{custom} "), Paint::Plain));
            }
            for (n, s) in line.scoped.iter().enumerate() {
                if n > 0 {
                    out.push(value_sep(ctx));
                }
                // Compact: the label and the percentage, no bar, no countdown.
                out.extend(window(l, line, &s.label, &s.window, COMPACT));
            }
            (!line.scoped.is_empty()).then_some(out)
        }
        SegmentKind::Context => {
            let r = line.context.as_ref()?;
            let paint = value_paint(line, r, Paint::of(Severity::of(r.value)));
            let label = lbl();
            let pct = l.fmt("sl-percent", &[("pct", Arg::Num(r.value))]);
            Some(match item.format {
                Some(Format::Bar) if d.bar && ctx.bars() => {
                    let mut out = Vec::new();
                    if !label.is_empty() {
                        out.push(span(format!("{label} "), paint));
                    }
                    out.extend(bar(r.value, None, paint));
                    out.push(span(" ", Paint::Plain));
                    out.push(span(pct, paint));
                    out
                }
                Some(Format::Fraction)
                    if facts.tokens_in.is_some() && facts.context_size.is_some() =>
                {
                    let used = count(l, facts.tokens_in.unwrap_or(0), Format::Compact);
                    let size = count(l, facts.context_size.unwrap_or(0), Format::Compact);
                    vec![span(labeled(&label, &format!("{used}/{size}")), paint)]
                }
                _ => vec![span(labeled(&label, &pct), paint)],
            })
        }
        SegmentKind::SessionShare => line.session_share.as_ref().map(|r| {
            // Always an estimate: the message carries the `≈`.
            let text = l.fmt("sl-session-share", &[("pct", Arg::Num(r.value))]);
            vec![span(
                labeled(&lbl(), &text),
                value_paint(line, r, Paint::Plain),
            )]
        }),
        SegmentKind::Cost => line.cost.as_ref().map(|r| {
            // The `API` mark is part of the message and cannot be relabeled (ADR-042).
            let key = match item.format {
                Some(Format::FourDecimals) => "sl-cost-precise",
                _ => "sl-cost",
            };
            let text = l.fmt(key, &[("usd", Arg::Num(r.value))]);
            vec![span(
                labeled(&lbl(), &text),
                value_paint(line, r, Paint::Plain),
            )]
        }),
        SegmentKind::Model => {
            let r = line.model.as_ref()?;
            let name = match item.format {
                Some(Format::Full) => facts.model_id.clone().unwrap_or_else(|| r.value.clone()),
                _ => r.value.clone(),
            };
            Some(vec![span(
                labeled(&lbl(), &name),
                value_paint(line, r, Paint::Plain),
            )])
        }
        SegmentKind::TokensIn | SegmentKind::TokensOut => {
            let n = match item.kind {
                SegmentKind::TokensIn => facts.tokens_in,
                _ => facts.tokens_out,
            }?;
            plain(labeled(
                &lbl(),
                &count(l, n, item.format.unwrap_or(Format::Compact)),
            ))
        }
        SegmentKind::Duration | SegmentKind::ApiDuration => {
            let ms = match item.kind {
                SegmentKind::Duration => facts.duration_ms,
                _ => facts.api_duration_ms,
            }?;
            let text = match item.format {
                Some(Format::Hms) => hms(ms),
                _ => l.fmt("sl-duration", &[("ms", ms_arg(ms))]),
            };
            plain(labeled(&lbl(), &text))
        }
        SegmentKind::LinesChanged => {
            let (added, removed) = facts.lines?;
            let num = |n: u64| count(l, n, Format::Full);
            let text = match item.format {
                Some(Format::Net) if removed > added => format!("-{}", num(removed - added)),
                Some(Format::Net) => format!("+{}", num(added - removed)),
                _ => format!("+{} -{}", num(added), num(removed)),
            };
            plain(labeled(&lbl(), &text))
        }
        SegmentKind::VimMode => plain(labeled(&lbl(), facts.vim_mode.as_ref()?)),
        SegmentKind::Version => plain(labeled(&lbl(), &format!("v{}", facts.version.as_ref()?))),
        SegmentKind::AgentName => plain(labeled(&lbl(), facts.agent_name.as_ref()?)),
        SegmentKind::SessionId => {
            let id = facts.session_id.as_ref()?;
            let id: String = match item.format {
                Some(Format::Full) => id.clone(),
                _ => id.chars().take(8).collect(),
            };
            plain(labeled(&lbl(), &id))
        }
        SegmentKind::Cwd | SegmentKind::ProjectDir => {
            let path = match item.kind {
                SegmentKind::Cwd => facts.cwd.as_ref(),
                _ => facts.project_dir.as_ref(),
            }?;
            let text = match item.format {
                Some(Format::Full) => path.clone(),
                Some(Format::Home) => home_short(path, facts.home.as_deref()),
                _ => basename(path).to_owned(),
            };
            plain(labeled(&lbl(), &text))
        }
        SegmentKind::GitBranch => plain(labeled(&lbl(), facts.git_branch.as_ref()?)),
        SegmentKind::Separator => Some(vec![span(
            item.text.clone(),
            if item.color.is_some() {
                Paint::Plain
            } else {
                Paint::Dim
            },
        )]),
        SegmentKind::CustomText => {
            (!item.text.is_empty()).then(|| vec![span(item.text.clone(), Paint::Plain)])
        }
        SegmentKind::LineBreak => None,
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

/// `7d ███│░░░░░ 41% ⏱43% · 3d4h ▲38m`.
fn window(l: &Localizer, line: &Line, label: &str, w: &Window, d: Detail) -> Vec<Span> {
    let dim = w.estimated() || line.is_stale(w.observed_at);
    let paint = if dim {
        Paint::Dim
    } else {
        Paint::of(Severity::of(w.percent))
    };
    let mut out = Vec::new();
    let mark = if w.estimated() { "≈" } else { "" };
    if !label.is_empty() {
        out.push(span(format!("{mark}{label} "), paint));
    } else if !mark.is_empty() {
        out.push(span(mark, paint));
    }
    let elapsed = w.elapsed.filter(|_| d.pace);
    if d.bar {
        out.extend(bar(w.percent, elapsed, paint));
        out.push(span(" ", Paint::Plain));
    }
    out.push(span(
        l.fmt("sl-percent", &[("pct", Arg::Num(w.percent))]),
        paint,
    ));
    if let Some(pct) = elapsed {
        out.push(span(" ", Paint::Plain));
        out.push(span(
            l.fmt("sl-elapsed", &[("pct", Arg::Num(pct))]),
            if dim { Paint::Dim } else { Paint::Plain },
        ));
    }
    if let Some(ms) = w.resets_in.filter(|_| d.resets) {
        out.push(span(" · ", Paint::Dim));
        out.push(span(
            l.fmt("sl-resets-in", &[("ms", ms_arg_i(ms))]),
            if dim { Paint::Dim } else { Paint::Plain },
        ));
    }
    if let Some(ms) = w.eta.filter(|_| d.eta) {
        out.push(span(" ", Paint::Plain));
        out.push(span(
            l.fmt("sl-eta", &[("ms", ms_arg_i(ms))]),
            if dim { Paint::Dim } else { Paint::Warn },
        ));
    }
    out
}

/// The bar's cells, with the time mark between two of them when `elapsed` is known.
fn bar(percent: f64, elapsed: Option<f64>, paint: Paint) -> Vec<Span> {
    let full = bar_cells(percent);
    let mut cells: Vec<(char, Paint)> = (0..BAR_CELLS)
        .map(|i| {
            if i < full {
                (BAR_FULL, paint)
            } else {
                (BAR_EMPTY, Paint::Dim)
            }
        })
        .collect();
    if let Some(pct) = elapsed {
        cells.insert(mark_at(pct), (BAR_MARK, Paint::Plain));
    }
    let mut out: Vec<Span> = Vec::new();
    for (c, p) in cells {
        match out.last_mut() {
            // Runs of one glyph: the filled and the empty part stay separate spans.
            Some(last) if last.paint == p && last.text.ends_with(c) => last.text.push(c),
            _ => out.push(span(c.to_string(), p)),
        }
    }
    out
}

/// The cell boundary nearest to `elapsed`: 0 before the first cell, 8 after the last.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub(super) fn mark_at(elapsed: f64) -> usize {
    let at = (elapsed.clamp(0.0, 100.0) / 100.0 * BAR_CELLS as f64).round();
    (at as usize).min(BAR_CELLS)
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

/// A count: `15.5k`, `76k`, `1.2M` (compact) or every digit with grouping (full).
#[allow(clippy::cast_precision_loss)] // token counts are far below 2^52
fn count(l: &Localizer, n: u64, format: Format) -> String {
    let n = n as f64;
    // One decimal below ten, unless it is zero: `1.2k`, `9k`, `12k`.
    let scaled = |key: &str, x: f64| {
        let one = x < 10.0 && (x * 10.0).round() % 10.0 != 0.0;
        l.fmt(
            key,
            &[
                ("n", Arg::Num(x)),
                ("d", Arg::Num(if one { 1.0 } else { 0.0 })),
            ],
        )
    };
    match format {
        Format::Compact if n >= 1_000_000.0 => scaled("sl-millions", n / 1_000_000.0),
        Format::Compact if n >= 1_000.0 => scaled("sl-thousands", n / 1_000.0),
        _ => l.fmt("sl-number", &[("n", Arg::Num(n))]),
    }
}

/// `1:48:03`.
fn hms(ms: u64) -> String {
    let s = ms / 1000;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// The last folder of `path`, either separator.
fn basename(path: &str) -> &str {
    let trimmed = path.trim_end_matches(['/', '\\']);
    trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed)
}

/// `path` with the home folder as `~` (case-insensitive on Windows).
fn home_short(path: &str, home: Option<&str>) -> String {
    let Some(home) = home
        .map(|h| h.trim_end_matches(['/', '\\']))
        .filter(|h| !h.is_empty())
    else {
        return path.to_owned();
    };
    let same = if cfg!(windows) {
        path.get(..home.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(home))
    } else {
        path.starts_with(home)
    };
    let rest = &path[if same { home.len() } else { 0 }..];
    if same && (rest.is_empty() || rest.starts_with(['/', '\\'])) {
        format!("~{rest}")
    } else {
        path.to_owned()
    }
}

#[allow(clippy::cast_precision_loss)] // durations are far below 2^52 ms
fn ms_arg_i(ms: i64) -> Arg<'static> {
    Arg::Num(ms as f64)
}

#[allow(clippy::cast_precision_loss)] // durations are far below 2^52 ms
fn ms_arg(ms: u64) -> Arg<'static> {
    Arg::Num(ms as f64)
}

/// Spans as terminal text; `color` false leaves the escape codes out.
pub fn to_ansi(lines: &[Vec<Span>], color: bool) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let mut codes = String::new();
    for (i, spans) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        // An empty span (the bar's empty part at 100 %) would only add escape codes.
        for s in spans.iter().filter(|s| !s.text.is_empty()) {
            codes.clear();
            if color {
                let mut push = |code: u8| {
                    if !codes.is_empty() {
                        codes.push(';');
                    }
                    let _ = write!(codes, "{code}");
                };
                if s.paint == Paint::Dim {
                    push(2);
                }
                match (s.fg, s.paint) {
                    (Some(c), _) => push(c.fg()),
                    (None, Paint::Ok) => push(36),
                    (None, Paint::Warn) => push(33),
                    (None, Paint::Crit) => push(31),
                    (None, Paint::Plain | Paint::Dim) => {}
                }
                if let Some(c) = s.bg {
                    push(c.bg());
                }
            }
            if codes.is_empty() {
                out.push_str(&s.text);
            } else {
                out.push_str("\x1b[");
                out.push_str(&codes);
                out.push('m');
                out.push_str(&s.text);
                out.push_str("\x1b[0m");
            }
        }
    }
    out
}
