//! `tokenradar statusline install|uninstall`, also used by the settings window (Step 11).
//!
//! Every change follows the same order (CLAUDE.md, "user config is sacred"): plan → show the
//! diff → the user confirms → the file is re-read and must be unchanged → backup → atomic
//! write. The previous `statusLine` value is kept verbatim in `config.toml`, so uninstalling
//! restores it byte for byte.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::settings::{self, EditError};
use crate::config::{self, AppConfig, ChainPosition, ConfigError, StatuslineConfig};
use crate::domain::Timestamp;

pub const SETTINGS_FILE: &str = "settings.json";
pub const STATUSLINE_KEY: &str = "statusLine";

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("Claude Code config directory not found: {0}")]
    NoConfigDir(PathBuf),
    #[error("{0} is not a valid JSON object")]
    Invalid(PathBuf),
    #[error("{0} changed after the diff was shown")]
    Changed(PathBuf),
    #[error("could not read or write {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Config(#[from] ConfigError),
}

/// What a plan would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Install {
        /// The command that was there before, if any.
        previous_command: Option<String>,
        chain: bool,
    },
    Uninstall {
        /// A previous value is put back; otherwise the key is removed.
        restores_previous: bool,
    },
}

/// The outcome of planning.
#[derive(Debug)]
pub enum Planned {
    Ready(Plan),
    AlreadyInstalled,
    NotInstalled,
}

/// A change to one `settings.json`, ready to be shown and applied.
#[derive(Debug, Clone)]
pub struct Plan {
    pub settings_path: PathBuf,
    pub config_dir: PathBuf,
    pub change: Change,
    /// `None`: the file does not exist yet.
    original: Option<String>,
    updated: String,
    state: StatuslineConfig,
}

impl Plan {
    /// Unified diff of the change, for the confirmation prompt or dialog.
    pub fn diff(&self) -> String {
        let label = self.settings_path.display().to_string();
        settings::unified_diff(
            self.original.as_deref().unwrap_or(""),
            &self.updated,
            &label,
        )
    }

    /// The new file content.
    pub fn updated(&self) -> &str {
        &self.updated
    }

    /// Applies the plan. Returns the backup path (`None` when there was no file to back up).
    ///
    /// The install state is stored before `settings.json` changes and cleared after it, so a
    /// failure in between never loses the previous value.
    pub fn apply(&self, app_config_path: &Path) -> Result<Option<PathBuf>, InstallError> {
        let current = read_optional(&self.settings_path)?;
        if current != self.original {
            return Err(InstallError::Changed(self.settings_path.clone()));
        }
        if matches!(self.change, Change::Install { .. }) {
            config::save_statusline(app_config_path, &self.config_dir, &self.state)?;
        }
        let backup = match &self.original {
            Some(text) => {
                let path = backup_path(&self.settings_path, Timestamp::now());
                std::fs::write(&path, text).map_err(|source| InstallError::Io {
                    path: path.clone(),
                    source,
                })?;
                Some(path)
            }
            None => None,
        };
        crate::fsutil::atomic_write(&self.settings_path, self.updated.as_bytes()).map_err(
            |source| InstallError::Io {
                path: self.settings_path.clone(),
                source,
            },
        )?;
        if matches!(self.change, Change::Uninstall { .. }) {
            config::save_statusline(app_config_path, &self.config_dir, &self.state)?;
        }
        Ok(backup)
    }
}

