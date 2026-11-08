//! Terminal settings files: which font they use, and selecting a Nerd Font in them (Step 12o).
//!
//! Every write follows CLAUDE.md's "user config is sacred" order: plan → diff → the user
//! confirms → the file is re-read and must be unchanged → timestamped backup → atomic write.
//! Undo needs no saved state: the newest backup next to the file holds the value from before.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

use super::fonts::{FontScan, NerdTier};
use crate::domain::Timestamp;
use crate::jsonc::{self, JsoncError};
use crate::statusline::install::backup_path;
use crate::statusline::settings::unified_diff;

/// The terminal a statusline call came from, told apart by its environment.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[serde(rename_all = "snake_case")]
pub enum TerminalKind {
    WindowsTerminal,
    Vscode,
    Other,
}

impl TerminalKind {
    /// Only whether the variables are set is looked at; their values are never kept.
    pub fn detect(env: &dyn Fn(&str) -> Option<OsString>) -> Self {
        // VS Code started from a Windows Terminal tab inherits `WT_SESSION`, so VS Code's own
        // marker wins.
        if env("TERM_PROGRAM").is_some_and(|v| v == "vscode") {
            Self::Vscode
        } else if env("WT_SESSION").is_some_and(|v| !v.is_empty()) {
            Self::WindowsTerminal
        } else {
            Self::Other
        }
    }
}

/// A settings file TokenRadar can write a font into.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[serde(rename_all = "snake_case")]
pub enum TargetId {
    WindowsTerminal,
    WindowsTerminalPreview,
    /// The zip or Scoop install of Windows Terminal.
    WindowsTerminalUnpackaged,
    Vscode,
    VscodeInsiders,
}

const WT_FACE: &[&str] = &["profiles", "defaults", "font", "face"];
const VSCODE_FONT: &[&str] = &["terminal.integrated.fontFamily"];
/// What Windows Terminal draws with when no font is set.
const WT_DEFAULT: &str = "Cascadia Mono";
/// VS Code's default `editor.fontFamily` on Windows, which its terminal falls back to.
const VSCODE_DEFAULT: &str = "Consolas, 'Courier New', monospace";

impl TargetId {
    pub const ALL: [Self; 5] = [
        Self::WindowsTerminal,
        Self::WindowsTerminalPreview,
        Self::WindowsTerminalUnpackaged,
        Self::Vscode,
        Self::VscodeInsiders,
    ];

    pub fn kind(self) -> TerminalKind {
        match self {
            Self::WindowsTerminal
            | Self::WindowsTerminalPreview
            | Self::WindowsTerminalUnpackaged => TerminalKind::WindowsTerminal,
            Self::Vscode | Self::VscodeInsiders => TerminalKind::Vscode,
        }
    }

    /// Where this terminal keeps its settings, whether or not the file exists.
    pub fn path(self, env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
        let local = || env("LOCALAPPDATA").map(PathBuf::from);
        let roaming = || env("APPDATA").map(PathBuf::from);
        let package = |name: &str| {
            local().map(|l| {
                l.join("Packages")
                    .join(name)
                    .join("LocalState")
                    .join("settings.json")
            })
        };
        match self {
            Self::WindowsTerminal => package("Microsoft.WindowsTerminal_8wekyb3d8bbwe"),
            Self::WindowsTerminalPreview => {
                package("Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe")
            }
            Self::WindowsTerminalUnpackaged => local().map(|l| {
                l.join("Microsoft")
                    .join("Windows Terminal")
                    .join("settings.json")
            }),
            Self::Vscode => roaming().map(|r| r.join("Code").join("User").join("settings.json")),
            Self::VscodeInsiders => {
                roaming().map(|r| r.join("Code - Insiders").join("User").join("settings.json"))
            }
        }
    }

    fn key(self) -> &'static [&'static str] {
        match self.kind() {
            TerminalKind::Vscode => VSCODE_FONT,
            _ => WT_FACE,
        }
    }
}

/// The settings files that exist on this machine.
pub fn targets(env: &dyn Fn(&str) -> Option<OsString>) -> Vec<(TargetId, PathBuf)> {
    TargetId::ALL
        .into_iter()
        .filter_map(|id| id.path(env).filter(|p| p.is_file()).map(|p| (id, p)))
        .collect()
}

