//! Drawing a [`Line`]: items → painted spans → ANSI text.
//!
//! Each item is drawn on its own; `line_break` items split the output into lines, and each
//! line is fitted to the terminal on its own. When a line is too wide it gives up detail in
//! this order, until it fits:
//!
//! 1. session and project items, from the end of the user's list (with the separators and
//!    texts right before them);
//! 2. a 12-cell bar becomes an 8-cell one;
//! 3. the bar (with its time mark);
//! 4. the elapsed time (`⏱ 43%`);
//! 5. the projection (`▲38m`);
//! 6. the resets, countdowns and clock times alike;
//! 7. limit items from the end, keeping the first; then the remaining texts.
//!
//! Colors (ADR-049): an item's own color replaces the automatic one, but on limits and the
//! context window only below the warning threshold: the warning and critical colors, and the
//! dim drawing of estimates and stale values, always win. Without color the Powerline themes
//! fall back to the plain one and the bar is left out. The Powerline blocks leave the bar out
//! too: drawn in the block's text color, its full and empty cells look alike. The capsule is
//! the exception (Step 12p): its cells differ in shape, so it is drawn in a block as well.
//!
//! Labels (Step 12p): an item's own label, else its symbol in the chosen icon set, else the
//! localized words. A label of only spaces shows nothing, not even a symbol.
//!
//! Layout (ADR-051): only the separators in the list are drawn, and only between two items
//! that are drawn in the same region; two neighbouring items are joined by one space, `space`
//! and `flex` items by their own columns. `flex` items split a line into regions: after one the
//! rest goes to the right, between two the middle region is centred. The line may be
//! [`CLAUDE_CODE_RESERVED`] plus the margin narrower than the terminal.

use serde::Serialize;
use specta::Type;
use tokenradar_i18n::{Arg, Localizer};
use unicode_width::UnicodeWidthStr;

use super::{
    AnsiColor, BarStyle, Format, IconSet, Item, Line, Reading, ResetFormat, SegmentKind,
    StatuslinePrefs, Theme, Window,
};
use crate::limits::Severity;
use crate::statusline::auto_compact::AutoCompact;

/// Cells in a bar when it has to narrow (and by default); each stands for 12.5 %.
const BAR_CELLS: usize = 8;

/// Columns Claude Code keeps free at the right of the statusline: a line wider than
/// `COLUMNS - 4` is cut with `…` there (observed 2026-11-08, ADR-051).
pub const CLAUDE_CODE_RESERVED: usize = 4;

/// The terminal cells `text` takes. `⏱`, `▲` and `≈` were observed to take one each, as the
/// Unicode tables say, so no glyph needs an exception (ADR-051).
pub fn display_width(text: &str) -> usize {
    text.width()
}

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
    /// The index of the item in `StatuslinePrefs::items` the span belongs to (`space` and
    /// `flex` items too); `None` for the spaces and joints between items.
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
    /// Terminal columns (`COLUMNS`); `None` is unknown: nothing is dropped and a `flex` is one
    /// space. The line itself keeps [`CLAUDE_CODE_RESERVED`] and the margin free.
    pub width: Option<usize>,
}

/// What an item may show; reduced step by step on a narrow terminal.
#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // one switch per part of a segment
struct Detail {
    /// A bar wider than [`BAR_CELLS`] keeps its width.
    wide: bool,
    bar: bool,
    pace: bool,
    eta: bool,
    resets: bool,
    /// The `compact` item's auto-compact part (Step 12n).
    compact_limit: bool,
}

const COMPACT: Detail = Detail {
    wide: false,
    bar: false,
    pace: false,
    eta: false,
    resets: false,
    compact_limit: false,
};

/// How an item draws a limit window: its format and its bar.
#[derive(Clone, Copy)]
struct Look {
    remaining: bool,
    clock: bool,
    cells: usize,
    style: BarStyle,
    /// Drawn in a Powerline block: every cell takes the block's text color.
    block: bool,
}

