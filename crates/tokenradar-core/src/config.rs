//! `config.toml`: the user's settings (architecture §14).
//!
//! Reading goes through serde and never fails on one bad section: a malformed `[notifications]`,
//! `[data]` or `[advanced]` table falls back to its defaults instead of discarding the whole
//! file (and with it the API consent). Writing goes through `toml_edit` and touches only the
//! keys it sets, so unknown fields and comments survive.
//!
//! ```toml
//! schema_version = 2
//!
//! [general]
//! locale = "tr"          # system | en | tr
//! theme = "dark"         # system | light | dark
//! panel_warm_ttl = 60    # seconds a hidden panel stays ready
//!
//! [notifications]        # see `notifier::NotifyPrefs`
//! five_hour_thresholds = [75, 90, 100]
//!
//! [data]
//! raw_retention_days = 90
//! update_check = true
//!
//! [advanced]
//! log_level = "info"
//!
//! [[accounts]]
//! provider = "claude"
//! config_dir = 'C:\Users\Ada\.claude'
//! label = "Work"
//!
//! [accounts.api_consent]
//! granted = true
//! granted_at = 2026-10-27T10:00:00Z
//! consent_version = 1
//!
//! [accounts.statusline]
//! installed = true
//! previous = '{"type": "command", "command": "python ~/.claude/statusline.py"}'
//! previous_command = "python ~/.claude/statusline.py"
//! chain = true
//! chain_position = "before"
//! ```
//!
//! Schema 1 kept one top-level `[api_consent]` (with `version`); [`migrate`] moves it into the
//! primary account. Until then, reading still honours it for accounts without their own.

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;
use tokenradar_i18n::Locale;
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

use crate::notifier::NotifyPrefs;
use crate::statusline::same_dir;

/// Version of the consent text. Raising it asks every user again: a consent given for an
/// older text no longer counts.
pub const API_CONSENT_VERSION: u32 = 1;

/// Debug builds only: set to `1` to simulate consent without editing `config.toml`.
pub const API_CONSENT_ENV: &str = "TOKENRADAR_API_CONSENT";

/// The schema this build writes; [`migrate`] brings older files up to it.
pub const SCHEMA_VERSION: i64 = 2;

/// Default seconds a hidden panel stays ready (architecture §2).
pub const DEFAULT_PANEL_WARM_TTL_S: u32 = 60;
/// Upper bound for `panel_warm_ttl`: longer keeps a `WebView` alive for little gain.
pub const MAX_PANEL_WARM_TTL_S: u32 = 600;
/// Default raw-event retention (ADR-020).
pub const DEFAULT_RAW_RETENTION_DAYS: u32 = 90;
/// Retention bounds: a week keeps the calibration window; ten years is "forever".
pub const MIN_RAW_RETENTION_DAYS: u32 = 7;
pub const MAX_RAW_RETENTION_DAYS: u32 = 3650;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub schema_version: u32,
    pub general: GeneralConfig,
    #[serde(deserialize_with = "lenient")]
    pub notifications: NotifyPrefs,
    #[serde(deserialize_with = "lenient")]
    pub data: DataConfig,
    #[serde(deserialize_with = "lenient")]
    pub advanced: AdvancedConfig,
    pub accounts: Vec<AccountConfig>,
    /// Schema 1's single consent; applies to accounts that have none of their own.
    #[serde(rename = "api_consent")]
    pub legacy_api_consent: ApiConsent,
}

/// A section that does not parse is replaced by its defaults; the rest of the file still counts.
fn lenient<'de, D, T>(de: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    let raw = toml::Value::deserialize(de)?;
    Ok(raw.try_into().unwrap_or_else(|error| {
        tracing::warn!(%error, "config.toml: a section is malformed, its defaults apply");
        T::default()
    }))
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// `system`, `en` or `tr`; anything else means `system`.
    pub locale: Option<String>,
    /// `system`, `light` or `dark`; anything else means `system`.
    pub theme: Option<String>,
    /// Seconds; see [`GeneralConfig::panel_warm_ttl_s`].
    pub panel_warm_ttl: Option<i64>,
}

impl GeneralConfig {
    /// The UI language: the configured one, or the operating system's.
    pub fn locale(&self) -> Locale {
        match self.locale_pref() {
            LocalePref::En => Locale::En,
            LocalePref::Tr => Locale::Tr,
            LocalePref::System => Locale::detect(),
        }
    }

    pub fn locale_pref(&self) -> LocalePref {
        match self.locale.as_deref() {
            Some("en") => LocalePref::En,
            Some("tr") => LocalePref::Tr,
            _ => LocalePref::System,
        }
    }

    pub fn theme_pref(&self) -> ThemePref {
        match self.theme.as_deref() {
            Some("light") => ThemePref::Light,
            Some("dark") => ThemePref::Dark,
            _ => ThemePref::System,
        }
    }

