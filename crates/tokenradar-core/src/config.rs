//! `config.toml`: the user's settings (architecture §14).
//!
//! This step only reads it. The settings window (Step 11) writes it and must keep unknown
//! fields, so nothing here may assume it owns the whole file.
//!
//! ```toml
//! schema_version = 1
//!
//! [api_consent]
//! granted = true
//! granted_at = 2026-10-27T10:00:00Z
//! version = 1
//! ```

use std::path::Path;

use serde::Deserialize;

/// Version of the consent text. Raising it asks every user again: a consent given for an
/// older text no longer counts.
pub const API_CONSENT_VERSION: u32 = 1;

/// Debug builds only: set to `1` to simulate consent without editing `config.toml`.
pub const API_CONSENT_ENV: &str = "TOKENRADAR_API_CONSENT";

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub schema_version: u32,
    pub api_consent: ApiConsent,
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

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read the config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("the config file is not valid TOML: {0}")]
    Parse(#[from] toml::de::Error),
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
}

fn debug_consent_override() -> bool {
    cfg!(debug_assertions) && std::env::var_os(API_CONSENT_ENV).is_some_and(|v| v == "1")
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
    fn consent_for_an_older_text_does_not_count() {
        let cfg = AppConfig::parse("[api_consent]\ngranted = true\nversion = 0\n").expect("parse");
        assert!(!cfg.api_consent.is_valid());
        let cfg = AppConfig::parse("[api_consent]\ngranted = false\nversion = 1\n").expect("parse");
        assert!(!cfg.api_consent.is_valid());
    }
}
