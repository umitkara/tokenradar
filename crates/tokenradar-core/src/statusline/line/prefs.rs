//! `[statusline]` in `config.toml` (Step 12i, ADR-049; layout Step 12k, ADR-051): the items of
//! the line, in order.
//!
//! ```toml
//! [statusline]
//! version = 2            # 2: separators are items
//! theme = "plain"        # plain | powerline | powerline_round
//! separator = "│"        # the glyph of new separators; older builds draw it between values
//! color = "auto"         # auto (follows NO_COLOR) | always | never
//! margin = 0             # columns kept free at the right, 0–8
//!
//! [[statusline.items]]
//! kind = "seven_day"
//! format = "bar"         # bar | percent | remaining
//! color = "cyan"         # limits: the color below the warning threshold only
//! reset_format = "clock" # countdown | clock (local time of the reset)
//! bar_width = 12         # 4 | 8 | 12
//! bar_style = "dots"     # blocks | thin | dots
//!
//! [[statusline.items]]
//! kind = "separator"
//!
//! [[statusline.items]]
//! kind = "flex"          # the rest goes to the right
//!
//! [[statusline.items]]
//! kind = "space"
//! width = 2              # 1–8
//!
//! [[statusline.items]]
//! kind = "line_break"
//! ```
//!
//! Files written before 0.1.1 have `segments`, `layout`, `bar` and `pace` instead; the items
//! are derived from them, and those keys are still written so an older build draws a sensible
//! line. A table without `version` gets a separator item between every two neighbouring values,
//! where the older builds drew one by themselves. Reading never fails: an unknown kind is
//! skipped, an unknown format or color falls back to the default, and a list with nothing to
//! show is the default line.

use std::collections::BTreeSet;

use serde::de::IgnoredAny;
use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;

/// One kind of item. The names are the `kind` values in `config.toml`.
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
    TokensIn,
    TokensOut,
    Duration,
    ApiDuration,
    LinesChanged,
    VimMode,
    Version,
    AgentName,
    SessionId,
    Cwd,
    ProjectDir,
    GitBranch,
    /// The weekly window's room left per day until its reset (Step 12m).
    DailyBudget,
    /// The session's API-priced cost per hour.
    BurnRate,
    SessionName,
    /// The share of the last request's input read from the prompt cache.
    CacheRatio,
    OutputStyle,
    /// How many times the session was compacted and the auto-compact window (Step 12n).
    Compact,
    /// A separator glyph of the user's choice.
    Separator,
    /// Starts a new output line.
    LineBreak,
    /// A fixed text of the user's choice.
    CustomText,
    /// `width` blank columns.
    Space,
    /// Takes the columns the line leaves free: what follows goes to the right, or between two
    /// of them to the middle (ADR-051).
    Flex,
}

/// Where an item belongs: the add-item palette groups by it, and a narrow terminal drops
/// session and project items before limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Limit,
    Session,
    Project,
    Layout,
}

/// How an item shows its value. Each kind accepts some of these; see [`SegmentKind::formats`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    /// The 8-cell bar and the percentage.
    Bar,
    Percent,
    /// `15k/200k`.
    Fraction,
    TwoDecimals,
    FourDecimals,
    /// Model: the display name. Duration: `1h48m`. Session id: 8 characters.
    Short,
    /// Model: the id. Paths: the whole path. Session id: all of it. Tokens: every digit.
    Full,
    /// `15.5k`.
    Compact,
    /// `1:48:03`.
    Hms,
    /// `+156 -23`.
    Both,
    /// `+133`.
    Net,
    /// The last folder.
    Basename,
    /// The path with the home folder as `~`.
    Home,
    /// Limits and the context window: the percentage left, `58% left`.
    Remaining,
    /// The context window: the tokens left, `185k left`.
    TokensLeft,
    /// Compactions: the total, `⟲3`.
    Total,
    /// Compactions: automatic and manual apart, `⟲2 auto 1 manual`.
    Split,
}

/// How a limit shows its reset (Step 12m).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ResetFormat {
    /// `3d4h`.
    #[default]
    Countdown,
    /// `Mon 09:00`, local time.
    Clock,
}

/// The glyphs of a bar (Step 12m). The new styles use only glyphs that take one cell in every
/// terminal: none of them is East Asian "Ambiguous".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum BarStyle {
    /// `███░░░░░`.
    #[default]
    Blocks,
    /// `▬▬▬▭▭▭▭▭`.
    Thin,
    /// `⣿⣿⣿⣀⣀⣀⣀⣀`.
    Dots,
}