/// Why a settings file cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TargetProblem {
    Unreadable,
    /// Not JSON (with comments) even after the comments are skipped.
    Invalid,
    /// Windows Terminal's old format: `profiles` is a list, so there is no `defaults`.
    Legacy,
}

/// One settings file, read only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct TargetState {
    pub id: TargetId,
    pub path: String,
    pub problem: Option<TargetProblem>,
    /// The font in effect: the first installed family of the list (or the first one).
    pub font: Option<String>,
    /// `false`: no font is set and the terminal's default applies.
    pub written: bool,
    pub tier: NerdTier,
    /// Windows Terminal profiles that set their own font, so `defaults` does not reach them.
    pub overrides: u32,
    /// A TokenRadar backup holds a different font for this key.
    pub can_undo: bool,
}

pub fn read_state(id: TargetId, path: &Path, scan: &FontScan) -> TargetState {
    let Ok(text) = std::fs::read_to_string(path) else {
        let mut state = state_of(id, path, "", scan);
        state.problem = Some(TargetProblem::Unreadable);
        return state;
    };
    let mut state = state_of(id, path, &text, scan);
    if state.problem.is_none() {
        state.can_undo = undo_source(id, path, &text).is_some();
    }
    state
}

/// [`read_state`] of a given content, without looking for backups.
fn state_of(id: TargetId, path: &Path, text: &str, scan: &FontScan) -> TargetState {
    let mut state = TargetState {
        id,
        path: path.display().to_string(),
        problem: None,
        font: None,
        written: false,
        tier: NerdTier::None,
        overrides: 0,
        can_undo: false,
    };
    let Ok(doc) = jsonc::parse(text) else {
        state.problem = Some(TargetProblem::Invalid);
        return state;
    };
    let list = if id.kind() == TerminalKind::Vscode {
        let set = non_empty(&doc["terminal.integrated.fontFamily"]);
        state.written = set.is_some();
        set.or_else(|| non_empty(&doc["editor.fontFamily"]))
            .unwrap_or(VSCODE_DEFAULT)
            .to_owned()
    } else {
        let profiles = &doc["profiles"];
        if profiles.is_array() {
            state.problem = Some(TargetProblem::Legacy);
            return state;
        }
        let defaults = &profiles["defaults"];
        let set = non_empty(&defaults["font"]["face"]).or_else(|| non_empty(&defaults["fontFace"]));
        state.written = set.is_some();
        state.overrides = profiles["list"].as_array().map_or(0, |list| {
            let own = list.iter().filter(|p| {
                non_empty(&p["font"]["face"]).is_some() || non_empty(&p["fontFace"]).is_some()
            });
            u32::try_from(own.count()).unwrap_or(u32::MAX)
        });
        set.unwrap_or(WT_DEFAULT).to_owned()
    };
    let font = in_effect(&list, scan);
    state.tier = scan.family(&font).map_or(NerdTier::None, |f| f.tier);
    state.font = Some(font);
    state
}

fn non_empty(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| !s.trim().is_empty())
}

/// The families of a CSS-style list (`'A B', C`), unquoted.
fn families(list: &str) -> impl Iterator<Item = &str> {
    list.split(',')
        .map(|f| f.trim().trim_matches(['\'', '"']).trim())
        .filter(|f| !f.is_empty())
}

/// The family a terminal draws with: the first one installed, as fallbacks are skipped over.
fn in_effect(list: &str, scan: &FontScan) -> String {
    families(list)
        .find(|f| scan.family(f).is_some())
        .or_else(|| families(list).next())
        .unwrap_or(list)
        .to_owned()
}

#[derive(Debug, thiserror::Error)]
pub enum TargetError {
    #[error("could not read or write {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{0} is not valid JSON")]
    Invalid(PathBuf),
    #[error("{0} uses the old Windows Terminal format")]
    Legacy(PathBuf),
    #[error("{0} changed after the diff was shown")]
    Changed(PathBuf),
}

/// What a plan does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum FontChange {
    Select,
    Undo,
}