impl Look {
    const PLAIN: Self = Self {
        remaining: false,
        clock: false,
        cells: BAR_CELLS,
        style: BarStyle::Blocks,
        block: false,
    };

    fn of(ctx: &Ctx<'_>, item: &Item, d: Detail) -> Self {
        let cells = usize::from(item.bar_width);
        Self {
            remaining: item.format == Some(Format::Remaining),
            clock: item.reset_format == ResetFormat::Clock,
            cells: if d.wide { cells } else { cells.min(BAR_CELLS) },
            style: item.bar_style,
            block: ctx.theme != Theme::Plain,
        }
    }
}

struct Ctx<'a> {
    l: &'a Localizer,
    line: &'a Line,
    items: &'a [Item],
    separator: &'a str,
    color: bool,
    theme: Theme,
    icons: IconSet,
}

impl Ctx<'_> {
    fn kind(&self, i: usize) -> SegmentKind {
        self.items[i].kind
    }

    /// Whether a bar in `style` is drawn: only in color, and only on the plain theme, except
    /// the capsule, which reads in a Powerline block's single color too (Step 12p).
    fn bars(&self, style: BarStyle) -> bool {
        self.color && (self.theme == Theme::Plain || style == BarStyle::Capsule)
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
        icons: prefs.icons,
    };
    let mut groups: Vec<Vec<usize>> = vec![Vec::new()];
    for (i, item) in prefs.items.iter().enumerate().filter(|(_, i)| i.enabled) {
        match item.kind {
            SegmentKind::LineBreak => groups.push(Vec::new()),
            _ => groups.last_mut().expect("never empty").push(i),
        }
    }
    let width = style
        .width
        .map(|w| w.saturating_sub(CLAUDE_CODE_RESERVED + usize::from(prefs.margin)));
    let lines: Vec<Vec<Span>> = groups
        .iter()
        .map(|items| fit(&ctx, items, width))
        .filter(|spans| !spans.is_empty())
        .collect();
    if lines.is_empty() {
        vec![vec![span(l.msg("sl-no-limits"), Paint::Dim)]]
    } else {
        lines
    }
}

/// The richest form of the items at `indices` that fits `width` columns, with every `flex` at
/// least one of them. Empty when none of the items has a value to show.
fn fit(ctx: &Ctx<'_>, indices: &[usize], width: Option<usize>) -> Vec<Span> {
    let mut d = Detail {
        wide: true,
        bar: true,
        pace: true,
        eta: true,
        resets: true,
        compact_limit: true,
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
        let regions = compose(ctx, &idx, d);
        let len: usize = regions.iter().map(|r| spans_width(r)).sum::<usize>() + regions.len() - 1;
        if width.is_none_or(|w| len <= w) {
            return place(ctx, &idx, regions, width);
        }
        let values = idx.iter().filter(|&&i| !ctx.kind(i).is_layout()).count();
        let next_session_drop = idx.iter().rposition(|&i| {
            let k = ctx.kind(i);
            !k.is_layout() && !k.is_limit()
        });
        // The `compact` item gives up its auto-compact part before it goes (Step 12n).
        if values > 1
            && d.compact_limit
            && let Some(p) = next_session_drop
            && ctx.kind(idx[p]) == SegmentKind::Compact
            && ctx.line.auto_compact.is_some()
            && ctx.line.compacts.is_some()
        {
            d.compact_limit = false;
        } else if values > 1
            && let Some(p) = next_session_drop
        {
            drop_at(ctx, &mut idx, p);
        } else if d.wide && wide_bars(ctx, &idx) {
            d.wide = false;
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
        } else if let Some(p) = idx.iter().rposition(|&i| droppable_layout(ctx.kind(i))) {
            idx.remove(p);
        } else {
            return place(ctx, &idx, regions, width);
        }
    }
}

/// Whether one of the items at `idx` draws a bar wider than [`BAR_CELLS`].
fn wide_bars(ctx: &Ctx<'_>, idx: &[usize]) -> bool {
    idx.iter().any(|&i| {
        let item = &ctx.items[i];
        item.format == Some(Format::Bar) && usize::from(item.bar_width) > BAR_CELLS
    })
}