/// The 16 terminal colors. The terminal's own palette decides what they look like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum AnsiColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Gray,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Plain,
    /// Colored blocks joined by U+E0B0; needs a Nerd Font.
    Powerline,
    /// The same with round joints (U+E0B4).
    PowerlineRound,
}

/// Whether the line is colored. Claude Code does not set `NO_COLOR` for the statusline
/// command (observed 2026-11-07), so `auto` keeps the colors there.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    /// Colors unless `NO_COLOR` is set.
    #[default]
    Auto,
    Always,
    Never,
}

/// The longest label, custom text and separator kept, in characters.
pub const MAX_LABEL: usize = 16;
pub const MAX_TEXT: usize = 32;
pub const MAX_SEPARATOR: usize = 3;
/// More items than this are dropped from the end. Separators are items (ADR-051), so a line
/// has about twice as many as values.
pub const MAX_ITEMS: usize = 64;
/// The widest `space` item, and the largest right margin, in columns.
pub const MAX_SPACE: u8 = 8;
pub const MAX_MARGIN: u8 = 8;
/// The `version` of the stored table: 2 has the separators as items (ADR-051). A table without
/// it (or with 1) gets them inserted where the older builds drew them by themselves.
pub const PREFS_VERSION: u8 = 2;
pub const DEFAULT_SEPARATOR: &str = "│";
/// The bar widths offered, in cells; the first is the default.
pub const BAR_WIDTHS: [u8; 3] = [8, 4, 12];

impl SegmentKind {
    pub const ALL: [Self; 30] = [
        Self::FiveHour,
        Self::SevenDay,
        Self::Scoped,
        Self::Context,
        Self::SessionShare,
        Self::Cost,
        Self::Model,
        Self::TokensIn,
        Self::TokensOut,
        Self::Duration,
        Self::ApiDuration,
        Self::LinesChanged,
        Self::VimMode,
        Self::Version,
        Self::AgentName,
        Self::SessionId,
        Self::Cwd,
        Self::ProjectDir,
        Self::GitBranch,
        Self::DailyBudget,
        Self::BurnRate,
        Self::SessionName,
        Self::CacheRatio,
        Self::OutputStyle,
        Self::Compact,
        Self::Separator,
        Self::LineBreak,
        Self::CustomText,
        Self::Space,
        Self::Flex,
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
            Self::TokensIn => "tokens_in",
            Self::TokensOut => "tokens_out",
            Self::Duration => "duration",
            Self::ApiDuration => "api_duration",
            Self::LinesChanged => "lines_changed",
            Self::VimMode => "vim_mode",
            Self::Version => "version",
            Self::AgentName => "agent_name",
            Self::SessionId => "session_id",
            Self::Cwd => "cwd",
            Self::ProjectDir => "project_dir",
            Self::GitBranch => "git_branch",
            Self::DailyBudget => "daily_budget",
            Self::BurnRate => "burn_rate",
            Self::SessionName => "session_name",
            Self::CacheRatio => "cache_ratio",
            Self::OutputStyle => "output_style",
            Self::Compact => "compact",
            Self::Separator => "separator",
            Self::LineBreak => "line_break",
            Self::CustomText => "custom_text",
            Self::Space => "space",
            Self::Flex => "flex",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }

    /// A limit window, or a value of one. Limits are dropped last when the terminal is narrow.
    pub const fn is_limit(self) -> bool {
        matches!(
            self,
            Self::FiveHour | Self::SevenDay | Self::Scoped | Self::DailyBudget
        )
    }

    /// The limit windows with a reset countdown and a time mark.
    pub const fn is_window(self) -> bool {
        matches!(self, Self::FiveHour | Self::SevenDay)
    }

    /// Whether one of its formats draws a bar.
    pub fn has_bar(self) -> bool {
        self.formats().contains(&Format::Bar)
    }

    /// An item that only arranges the line and shows no value.
    pub const fn is_layout(self) -> bool {
        matches!(
            self,
            Self::Separator | Self::LineBreak | Self::CustomText | Self::Space | Self::Flex
        )
    }

    /// Blank columns: drawn without the joining space beside them (ADR-051).
    pub const fn is_blank(self) -> bool {
        matches!(self, Self::Space | Self::Flex)
    }

    /// The kinds of the 12c line, the only ones an older build knows.
    pub const fn is_legacy(self) -> bool {
        matches!(
            self,
            Self::FiveHour
                | Self::SevenDay
                | Self::Scoped
                | Self::Context
                | Self::SessionShare
                | Self::Cost
                | Self::Model
        )
    }