/// A change to one settings file, ready to be shown and applied.
#[derive(Debug, Clone)]
pub struct FontPlan {
    pub id: TargetId,
    pub path: PathBuf,
    pub change: FontChange,
    original: String,
    updated: String,
}

impl FontPlan {
    pub fn diff(&self) -> String {
        unified_diff(
            &self.original,
            &self.updated,
            &self.path.display().to_string(),
        )
    }

    /// The file before and after the plan, as the settings card shows them.
    pub fn effect(&self, scan: &FontScan) -> (TargetState, TargetState) {
        (
            state_of(self.id, &self.path, &self.original, scan),
            state_of(self.id, &self.path, &self.updated, scan),
        )
    }

    /// Applies the plan and returns the backup's path.
    pub fn apply(&self) -> Result<PathBuf, TargetError> {
        let io = |path: &Path| {
            let path = path.to_path_buf();
            move |source| TargetError::Io { path, source }
        };
        let current = std::fs::read_to_string(&self.path).map_err(io(&self.path))?;
        if current != self.original {
            return Err(TargetError::Changed(self.path.clone()));
        }
        let mut backup = backup_path(&self.path, Timestamp::now());
        if self.change == FontChange::Undo {
            // An undo's own backup is not a source for the next undo.
            backup.set_extension("undo.bak");
        }
        std::fs::write(&backup, &self.original).map_err(io(&backup))?;
        crate::fsutil::atomic_write(&self.path, self.updated.as_bytes()).map_err(io(&self.path))?;
        Ok(backup)
    }
}