/// Layout items a narrow line may give up; a `flex` always stays.
fn droppable_layout(kind: SegmentKind) -> bool {
    kind.is_layout() && kind != SegmentKind::Flex
}

/// Removes the value at `p` and the separators, texts and spaces right before it.
fn drop_at(ctx: &Ctx<'_>, idx: &mut Vec<usize>, p: usize) {
    idx.remove(p);
    let mut q = p;
    while q > 0 && droppable_layout(ctx.kind(idx[q - 1])) {
        idx.remove(q - 1);
        q -= 1;
    }
}

fn spans_width(spans: &[Span]) -> usize {
    spans.iter().map(|s| display_width(&s.text)).sum()
}

/// Which way the Powerline joints of a region point: to the right, or to the left in the
/// region after the last `flex`, so that it leans on the right edge.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    Right,
    Left,
}

/// The regions between the `flex` items at `idx`, each composed on its own.
fn compose(ctx: &Ctx<'_>, idx: &[usize], d: Detail) -> Vec<Vec<Span>> {
    let parts: Vec<&[usize]> = idx.split(|&i| ctx.kind(i) == SegmentKind::Flex).collect();
    let last = parts.len() - 1;
    parts
        .iter()
        .enumerate()
        .map(|(n, part)| {
            let dir = if last > 0 && n == last {
                Dir::Left
            } else {
                Dir::Right
            };
            match ctx.theme {
                Theme::Plain => compose_plain(ctx, part, d),
                Theme::Powerline => compose_blocks(ctx, part, d, &POWERLINE, dir),
                Theme::PowerlineRound => compose_blocks(ctx, part, d, &POWERLINE_ROUND, dir),
            }
        })
        .collect()
}

/// The regions joined by the `flex` items at `idx`: the free columns of `width` shared out
/// between them, or one space each when the width is unknown.
fn place(ctx: &Ctx<'_>, idx: &[usize], regions: Vec<Vec<Span>>, width: Option<usize>) -> Vec<Span> {
    let flexes: Vec<usize> = idx
        .iter()
        .copied()
        .filter(|&i| ctx.kind(i) == SegmentKind::Flex)
        .collect();
    let widths: Vec<usize> = regions.iter().map(|r| spans_width(r)).collect();
    let gaps = gaps(&widths, width);
    let mut out = Vec::new();
    for (n, region) in regions.into_iter().enumerate() {
        out.extend(region);
        if let (Some(&gap), Some(&i)) = (gaps.get(n), flexes.get(n)) {
            out.push(Span {
                item: Some(tag(i)),
                ..span(" ".repeat(gap), Paint::Plain)
            });
        }
    }
    out
}

/// The columns of each `flex` between regions `widths` wide, on a line `width` wide; at least
/// one each. One: the rest of the line. Two: the middle region centred on the line, a column
/// off its neighbours at least. More: the free columns shared alike, the remainder to the last.
fn gaps(widths: &[usize], width: Option<usize>) -> Vec<usize> {
    let n = widths.len() - 1;
    let Some(w) = width else {
        return vec![1; n];
    };
    let free = w.saturating_sub(widths.iter().sum()).max(n);
    match n {
        0 => Vec::new(),
        1 => vec![free],
        2 => {
            let (left, middle, right) = (widths[0], widths[1], widths[2]);
            let lo = left + 1;
            let hi = w.saturating_sub(right + 1 + middle).max(lo);
            let start = (w.saturating_sub(middle) / 2).clamp(lo, hi);
            let first = start - left;
            vec![first, free.saturating_sub(first).max(1)]
        }
        _ => {
            let each = free / n;
            let mut gaps = vec![each; n];
            gaps[n - 1] += free - each * n;
            gaps
        }
    }
}