    /// How long a hidden panel stays ready, clamped to `0..=`[`MAX_PANEL_WARM_TTL_S`].
    pub fn panel_warm_ttl_s(&self) -> u32 {
        self.panel_warm_ttl.map_or(DEFAULT_PANEL_WARM_TTL_S, |s| {
            u32::try_from(s.clamp(0, i64::from(MAX_PANEL_WARM_TTL_S))).unwrap_or(0)
        })
    }
}

/// The user's language choice; `System` follows the operating system.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum LocalePref {
    #[default]
    System,
    En,
    Tr,
}

impl LocalePref {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::En => "en",
            Self::Tr => "tr",
        }
    }
}

/// The user's theme choice; `System` follows the operating system's app theme.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ThemePref {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePref {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

/// `[data]`: what is kept and whether updates are looked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct DataConfig {
    /// Days raw usage events are kept; the 5-minute rollup stays forever (ADR-020).
    pub raw_retention_days: u32,
    /// Look for a new release on GitHub (Step 12).
    pub update_check: bool,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            raw_retention_days: DEFAULT_RAW_RETENTION_DAYS,
            update_check: true,
        }
    }
}

impl DataConfig {
    /// Retention within [`MIN_RAW_RETENTION_DAYS`]`..=`[`MAX_RAW_RETENTION_DAYS`].
    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.raw_retention_days = self
            .raw_retention_days
            .clamp(MIN_RAW_RETENTION_DAYS, MAX_RAW_RETENTION_DAYS);
        self
    }
}

/// `[advanced]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct AdvancedConfig {
    pub log_level: LogLevel,
}

/// How much goes into the log file. Never content or tokens, at any level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
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
    /// [`API_CONSENT_VERSION`] at the time of granting (`version` in schema 1).
    #[serde(alias = "version")]
    pub consent_version: u32,
}

impl ApiConsent {
    /// Granted for the current consent text.
    pub fn is_valid(&self) -> bool {
        self.granted && self.consent_version == API_CONSENT_VERSION
    }
}

