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
/// Claude Code's live refresh key inside `statusLine` (Step 12r).
pub const REFRESH_KEY: &str = "refreshInterval";

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
    /// The live refresh interval changes (Step 12r). Without our statusline installed only
    /// `config.toml` changes; install writes it later.
    Refresh {
        /// What `settings.json` has now (any number the user wrote).
        from: Option<u64>,
        to: Option<u16>,
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

    /// Whether applying changes `settings.json` at all: a refresh change while our statusline
    /// is not installed only touches `config.toml`.
    pub fn touches_settings(&self) -> bool {
        self.original.as_deref() != Some(self.updated.as_str())
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
        if !matches!(self.change, Change::Uninstall { .. }) {
            config::save_statusline(app_config_path, &self.config_dir, &self.state)?;
        }
        if !self.touches_settings() {
            return Ok(None);
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
    let saved = app_config.account(config_dir).map(|a| &a.statusline);
    let refresh = saved.and_then(|s| s.refresh_interval_sec);
    let invalid = |EditError::Invalid| InstallError::Invalid(settings_path.clone());
    let mut updated = if raw.as_deref().is_some_and(|r| r.starts_with('{')) {
        // Merge: the user's other keys (`padding`, `hideVimModeIndicator`, their own
        // `refreshInterval`) stay. Uninstall puts the whole previous value back anyway.
        let command = Value::String(full.clone()).to_string();
        let mut t = settings::set_nested(text, STATUSLINE_KEY, "type", Some("\"command\""))
            .map_err(invalid)?;
        t = settings::set_nested(&t, STATUSLINE_KEY, "command", Some(&command)).map_err(invalid)?;
        if let Some(n) = refresh {
            t = settings::set_nested(&t, STATUSLINE_KEY, REFRESH_KEY, Some(&n.to_string()))
                .map_err(invalid)?;
        }
        t
    } else {
        let value = CommandValue {
            kind: "command",
            command: &full,
            refresh_interval: refresh,
        };
        settings::set_member(text, STATUSLINE_KEY, &value).map_err(invalid)?
    };
    if original.is_none() {
        updated.push('\n');
    }
    let position = saved.map_or(ChainPosition::Before, |s| s.chain_position);
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
            refresh_interval_sec: refresh,
        },
    }))
}

/// Plans changing the live refresh interval of `config_dir` to `to` (Step 12r). With our
/// statusline installed, `refreshInterval` in `settings.json` changes too; otherwise only the
/// setting does and install writes it later. `None` when nothing would change.
pub fn plan_refresh(
    config_dir: &Path,
    app_config: &AppConfig,
    to: Option<u16>,
) -> Result<Option<Plan>, InstallError> {
    let (settings_path, original) = read_settings(config_dir)?;
    let saved = app_config
        .account(config_dir)
        .map(|a| a.statusline.clone())
        .unwrap_or_default();
    let text = original.as_deref().unwrap_or("{}");
    let invalid = |EditError::Invalid| InstallError::Invalid(settings_path.clone());
    let raw = settings::raw_member(text, STATUSLINE_KEY).map_err(invalid)?;
    let ours = raw
        .and_then(command_of)
        .as_deref()
        .is_some_and(is_tokenradar_command);
    let from = raw.and_then(refresh_of);
    let settings_change = ours && from != to.map(u64::from);
    if !settings_change && saved.refresh_interval_sec == to {
        return Ok(None);
    }
    let updated = if settings_change {
        let value = to.map(|n| n.to_string());
        settings::set_nested(text, STATUSLINE_KEY, REFRESH_KEY, value.as_deref())
            .map_err(invalid)?
    } else {
        // Unchanged text: `apply` writes `config.toml` only.
        original.clone().unwrap_or_default()
    };
    Ok(Some(Plan {
        settings_path,
        config_dir: config_dir.to_path_buf(),
        change: Change::Refresh { from, to },
        original: if settings_change {
            original
        } else {
            Some(updated.clone())
        },
        updated,
        state: StatuslineConfig {
            refresh_interval_sec: to,
            ..saved
        },
    }))
}

/// The live refresh interval in `config_dir`'s `settings.json`: what Claude Code actually
/// does, for doctor and the editor. `None` without one or without a readable file.
pub fn effective_refresh(config_dir: &Path) -> Option<u64> {
    let (_, text) = read_settings(config_dir).ok()?;
    let raw = settings::raw_member(text.as_deref()?, STATUSLINE_KEY).ok()??;
    refresh_of(raw)
}