/// Plain: the items joined by a space; `space` items by their own columns instead. A separator
/// is drawn only between two items drawn in the region, and once in a row (the first).
fn compose_plain(ctx: &Ctx<'_>, idx: &[usize], d: Detail) -> Vec<Span> {
    // The items drawn, in order: a separator waits (with the spaces after it) until something
    // follows it, so a separator at either end of the region or next to a missing value is
    // left out.
    let mut drawn: Vec<(usize, Vec<Span>)> = Vec::new();
    let mut waiting: Vec<(usize, Vec<Span>)> = Vec::new();
    let mut shown = false;
    for &i in idx {
        let Some(spans) = piece(ctx, i, d) else {
            continue;
        };
        match ctx.kind(i) {
            SegmentKind::Separator => {
                if shown && waiting.is_empty() {
                    waiting.push((i, spans));
                }
            }
            SegmentKind::Space if !waiting.is_empty() => waiting.push((i, spans)),
            SegmentKind::Space => drawn.push((i, spans)),
            _ => {
                drawn.append(&mut waiting);
                drawn.push((i, spans));
                shown = true;
            }
        }
    }
    // A separator nothing followed: its spaces stay.
    drawn.extend(
        waiting
            .into_iter()
            .filter(|(i, _)| ctx.kind(*i) == SegmentKind::Space),
    );

    let mut out = Vec::new();
    let mut prev: Option<SegmentKind> = None;
    for (i, spans) in drawn {
        let item = &ctx.items[i];
        if prev.is_some_and(|p| !p.is_blank() && !item.kind.is_blank()) {
            out.push(span(" ", Paint::Plain));
        }
        out.extend(spans.into_iter().map(|mut s| {
            if let Some(c) = item.color {
                if s.bg.is_some() {
                    // A solid bar's fill takes the user's color below the warning threshold;
                    // the time mark on it keeps a color that shows.
                    if s.paint == Paint::Ok {
                        s.bg = Some(c);
                        s.fg = s.fg.map(|_| c.contrast());
                    }
                } else if matches!(s.paint, Paint::Ok | Paint::Plain) && !s.text.trim().is_empty() {
                    s.fg = Some(c);
                }
            }
            s.item = Some(tag(i));
            s
        }));
        prev = Some(item.kind);
    }
    out
}

/// The joints of a Powerline theme: full and thin, pointing right, then left.
struct Joints {
    right: char,
    right_thin: char,
    left: char,
    left_thin: char,
}

const POWERLINE: Joints = Joints {
    right: '\u{E0B0}',
    right_thin: '\u{E0B1}',
    left: '\u{E0B2}',
    left_thin: '\u{E0B3}',
};

const POWERLINE_ROUND: Joints = Joints {
    right: '\u{E0B4}',
    right_thin: '\u{E0B5}',
    left: '\u{E0B6}',
    left_thin: '\u{E0B7}',
};

