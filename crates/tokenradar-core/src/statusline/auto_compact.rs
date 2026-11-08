//! Claude Code's auto-compact setting, as the statusline's `compact` item shows it (Step 12n,
//! ADR-054).
//!
//! Read-only. Each settings file is parsed for three keys only (`autoCompactEnabled`,
//! `autoCompactWindow`, `modelSettings`); everything else in it, `env` included, is skipped by
//! serde. A missing or unreadable file is no file.
//!
//! Sources, as documented by Claude Code (2026-11-08):
//! - Files, highest first: managed (`managed-settings.d/*.json` in reverse alphabetical order,
//!   then `managed-settings.json`), `<project>/.claude/settings.local.json`,
//!   `<project>/.claude/settings.json`, `<config>/settings.json`.
//! - Within one file, `modelSettings.<model>.autoCompactWindow` beats the top-level window.
//! - `DISABLE_AUTO_COMPACT=1` or `DISABLE_COMPACT=1` turn auto-compact off whatever the files
//!   say; `CLAUDE_CODE_AUTO_COMPACT_WINDOW` beats every window setting. The statusline runs as
//!   Claude Code's subprocess, so these include the settings files' `env` blocks.
//! - The window is capped at the model's context window.
//!
//! Not visible from here: `--autocompact` and `--settings` on Claude Code's command line,
//! registry and server-managed policies. The value shown is the configuration as the files
//! and the environment state it; the trigger point inside the window is never shown.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

/// The window Claude Code accepts, in tokens.
pub const WINDOW_RANGE: std::ops::RangeInclusive<u64> = 100_000..=1_000_000;

/// Settings files larger than this are not settings files.
const MAX_FILE_BYTES: u64 = 1 << 20;

/// What the `compact` item shows after the count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoCompact {
    /// Auto-compact is off.
    Off,
    /// On, with a window the user set, in tokens.
    Window(u64),
}

/// Where the settings files are.
#[derive(Debug, Clone, Default)]
pub struct Locations {
    /// Claude Code's system directory for managed settings.
    pub managed_dir: Option<PathBuf>,
    /// The project root Claude Code was started in.
    pub project_dir: Option<PathBuf>,
    /// The account's config directory (`~/.claude` or `CLAUDE_CONFIG_DIR`).
    pub config_dir: Option<PathBuf>,
}

impl Locations {
    /// The managed settings directory of this platform; `None` where v0.1 does not run.
    pub fn system_managed_dir() -> Option<PathBuf> {
        if cfg!(windows) {
            let base = std::env::var_os("ProgramFiles")
                .map_or_else(|| PathBuf::from(r"C:\Program Files"), PathBuf::from);
            Some(base.join("ClaudeCode"))
        } else {
            None
        }
    }

    /// Every settings file, highest precedence first.
    pub fn files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Some(dir) = &self.managed_dir {
            files.extend(drop_ins(&dir.join("managed-settings.d")));
            files.push(dir.join("managed-settings.json"));
        }
        if let Some(dir) = &self.project_dir {
            files.push(dir.join(".claude").join("settings.local.json"));
            files.push(dir.join(".claude").join("settings.json"));
        }
        if let Some(dir) = &self.config_dir {
            files.push(dir.join("settings.json"));
        }
        files
    }
}

/// `*.json` drop-ins, the one merged last (and so winning) first. Hidden files are skipped.
fn drop_ins(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "json")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| !n.starts_with('.'))
                && p.is_file()
        })
        .collect();
    files.sort();
    files.reverse();
    files
}

/// The three keys; serde skips every other one without building it.
#[derive(Deserialize, Default)]
struct RawSettings {
    #[serde(rename = "autoCompactEnabled", default)]
    enabled: Option<Value>,
    #[serde(rename = "autoCompactWindow", default)]
    window: Option<Value>,
    #[serde(rename = "modelSettings", default)]
    models: Option<HashMap<String, RawModel>>,
}

#[derive(Deserialize, Default)]
struct RawModel {
    #[serde(rename = "autoCompactWindow", default)]
    window: Option<Value>,
}

