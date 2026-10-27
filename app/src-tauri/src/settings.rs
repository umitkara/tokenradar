//! The settings window (Step 11b). Its warm TTL is 0 s (architecture.md §2): closing it
//! destroys the webview.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_specta::Event;

use crate::chrome::{self, Surface};

pub const LABEL: &str = "settings";

/// A part of the settings window the panel's calls to action point at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SettingsSection {
    General,
    /// Claude Code folders (`CLAUDE_CONFIG_DIR`).
    Accounts,
    /// Installing the statusline bridge.
    Statusline,
    /// Consent for the usage API.
    Api,
    Diagnostics,
    /// About: a found update and "Install and restart".
    Updates,
}

impl SettingsSection {
    const fn as_str(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Accounts => "accounts",
            Self::Statusline => "statusline",
            Self::Api => "api",
            Self::Diagnostics => "diagnostics",
            Self::Updates => "updates",
        }
    }
}

/// Sent to an open settings window that should switch to `0`. A new window reads the section
/// from its URL fragment (`settings.html#api`).
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct SettingsSectionRequested(pub SettingsSection);

/// Shows and focuses the settings window, creating it if needed.
pub fn show(app: &AppHandle, section: Option<SettingsSection>) -> tauri::Result<()> {
    let window = if let Some(window) = app.get_webview_window(LABEL) {
        if let Some(section) = section {
            let _ = SettingsSectionRequested(section).emit_to(app, LABEL);
        }
        window
    } else {
        let page = match section {
            Some(section) => format!("settings.html#{}", section.as_str()),
            None => "settings.html".to_owned(),
        };
        let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(page.into()));
        chrome::dress(app, builder, Surface::Window)
            .title("TokenRadar")
            .inner_size(800.0, 600.0)
            .min_inner_size(640.0, 480.0)
            .build()?
    };
    window.show()?;
    window.set_focus()
}