/// Plans installing `command` (the TokenRadar part, e.g. `C:/…/tokenradar.exe statusline`)
/// into `config_dir`. `chain`: keep running the previous command; `None` chains whenever there
/// is one, so nothing the user had stops working by default.
pub fn plan_install(
    config_dir: &Path,
    app_config: &AppConfig,
    command: &str,
    chain: Option<bool>,
) -> Result<Planned, InstallError> {
    let (settings_path, original) = read_settings(config_dir)?;
    let text = original.as_deref().unwrap_or("{}");
    let raw = settings::raw_member(text, STATUSLINE_KEY)
        .map_err(|EditError::Invalid| InstallError::Invalid(settings_path.clone()))?
        .map(str::to_owned);
    let current_command = raw.as_deref().and_then(command_of);
    if current_command
        .as_deref()
        .is_some_and(is_tokenradar_command)
    {
        return Ok(Planned::AlreadyInstalled);
    }
    let previous_command = current_command.filter(|c| !c.trim().is_empty());
    let chain = chain.unwrap_or(true) && previous_command.is_some();
    let full = if chain {
        format!("{command} --chain")
    } else {
        command.to_owned()
    };
    let value = CommandValue {
        kind: "command",
        command: &full,
    };
    let mut updated = settings::set_member(text, STATUSLINE_KEY, &value)
        .map_err(|EditError::Invalid| InstallError::Invalid(settings_path.clone()))?;
    if original.is_none() {
        updated.push('\n');
    }
    let position = app_config
        .account(config_dir)
        .map_or(ChainPosition::Before, |a| a.statusline.chain_position);
    Ok(Planned::Ready(Plan {
        settings_path,
        config_dir: config_dir.to_path_buf(),
        change: Change::Install {
            previous_command: previous_command.clone(),
            chain,
        },
        original,
        updated,
        state: StatuslineConfig {
            installed: true,
            previous: raw,
            previous_command,
            chain,
            chain_position: position,
        },
    }))
}

/// Plans removing TokenRadar's statusline from `config_dir`, restoring what was there before.
pub fn plan_uninstall(config_dir: &Path, app_config: &AppConfig) -> Result<Planned, InstallError> {
    let (settings_path, original) = read_settings(config_dir)?;
    let Some(text) = original.as_deref() else {
        return Ok(Planned::NotInstalled);
    };
    let raw = settings::raw_member(text, STATUSLINE_KEY)
        .map_err(|EditError::Invalid| InstallError::Invalid(settings_path.clone()))?;
    if !raw
        .and_then(command_of)
        .as_deref()
        .is_some_and(is_tokenradar_command)
    {
        return Ok(Planned::NotInstalled);
    }
    let saved = app_config.account(config_dir).map(|a| &a.statusline);
    let previous = saved.and_then(|s| s.previous.as_deref());
    let updated = match previous {
        Some(prev) => settings::set_member_raw(text, STATUSLINE_KEY, prev),
        None => settings::remove_member(text, STATUSLINE_KEY),
    }
    .map_err(|EditError::Invalid| InstallError::Invalid(settings_path.clone()))?;
    Ok(Planned::Ready(Plan {
        settings_path,
        config_dir: config_dir.to_path_buf(),
        change: Change::Uninstall {
            restores_previous: previous.is_some(),
        },
        original,
        updated,
        state: StatuslineConfig {
            chain_position: saved.map_or(ChainPosition::Before, |s| s.chain_position),
            ..StatuslineConfig::default()
        },
    }))
}

/// The installed `statusLine` value; `type` first, as Claude Code's docs write it.
#[derive(Serialize)]
struct CommandValue<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    command: &'a str,
}

/// `command` of a `{"type": "command", …}` value.
fn command_of(raw: &str) -> Option<String> {
    let v: Value = serde_json::from_str(raw).ok()?;
    v.get("command")?.as_str().map(str::to_owned)
}

/// Whether `command` runs our statusline: its program is `tokenradar[.exe]` and it has the
/// `statusline` subcommand.
pub fn is_tokenradar_command(command: &str) -> bool {
    let rest = command.trim_start().trim_start_matches('&').trim_start();
    let program = match rest.chars().next() {
        Some(q @ ('"' | '\'')) => rest[1..].split(q).next().unwrap_or(""),
        _ => rest.split_whitespace().next().unwrap_or(""),
    };
    let name = program.rsplit(['/', '\\']).next().unwrap_or("");
    let stem = match name.len().checked_sub(4) {
        Some(i)
            if name
                .get(i..)
                .is_some_and(|ext| ext.eq_ignore_ascii_case(".exe")) =>
        {
            &name[..i]
        }
        _ => name,
    };
    stem.eq_ignore_ascii_case("tokenradar") && command.split_whitespace().any(|w| w == "statusline")
}