    pub const fn group(self) -> Group {
        match self {
            Self::FiveHour | Self::SevenDay | Self::Scoped | Self::DailyBudget => Group::Limit,
            Self::Cwd | Self::ProjectDir | Self::GitBranch => Group::Project,
            Self::Separator | Self::LineBreak | Self::CustomText | Self::Space | Self::Flex => {
                Group::Layout
            }
            _ => Group::Session,
        }
    }

    /// The formats this kind accepts; the first is the default. Empty: one way to show it.
    pub const fn formats(self) -> &'static [Format] {
        match self {
            Self::FiveHour | Self::SevenDay => &[Format::Bar, Format::Percent, Format::Remaining],
            Self::Scoped => &[Format::Percent, Format::Remaining],
            Self::Context => &[
                Format::Percent,
                Format::Bar,
                Format::Fraction,
                Format::Remaining,
                Format::TokensLeft,
            ],
            Self::Cost => &[Format::TwoDecimals, Format::FourDecimals],
            Self::Model | Self::SessionId => &[Format::Short, Format::Full],
            Self::TokensIn | Self::TokensOut => &[Format::Compact, Format::Full],
            Self::Duration | Self::ApiDuration => &[Format::Short, Format::Hms],
            Self::LinesChanged => &[Format::Both, Format::Net],
            Self::Cwd => &[Format::Basename, Format::Home, Format::Full],
            Self::ProjectDir => &[Format::Basename, Format::Full],
            Self::Compact => &[Format::Total, Format::Split],
            _ => &[],
        }
    }

    pub fn default_format(self) -> Option<Format> {
        self.formats().first().copied()
    }

    /// Whether the item can carry a label of the user's.
    pub const fn labeled(self) -> bool {
        !self.is_layout()
    }

    /// Whether the item can carry a color of the user's.
    pub const fn colored(self) -> bool {
        !matches!(self, Self::LineBreak | Self::Space | Self::Flex)
    }

    /// The block color in the Powerline themes when the user picked none.
    pub const fn block_color(self) -> AnsiColor {
        match self {
            Self::FiveHour | Self::SevenDay | Self::Scoped | Self::DailyBudget => AnsiColor::Cyan,
            Self::Context | Self::Model | Self::Cwd | Self::ProjectDir => AnsiColor::Blue,
            Self::SessionShare | Self::TokensIn | Self::TokensOut | Self::CacheRatio => {
                AnsiColor::Magenta
            }
            Self::Cost | Self::BurnRate | Self::LinesChanged | Self::GitBranch => AnsiColor::Green,
            Self::SessionName | Self::OutputStyle | Self::Compact => AnsiColor::BrightBlue,
            Self::VimMode => AnsiColor::Yellow,
            Self::AgentName => AnsiColor::BrightCyan,
            _ => AnsiColor::Gray,
        }
    }
}

impl Format {
    pub const ALL: [Self; 17] = [
        Self::Bar,
        Self::Percent,
        Self::Fraction,
        Self::TwoDecimals,
        Self::FourDecimals,
        Self::Short,
        Self::Full,
        Self::Compact,
        Self::Hms,
        Self::Both,
        Self::Net,
        Self::Basename,
        Self::Home,
        Self::Remaining,
        Self::TokensLeft,
        Self::Total,
        Self::Split,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Percent => "percent",
            Self::Fraction => "fraction",
            Self::TwoDecimals => "two_decimals",
            Self::FourDecimals => "four_decimals",
            Self::Short => "short",
            Self::Full => "full",
            Self::Compact => "compact",
            Self::Hms => "hms",
            Self::Both => "both",
            Self::Net => "net",
            Self::Basename => "basename",
            Self::Home => "home",
            Self::Remaining => "remaining",
            Self::TokensLeft => "tokens_left",
            Self::Total => "total",
            Self::Split => "split",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.as_str() == s)
    }
}

impl ResetFormat {
    pub const ALL: [Self; 2] = [Self::Countdown, Self::Clock];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Countdown => "countdown",
            Self::Clock => "clock",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        [Self::Countdown, Self::Clock]
            .into_iter()
            .find(|f| f.as_str() == s)
    }
}

