//! `tokenradar mod install|uninstall`, also used by the settings window and onboarding
//! (Step 12g, ADR-047): Claude Code loads TokenRadar's mod from the install directory once
//! that directory is listed in `env.CLAUDE_CODE_PLUGIN_DIRS` of its `settings.json`.
//!
//! The change follows the statusline's order (ADR-009): plan → diff → confirm → the file is
//! re-read and must be unchanged → backup → atomic write. Only that one key changes. Other
//! directories in the list are kept; uninstalling removes ours alone, then the key once it is
//! empty, then `env` once that is empty, so a file TokenRadar added both to comes back byte
//! for byte. No state is kept in `config.toml`: the list itself says what is installed.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::install::{InstallError, backup_path, read_optional, read_settings};
use super::payload::same_dir;
use super::settings::{self, EditError};
use crate::domain::Timestamp;

pub const ENV_KEY: &str = "env";
pub const PLUGIN_DIRS_KEY: &str = "CLAUDE_CODE_PLUGIN_DIRS";

/// The mod's folder name inside the install directory.
pub const MOD_DIR_NAME: &str = "claude-code";

/// Claude Code splits the list with the platform's path-list separator.
const SEPARATOR: char = if cfg!(windows) { ';' } else { ':' };

/// What a plan would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModChange {
    Install {
        /// Directories already in the list; they stay.
        kept: Vec<String>,
    },
    Uninstall {
        /// Directories that stay in the list after ours is removed.
        kept: Vec<String>,
    },
}

#[derive(Debug)]
pub enum ModPlanned {
    Ready(ModPlan),
    AlreadyInstalled,
    NotInstalled,
}

/// Whether `settings.json` lists the mod directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModState {
    Installed,
    NotInstalled,
}

/// A change to one `settings.json`, ready to be shown and applied.
#[derive(Debug, Clone)]
pub struct ModPlan {
    pub settings_path: PathBuf,
    pub change: ModChange,
    original: Option<String>,
    updated: String,
}

impl ModPlan {
    pub fn diff(&self) -> String {
        let label = self.settings_path.display().to_string();
        settings::unified_diff(
            self.original.as_deref().unwrap_or(""),
            &self.updated,
            &label,
        )
    }

    pub fn updated(&self) -> &str {
        &self.updated
    }

    /// Applies the plan. Returns the backup path (`None` when there was no file to back up).
    pub fn apply(&self) -> Result<Option<PathBuf>, InstallError> {
        let current = read_optional(&self.settings_path)?;
        if current != self.original {
            return Err(InstallError::Changed(self.settings_path.clone()));
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
        Ok(backup)
    }
}

/// The mod directory next to a binary: `<install>\claude-code` beside the app, or one level
/// up from the CLI in `<install>\bin`. `None` when neither holds the mod's manifest.
pub fn mod_dir_near(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    [Some(dir), dir.parent()]
        .into_iter()
        .flatten()
        .map(|d| d.join(MOD_DIR_NAME))
        .find(|d| d.join(".claude-plugin").join("plugin.json").is_file())
}

/// Whether `config_dir`'s `settings.json` lists `mod_dir`. A missing or unreadable file lists
/// nothing.
pub fn state(config_dir: &Path, mod_dir: &Path) -> ModState {
    let text =
        std::fs::read_to_string(config_dir.join(super::install::SETTINGS_FILE)).unwrap_or_default();
    let listed = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| {
            v.get(ENV_KEY)?
                .get(PLUGIN_DIRS_KEY)?
                .as_str()
                .map(str::to_owned)
        })
        .is_some_and(|list| entries(&list).iter().any(|e| is_ours(e, mod_dir)));
    if listed {
        ModState::Installed
    } else {
        ModState::NotInstalled
    }
}

/// Plans listing `mod_dir` in `config_dir`'s `settings.json`.
pub fn plan_install(config_dir: &Path, mod_dir: &Path) -> Result<ModPlanned, InstallError> {
    let (settings_path, original) = read_settings(config_dir)?;
    let text = original.as_deref().unwrap_or("{}");
    let invalid = |EditError::Invalid| InstallError::Invalid(settings_path.clone());
    let ours = mod_dir.display().to_string();
    let updated = match settings::raw_member(text, ENV_KEY).map_err(invalid)? {
        None => {
            let mut env = Map::new();
            env.insert(PLUGIN_DIRS_KEY.to_owned(), Value::String(ours));
            let mut updated = settings::set_member(text, ENV_KEY, &env).map_err(invalid)?;
            if original.is_none() {
                updated.push('\n');
            }
            (updated, Vec::new())
        }
        Some(env) => {
            let kept = match list_in(env) {
                Listed::Absent => Vec::new(),
                Listed::List(list) => entries(&list),
                Listed::Foreign => return Err(InstallError::Invalid(settings_path.clone())),
            };
            if kept.iter().any(|e| is_ours(e, mod_dir)) {
                return Ok(ModPlanned::AlreadyInstalled);
            }
            let value = kept
                .iter()
                .cloned()
                .chain([ours])
                .collect::<Vec<_>>()
                .join(&SEPARATOR.to_string());
            let env = settings::set_member(env, PLUGIN_DIRS_KEY, &value).map_err(invalid)?;
            let updated = settings::set_member_raw(text, ENV_KEY, &env).map_err(invalid)?;
            (updated, kept)
        }
    };
    let (updated, kept) = updated;
    Ok(ModPlanned::Ready(ModPlan {
        settings_path,
        change: ModChange::Install { kept },
        original,
        updated,
    }))
}