/// Powerline: each item a colored block, joined by the full joint; two blocks of the same
/// color by the thin one. Pointing right, a chain of blocks ends with a joint; pointing left,
/// it starts with one. A `space` item ends the chain and is drawn without a block; separator
/// items have no place here and are skipped.
fn compose_blocks(ctx: &Ctx<'_>, idx: &[usize], d: Detail, j: &Joints, dir: Dir) -> Vec<Span> {
    let mut out = Vec::new();
    let mut prev: Option<AnsiColor> = None;
    let joint = |c: char, fg: AnsiColor, bg: Option<AnsiColor>| Span {
        text: c.to_string(),
        paint: Paint::Plain,
        fg: Some(fg),
        bg,
        item: None,
    };
    let block = |text: &str, paint: Paint, fg: AnsiColor, bg: AnsiColor, item: u16| Span {
        text: text.to_owned(),
        paint,
        fg: Some(fg),
        bg: Some(bg),
        item: Some(item),
    };
    let close = |out: &mut Vec<Span>, prev: Option<AnsiColor>| {
        if let (Dir::Right, Some(p)) = (dir, prev) {
            out.push(joint(j.right, p, None));
        }
    };
    for &i in idx {
        let item = &ctx.items[i];
        if item.kind == SegmentKind::Separator {
            continue;
        }
        let Some(spans) = piece(ctx, i, d) else {
            continue;
        };
        if item.kind == SegmentKind::Space {
            close(&mut out, prev.take());
            out.extend(spans.into_iter().map(|s| Span {
                item: Some(tag(i)),
                ..s
            }));
            continue;
        }
        let bg = block_color(ctx, item);
        let fg = bg.contrast();
        match (dir, prev) {
            (Dir::Right, Some(p)) if p == bg => out.push(joint(j.right_thin, fg, Some(bg))),
            (Dir::Right, Some(p)) => out.push(joint(j.right, p, Some(bg))),
            (Dir::Right, None) => {}
            (Dir::Left, Some(p)) if p == bg => out.push(joint(j.left_thin, fg, Some(bg))),
            (Dir::Left, p) => out.push(joint(j.left, bg, p)),
        }
        out.push(block(" ", Paint::Plain, fg, bg, tag(i)));
        for s in spans {
            // The block's color carries the severity; inside it only dim stays, in a grey
            // that shows (ADR-049: estimates and stale values stay told apart).
            if s.paint == Paint::Dim {
                out.push(block(&s.text, Paint::Dim, bg.muted(), bg, tag(i)));
            } else {
                out.push(block(&s.text, Paint::Plain, fg, bg, tag(i)));
            }
        }
        out.push(block(" ", Paint::Plain, fg, bg, tag(i)));
        prev = Some(bg);
    }
    close(&mut out, prev);
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
        SegmentKind::DailyBudget => line
            .daily_budget()
            .filter(|b| {
                b.source != crate::domain::Source::Estimate && !line.is_stale(b.observed_at)
            })
            .map(|b| b.severity),
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
        SegmentKind::CacheRatio => "sl-label-cache",
        _ => return String::new(),
    };
    l.msg(key)
}

/// The label an item shows when the user gave none, in icon set `icons`: the set's symbol, or
/// the words when it has none (Step 12p).
pub fn shown_label(l: &Localizer, kind: SegmentKind, icons: IconSet) -> String {
    kind.icon(icons)
        .map_or_else(|| default_label(l, kind), str::to_owned)
}