impl BarStyle {
    pub const ALL: [Self; 3] = [Self::Blocks, Self::Thin, Self::Dots];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Blocks => "blocks",
            Self::Thin => "thin",
            Self::Dots => "dots",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|b| b.as_str() == s)
    }

    /// The full cell, the empty cell and the time mark. The blocks were observed in Claude
    /// Code (ADR-044); the other two keep to glyphs that are not "Ambiguous" (Step 12m).
    pub const fn glyphs(self) -> (char, char, char) {
        match self {
            Self::Blocks => ('█', '░', '│'),
            Self::Thin => ('▬', '▭', '|'),
            Self::Dots => ('⣿', '⣀', '|'),
        }
    }
}

impl AnsiColor {
    pub const ALL: [Self; 16] = [
        Self::Black,
        Self::Red,
        Self::Green,
        Self::Yellow,
        Self::Blue,
        Self::Magenta,
        Self::Cyan,
        Self::White,
        Self::Gray,
        Self::BrightRed,
        Self::BrightGreen,
        Self::BrightYellow,
        Self::BrightBlue,
        Self::BrightMagenta,
        Self::BrightCyan,
        Self::BrightWhite,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::Red => "red",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Blue => "blue",
            Self::Magenta => "magenta",
            Self::Cyan => "cyan",
            Self::White => "white",
            Self::Gray => "gray",
            Self::BrightRed => "bright_red",
            Self::BrightGreen => "bright_green",
            Self::BrightYellow => "bright_yellow",
            Self::BrightBlue => "bright_blue",
            Self::BrightMagenta => "bright_magenta",
            Self::BrightCyan => "bright_cyan",
            Self::BrightWhite => "bright_white",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }

    /// The SGR foreground code: 30–37, 90–97.
    pub const fn fg(self) -> u8 {
        match self {
            Self::Black => 30,
            Self::Red => 31,
            Self::Green => 32,
            Self::Yellow => 33,
            Self::Blue => 34,
            Self::Magenta => 35,
            Self::Cyan => 36,
            Self::White => 37,
            Self::Gray => 90,
            Self::BrightRed => 91,
            Self::BrightGreen => 92,
            Self::BrightYellow => 93,
            Self::BrightBlue => 94,
            Self::BrightMagenta => 95,
            Self::BrightCyan => 96,
            Self::BrightWhite => 97,
        }
    }

    /// The SGR background code: 40–47, 100–107.
    pub const fn bg(self) -> u8 {
        self.fg() + 10
    }

    /// Text on a block of this color: black on the light ones, white on the dark ones.
    #[must_use]
    pub const fn contrast(self) -> Self {
        match self {
            Self::Green
            | Self::Yellow
            | Self::Cyan
            | Self::White
            | Self::Gray
            | Self::BrightRed
            | Self::BrightGreen
            | Self::BrightYellow
            | Self::BrightBlue
            | Self::BrightMagenta
            | Self::BrightCyan
            | Self::BrightWhite => Self::Black,
            Self::Black | Self::Red | Self::Blue | Self::Magenta => Self::BrightWhite,
        }
    }

    /// Dim text on a block of this color. SGR dim does not show on black text, so estimates
    /// and stale values get a grey instead, where the block leaves room for one.
    #[must_use]
    pub const fn muted(self) -> Self {
        match self.contrast() {
            Self::BrightWhite => Self::White,
            _ => match self {
                Self::Gray | Self::White => Self::Black,
                _ => Self::Gray,
            },
        }
    }
}

impl Theme {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Powerline => "powerline",
            Self::PowerlineRound => "powerline_round",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        [Self::Plain, Self::Powerline, Self::PowerlineRound]
            .into_iter()
            .find(|t| t.as_str() == s)
    }
}

impl ColorMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        [Self::Auto, Self::Always, Self::Never]
            .into_iter()
            .find(|m| m.as_str() == s)
    }

    /// Whether to color, given whether the environment asks for no color (`NO_COLOR`).
    pub const fn resolve(self, no_color_env: bool) -> bool {
        match self {
            Self::Auto => !no_color_env,
            Self::Always => true,
            Self::Never => false,
        }
    }
}

/// One item of the line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Item {
    pub kind: SegmentKind,
    /// Off: kept in the list but not drawn.
    pub enabled: bool,
    /// `None` only for kinds with one way to show them.
    pub format: Option<Format>,
    /// Shown instead of the default label; empty: the default.
    pub label: String,
    /// `None`: the automatic color. On limits and the context window this is the color below
    /// the warning threshold; the warning and critical colors always win (ADR-049).
    pub color: Option<AnsiColor>,
    /// Limit windows: the reset countdown.
    pub reset: bool,
    /// Limit windows: the time mark and the elapsed time (ADR-044).
    pub pace: bool,
    /// `custom_text`: the text. `separator`: the glyph.
    pub text: String,
    /// `space`: the columns, 1–[`MAX_SPACE`]. 1 on every other kind.
    pub width: u8,
    /// Limit windows: the reset as a countdown or a local time (Step 12m).
    pub reset_format: ResetFormat,
    /// Kinds with a bar: its cells, one of [`BAR_WIDTHS`]; 8 on every other kind.
    pub bar_width: u8,
    pub bar_style: BarStyle,
}