/// Plans removing `mod_dir` from `config_dir`'s `settings.json`.
pub fn plan_uninstall(config_dir: &Path, mod_dir: &Path) -> Result<ModPlanned, InstallError> {
    let (settings_path, original) = read_settings(config_dir)?;
    let Some(text) = original.as_deref() else {
        return Ok(ModPlanned::NotInstalled);
    };
    let invalid = |EditError::Invalid| InstallError::Invalid(settings_path.clone());
    let Some(env) = settings::raw_member(text, ENV_KEY).map_err(invalid)? else {
        return Ok(ModPlanned::NotInstalled);
    };
    let Listed::List(list) = list_in(env) else {
        return Ok(ModPlanned::NotInstalled);
    };
    let all = entries(&list);
    if !all.iter().any(|e| is_ours(e, mod_dir)) {
        return Ok(ModPlanned::NotInstalled);
    }
    let kept: Vec<String> = all.into_iter().filter(|e| !is_ours(e, mod_dir)).collect();
    let env_after = if kept.is_empty() {
        settings::remove_member(env, PLUGIN_DIRS_KEY)
    } else {
        settings::set_member(env, PLUGIN_DIRS_KEY, &kept.join(&SEPARATOR.to_string()))
    }
    .map_err(invalid)?;
    let empty = serde_json::from_str::<Map<String, Value>>(&env_after).is_ok_and(|m| m.is_empty());
    let updated = if empty {
        settings::remove_member(text, ENV_KEY)
    } else {
        settings::set_member_raw(text, ENV_KEY, &env_after)
    }
    .map_err(invalid)?;
    Ok(ModPlanned::Ready(ModPlan {
        settings_path,
        change: ModChange::Uninstall { kept },
        original,
        updated,
    }))
}

/// What `env` holds under the plugin directory key.
enum Listed {
    Absent,
    List(String),
    /// `env` is not an object or the value is not a string: left alone rather than guessed at.
    Foreign,
}

fn list_in(env: &str) -> Listed {
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(env) else {
        return Listed::Foreign;
    };
    match map.get(PLUGIN_DIRS_KEY) {
        None => Listed::Absent,
        Some(Value::String(s)) => Listed::List(s.clone()),
        Some(_) => Listed::Foreign,
    }
}

fn entries(list: &str) -> Vec<String> {
    list.split(SEPARATOR)
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(str::to_owned)
        .collect()
}

