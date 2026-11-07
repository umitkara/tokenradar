//! The terminal side of the statusline line; the line itself lives in
//! `tokenradar_core::statusline::line`, shared with the settings preview.

use tokenradar_core::statusline::line::{ColorMode, Style};

/// Colors per `mode` (`auto` honours `NO_COLOR`); the width from `COLUMNS`.
pub fn style_from_env(mode: ColorMode) -> Style {
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    Style {
        color: mode.resolve(no_color),
        width: std::env::var("COLUMNS")
            .ok()
            .and_then(|c| c.trim().parse().ok())
            .filter(|w| *w > 0),
    }
}