impl Item {
    pub fn new(kind: SegmentKind) -> Self {
        Self {
            kind,
            enabled: true,
            format: kind.default_format(),
            label: String::new(),
            color: None,
            reset: true,
            pace: true,
            text: match kind {
                SegmentKind::Separator => DEFAULT_SEPARATOR.to_owned(),
                _ => String::new(),
            },
            width: 1,
            reset_format: ResetFormat::Countdown,
            bar_width: BAR_WIDTHS[0],
            bar_style: BarStyle::Blocks,
        }
    }

    #[must_use]
    pub fn with_format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    #[must_use]
    pub fn with_color(mut self, color: AnsiColor) -> Self {
        self.color = Some(color);
        self
    }

    #[must_use]
    pub fn with_text(mut self, text: &str) -> Self {
        text.clone_into(&mut self.text);
        self
    }

    /// Only what this kind can use, cleaned of control characters and cut to size.
    fn normalized(mut self) -> Self {
        let formats = self.kind.formats();
        self.format = match self.format {
            Some(f) if formats.contains(&f) => Some(f),
            _ => self.kind.default_format(),
        };
        self.label = if self.kind.labeled() {
            clean(&self.label, MAX_LABEL)
        } else {
            String::new()
        };
        if !self.kind.colored() {
            self.color = None;
        }
        self.text = match self.kind {
            SegmentKind::CustomText => clean(&self.text, MAX_TEXT),
            SegmentKind::Separator => {
                let glyph = clean(&self.text, MAX_SEPARATOR);
                if glyph.trim().is_empty() {
                    DEFAULT_SEPARATOR.to_owned()
                } else {
                    glyph
                }
            }
            _ => String::new(),
        };
        if !self.kind.is_window() {
            self.reset = true;
            self.pace = true;
            self.reset_format = ResetFormat::Countdown;
        }
        self.width = match self.kind {
            SegmentKind::Space => self.width.clamp(1, MAX_SPACE),
            _ => 1,
        };
        if !self.kind.has_bar() || !BAR_WIDTHS.contains(&self.bar_width) {
            self.bar_width = BAR_WIDTHS[0];
        }
        if !self.kind.has_bar() {
            self.bar_style = BarStyle::Blocks;
        }
        self
    }

    /// A separator with `glyph`. A blank glyph is a space as wide as the gap the older builds
    /// drew for it: one column for an empty one, the glyph and a space on each side otherwise.
    pub fn separator(glyph: &str) -> Self {
        let glyph = clean(glyph, MAX_SEPARATOR);
        if glyph.trim().is_empty() {
            let mut space = Self::new(SegmentKind::Space);
            let gap = if glyph.is_empty() {
                1
            } else {
                glyph.chars().count() + 2
            };
            space.width = u8::try_from(gap).unwrap_or(MAX_SPACE);
            space
        } else {
            Self::new(SegmentKind::Separator).with_text(&glyph)
        }
    }
}

/// `text` without control characters (C0, DEL, C1: no escape sequence can get into the
/// terminal), cut to `max` characters.
pub fn clean(text: &str, max: usize) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect::<String>()
}

/// The 12c line's segments: `five_hour` shows only on accounts that have the window.
pub const DEFAULT_SEGMENTS: [SegmentKind; 6] = [
    SegmentKind::FiveHour,
    SegmentKind::SevenDay,
    SegmentKind::Scoped,
    SegmentKind::Context,
    SegmentKind::SessionShare,
    SegmentKind::Cost,
];

/// The 12c layout: one line, or limits on the first and the session on the second.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Layout {
    #[default]
    Single,
    Double,
}

/// What the line shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct StatuslinePrefs {
    /// Always [`PREFS_VERSION`] once read: what tells a migrated list from an older one.
    pub version: u8,
    pub items: Vec<Item>,
    pub theme: Theme,
    /// The glyph of separators added from now on; older builds draw it between two values.
    pub separator: String,
    pub color: ColorMode,
    /// Columns kept free at the right, past the ones Claude Code keeps (ADR-051).
    pub margin: u8,
}