/// Per-account settings, keyed by the Claude Code config directory. The list already holds
/// several accounts; their UI arrives in Step 19.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct AccountConfig {
    pub provider: String,
    pub config_dir: PathBuf,
    /// A name the user chose; the UI shows the directory otherwise.
    pub label: Option<String>,
    /// `None`: this account has no consent entry (schema 1's top-level one may apply).
    pub api_consent: Option<ApiConsent>,
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
    #[error("a setting could not be written: {0}")]
    Serialize(#[from] toml::ser::Error),
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

    /// Whether the usage API may be called for the account at `config_dir`: its own valid
    /// consent (or schema 1's top-level one if it has none), or the [`API_CONSENT_ENV`]
    /// override in a debug build.
    pub fn api_consent_granted(&self, config_dir: &Path) -> bool {
        let consent = self
            .account(config_dir)
            .and_then(|a| a.api_consent.as_ref())
            .unwrap_or(&self.legacy_api_consent);
        consent.is_valid() || debug_consent_override()
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

// ---------------------------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------------------------

/// Stores one `[general]` string key (`locale`, `theme`). Every other key is left as it was.
pub fn save_general(path: &Path, key: &str, val: &str) -> Result<(), ConfigError> {
    edit(path, |doc| {
        table_mut(doc.as_table_mut(), "general")[key] = value(val);
        Ok(())
    })
}

/// Stores `[general] panel_warm_ttl`, clamped like [`GeneralConfig::panel_warm_ttl_s`].
pub fn save_panel_warm_ttl(path: &Path, seconds: u32) -> Result<(), ConfigError> {
    let seconds = seconds.min(MAX_PANEL_WARM_TTL_S);
    edit(path, |doc| {
        table_mut(doc.as_table_mut(), "general")["panel_warm_ttl"] = value(i64::from(seconds));
        Ok(())
    })
}

/// Stores `[notifications]` (normalized); keys this build does not know are kept.
pub fn save_notifications(path: &Path, prefs: &NotifyPrefs) -> Result<(), ConfigError> {
    save_section(path, "notifications", &prefs.clone().normalized())
}

/// Stores `[data]` (normalized).
pub fn save_data(path: &Path, data: &DataConfig) -> Result<(), ConfigError> {
    save_section(path, "data", &data.clone().normalized())
}

/// Stores `[advanced]`.
pub fn save_advanced(path: &Path, advanced: &AdvancedConfig) -> Result<(), ConfigError> {
    save_section(path, "advanced", advanced)
}

fn save_section<T: Serialize>(path: &Path, name: &str, section: &T) -> Result<(), ConfigError> {
    let toml::Value::Table(fields) = toml::Value::try_from(section)? else {
        unreachable!("sections are structs");
    };
    edit(path, |doc| {
        merge(table_mut(doc.as_table_mut(), name), &fields);
        Ok(())
    })
}

/// Writes `fields` into `table`, recursing into sub-tables so their unknown keys survive.
fn merge(table: &mut Table, fields: &toml::Table) {
    for (key, field) in fields {
        match field {
            toml::Value::Table(inner) => merge(table_mut(table, key), inner),
            other => table[key.as_str()] = Item::Value(to_edit_value(other)),
        }
    }
}

fn to_edit_value(v: &toml::Value) -> toml_edit::Value {
    match v {
        toml::Value::String(s) => s.as_str().into(),
        toml::Value::Integer(i) => (*i).into(),
        toml::Value::Float(f) => (*f).into(),
        toml::Value::Boolean(b) => (*b).into(),
        toml::Value::Datetime(d) => (*d).into(),
        toml::Value::Array(items) => {
            toml_edit::Value::Array(items.iter().map(to_edit_value).collect())
        }
        toml::Value::Table(t) => {
            let mut inline = toml_edit::InlineTable::new();
            for (k, v) in t {
                inline.insert(k, to_edit_value(v));
            }
            toml_edit::Value::InlineTable(inline)
        }
    }
}

/// Stores the statusline state of the account at `config_dir`, creating the account entry
/// if needed. Every other key in the file is left as it was.
pub fn save_statusline(
    path: &Path,
    config_dir: &Path,
    state: &StatuslineConfig,
) -> Result<(), ConfigError> {
    edit(path, |doc| {
        let account = account_mut(doc, config_dir);
        let sl = table_mut(account, "statusline");
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
        Ok(())
    })
}

/// Grants (with the current text version and time) or withdraws the API consent of the
/// account at `config_dir`.
pub fn save_api_consent(
    path: &Path,
    config_dir: &Path,
    granted: bool,
    at: toml::value::Datetime,
) -> Result<(), ConfigError> {
    edit(path, |doc| {
        let consent = table_mut(account_mut(doc, config_dir), "api_consent");
        consent["granted"] = value(granted);
        if granted {
            consent["granted_at"] = value(at);
            consent["consent_version"] = value(i64::from(API_CONSENT_VERSION));
        } else {
            consent.remove("granted_at");
            consent.remove("consent_version");
        }
        Ok(())
    })
}

/// Brings `path` up to [`SCHEMA_VERSION`]; `true` if the file was rewritten. A missing file
/// stays missing. Newer schemas are left alone.
///
/// 1 → 2: the top-level `[api_consent]` moves into the account at `primary_dir` (created if
/// needed) as `consent_version`. Without a primary account it stays where it is; reading still
/// applies it to every account without its own.
pub fn migrate(path: &Path, primary_dir: Option<&Path>) -> Result<bool, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    let mut doc: DocumentMut = text.parse()?;
    let version = doc
        .get("schema_version")
        .and_then(Item::as_integer)
        .unwrap_or(1);
    if version >= SCHEMA_VERSION {
        return Ok(false);
    }
    if let Some(dir) = primary_dir
        && let Some(Item::Table(legacy)) = doc.remove("api_consent")
    {
        let account = account_mut(&mut doc, dir);
        if !account.contains_key("api_consent") {
            let mut consent = Table::new();
            for (key, item) in &legacy {
                let key = if key == "version" {
                    "consent_version"
                } else {
                    key
                };
                consent[key] = item.clone();
            }
            account["api_consent"] = Item::Table(consent);
        }
    }
    doc["schema_version"] = value(SCHEMA_VERSION);
    crate::fsutil::atomic_write(path, doc.to_string().as_bytes())?;
    Ok(true)
}

/// Reads `path` (an empty document when missing; `schema_version` added to new files), applies
/// `f` and writes the result atomically.
fn edit(
    path: &Path,
    f: impl FnOnce(&mut DocumentMut) -> Result<(), ConfigError>,
) -> Result<(), ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let mut doc: DocumentMut = text.parse()?;
    if !doc.contains_key("schema_version") {
        doc["schema_version"] = value(SCHEMA_VERSION);
    }
    f(&mut doc)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    crate::fsutil::atomic_write(path, doc.to_string().as_bytes())?;
    Ok(())
}

/// The sub-table `key` of `parent`, created (or replacing a non-table value) if needed.
fn table_mut<'a>(parent: &'a mut Table, key: &str) -> &'a mut Table {
    if !parent.get(key).is_some_and(Item::is_table) {
        parent[key] = Item::Table(Table::new());
    }
    match parent.get_mut(key) {
        Some(Item::Table(t)) => t,
        _ => unreachable!("set just above"),
    }
}

/// The `[[accounts]]` entry for `config_dir`, appended if missing.
fn account_mut<'a>(doc: &'a mut DocumentMut, config_dir: &Path) -> &'a mut Table {
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
    account
}

#[cfg(test)]
mod tests;