fn is_ours(entry: &str, mod_dir: &Path) -> bool {
    same_dir(Path::new(entry), mod_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Env {
        _dir: tempfile::TempDir,
        claude: PathBuf,
        mod_dir: PathBuf,
    }

    fn env(settings: Option<&str>) -> Env {
        let dir = tempfile::tempdir().expect("tempdir");
        let claude = dir.path().join(".claude");
        std::fs::create_dir(&claude).expect("mkdir");
        if let Some(s) = settings {
            std::fs::write(claude.join("settings.json"), s).expect("seed");
        }
        let mod_dir = dir.path().join("TokenRadar").join(MOD_DIR_NAME);
        Env {
            _dir: dir,
            claude,
            mod_dir,
        }
    }

    fn ready(p: ModPlanned) -> ModPlan {
        match p {
            ModPlanned::Ready(plan) => plan,
            other => panic!("expected a plan, got {other:?}"),
        }
    }

    fn settings(e: &Env) -> String {
        std::fs::read_to_string(e.claude.join("settings.json")).expect("read")
    }

    fn json(e: &Env) -> Value {
        serde_json::from_str(&settings(e)).expect("json")
    }

    /// Install, then uninstall: the original bytes come back.
    fn cycle(original: &str) -> Env {
        let e = env(Some(original));
        let plan = ready(plan_install(&e.claude, &e.mod_dir).expect("plan"));
        let backup = plan.apply().expect("apply").expect("backup");
        assert_eq!(std::fs::read_to_string(backup).expect("backup"), original);
        assert_eq!(state(&e.claude, &e.mod_dir), ModState::Installed);
        assert!(matches!(
            plan_install(&e.claude, &e.mod_dir).expect("plan"),
            ModPlanned::AlreadyInstalled
        ));
        ready(plan_uninstall(&e.claude, &e.mod_dir).expect("plan"))
            .apply()
            .expect("apply");
        assert_eq!(settings(&e), original, "uninstall restores the exact bytes");
        assert_eq!(state(&e.claude, &e.mod_dir), ModState::NotInstalled);
        e
    }

    #[test]
    fn install_then_uninstall_is_byte_identical() {
        cycle("{\n  \"model\": \"opus\",\n  \"theme\": \"dark\"\n}\n");
        cycle("{\n  \"env\": {\n    \"FOO\": \"1\"\n  },\n  \"model\": \"opus\"\n}\n");
        cycle("{\r\n    \"env\": {\"A\": \"b\"}\r\n}");
        cycle("{}");
    }

    #[test]
    fn installing_adds_the_directory_and_keeps_every_other_setting() {
        let e = env(Some("{\n  \"env\": {\n    \"FOO\": \"1\"\n  }\n}\n"));
        let plan = ready(plan_install(&e.claude, &e.mod_dir).expect("plan"));
        assert_eq!(plan.change, ModChange::Install { kept: Vec::new() });
        plan.apply().expect("apply");
        let v = json(&e);
        assert_eq!(v["env"]["FOO"], "1");
        assert_eq!(
            v["env"][PLUGIN_DIRS_KEY].as_str(),
            Some(e.mod_dir.display().to_string().as_str())
        );
    }

    #[test]
    fn other_plugin_directories_are_kept_both_ways() {
        let other = if cfg!(windows) {
            "D:\\dev\\my-mod"
        } else {
            "/dev/my-mod"
        };
        let original = serde_json::json!({ "env": { PLUGIN_DIRS_KEY: other } }).to_string();
        let e = env(Some(&original));
        let plan = ready(plan_install(&e.claude, &e.mod_dir).expect("plan"));
        assert_eq!(
            plan.change,
            ModChange::Install {
                kept: vec![other.to_owned()]
            }
        );
        plan.apply().expect("apply");
        let list = json(&e)["env"][PLUGIN_DIRS_KEY]
            .as_str()
            .expect("list")
            .to_owned();
        assert_eq!(list, format!("{other}{SEPARATOR}{}", e.mod_dir.display()));
        let plan = ready(plan_uninstall(&e.claude, &e.mod_dir).expect("plan"));
        assert_eq!(
            plan.change,
            ModChange::Uninstall {
                kept: vec![other.to_owned()]
            }
        );
        plan.apply().expect("apply");
        assert_eq!(settings(&e), original);
    }

    #[test]
    fn a_missing_file_is_created_and_a_foreign_value_left_alone() {
        let e = env(None);
        let plan = ready(plan_install(&e.claude, &e.mod_dir).expect("plan"));
        assert_eq!(plan.apply().expect("apply"), None, "no file, no backup");
        assert!(settings(&e).ends_with("}\n"));
        assert_eq!(state(&e.claude, &e.mod_dir), ModState::Installed);

        // A list that is not a string is not ours to rewrite.
        let e = env(Some("{\"env\": {\"CLAUDE_CODE_PLUGIN_DIRS\": 3}}"));
        assert!(matches!(
            plan_install(&e.claude, &e.mod_dir),
            Err(InstallError::Invalid(_))
        ));
        assert!(matches!(
            plan_uninstall(&e.claude, &e.mod_dir).expect("plan"),
            ModPlanned::NotInstalled
        ));
    }

    #[test]
    fn a_file_changed_after_the_diff_is_not_overwritten() {
        let e = env(Some("{}"));
        let plan = ready(plan_install(&e.claude, &e.mod_dir).expect("plan"));
        std::fs::write(e.claude.join("settings.json"), "{\"model\": \"x\"}").expect("edit");
        assert!(matches!(plan.apply(), Err(InstallError::Changed(_))));
        assert_eq!(settings(&e), "{\"model\": \"x\"}");
    }

    #[test]
    fn the_mod_is_found_beside_the_app_or_above_the_cli() {
        let dir = tempfile::tempdir().expect("tempdir");
        let install = dir.path();
        let manifest = install.join(MOD_DIR_NAME).join(".claude-plugin");
        std::fs::create_dir_all(&manifest).expect("mkdir");
        std::fs::write(manifest.join("plugin.json"), "{}").expect("write");
        let want = Some(install.join(MOD_DIR_NAME));
        assert_eq!(mod_dir_near(&install.join("tokenradar-app.exe")), want);
        assert_eq!(
            mod_dir_near(&install.join("bin").join("tokenradar.exe")),
            want
        );
        assert_eq!(
            mod_dir_near(&dir.path().join("x").join("y").join("z.exe")),
            None
        );
    }
}