/// Version 1 drew a separator between two values next to each other by itself; version 2 draws
/// only separator items. One between every two neighbouring values keeps the line as it was:
/// a separator next to a value that is not drawn is left out (ADR-051).
/// A value listed twice is dropped first, as [`StatuslinePrefs::normalized`] would.
pub(crate) fn with_separators(items: Vec<Item>, glyph: &str) -> Vec<Item> {
    let mut seen = BTreeSet::new();
    let mut out: Vec<Item> = Vec::with_capacity(items.len() * 2);
    for item in items {
        if !item.kind.is_layout() && !seen.insert(item.kind.as_str()) {
            continue;
        }
        if out
            .last()
            .is_some_and(|prev| !prev.kind.is_layout() && !item.kind.is_layout())
        {
            out.push(Item::separator(glyph));
        }
        out.push(item);
    }
    out
}

impl Default for StatuslinePrefs {
    fn default() -> Self {
        Self::legacy(&DEFAULT_SEGMENTS, Layout::Single, true, true)
    }
}

impl StatuslinePrefs {
    /// The items a 12c `[statusline]` table stands for. The double layout puts the limits on
    /// the first line and the rest on the second, each in the user's order.
    pub fn legacy(segments: &[SegmentKind], layout: Layout, bar: bool, pace: bool) -> Self {
        Self {
            version: PREFS_VERSION,
            items: with_separators(
                Self::legacy_items(segments, layout, bar, pace),
                DEFAULT_SEPARATOR,
            ),
            theme: Theme::Plain,
            separator: DEFAULT_SEPARATOR.to_owned(),
            color: ColorMode::Auto,
            margin: 0,
        }
        .normalized()
    }

    /// The items of a 12c table, without separators (version 1).
    fn legacy_items(segments: &[SegmentKind], layout: Layout, bar: bool, pace: bool) -> Vec<Item> {
        let item = |kind: SegmentKind| {
            let mut item = Item::new(kind);
            if matches!(kind, SegmentKind::FiveHour | SegmentKind::SevenDay) {
                item.format = Some(if bar { Format::Bar } else { Format::Percent });
                item.pace = pace;
            }
            item
        };
        let segments: Vec<SegmentKind> =
            segments.iter().copied().filter(|k| k.is_legacy()).collect();
        match layout {
            Layout::Single => segments.into_iter().map(item).collect(),
            Layout::Double => {
                let (limits, session): (Vec<_>, Vec<_>) =
                    segments.into_iter().partition(|k| k.is_limit());
                let mut items: Vec<Item> = limits.into_iter().map(item).collect();
                items.push(Item::new(SegmentKind::LineBreak));
                items.extend(session.into_iter().map(item));
                items
            }
        }
    }