/// The item's label: the user's, else the icon set's symbol, else the words. Only spaces: no
/// label, and no symbol either.
fn label(ctx: &Ctx<'_>, item: &Item) -> String {
    if item.label.is_empty() {
        shown_label(ctx.l, item.kind, ctx.icons)
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
    let lbl = || label(ctx, item);
    match item.kind {
        SegmentKind::FiveHour | SegmentKind::SevenDay => {
            let w = match item.kind {
                SegmentKind::FiveHour => line.five_hour.as_ref(),
                _ => line.seven_day.as_ref(),
            }?;
            let look = Look::of(ctx, item, d);
            let d = Detail {
                bar: d.bar && ctx.bars(item.bar_style) && item.format == Some(Format::Bar),
                pace: d.pace && item.pace,
                resets: d.resets && item.reset,
                ..d
            };
            Some(window(ctx, &lbl(), w, d, look))
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
                let look = Look {
                    remaining: item.format == Some(Format::Remaining),
                    ..Look::PLAIN
                };
                out.extend(window(ctx, &s.label, &s.window, COMPACT, look));
            }
            (!line.scoped.is_empty()).then_some(out)
        }
        SegmentKind::Context => {
            let r = line.context.as_ref()?;
            let paint = value_paint(line, r, Paint::of(Severity::of(r.value)));
            let label = lbl();
            let pct = l.fmt("sl-percent", &[("pct", Arg::Num(r.value))]);
            Some(match item.format {
                Some(Format::Bar) if d.bar && ctx.bars(item.bar_style) => {
                    let look = Look::of(ctx, item, d);
                    let mut out = Vec::new();
                    if !label.is_empty() {
                        out.push(span(format!("{label} "), paint));
                    }
                    out.extend(bar(r.value, None, paint, look));
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
                Some(Format::Remaining) => {
                    let left = (100.0 - r.value).max(0.0);
                    let text = l.fmt("sl-remaining", &[("pct", Arg::Num(left))]);
                    vec![span(labeled(&label, &text), paint)]
                }
                Some(Format::TokensLeft) if line.tokens_left().is_some() => {
                    let left = count(l, line.tokens_left().unwrap_or(0), Format::Compact);
                    let text = l.fmt("sl-tokens-left", &[("n", Arg::Str(&left))]);
                    vec![span(labeled(&label, &text), paint)]
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
        SegmentKind::DailyBudget => {
            let budget = line.daily_budget()?;
            let estimated = budget.source == crate::domain::Source::Estimate;
            let paint = if estimated || line.is_stale(budget.observed_at) {
                Paint::Dim
            } else {
                Paint::of(budget.severity)
            };
            // Whole points from 10 up, one decimal below: `9%/day` reads better than `9.4%`.
            let per_day = if budget.per_day >= 10.0 {
                budget.per_day.round()
            } else {
                (budget.per_day * 10.0).round() / 10.0
            };
            let mark = if estimated { "≈" } else { "" };
            let text = l.fmt("sl-daily-budget", &[("pct", Arg::Num(per_day))]);
            Some(vec![span(labeled(&lbl(), &format!("{mark}{text}")), paint)])
        }
        SegmentKind::BurnRate => line.burn_rate().map(|r| {
            // Derived from the API-priced cost: the `API` mark stays (ADR-042).
            let text = l.fmt("sl-burn-rate", &[("usd", Arg::Num(r.value))]);
            vec![span(
                labeled(&lbl(), &text),
                value_paint(line, &r, Paint::Plain),
            )]
        }),
        SegmentKind::SessionName => plain(labeled(&lbl(), facts.session_name.as_ref()?)),
        SegmentKind::OutputStyle => plain(labeled(&lbl(), facts.output_style.as_ref()?)),
        SegmentKind::CacheRatio => {
            let pct = facts.cache_ratio?;
            plain(labeled(
                &lbl(),
                &l.fmt("sl-percent", &[("pct", Arg::Num(pct.round()))]),
            ))
        }
        SegmentKind::Compact => {
            let text = compact(l, line, item.format, d.compact_limit, ctx.icons)?;
            plain(labeled(&lbl(), &text))
        }
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
        SegmentKind::Space => Some(vec![span(
            " ".repeat(usize::from(item.width)),
            Paint::Plain,
        )]),
        // Its columns are given out when the line is placed.
        SegmentKind::Flex => Some(Vec::new()),
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

/// `7d ███│░░░░░ 41% ⏱ 43% · 3d4h ▲38m`. With `look.remaining` the percentage is what is
/// left (`59% left`), the colors still follow what is used.
fn window(ctx: &Ctx<'_>, label: &str, w: &Window, d: Detail, look: Look) -> Vec<Span> {
    let (l, line) = (ctx.l, ctx.line);
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
        out.extend(bar(w.percent, elapsed, paint, look));
        out.push(span(" ", Paint::Plain));
    }
    out.push(span(
        if look.remaining {
            let left = (100.0 - w.percent).max(0.0);
            l.fmt("sl-remaining", &[("pct", Arg::Num(left))])
        } else {
            l.fmt("sl-percent", &[("pct", Arg::Num(w.percent))])
        },
        paint,
    ));
    if let Some(pct) = elapsed {
        out.push(span(" ", Paint::Plain));
        out.push(span(
            l.fmt(
                "sl-elapsed",
                &[
                    ("mark", Arg::Str(ctx.icons.elapsed_mark())),
                    ("pct", Arg::Num(pct)),
                ],
            ),
            if dim { Paint::Dim } else { Paint::Plain },
        ));
    }
    if let Some(ms) = w.resets_in.filter(|_| d.resets) {
        let clock = look.clock.then(|| line.reset_clock(ms)).flatten();
        let text = match clock {
            Some(c) => l.fmt(
                "sl-reset-clock",
                &[
                    (
                        "day",
                        Arg::Str(&l.msg(WEEKDAYS[usize::from(c.weekday % 7)])),
                    ),
                    ("time", Arg::Str(&format!("{:02}:{:02}", c.hour, c.minute))),
                ],
            ),
            // Without a known zone the countdown stands in: never UTC passed off as local.
            None => l.fmt("sl-resets-in", &[("ms", ms_arg_i(ms))]),
        };
        out.push(span(" · ", Paint::Dim));
        out.push(span(text, if dim { Paint::Dim } else { Paint::Plain }));
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

/// A short bar in `style` for the editor's style picker: two of five cells full, as drawn
/// below the warning threshold (the solid style needs its backgrounds).
pub fn bar_sample(style: BarStyle) -> Vec<Span> {
    let look = Look {
        cells: 5,
        style,
        ..Look::PLAIN
    };
    bar(40.0, None, Paint::Ok, look)
}

/// The weekday names, Monday first.
const WEEKDAYS: [&str; 7] = [
    "sl-weekday-mon",
    "sl-weekday-tue",
    "sl-weekday-wed",
    "sl-weekday-thu",
    "sl-weekday-fri",
    "sl-weekday-sat",
    "sl-weekday-sun",
];

/// The background of a solid bar's full cells, by paint; the empty ones are [`SOLID_EMPTY`].
const fn solid_fill(paint: Paint) -> AnsiColor {
    match paint {
        Paint::Ok => AnsiColor::Cyan,
        Paint::Warn => AnsiColor::Yellow,
        Paint::Crit => AnsiColor::Red,
        // An estimate or a stale value: light grey, still told apart from the empty cells.
        Paint::Plain | Paint::Dim => AnsiColor::White,
    }
}

const SOLID_EMPTY: AnsiColor = AnsiColor::Gray;

/// The bar's cells. When `elapsed` is known the time mark goes between two cells, or in the
/// capsule and solid styles takes the cell the time falls in, so that the frame and the fill
/// stay whole (Step 12p): a full capsule cell in plain, a `|` on a solid one. In a Powerline
/// block (only the capsule is drawn there) every cell takes the block's text color and the mark
/// the muted one. Color alone tells the mark from the fill, which is safe: bars are drawn only
/// with color.
fn bar(percent: f64, elapsed: Option<f64>, paint: Paint, look: Look) -> Vec<Span> {
    let (n, style) = (look.cells, look.style);
    let full = bar_cells(percent, n);
    let in_cell = elapsed
        .filter(|_| style.marks_in_cell())
        .map(|e| mark_cell(e, n));
    let mut cells: Vec<Span> = (0..n)
        .map(|i| {
            let filled = i < full;
            let mark = in_cell == Some(i);
            if style == BarStyle::Solid {
                let bg = if filled {
                    solid_fill(paint)
                } else {
                    SOLID_EMPTY
                };
                let p = if filled { paint } else { Paint::Plain };
                let (text, fg) = if mark {
                    ("|", Some(bg.contrast()))
                } else {
                    (" ", None)
                };
                return Span {
                    fg,
                    bg: Some(bg),
                    ..span(text, p)
                };
            }
            // The mark is a full cell that stands out: bright on the line, muted in a block (an
            // empty-shaped mark only recolored the thin frame and was lost in a block). In a
            // block every other cell takes the block's text color, its shape tells full from
            // empty.
            let p = if mark {
                if look.block { Paint::Dim } else { Paint::Plain }
            } else if filled || look.block {
                paint
            } else {
                Paint::Dim
            };
            span(style.cell(i, n, filled || mark).to_string(), p)
        })
        .collect();
    if let Some(pct) = elapsed.filter(|_| !style.marks_in_cell()) {
        let (_, _, mark) = style.glyphs();
        cells.insert(mark_at(pct, n), span(mark.to_string(), Paint::Plain));
    }
    let mut out: Vec<Span> = Vec::new();
    for cell in cells {
        match out.last_mut() {
            // Runs of one glyph: the filled and the empty part stay separate spans. The
            // capsule's ends join their neighbours.
            Some(last)
                if last.paint == cell.paint
                    && last.fg == cell.fg
                    && last.bg == cell.bg
                    && (style == BarStyle::Capsule || last.text.ends_with(cell.text.as_str())) =>
            {
                last.text.push_str(&cell.text);
            }
            _ => out.push(cell),
        }
    }
    out
}

/// The cell boundary nearest to `elapsed`: 0 before the first of `n` cells, `n` after the last.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub(super) fn mark_at(elapsed: f64, n: usize) -> usize {
    let at = (elapsed.clamp(0.0, 100.0) / 100.0 * n as f64).round();
    (at as usize).min(n)
}

/// The cell of `n` that `elapsed` falls in: the capsule and solid styles mark a cell.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub(super) fn mark_cell(elapsed: f64, n: usize) -> usize {
    let at = (elapsed.clamp(0.0, 100.0) / 100.0 * n as f64).floor();
    (at as usize).min(n.saturating_sub(1))
}

/// Filled cells of `n`: any use shows at least one, only 100 % fills all.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub(super) fn bar_cells(percent: f64, n: usize) -> usize {
    let cells = (percent.clamp(0.0, 100.0) / 100.0 * n as f64).ceil();
    (cells as usize).min(n)
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

/// `⟲ 3 · 300k`, `⟲ 2 auto 1 manual · auto off`. Without a count (the app is not running, or
/// the session is not indexed yet) only the setting shows, and the other way round; with
/// neither, nothing. `limit` false leaves the setting out (a narrow terminal).
fn compact(
    l: &Localizer,
    line: &Line,
    format: Option<Format>,
    limit: bool,
    icons: IconSet,
) -> Option<String> {
    let mark = Arg::Str(icons.compact_mark());
    let counted = line.compacts.map(|c| match format {
        Some(Format::Split) => {
            let mut text = l.fmt(
                "sl-compact-split",
                &[
                    ("mark", mark),
                    ("auto", Arg::Num(f64::from(c.auto))),
                    ("manual", Arg::Num(f64::from(c.manual))),
                ],
            );
            if c.other > 0 {
                text.push(' ');
                text.push_str(&l.fmt("sl-compact-other", &[("n", Arg::Num(f64::from(c.other)))]));
            }
            text
        }
        _ => l.fmt(
            "sl-compact-total",
            &[("mark", mark), ("n", Arg::Num(f64::from(c.total())))],
        ),
    });
    let setting = line
        .auto_compact
        .filter(|_| limit || counted.is_none())
        .map(|a| match a {
            AutoCompact::Off => l.msg("sl-compact-off"),
            AutoCompact::Window(t) => count(l, t, Format::Compact),
        });
    match (counted, setting) {
        (Some(c), Some(s)) => Some(l.fmt(
            "sl-compact-with-limit",
            &[("count", Arg::Str(&c)), ("limit", Arg::Str(&s))],
        )),
        (Some(c), None) => Some(c),
        // The message puts its own space after the mark.
        (None, Some(s)) => Some(l.fmt(
            "sl-compact-limit-only",
            &[
                ("mark", Arg::Str(icons.compact_mark().trim_end())),
                ("limit", Arg::Str(&s)),
            ],
        )),
        (None, None) => None,
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

/// Spans as terminal text; `color` false leaves the color codes out. A line that starts with a
/// space starts with a reset (`ESC[0m`) even then: Claude Code trims the leading spaces of a
/// statusline line, and the reset keeps them (observed 2026-11-08, ADR-051).
pub fn to_ansi(lines: &[Vec<Span>], color: bool) -> String {
    use std::fmt::Write as _;
    let mut all = String::new();
    let mut out = String::new();
    let mut codes = String::new();
    for (i, spans) in lines.iter().enumerate() {
        if i > 0 {
            all.push('\n');
        }
        out.clear();
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
        if out.starts_with(' ') {
            all.push_str("\x1b[0m");
        }
        all.push_str(&out);
    }
    all
}