fn read(path: &Path) -> Result<String, TargetError> {
    std::fs::read_to_string(path).map_err(|source| TargetError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn map_err(path: &Path) -> impl Fn(JsoncError) -> TargetError + '_ {
    move |e| match e {
        JsoncError::Invalid => TargetError::Invalid(path.to_path_buf()),
        JsoncError::NotAnObject(_) => TargetError::Legacy(path.to_path_buf()),
    }
}

/// Plans selecting `family` in the file. `None`: it is already the font in effect.
pub fn plan_select(
    id: TargetId,
    path: &Path,
    family: &str,
) -> Result<Option<FontPlan>, TargetError> {
    let text = read(path)?;
    let err = map_err(path);
    let raw = if id.kind() == TerminalKind::Vscode {
        let doc = jsonc::parse(&text).map_err(&err)?;
        let list = non_empty(&doc["terminal.integrated.fontFamily"])
            .or_else(|| non_empty(&doc["editor.fontFamily"]))
            .unwrap_or(VSCODE_DEFAULT);
        if families(list)
            .next()
            .is_some_and(|f| f.eq_ignore_ascii_case(family))
        {
            return Ok(None);
        }
        // The user's fallbacks stay behind the new font.
        let mut new = format!("'{family}'");
        for f in list.split(',').filter(|f| {
            let name = f.trim().trim_matches(['\'', '"']).trim();
            !name.is_empty() && !name.eq_ignore_ascii_case(family)
        }) {
            new.push_str(", ");
            new.push_str(f.trim());
        }
        serde_json::to_string(&new).map_err(|_| TargetError::Invalid(path.to_path_buf()))?
    } else {
        let raw =
            serde_json::to_string(family).map_err(|_| TargetError::Invalid(path.to_path_buf()))?;
        if jsonc::get(&text, WT_FACE).map_err(&err)? == Some(raw.as_str()) {
            return Ok(None);
        }
        raw
    };
    let updated = jsonc::set(&text, id.key(), &raw).map_err(&err)?;
    Ok(Some(FontPlan {
        id,
        path: path.to_path_buf(),
        change: FontChange::Select,
        original: text,
        updated,
    }))
}

/// Plans putting back the font from before TokenRadar's last change. `None`: nothing to undo.
pub fn plan_undo(id: TargetId, path: &Path) -> Result<Option<FontPlan>, TargetError> {
    let text = read(path)?;
    let Some(updated) = undo_source(id, path, &text) else {
        return Ok(None);
    };
    Ok(Some(FontPlan {
        id,
        path: path.to_path_buf(),
        change: FontChange::Undo,
        original: text,
        updated,
    }))
}

/// Plans the undo of a restore-everything: only while the key still starts with a Nerd Font
/// (or with `ours`, the family TokenRadar installs, should it already be gone). A font the user
/// picked themselves since then is theirs, and stays. `None`: nothing of TokenRadar's is left.
pub fn plan_restore(
    id: TargetId,
    path: &Path,
    scan: &FontScan,
    ours: &str,
) -> Result<Option<FontPlan>, TargetError> {
    let text = read(path)?;
    let doc = jsonc::parse(&text).map_err(map_err(path))?;
    let value = id.key().iter().fold(&doc, |v, k| &v[*k]);
    let Some(first) = non_empty(value).and_then(|list| families(list).next()) else {
        return Ok(None);
    };
    let nerd = first.eq_ignore_ascii_case(ours)
        || scan.family(first).is_some_and(|f| f.tier != NerdTier::None);
    if !nerd {
        return Ok(None);
    }
    plan_undo(id, path)
}

/// The file content an undo writes, when the newest backup holds a different font.
fn undo_source(id: TargetId, path: &Path, text: &str) -> Option<String> {
    let key = id.key();
    let backup = std::fs::read_to_string(newest_backup(path)?).ok()?;
    let now = jsonc::get(text, key).ok()?;
    let before = jsonc::get(&backup, key).ok()?;
    if now == before {
        return None;
    }
    // Untouched since TokenRadar wrote it: the backup comes back byte for byte.
    if let Some(now) = now
        && jsonc::set(&backup, key, now).ok().as_deref() == Some(text)
    {
        return Some(backup);
    }
    // Changed since: only this key goes back, every other edit stays.
    if let Some(raw) = before {
        jsonc::set(text, key, raw).ok()
    } else {
        let mut out = jsonc::remove(text, key).ok()?;
        // Objects the change created go too, if they are empty now.
        for depth in (1..key.len()).rev() {
            let parent = &key[..depth];
            let created = jsonc::get(&backup, parent).ok()?.is_none();
            let empty = jsonc::get(&out, parent).ok()?.is_some_and(|raw| {
                raw.trim_start_matches('{')
                    .trim_end_matches('}')
                    .trim()
                    .is_empty()
            });
            if !(created && empty) {
                break;
            }
            out = jsonc::remove(&out, parent).ok()?;
        }
        Some(out)
    }
}

/// The newest `settings.json.tokenradar-<UTC time>.bak` next to `path`, undo backups excluded.
fn newest_backup(path: &Path) -> Option<PathBuf> {
    let dir = path.parent()?;
    let prefix = format!("{}.tokenradar-", path.file_name()?.to_string_lossy());
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let stamp = name.strip_prefix(&prefix)?.strip_suffix(".bak")?;
            (!stamp.to_ascii_lowercase().ends_with(".undo")).then(|| (stamp.to_owned(), e.path()))
        })
        .max()
        .map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_font::fonts::Face;

    const FAMILY: &str = "CaskaydiaMono Nerd Font Mono";

    fn scan() -> FontScan {
        let face = |family: &str, short: Option<&str>, tier| Face {
            family: family.into(),
            short_family: short.map(Into::into),
            tier,
            mono: true,
            path: PathBuf::new(),
        };
        FontScan {
            faces: vec![
                face(FAMILY, Some("CaskaydiaMono NFM"), NerdTier::Full),
                face("Consolas", None, NerdTier::None),
                face("Cascadia Mono", None, NerdTier::None),
            ],
        }
    }

    fn env_of(vars: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
        move |k| {
            vars.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| OsString::from(v))
        }
    }

    #[test]
    fn terminal_kind_comes_from_markers_only() {
        assert_eq!(
            TerminalKind::detect(&env_of(&[("WT_SESSION", "x")])),
            TerminalKind::WindowsTerminal
        );
        assert_eq!(
            TerminalKind::detect(&env_of(&[("WT_SESSION", "x"), ("TERM_PROGRAM", "vscode")])),
            TerminalKind::Vscode
        );
        assert_eq!(
            TerminalKind::detect(&env_of(&[("TERM_PROGRAM", "WezTerm")])),
            TerminalKind::Other
        );
        assert_eq!(
            TerminalKind::detect(&env_of(&[("WT_SESSION", "")])),
            TerminalKind::Other
        );
        assert_eq!(
            serde_json::to_string(&TerminalKind::WindowsTerminal).expect("json"),
            "\"windows_terminal\""
        );
    }

    #[test]
    fn paths_follow_the_install_kinds() {
        let env = env_of(&[("LOCALAPPDATA", "L"), ("APPDATA", "R")]);
        let p = |id: TargetId| id.path(&env).expect("path");
        assert!(p(TargetId::WindowsTerminal).ends_with(
            r"Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState/settings.json"
        ));
        assert!(
            p(TargetId::WindowsTerminalUnpackaged)
                .ends_with("Microsoft/Windows Terminal/settings.json")
        );
        assert!(p(TargetId::VscodeInsiders).ends_with("Code - Insiders/User/settings.json"));
        assert!(TargetId::Vscode.path(&env_of(&[])).is_none());
    }

    const WT: &str = "// This file was initially generated by Windows Terminal\n{\n    \"$schema\": \"https://aka.ms/terminal-profiles-schema\",\n    \"profiles\": \n    {\n        \"defaults\": {},\n        \"list\": \n        [\n            { \"name\": \"cmd\", \"font\": { \"face\": \"Consolas\" } },\n            { \"name\": \"pwsh\" }, // mine\n        ]\n    }\n}\n";

    fn seeded(name: &str, text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(name);
        std::fs::write(&path, text).expect("seed");
        (dir, path)
    }

    #[test]
    fn windows_terminal_select_backup_and_undo() {
        let (dir, path) = seeded("settings.json", WT);
        let before = read_state(TargetId::WindowsTerminal, &path, &scan());
        assert_eq!(
            (
                before.font.as_deref(),
                before.written,
                before.tier,
                before.overrides,
                before.can_undo
            ),
            (Some("Cascadia Mono"), false, NerdTier::None, 1, false)
        );
        let plan = plan_select(TargetId::WindowsTerminal, &path, FAMILY)
            .expect("plan")
            .expect("change");
        assert!(
            plan.diff()
                .contains("+                \"face\": \"CaskaydiaMono Nerd Font Mono\""),
            "{}",
            plan.diff()
        );
        let backup = plan.apply().expect("apply");
        assert_eq!(std::fs::read_to_string(&backup).expect("backup"), WT);
        let after = read_state(TargetId::WindowsTerminal, &path, &scan());
        assert_eq!(
            (
                after.font.as_deref(),
                after.written,
                after.tier,
                after.can_undo
            ),
            (Some(FAMILY), true, NerdTier::Full, true)
        );
        assert!(
            std::fs::read_to_string(&path)
                .expect("read")
                .contains("// mine")
        );
        assert!(
            plan_select(TargetId::WindowsTerminal, &path, FAMILY)
                .expect("plan")
                .is_none()
        );

        let undo = plan_undo(TargetId::WindowsTerminal, &path)
            .expect("plan")
            .expect("undo");
        undo.apply().expect("undo");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), WT);
        assert!(!read_state(TargetId::WindowsTerminal, &path, &scan()).can_undo);
        assert!(
            plan_undo(TargetId::WindowsTerminal, &path)
                .expect("plan")
                .is_none()
        );
        assert_eq!(
            std::fs::read_dir(dir.path()).expect("ls").count(),
            3,
            "file, backup, undo backup"
        );
    }

    #[test]
    fn undo_after_other_edits_only_reverts_the_font() {
        let (_dir, path) = seeded("settings.json", WT);
        plan_select(TargetId::WindowsTerminal, &path, FAMILY)
            .expect("plan")
            .expect("change")
            .apply()
            .expect("apply");
        let edited = std::fs::read_to_string(&path)
            .expect("read")
            .replace("\"pwsh\"", "\"PowerShell 7\"");
        std::fs::write(&path, &edited).expect("edit");
        plan_undo(TargetId::WindowsTerminal, &path)
            .expect("plan")
            .expect("undo")
            .apply()
            .expect("undo");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            WT.replace("\"pwsh\"", "\"PowerShell 7\"")
        );
    }

    #[test]
    fn restore_reverts_only_what_is_still_tokenradars() {
        let (_dir, path) = seeded("settings.json", WT);
        let id = TargetId::WindowsTerminal;
        assert!(
            plan_restore(id, &path, &scan(), FAMILY)
                .expect("plan")
                .is_none()
        );
        plan_select(id, &path, FAMILY)
            .expect("plan")
            .expect("change")
            .apply()
            .expect("apply");
        let plan = plan_restore(id, &path, &scan(), FAMILY)
            .expect("plan")
            .expect("restore");
        let (before, after) = plan.effect(&scan());
        assert_eq!(
            (before.font.as_deref(), after.font.as_deref(), after.written),
            (Some(FAMILY), Some("Cascadia Mono"), false)
        );
        // The font is gone from the scan already: the family name still marks the change.
        let empty = FontScan { faces: vec![] };
        assert!(
            plan_restore(id, &path, &empty, FAMILY)
                .expect("plan")
                .is_some()
        );
        // The user picked another font since: theirs stays.
        let text = std::fs::read_to_string(&path).expect("read");
        std::fs::write(&path, text.replace(FAMILY, "Consolas")).expect("edit");
        assert!(read_state(id, &path, &scan()).can_undo);
        assert!(
            plan_restore(id, &path, &scan(), FAMILY)
                .expect("plan")
                .is_none()
        );
    }

    #[test]
    fn vscode_keeps_the_fallbacks_behind_the_new_font() {
        let text = "{\r\n    // fonts\r\n    \"editor.fontFamily\": \"'Fira Code', Consolas, monospace\",\r\n}";
        let (_dir, path) = seeded("settings.json", text);
        let before = read_state(TargetId::Vscode, &path, &scan());
        // Fira Code is not installed, so Consolas is in effect.
        assert_eq!(
            (before.font.as_deref(), before.written),
            (Some("Consolas"), false)
        );
        let plan = plan_select(TargetId::Vscode, &path, FAMILY)
            .expect("plan")
            .expect("change");
        plan.apply().expect("apply");
        let out = std::fs::read_to_string(&path).expect("read");
        assert!(out.ends_with("monospace\",\r\n    \"terminal.integrated.fontFamily\": \"'CaskaydiaMono Nerd Font Mono', 'Fira Code', Consolas, monospace\",\r\n}"), "{out}");
        assert_eq!(
            read_state(TargetId::Vscode, &path, &scan()).tier,
            NerdTier::Full
        );
        plan_undo(TargetId::Vscode, &path)
            .expect("plan")
            .expect("undo")
            .apply()
            .expect("undo");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), text);
    }

    #[test]
    fn a_changed_file_is_not_overwritten() {
        let (_dir, path) = seeded("settings.json", "{}");
        let plan = plan_select(TargetId::Vscode, &path, FAMILY)
            .expect("plan")
            .expect("change");
        std::fs::write(&path, "{ \"x\": 1 }").expect("edit");
        assert!(matches!(plan.apply(), Err(TargetError::Changed(_))));
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "{ \"x\": 1 }"
        );
    }

    #[test]
    fn legacy_and_broken_files_are_reported() {
        let (_d1, legacy) = seeded(
            "settings.json",
            "{ \"profiles\": [ { \"name\": \"cmd\" } ] }",
        );
        assert_eq!(
            read_state(TargetId::WindowsTerminal, &legacy, &scan()).problem,
            Some(TargetProblem::Legacy)
        );
        assert!(matches!(
            plan_select(TargetId::WindowsTerminal, &legacy, FAMILY),
            Err(TargetError::Legacy(_))
        ));
        let (_d2, broken) = seeded("settings.json", "{ \"a\": ");
        assert_eq!(
            read_state(TargetId::Vscode, &broken, &scan()).problem,
            Some(TargetProblem::Invalid)
        );
        assert!(matches!(
            plan_select(TargetId::Vscode, &broken, FAMILY),
            Err(TargetError::Invalid(_))
        ));
    }
}