    /// Each value item at most once (the first wins), at most [`MAX_ITEMS`], every item
    /// cleaned; a list with no value to show becomes the default.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        let mut seen = BTreeSet::new();
        self.items
            .retain(|i| i.kind.is_layout() || seen.insert(i.kind.as_str()));
        self.items.truncate(MAX_ITEMS);
        self.items = std::mem::take(&mut self.items)
            .into_iter()
            .map(Item::normalized)
            .collect();
        if !self.items.iter().any(|i| i.enabled && !i.kind.is_layout()) {
            self.items = Self::default().items;
        }
        self.separator = clean(&self.separator, MAX_SEPARATOR);
        self.version = PREFS_VERSION;
        self.margin = self.margin.min(MAX_MARGIN);
        self
    }

    /// Whether an enabled item of `kind` is in the list.
    pub fn shows(&self, kind: SegmentKind) -> bool {
        self.items.iter().any(|i| i.enabled && i.kind == kind)
    }

    /// Writes the table into `table` as `config.toml` stores it: the items as an array of
    /// tables (only what differs from an item's defaults), and with `legacy` the 12c keys
    /// derived from them, so an older build still draws a sensible line. Other keys in
    /// `table` are left as they are.
    pub fn write_toml(&self, table: &mut toml_edit::Table, legacy: bool) {
        use toml_edit::{Array, ArrayOfTables, Item as TomlItem, Table, value};
        table["version"] = value(i64::from(PREFS_VERSION));
        table["theme"] = value(self.theme.as_str());
        table["separator"] = value(self.separator.as_str());
        table["color"] = value(self.color.as_str());
        table["margin"] = value(i64::from(self.margin));
        if legacy {
            let windows: Vec<&Item> = self
                .items
                .iter()
                .filter(|i| {
                    i.enabled && matches!(i.kind, SegmentKind::FiveHour | SegmentKind::SevenDay)
                })
                .collect();
            let segments: Array = self
                .items
                .iter()
                .filter(|i| i.enabled && i.kind.is_legacy())
                .map(|i| i.kind.as_str())
                .collect();
            table["segments"] = value(segments);
            table["layout"] = value(if self.shows(SegmentKind::LineBreak) {
                "double"
            } else {
                "single"
            });
            table["bar"] =
                value(windows.is_empty() || windows.iter().any(|i| i.format == Some(Format::Bar)));
            table["pace"] = value(windows.is_empty() || windows.iter().any(|i| i.pace));
        }
        let mut items = ArrayOfTables::new();
        for i in &self.items {
            let mut t = Table::new();
            t["kind"] = value(i.kind.as_str());
            if !i.enabled {
                t["enabled"] = value(false);
            }
            if let Some(f) = i.format.filter(|f| Some(*f) != i.kind.default_format()) {
                t["format"] = value(f.as_str());
            }
            if !i.label.is_empty() {
                t["label"] = value(i.label.as_str());
            }
            if let Some(c) = i.color {
                t["color"] = value(c.as_str());
            }
            if !i.reset {
                t["reset"] = value(false);
            }
            if !i.pace {
                t["pace"] = value(false);
            }
            let default_text = Item::new(i.kind).text;
            if i.text != default_text {
                t["text"] = value(i.text.as_str());
            }
            if i.kind == SegmentKind::Space && i.width != 1 {
                t["width"] = value(i64::from(i.width));
            }
            if i.reset_format != ResetFormat::Countdown {
                t["reset_format"] = value(i.reset_format.as_str());
            }
            if i.bar_width != BAR_WIDTHS[0] {
                t["bar_width"] = value(i64::from(i.bar_width));
            }
            if i.bar_style != BarStyle::Blocks {
                t["bar_style"] = value(i.bar_style.as_str());
            }
            items.push(t);
        }
        table["items"] = TomlItem::ArrayOfTables(items);
    }

    /// Only the `[statusline]` table, as TOML text to share (without the 12c keys).
    pub fn export_toml(&self) -> String {
        let mut doc = toml_edit::DocumentMut::new();
        let mut table = toml_edit::Table::new();
        self.clone().normalized().write_toml(&mut table, false);
        doc["statusline"] = toml_edit::Item::Table(table);
        doc.to_string()
    }

    /// Reads TOML text from [`StatuslinePrefs::export_toml`], with or without the
    /// `[statusline]` header. Keys and kinds this build does not know are listed in
    /// `ignored`; only a syntax error fails.
    pub fn import_toml(text: &str) -> Result<Imported, toml::de::Error> {
        let mut table: toml::Table = toml::from_str(text)?;
        if let Some(toml::Value::Table(inner)) = table.remove("statusline") {
            table = inner;
        }
        let mut ignored = Vec::new();
        for key in table.keys() {
            if !KNOWN_KEYS.contains(&key.as_str()) {
                ignored.push(key.clone());
            }
        }
        if let Some(toml::Value::Array(items)) = table.get("items") {
            for item in items {
                let kind = item.get("kind").and_then(toml::Value::as_str);
                if kind.and_then(SegmentKind::parse).is_none() {
                    ignored.push(format!("items.kind = {}", kind.unwrap_or("?")));
                }
            }
        }
        let prefs = Self::deserialize(toml::Value::Table(table)).unwrap_or_default();
        Ok(Imported { prefs, ignored })
    }
}

const KNOWN_KEYS: [&str; 10] = [
    "version",
    "margin",
    "items",
    "theme",
    "separator",
    "color",
    "segments",
    "layout",
    "bar",
    "pace",
];

/// The result of [`StatuslinePrefs::import_toml`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct Imported {
    pub prefs: StatuslinePrefs,
    /// Keys and item kinds that were skipped.
    pub ignored: Vec<String>,
}

/// A value that may be of the wrong type: kept if it reads, skipped if not.
#[derive(Deserialize)]
#[serde(untagged)]
enum Lax<T> {
    Ok(T),
    Bad(IgnoredAny),
}

impl<T> Lax<T> {
    fn ok(self) -> Option<T> {
        match self {
            Self::Ok(v) => Some(v),
            Self::Bad(_) => None,
        }
    }
}

fn lax<T>(v: Option<Lax<T>>) -> Option<T> {
    v.and_then(Lax::ok)
}

/// A TOML integer cut to `0..=max`.
fn small(n: i64, max: u8) -> u8 {
    u8::try_from(n.clamp(0, i64::from(max))).unwrap_or(max)
}

