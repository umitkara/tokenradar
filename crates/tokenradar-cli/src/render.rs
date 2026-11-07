//! The terminal side of the statusline line; the line itself lives in
//! `tokenradar_core::statusline::line`, shared with the settings preview.

use tokenradar_core::statusline::line::Style;

/// ANSI colors unless `NO_COLOR` is set; the width from `COLUMNS`.
pub fn style_from_env() -> Style {
    Style {
        color: std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty()),
        width: std::env::var("COLUMNS")
            .ok()
            .and_then(|c| c.trim().parse().ok())
            .filter(|w| *w > 0),
    }
}
