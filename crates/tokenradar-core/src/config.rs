//! `config.toml`: the user's settings (architecture §14).
//!
//! Reading goes through serde. Writing goes through `toml_edit` and touches only the keys it
//! sets, so unknown fields and comments survive (the settings window, Step 11, writes the rest).
//!
//! ```toml
//! schema_version = 1
//!
//! [general]
//! locale = "tr"          # system | en | tr
//!
//! [api_consent]
//! granted = true
//! granted_at = 2026-10-27T10:00:00Z
//! version = 1
//!
//! [[accounts]]
//! provider = "claude"
//! config_dir = 'C:\Users\Ada\.claude'
//!
//! [accounts.statusline]
//! installed = true
//! previous = '{"type": "command", "command": "python ~/.claude/statusline.py"}'
//! previous_command = "python ~/.claude/statusline.py"
//! chain = true
//! chain_position = "before"
//! ```

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokenradar_i18n::Locale;

use crate::statusline::same_dir;

/// Version of the consent text. Raising it asks every user again: a consent given for an
/// older text no longer counts.
pub const API_CONSENT_VERSION: u32 = 1;

/// Debug builds only: set to `1` to simulate consent without editing `config.toml`.
pub const API_CONSENT_ENV: &str = "TOKENRADAR_API_CONSENT";

/// Written into a new file; the settings window migrates older ones (Step 11).
pub const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub schema_version: u32,
    pub general: GeneralConfig,
    pub api_consent: ApiConsent,
    pub accounts: Vec<AccountConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// `system`, `en` or `tr`; anything else means `system`.
    pub locale: Option<String>,
}

impl GeneralConfig {
    /// The UI language: the configured one, or the operating system's.
    pub fn locale(&self) -> Locale {
        match self.locale.as_deref() {
            Some(tag @ ("en" | "tr")) => Locale::from_tag(tag),
            _ => Locale::detect(),
        }
    }
}

/// The user's permission to call the OAuth usage API with Claude Code's token (ADR-006).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct ApiConsent {
    pub granted: bool,
    /// When it was given; kept for the settings page, not used in decisions.
    pub granted_at: Option<toml::value::Datetime>,
    /// [`API_CONSENT_VERSION`] at the time of granting.
    pub version: u32,
}

impl ApiConsent {
    /// Granted for the current consent text.
    pub fn is_valid(&self) -> bool {
        self.granted && self.version == API_CONSENT_VERSION
    }
}

/// Per-account settings, keyed by the Claude Code config directory.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct AccountConfig {
    pub provider: String,
    pub config_dir: PathBuf,
    pub statusline: StatuslineConfig,
}

/// What `tokenradar statusline install` changed, so `uninstall` can put it back (ADR-009).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct StatuslineConfig {
    pub installed: bool,
    /// The previous `statusLine` value exactly as it was written in `settings.json`.
    pub previous: Option<String>,
    /// Its `command`, run with the same stdin in chain mode.
    pub previous_command: Option<String>,
    pub chain: bool,
    pub chain_position: ChainPosition,
}

/// Where the previous command's output goes relative to TokenRadar's line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChainPosition {
    #[default]
    Before,
    After,
}

impl ChainPosition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read or write the config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("the config file is not valid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("the config file is not valid TOML: {0}")]
    Edit(#[from] toml_edit::TomlError),
}

impl AppConfig {
    /// Reads `path`. A missing file is the default config, not an error.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        Ok(toml::from_str(text)?)
    }

    /// Whether the usage API may be called: valid consent in the file, or the
    /// [`API_CONSENT_ENV`] override in a debug build.
    pub fn api_consent_granted(&self) -> bool {
        self.api_consent.is_valid() || debug_consent_override()
    }

    /// The settings of the account whose config directory is `config_dir`.
    pub fn account(&self, config_dir: &Path) -> Option<&AccountConfig> {
        self.accounts
            .iter()
            .find(|a| same_dir(&a.config_dir, config_dir))
    }
}

fn debug_consent_override() -> bool {
    cfg!(debug_assertions) && std::env::var_os(API_CONSENT_ENV).is_some_and(|v| v == "1")
}