/// `refreshInterval` of a `statusLine` value: a positive number of seconds.
fn refresh_of(raw: &str) -> Option<u64> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let n = v.get(REFRESH_KEY)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // checked to be ≥ 1
    let whole = n.as_u64().or_else(|| {
        n.as_f64()
            .filter(|f| f.is_finite() && *f >= 1.0)
            .map(|f| f as u64)
    });
    whole.filter(|&n| n > 0)
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
        // The interval the user picked survives a reinstall.
        state: StatuslineConfig {
            chain_position: saved.map_or(ChainPosition::Before, |s| s.chain_position),
            refresh_interval_sec: saved.and_then(|s| s.refresh_interval_sec),
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
    #[serde(rename = "refreshInterval", skip_serializing_if = "Option::is_none")]
    refresh_interval: Option<u16>,
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

pub(super) fn read_settings(config_dir: &Path) -> Result<(PathBuf, Option<String>), InstallError> {
    if !config_dir.is_dir() {
        return Err(InstallError::NoConfigDir(config_dir.to_path_buf()));
    }
    let path = config_dir.join(SETTINGS_FILE);
    let text = read_optional(&path)?;
    Ok((path, text))
}

pub(super) fn read_optional(path: &Path) -> Result<Option<String>, InstallError> {
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
pub(crate) fn backup_path(settings: &Path, now: Timestamp) -> PathBuf {
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
    fn install_keeps_the_users_other_statusline_keys() {
        let original = "{\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \
                        \"py s.py\",\n    \"padding\": 2,\n    \"refreshInterval\": 3\n  }\n}\n";
        let e = cycle(original, None);
        // While installed: `padding` and the user's own interval stayed.
        let cfg = AppConfig::load(&e.config).expect("cfg");
        let plan = ready(plan_install(&e.claude, &cfg, CMD, None).expect("plan"));
        let v: Value = serde_json::from_str(plan.updated()).expect("json");
        assert_eq!(v["statusLine"]["padding"], 2);
        assert_eq!(v["statusLine"]["refreshInterval"], 3);
        assert_eq!(
            v["statusLine"]["command"],
            "C:/Tools/tokenradar.exe statusline --chain"
        );
    }

    #[test]
    fn the_refresh_interval_is_written_changed_and_removed() {
        let e = env(Some("{\n  \"model\": \"opus\"\n}\n"));
        // Not installed yet: only the setting changes.
        let cfg = AppConfig::load(&e.config).expect("cfg");
        let plan = plan_refresh(&e.claude, &cfg, Some(10))
            .expect("plan")
            .expect("a change");
        assert!(!plan.touches_settings());
        assert_eq!(plan.apply(&e.config).expect("apply"), None, "no backup");
        assert_eq!(settings(&e), "{\n  \"model\": \"opus\"\n}\n");
        let cfg = AppConfig::load(&e.config).expect("cfg");
        assert_eq!(
            cfg.account(&e.claude)
                .expect("account")
                .statusline
                .refresh_interval_sec,
            Some(10)
        );
        assert!(
            plan_refresh(&e.claude, &cfg, Some(10))
                .expect("plan")
                .is_none()
        );

        // Install writes it.
        let plan = ready(plan_install(&e.claude, &cfg, CMD, None).expect("plan"));
        assert!(
            plan.diff().contains("+    \"refreshInterval\": 10"),
            "{}",
            plan.diff()
        );
        plan.apply(&e.config).expect("apply");
        assert_eq!(effective_refresh(&e.claude), Some(10));

        // A change while installed touches only that key, with a diff and a backup.
        let cfg = AppConfig::load(&e.config).expect("cfg");
        let plan = plan_refresh(&e.claude, &cfg, Some(5))
            .expect("plan")
            .expect("a change");
        assert_eq!(
            plan.change,
            Change::Refresh {
                from: Some(10),
                to: Some(5)
            }
        );
        assert!(plan.diff().contains("-    \"refreshInterval\": 10"));
        assert!(plan.apply(&e.config).expect("apply").is_some(), "backup");
        assert_eq!(effective_refresh(&e.claude), Some(5));

        // Off removes the key.
        let cfg = AppConfig::load(&e.config).expect("cfg");
        let plan = plan_refresh(&e.claude, &cfg, None)
            .expect("plan")
            .expect("a change");
        plan.apply(&e.config).expect("apply");
        assert_eq!(effective_refresh(&e.claude), None);
        assert!(!settings(&e).contains("refreshInterval"));

        // Uninstall gives back the original bytes; the choice stays for a reinstall.
        let cfg = AppConfig::load(&e.config).expect("cfg");
        let plan = plan_refresh(&e.claude, &cfg, Some(30))
            .expect("plan")
            .expect("change");
        plan.apply(&e.config).expect("apply");
        let cfg = AppConfig::load(&e.config).expect("cfg");
        ready(plan_uninstall(&e.claude, &cfg).expect("plan"))
            .apply(&e.config)
            .expect("apply");
        assert_eq!(settings(&e), "{\n  \"model\": \"opus\"\n}\n");
        let cfg = AppConfig::load(&e.config).expect("cfg");
        assert_eq!(
            cfg.account(&e.claude)
                .expect("account")
                .statusline
                .refresh_interval_sec,
            Some(30)
        );
    }

    #[test]
    fn the_users_own_interval_comes_back_on_uninstall() {
        let original = "{\"statusLine\": {\"type\": \"command\", \"command\": \"py s.py\", \"refreshInterval\": 2}}";
        let e = env(Some(original));
        let cfg = AppConfig::load(&e.config).expect("cfg");
        ready(plan_install(&e.claude, &cfg, CMD, None).expect("plan"))
            .apply(&e.config)
            .expect("apply");
        assert_eq!(effective_refresh(&e.claude), Some(2));
        let cfg = AppConfig::load(&e.config).expect("cfg");
        plan_refresh(&e.claude, &cfg, Some(5))
            .expect("plan")
            .expect("change")
            .apply(&e.config)
            .expect("apply");
        assert_eq!(effective_refresh(&e.claude), Some(5));
        let cfg = AppConfig::load(&e.config).expect("cfg");
        ready(plan_uninstall(&e.claude, &cfg).expect("plan"))
            .apply(&e.config)
            .expect("apply");
        assert_eq!(settings(&e), original);
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