/// One file's say about the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowSetting {
    Tokens(u64),
    /// `"auto"`: Claude Code picks the window, so there is none to show.
    Auto,
}

fn window_setting(v: &Value) -> Option<WindowSetting> {
    match v {
        Value::Number(n) => n
            .as_u64()
            .filter(|t| WINDOW_RANGE.contains(t))
            .map(WindowSetting::Tokens),
        Value::String(s) if s.eq_ignore_ascii_case("auto") => Some(WindowSetting::Auto),
        _ => None,
    }
}

fn read_settings(path: &Path) -> Option<RawSettings> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    serde_json::from_slice(bytes).ok()
}

/// Whether a `modelSettings` key names this model: the canonical name, with or without a
/// date suffix (`-20261001`, `@20261001`) or a context tag (`[1m]`).
pub fn model_matches(key: &str, model_id: &str) -> bool {
    let id = model_id.trim();
    let id = id.find('[').map_or(id, |i| &id[..i]).trim();
    if id.eq_ignore_ascii_case(key) {
        return true;
    }
    let Some(rest) = id
        .get(..key.len())
        .filter(|head| head.eq_ignore_ascii_case(key))
        .and_then(|_| id.get(key.len()..))
    else {
        return false;
    };
    rest.strip_prefix('-')
        .or_else(|| rest.strip_prefix('@'))
        .is_some_and(|date| date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()))
}

/// `200000`, `200k`, `1M`; outside [`WINDOW_RANGE`] Claude Code ignores it, and so do we.
pub fn parse_env_window(value: &str) -> Option<u64> {
    let v = value.trim();
    let (digits, scale) = match v.as_bytes().last()? {
        b'k' | b'K' => (&v[..v.len() - 1], 1_000),
        b'm' | b'M' => (&v[..v.len() - 1], 1_000_000),
        _ => (v, 1),
    };
    let tokens = digits.trim().parse::<u64>().ok()?.checked_mul(scale)?;
    WINDOW_RANGE.contains(&tokens).then_some(tokens)
}

fn env_flag(value: Option<String>) -> bool {
    value.is_some_and(|v| {
        let v = v.trim();
        v == "1" || v.eq_ignore_ascii_case("true")
    })
}

/// The effective setting: `None` when auto-compact is on and no window is set (the item then
/// shows the count alone; Claude Code's default window is never guessed).
pub fn resolve(
    files: &[PathBuf],
    env: impl Fn(&str) -> Option<String>,
    model_id: Option<&str>,
    context_size: Option<u64>,
) -> Option<AutoCompact> {
    if env_flag(env("DISABLE_AUTO_COMPACT")) || env_flag(env("DISABLE_COMPACT")) {
        return Some(AutoCompact::Off);
    }
    let env_window = env("CLAUDE_CODE_AUTO_COMPACT_WINDOW")
        .as_deref()
        .and_then(parse_env_window);
    let mut enabled = None;
    let mut window = env_window.map(WindowSetting::Tokens);
    for settings in files.iter().filter_map(|f| read_settings(f)) {
        if enabled.is_none() {
            enabled = settings.enabled.as_ref().and_then(Value::as_bool);
        }
        if window.is_none() {
            let per_model = model_id.and_then(|id| {
                settings
                    .models
                    .as_ref()?
                    .iter()
                    .filter(|(key, _)| model_matches(key, id))
                    .find_map(|(_, m)| m.window.as_ref().and_then(window_setting))
            });
            window = per_model.or_else(|| settings.window.as_ref().and_then(window_setting));
        }
        if enabled.is_some() && window.is_some() {
            break;
        }
    }
    if enabled == Some(false) {
        return Some(AutoCompact::Off);
    }
    match window? {
        WindowSetting::Auto => None,
        WindowSetting::Tokens(t) => Some(AutoCompact::Window(
            context_size.filter(|c| *c > 0).map_or(t, |c| t.min(c)),
        )),
    }
}

#[cfg(test)]
mod tests;