/// Stores the statusline state of the account at `config_dir`, creating the account entry
/// if needed. Every other key in the file is left as it was.
pub fn save_statusline(
    path: &Path,
    config_dir: &Path,
    state: &StatuslineConfig,
) -> Result<(), ConfigError> {
    use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let mut doc: DocumentMut = text.parse()?;
    if !doc.contains_key("schema_version") {
        doc["schema_version"] = value(SCHEMA_VERSION);
    }
    if !doc.get("accounts").is_some_and(Item::is_array_of_tables) {
        doc["accounts"] = Item::ArrayOfTables(ArrayOfTables::new());
    }
    let Some(accounts) = doc["accounts"].as_array_of_tables_mut() else {
        unreachable!("set just above");
    };
    let index = accounts.iter().position(|t| {
        t.get("config_dir")
            .and_then(Item::as_str)
            .is_some_and(|d| same_dir(Path::new(d), config_dir))
    });
    let index = index.unwrap_or_else(|| {
        let mut t = Table::new();
        t["provider"] = value("claude");
        t["config_dir"] = value(config_dir.to_string_lossy().as_ref());
        accounts.push(t);
        accounts.len() - 1
    });
    let Some(account) = accounts.get_mut(index) else {
        unreachable!("index is in range");
    };
    if !account.get("statusline").is_some_and(Item::is_table) {
        account["statusline"] = Item::Table(Table::new());
    }
    let Some(sl) = account["statusline"].as_table_mut() else {
        unreachable!("set just above");
    };
    sl["installed"] = value(state.installed);
    for (key, v) in [
        ("previous", &state.previous),
        ("previous_command", &state.previous_command),
    ] {
        match v {
            Some(v) => sl[key] = value(v.as_str()),
            None => {
                sl.remove(key);
            }
        }
    }
    sl["chain"] = value(state.chain);
    sl["chain_position"] = value(state.chain_position.as_str());
    crate::fsutil::atomic_write(path, doc.to_string().as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_means_no_consent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = AppConfig::load(&dir.path().join("config.toml")).expect("load");
        assert_eq!(cfg, AppConfig::default());
        assert!(!cfg.api_consent.is_valid());
    }

    #[test]
    fn reads_consent_and_ignores_unknown_fields() {
        let cfg = AppConfig::parse(
            "schema_version = 1\nfuture_setting = \"x\"\n\n[api_consent]\ngranted = true\n\
             granted_at = 2026-10-27T10:00:00Z\nversion = 1\n\n[notifications]\nquiet = true\n",
        )
        .expect("parse");
        assert_eq!(cfg.schema_version, 1);
        assert!(cfg.api_consent.is_valid());
        assert!(cfg.api_consent.granted_at.is_some());
    }

    #[test]
    fn statusline_state_round_trips_and_keeps_unknown_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "# my comment\nschema_version = 1\nfuture = \"x\"\n\n[general]\nlocale = \"tr\"\n",
        )
        .expect("seed");
        let claude = dir.path().join(".claude");
        let state = StatuslineConfig {
            installed: true,
            previous: Some(r#"{"type": "command", "command": "py 's.py'"}"#.into()),
            previous_command: Some("py 's.py'".into()),
            chain: true,
            chain_position: ChainPosition::Before,
        };
        save_statusline(&path, &claude, &state).expect("save");
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.starts_with("# my comment\n"), "{text}");
        assert!(text.contains("future = \"x\""));
        let cfg = AppConfig::load(&path).expect("load");
        assert_eq!(cfg.general.locale(), Locale::Tr);
        assert_eq!(cfg.account(&claude).map(|a| &a.statusline), Some(&state));

        // A second save updates the same entry instead of adding one.
        let cleared = StatuslineConfig::default();
        save_statusline(&path, &claude, &cleared).expect("save again");
        let cfg = AppConfig::load(&path).expect("load");
        assert_eq!(cfg.accounts.len(), 1);
        assert_eq!(cfg.account(&claude).map(|a| &a.statusline), Some(&cleared));
    }

    #[test]
    fn consent_for_an_older_text_does_not_count() {
        let cfg = AppConfig::parse("[api_consent]\ngranted = true\nversion = 0\n").expect("parse");
        assert!(!cfg.api_consent.is_valid());
        let cfg = AppConfig::parse("[api_consent]\ngranted = false\nversion = 1\n").expect("parse");
        assert!(!cfg.api_consent.is_valid());
    }
}