impl<'de> Deserialize<'de> for StatuslinePrefs {
    #[allow(clippy::too_many_lines)] // one line or two per key, the 12c keys included
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct RawItem {
            kind: Option<Lax<String>>,
            enabled: Option<Lax<bool>>,
            format: Option<Lax<String>>,
            label: Option<Lax<String>>,
            color: Option<Lax<String>>,
            reset: Option<Lax<bool>>,
            pace: Option<Lax<bool>>,
            text: Option<Lax<String>>,
            width: Option<Lax<i64>>,
            reset_format: Option<Lax<String>>,
            bar_width: Option<Lax<i64>>,
            bar_style: Option<Lax<String>>,
        }
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct Raw {
            version: Option<Lax<i64>>,
            margin: Option<Lax<i64>>,
            items: Option<Lax<Vec<Lax<RawItem>>>>,
            theme: Option<Lax<String>>,
            separator: Option<Lax<String>>,
            color: Option<Lax<String>>,
            segments: Option<Lax<Vec<Lax<String>>>>,
            layout: Option<Lax<String>>,
            bar: Option<Lax<bool>>,
            pace: Option<Lax<bool>>,
        }

        let raw = Raw::deserialize(de)?;
        let items: Option<Vec<Item>> = lax(raw.items).map(|items| {
            items
                .into_iter()
                .filter_map(Lax::ok)
                .filter_map(|r| {
                    let kind = SegmentKind::parse(&lax(r.kind)?)?;
                    let mut item = Item::new(kind);
                    item.enabled = lax(r.enabled).unwrap_or(true);
                    if let Some(f) = lax(r.format).as_deref().and_then(Format::parse) {
                        item.format = Some(f);
                    }
                    item.label = lax(r.label).unwrap_or_default();
                    item.color = lax(r.color).as_deref().and_then(AnsiColor::parse);
                    item.reset = lax(r.reset).unwrap_or(true);
                    item.pace = lax(r.pace).unwrap_or(true);
                    if let Some(text) = lax(r.text) {
                        item.text = text;
                    }
                    if let Some(width) = lax(r.width) {
                        item.width = small(width, MAX_SPACE);
                    }
                    if let Some(f) = lax(r.reset_format).as_deref().and_then(ResetFormat::parse) {
                        item.reset_format = f;
                    }
                    // Anything but 4, 8 or 12 falls back to 8 in `normalized`.
                    if let Some(w) = lax(r.bar_width) {
                        item.bar_width = u8::try_from(w).unwrap_or(0);
                    }
                    if let Some(b) = lax(r.bar_style).as_deref().and_then(BarStyle::parse) {
                        item.bar_style = b;
                    }
                    Some(item)
                })
                .collect()
        });
        let separator = lax(raw.separator).unwrap_or_else(|| DEFAULT_SEPARATOR.to_owned());
        let migrated = lax(raw.version).is_some_and(|v| v >= i64::from(PREFS_VERSION));
        let items = match items {
            Some(items) if !items.is_empty() && migrated => items,
            Some(items) if !items.is_empty() => with_separators(items, &separator),
            // A 12c table, or a file without `[statusline]`.
            _ => {
                let segments: Vec<SegmentKind> = lax(raw.segments).map_or_else(
                    || DEFAULT_SEGMENTS.to_vec(),
                    |names| {
                        names
                            .into_iter()
                            .filter_map(Lax::ok)
                            .filter_map(|n| SegmentKind::parse(&n))
                            .collect()
                    },
                );
                let segments = if segments.iter().any(|k| k.is_legacy()) {
                    segments
                } else {
                    DEFAULT_SEGMENTS.to_vec()
                };
                let layout = match lax(raw.layout).as_deref() {
                    Some("double") => Layout::Double,
                    _ => Layout::Single,
                };
                let items = Self::legacy_items(
                    &segments,
                    layout,
                    lax(raw.bar).unwrap_or(true),
                    lax(raw.pace).unwrap_or(true),
                );
                with_separators(items, &separator)
            }
        };
        Ok(Self {
            version: PREFS_VERSION,
            items,
            theme: lax(raw.theme)
                .as_deref()
                .and_then(Theme::parse)
                .unwrap_or_default(),
            separator,
            color: lax(raw.color)
                .as_deref()
                .and_then(ColorMode::parse)
                .unwrap_or_default(),
            margin: lax(raw.margin).map_or(0, |m| small(m, MAX_MARGIN)),
        }
        .normalized())
    }
}
