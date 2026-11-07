//! Built-in starting points for the line (Step 12i, ADR-049). There is no online gallery
//! (ADR-013); the user picks one of these and edits it, or imports a table someone shared.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::{
    AnsiColor, ColorMode, DEFAULT_SEGMENTS, DEFAULT_SEPARATOR, Format, Item, Layout,
    SegmentKind as K, StatuslinePrefs, Theme,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    /// The 12c line.
    Default,
    /// The limits and nothing else.
    Minimal,
    /// Limits and the session on the first line, the project on the second.
    Detailed,
    /// The 12c double layout: limits first, the session second.
    TwoLine,
    Powerline,
}

impl Preset {
    pub const ALL: [Self; 5] = [
        Self::Default,
        Self::Minimal,
        Self::Detailed,
        Self::TwoLine,
        Self::Powerline,
    ];

    pub fn prefs(self) -> StatuslinePrefs {
        let with = |items: Vec<Item>, theme: Theme| {
            StatuslinePrefs {
                items,
                theme,
                separator: DEFAULT_SEPARATOR.to_owned(),
                color: ColorMode::Auto,
            }
            .normalized()
        };
        match self {
            Self::Default => StatuslinePrefs::default(),
            Self::TwoLine => StatuslinePrefs::legacy(&DEFAULT_SEGMENTS, Layout::Double, true, true),
            Self::Minimal => with(
                vec![
                    Item::new(K::FiveHour).with_format(Format::Percent),
                    Item::new(K::SevenDay).with_format(Format::Percent),
                    Item::new(K::Scoped),
                ],
                Theme::Plain,
            ),
            Self::Detailed => with(
                vec![
                    Item::new(K::FiveHour),
                    Item::new(K::SevenDay),
                    Item::new(K::Scoped),
                    Item::new(K::Context).with_format(Format::Bar),
                    Item::new(K::SessionShare),
                    Item::new(K::Cost),
                    Item::new(K::LineBreak),
                    Item::new(K::Model).with_color(AnsiColor::BrightBlue),
                    Item::new(K::Cwd).with_color(AnsiColor::BrightMagenta),
                    Item::new(K::GitBranch).with_color(AnsiColor::Green),
                    Item::new(K::LinesChanged),
                    Item::new(K::Duration),
                ],
                Theme::Plain,
            ),
            Self::Powerline => with(
                vec![
                    Item::new(K::FiveHour),
                    Item::new(K::SevenDay),
                    Item::new(K::Scoped),
                    Item::new(K::Context),
                    Item::new(K::Cost),
                    Item::new(K::Model),
                ],
                Theme::Powerline,
            ),
        }
    }
}