fn read_settings(config_dir: &Path) -> Result<(PathBuf, Option<String>), InstallError> {
    if !config_dir.is_dir() {
        return Err(InstallError::NoConfigDir(config_dir.to_path_buf()));
    }
    let path = config_dir.join(SETTINGS_FILE);
    let text = read_optional(&path)?;
    Ok((path, text))
}

fn read_optional(path: &Path) -> Result<Option<String>, InstallError> {
    match std::fs::read_to_string(path) {
        Ok(t) => Ok(Some(t)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(InstallError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// `settings.json.tokenradar-20261027T101500Z.bak`, next to the file.
fn backup_path(settings: &Path, now: Timestamp) -> PathBuf {
    let stamp = jiff::Timestamp::from_millisecond(now.as_millis()).map_or_else(
        |_| now.as_millis().to_string(),
        |t| t.strftime("%Y%m%dT%H%M%SZ").to_string(),
    );
    let name = settings.file_name().map_or_else(
        || SETTINGS_FILE.into(),
        |n| n.to_string_lossy().into_owned(),
    );
    let mut path = settings.with_file_name(format!("{name}.tokenradar-{stamp}.bak"));
    let mut n = 1;
    while path.exists() {
        path = settings.with_file_name(format!("{name}.tokenradar-{stamp}-{n}.bak"));
        n += 1;
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    const CMD: &str = "C:/Tools/tokenradar.exe statusline";

    struct Env {
        _dir: tempfile::TempDir,
        claude: PathBuf,
        config: PathBuf,
    }

    fn env(settings: Option<&str>) -> Env {
        let dir = tempfile::tempdir().expect("tempdir");
        let claude = dir.path().join(".claude");
        std::fs::create_dir(&claude).expect("mkdir");
        if let Some(s) = settings {
            std::fs::write(claude.join(SETTINGS_FILE), s).expect("seed");
        }
        let config = dir.path().join("config.toml");
        Env {
            _dir: dir,
            claude,
            config,
        }
    }

    fn ready(p: Planned) -> Plan {
        match p {
            Planned::Ready(plan) => plan,
            other => panic!("expected a plan, got {other:?}"),
        }
    }

    fn settings(e: &Env) -> String {
        std::fs::read_to_string(e.claude.join(SETTINGS_FILE)).expect("read")
    }

    fn cycle(original: &str, chain: Option<bool>) -> Env {
        let e = env(Some(original));
        let cfg = AppConfig::load(&e.config).expect("cfg");
        let plan = ready(plan_install(&e.claude, &cfg, CMD, chain).expect("plan"));
        let backup = plan.apply(&e.config).expect("apply").expect("backup");
        assert_eq!(std::fs::read_to_string(backup).expect("backup"), original);
        assert_ne!(settings(&e), original);

        let cfg = AppConfig::load(&e.config).expect("cfg");
        assert!(matches!(
            plan_install(&e.claude, &cfg, CMD, chain).expect("plan"),
            Planned::AlreadyInstalled
        ));
        let plan = ready(plan_uninstall(&e.claude, &cfg).expect("plan"));
        plan.apply(&e.config).expect("apply");
        assert_eq!(settings(&e), original, "uninstall restores the exact bytes");
        let cfg = AppConfig::load(&e.config).expect("cfg");
        assert!(
            !cfg.account(&e.claude)
                .expect("account")
                .statusline
                .installed
        );
        e
    }

    #[test]
    fn install_then_uninstall_is_byte_identical() {
        // The user's real layout: JSON.stringify(…, null, 2) plus a newline.
        cycle(
            "{\n  \"model\": \"opus\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \
             \"command\": \"python C:\\\\Users\\\\x\\\\statusline.py\"\n  },\n  \"theme\": \"dark\"\n}\n",
            None,
        );
        cycle("{\n  \"model\": \"opus\"\n}\n", None);
        cycle(
            "{\r\n    \"statusLine\": {\"type\":\"command\",\"command\":\"x\"}\r\n}",
            Some(false),
        );
        cycle("{}", None);
    }

    #[test]
    fn the_previous_command_is_chained_by_default_and_kept() {
        let e = env(Some(
            "{\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"py s.py\"\n  }\n}\n",
        ));
        let cfg = AppConfig::load(&e.config).expect("cfg");
        let plan = ready(plan_install(&e.claude, &cfg, CMD, None).expect("plan"));
        assert_eq!(
            plan.change,
            Change::Install {
                previous_command: Some("py s.py".into()),
                chain: true
            }
        );
        assert!(
            plan.diff()
                .contains("+    \"command\": \"C:/Tools/tokenradar.exe statusline --chain\"")
        );
        plan.apply(&e.config).expect("apply");
        let cfg = AppConfig::load(&e.config).expect("cfg");
        let sl = &cfg.account(&e.claude).expect("account").statusline;
        assert!(sl.installed && sl.chain);
        assert_eq!(sl.previous_command.as_deref(), Some("py s.py"));
    }

    #[test]
    fn without_a_previous_command_there_is_nothing_to_chain() {
        let e = env(None);
        let cfg = AppConfig::default();
        let plan = ready(plan_install(&e.claude, &cfg, CMD, Some(true)).expect("plan"));
        assert_eq!(
            plan.change,
            Change::Install {
                previous_command: None,
                chain: false
            }
        );
        assert_eq!(
            plan.apply(&e.config).expect("apply"),
            None,
            "no file, no backup"
        );
        assert_eq!(
            settings(&e),
            "{\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \
             \"C:/Tools/tokenradar.exe statusline\"\n  }\n}\n"
        );
    }

    #[test]
    fn a_file_changed_after_the_diff_is_not_overwritten() {
        let e = env(Some("{}"));
        let plan = ready(plan_install(&e.claude, &AppConfig::default(), CMD, None).expect("plan"));
        std::fs::write(e.claude.join(SETTINGS_FILE), "{\"model\": \"x\"}").expect("edit");
        assert!(matches!(
            plan.apply(&e.config),
            Err(InstallError::Changed(_))
        ));
        assert_eq!(settings(&e), "{\"model\": \"x\"}");
        assert!(
            !e.config.exists(),
            "no state saved for a change that did not happen"
        );
    }

    #[test]
    fn invalid_settings_and_foreign_statuslines_are_left_alone() {
        let e = env(Some("{\"a\": "));
        assert!(matches!(
            plan_install(&e.claude, &AppConfig::default(), CMD, None),
            Err(InstallError::Invalid(_))
        ));
        let e = env(Some(
            "{\"statusLine\": {\"type\": \"command\", \"command\": \"py s.py\"}}",
        ));
        assert!(matches!(
            plan_uninstall(&e.claude, &AppConfig::default()).expect("plan"),
            Planned::NotInstalled
        ));
        assert!(matches!(
            plan_install(
                Path::new("/does/not/exist"),
                &AppConfig::default(),
                CMD,
                None
            ),
            Err(InstallError::NoConfigDir(_))
        ));
    }

    #[test]
    fn our_command_is_recognized_in_every_quoting() {
        for c in [
            "C:/x/tokenradar.exe statusline",
            "\"C:/Program Files/TokenRadar/tokenradar.exe\" statusline --chain",
            "& 'C:/a b/TokenRadar.EXE' statusline",
            "tokenradar statusline",
        ] {
            assert!(is_tokenradar_command(c), "{c}");
        }
        for c in [
            "python statusline.py",
            "tokenradar status",
            "C:/tokenradar-app.exe statusline",
        ] {
            assert!(!is_tokenradar_command(c), "{c}");
        }
    }
}
