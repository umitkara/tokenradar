//! "Data sources" in the settings: Claude Code directories, the statusline bridge, the Claude
//! Code mod (ADR-047) and the usage API consent.
//!
//! `settings.json` changes follow ADR-009: the window asks for a plan, shows its diff, and
//! only the plan the user saw can be applied (by its token). `Plan::apply` re-reads the file
//! and refuses if it changed in between.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::PoisonError;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt as _;
use tokenradar_core::config::{self, API_CONSENT_VERSION, AppConfig};
use tokenradar_core::domain::Timestamp;
use tokenradar_core::limits::ApiStatusView;
use tokenradar_core::paths::Paths;
use tokenradar_core::providers::claude::{self, AccountDescriptor};
use tokenradar_core::statusline::claude_mod::{self, ModChange, ModPlan, ModPlanned, ModState};
use tokenradar_core::statusline::install::{
    Change, InstallError, Plan, Planned, plan_install, plan_uninstall,
};
use tokenradar_core::statusline::same_dir;
use tokenradar_core::statusline::shell::{Shell, exe_for_command};
use tokenradar_core::store::{AccountStats, ReadOnlyStore};
use tokenradar_core::terminal_font::terminals::FontPlan;

use crate::core_bridge;

/// The CLI next to the app: `bin\tokenradar.exe` once installed (architecture §2), beside the
/// app in a development build.
const CLI_CANDIDATES: [&str; 2] = ["bin/tokenradar.exe", "tokenradar.exe"];

/// One Claude Code directory the app watches.
#[derive(Debug, Clone, Serialize, Type)]
pub struct ClaudeDirView {
    pub path: String,
    /// The panel shows this account.
    pub primary: bool,
    /// Picked in the settings.
    pub chosen: bool,
    /// `None` until the app indexed it.
    pub stats: Option<AccountStats>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum StatuslineView {
    /// No Claude Code directory: nothing to install into.
    NoDirectory,
    NotInstalled {
        /// The user's own statusline command, kept and chained by default.
        current_command: Option<String>,
    },
    Installed {
        /// Chained: this command still runs before or after TokenRadar's line.
        chained_command: Option<String>,
    },
    /// TokenRadar runs as the statusline but the app did not install it (CLI, by hand).
    InstalledElsewhere,
}

/// TokenRadar's Claude Code mod in the primary account's `settings.json`.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ClaudeModView {
    /// No Claude Code directory: nothing to install into.
    NoDirectory,
    /// The mod's files are not next to the app (a development build without them).
    Missing,
    NotInstalled,
    Installed {
        /// The mod's folder, as listed in `CLAUDE_CODE_PLUGIN_DIRS`.
        path: String,
    },
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ApiConsentView {
    pub granted: bool,
    /// RFC 3339, when the consent was given.
    pub granted_at: Option<String>,
    /// A consent for an older text no longer counts and is asked again.
    pub outdated: bool,
    /// The poller's state; `None` while the core is not running.
    pub status: Option<ApiStatusView>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct DataSourcesView {
    pub dirs: Vec<ClaudeDirView>,
    /// A picked directory that no longer exists.
    pub chosen_missing: Option<String>,
    /// `CLAUDE_CONFIG_DIR`, shown to explain where a directory came from.
    pub env_dir: Option<String>,
    pub statusline: StatuslineView,
    /// Last statusline reading the app received.
    pub statusline_last_seen: Option<Timestamp>,
    /// The CLI could not be found, so the bridge cannot be installed from here.
    pub cli_missing: bool,
    pub claude_mod: ClaudeModView,
    pub api: ApiConsentView,
}

/// One line of a unified diff, classified for colouring.
#[derive(Debug, Clone, Serialize, Type)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    Header,
    Hunk,
    Added,
    Removed,
    Context,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum StatuslineAction {
    Install,
    /// Install without keeping the previous command.
    InstallNoChain,
    Uninstall,
}

/// A planned `settings.json` change, shown before anything is written.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum StatuslinePlanView {
    Ready {
        /// Pass back to `apply_statusline`; only this plan can be applied.
        token: u32,
        settings_path: String,
        diff: Vec<DiffLine>,
        /// Install: the previous command keeps running (chained).
        chained_command: Option<String>,
        /// Install: Git Bash would drop the backslashes of the chained command.
        chained_mangled: bool,
        /// Uninstall: the previous value comes back.
        restores_previous: bool,
    },
    AlreadyInstalled,
    NotInstalled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeModAction {
    Install,
    Uninstall,
}

/// A planned change for the mod, shown before anything is written.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ClaudeModPlanView {
    Ready {
        /// Pass back to `apply_claude_mod`; only this plan can be applied.
        token: u32,
        settings_path: String,
        diff: Vec<DiffLine>,
        /// Other plugin directories in the list; they stay.
        kept: Vec<String>,
    },
    AlreadyInstalled,
    NotInstalled,
}

/// Why a `settings.json` change failed; the window words it.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum StatuslineError {
    NoDirectory,
    CliMissing,
    /// The mod's files are not next to the app.
    ModMissing,
    /// `settings.json` is not a JSON object; nothing was written.
    InvalidSettings,
    /// The file changed after the diff was shown; nothing was written.
    ChangedMeanwhile,
    /// The plan was already applied or replaced by a newer one.
    StalePlan,
    Io {
        message: String,
    },
}

/// A plan shown to the user: the statusline's, the mod's or a terminal font's (Step 12o).
pub(crate) enum Shown {
    Statusline(Plan),
    Mod(ModPlan),
    Font(FontPlan),
}

/// The plan last shown to the user. Managed by Tauri.
#[derive(Default)]
pub struct Pending {
    next: AtomicU32,
    plan: Mutex<Option<(u32, Shown)>>,
}

impl Pending {
    /// Keeps `plan` as the one shown; returns its token.
    pub(crate) fn show(&self, plan: Shown) -> u32 {
        let token = self.next.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        *self.plan.lock().unwrap_or_else(PoisonError::into_inner) = Some((token, plan));
        token
    }

    /// The plan with `token`, taken; a stale token leaves the shown plan in place.
    pub(crate) fn take(&self, token: u32) -> Option<Shown> {
        let mut slot = self.plan.lock().unwrap_or_else(PoisonError::into_inner);
        match slot.take() {
            Some((t, plan)) if t == token => Some(plan),
            other => {
                *slot = other;
                None
            }
        }
    }
}

pub fn init(app: &AppHandle) {
    app.manage(Pending::default());
}

fn paths() -> Result<Paths, String> {
    Paths::resolve().map_err(|e| e.to_string())
}

fn load_config(paths: &Paths) -> AppConfig {
    AppConfig::load(paths.config_path()).unwrap_or_default()
}

fn accounts(config: &AppConfig) -> Vec<AccountDescriptor> {
    claude::discover_configured(config.general.claude_config_dir.as_deref())
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

/// The mod's folder: `claude-code\` beside the installed app; in a development build, the
/// repository's `integrations/claude-code`.
fn mod_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    if let Some(dir) = claude_mod::mod_dir_near(&exe) {
        return Some(dir);
    }
    #[cfg(debug_assertions)]
    {
        let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations/claude-code");
        if dev.join(".claude-plugin/plugin.json").is_file() {
            return dev.canonicalize().ok().map(strip_verbatim);
        }
    }
    None
}

/// `\\?\C:\…` from `canonicalize` is not a path Claude Code expects in its settings.
#[cfg(debug_assertions)]
fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = p.display().to_string();
    s.strip_prefix(r"\\?\").map_or(p, PathBuf::from)
}

fn cli_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    CLI_CANDIDATES
        .iter()
        .map(|c| dir.join(c))
        .find(|p| p.is_file())
}

/// Everything the "Data sources" tab shows. Reads the database, so it runs off the main thread.
pub fn view(app: &AppHandle) -> Result<DataSourcesView, String> {
    let paths = paths()?;
    let config = load_config(&paths);
    let found = accounts(&config);
    let chosen = config.general.claude_config_dir.as_deref();
    let store = ReadOnlyStore::open(paths.db_path()).ok().flatten();
    let stored = store
        .as_ref()
        .and_then(|s| s.accounts().ok())
        .unwrap_or_default();
    let dirs = found
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let stats = stored
                .iter()
                .find(|s| same_dir(Path::new(&s.spec.config_dir), &a.config_dir))
                .and_then(|s| store.as_ref()?.account_stats(s.id).ok());
            ClaudeDirView {
                path: display(&a.config_dir),
                primary: i == 0,
                chosen: chosen.is_some_and(|c| same_dir(c, &a.config_dir)),
                stats,
            }
        })
        .collect();
    let primary = found.first().map(|a| a.config_dir.as_path());
    let snapshot = core_bridge::panel_snapshot(app);
    let consent = primary
        .and_then(|d| config.account(d))
        .and_then(|a| a.api_consent.clone())
        .unwrap_or_else(|| config.legacy_api_consent.clone());
    Ok(DataSourcesView {
        dirs,
        chosen_missing: chosen.filter(|c| !c.is_dir()).map(display),
        env_dir: std::env::var_os("CLAUDE_CONFIG_DIR")
            .filter(|v| !v.is_empty())
            .map(|v| PathBuf::from(v).display().to_string()),
        statusline: primary.map_or(StatuslineView::NoDirectory, |d| statusline(d, &config)),
        statusline_last_seen: snapshot.as_ref().and_then(|s| s.statusline.last_seen),
        cli_missing: cli_path().is_none(),
        claude_mod: primary.map_or(ClaudeModView::NoDirectory, |d| match mod_dir() {
            None => ClaudeModView::Missing,
            Some(m) => match claude_mod::state(d, &m) {
                ModState::Installed => ClaudeModView::Installed { path: display(&m) },
                ModState::NotInstalled => ClaudeModView::NotInstalled,
            },
        }),
        api: ApiConsentView {
            granted: consent.is_valid(),
            granted_at: consent
                .granted_at
                .filter(|_| consent.granted)
                .map(|t| t.to_string()),
            outdated: consent.granted && consent.consent_version != API_CONSENT_VERSION,
            status: snapshot.map(|s| s.api),
        },
    })
}

/// The bridge's state in `dir`, from `settings.json` itself: the user may have changed it.
fn statusline(dir: &Path, config: &AppConfig) -> StatuslineView {
    let text = std::fs::read_to_string(dir.join("settings.json")).unwrap_or_default();
    let current = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| {
            v.get("statusLine")?
                .get("command")?
                .as_str()
                .map(str::to_owned)
        })
        .filter(|c| !c.trim().is_empty());
    let saved = config.account(dir).map(|a| &a.statusline);
    match current {
        Some(c) if tokenradar_core::statusline::install::is_tokenradar_command(&c) => {
            match saved.filter(|s| s.installed) {
                Some(s) => StatuslineView::Installed {
                    chained_command: s.previous_command.clone().filter(|_| s.chain),
                },
                None => StatuslineView::InstalledElsewhere,
            }
        }
        current => StatuslineView::NotInstalled {
            current_command: current,
        },
    }
}

/// Plans a change to the primary account's `settings.json`; nothing is written.
pub fn plan(
    app: &AppHandle,
    action: StatuslineAction,
) -> Result<StatuslinePlanView, StatuslineError> {
    let paths = paths().map_err(|message| StatuslineError::Io { message })?;
    let config = load_config(&paths);
    let dir = accounts(&config)
        .into_iter()
        .next()
        .ok_or(StatuslineError::NoDirectory)?
        .config_dir;
    let shell = Shell::detect();
    let planned = match action {
        StatuslineAction::Install | StatuslineAction::InstallNoChain => {
            let cli = cli_path().ok_or(StatuslineError::CliMissing)?;
            let command = format!("{} statusline", exe_for_command(&cli, &shell));
            let chain = (action == StatuslineAction::InstallNoChain).then_some(false);
            plan_install(&dir, &config, &command, chain)
        }
        StatuslineAction::Uninstall => plan_uninstall(&dir, &config),
    }
    .map_err(install_error)?;
    let plan = match planned {
        Planned::Ready(plan) => plan,
        Planned::AlreadyInstalled => return Ok(StatuslinePlanView::AlreadyInstalled),
        Planned::NotInstalled => return Ok(StatuslinePlanView::NotInstalled),
    };
    let (chained_command, restores_previous) = match &plan.change {
        Change::Install {
            previous_command,
            chain,
        } => (previous_command.clone().filter(|_| *chain), false),
        Change::Uninstall { restores_previous } => (None, *restores_previous),
    };
    let settings_path = display(&plan.settings_path);
    let diff = diff_lines(&plan.diff());
    let token = app.state::<Pending>().show(Shown::Statusline(plan));
    Ok(StatuslinePlanView::Ready {
        token,
        settings_path,
        diff,
        chained_mangled: chained_command.as_deref().is_some_and(|c| shell.mangles(c)),
        chained_command,
        restores_previous,
    })
}

/// Plans installing or removing the Claude Code mod in the primary account; nothing is written.
pub fn plan_mod(
    app: &AppHandle,
    action: ClaudeModAction,
) -> Result<ClaudeModPlanView, StatuslineError> {
    let paths = paths().map_err(|message| StatuslineError::Io { message })?;
    let config = load_config(&paths);
    let dir = accounts(&config)
        .into_iter()
        .next()
        .ok_or(StatuslineError::NoDirectory)?
        .config_dir;
    let mod_dir = mod_dir().ok_or(StatuslineError::ModMissing)?;
    let planned = match action {
        ClaudeModAction::Install => claude_mod::plan_install(&dir, &mod_dir),
        ClaudeModAction::Uninstall => claude_mod::plan_uninstall(&dir, &mod_dir),
    }
    .map_err(install_error)?;
    let plan = match planned {
        ModPlanned::Ready(plan) => plan,
        ModPlanned::AlreadyInstalled => return Ok(ClaudeModPlanView::AlreadyInstalled),
        ModPlanned::NotInstalled => return Ok(ClaudeModPlanView::NotInstalled),
    };
    let kept = match &plan.change {
        ModChange::Install { kept } | ModChange::Uninstall { kept } => kept.clone(),
    };
    let settings_path = display(&plan.settings_path);
    let diff = diff_lines(&plan.diff());
    let token = app.state::<Pending>().show(Shown::Mod(plan));
    Ok(ClaudeModPlanView::Ready {
        token,
        settings_path,
        diff,
        kept,
    })
}

/// Applies the mod plan the user saw. Returns the backup's path.
pub fn apply_mod(app: &AppHandle, token: u32) -> Result<Option<String>, StatuslineError> {
    let Some(Shown::Mod(plan)) = app.state::<Pending>().take(token) else {
        return Err(StatuslineError::StalePlan);
    };
    let backup = plan.apply().map_err(install_error)?;
    tracing::info!(change = ?plan.change, "claude code mod settings changed");
    Ok(backup.as_deref().map(display))
}

/// Applies the plan the user saw. Returns the backup's path, if there was a file to back up.
pub fn apply(app: &AppHandle, token: u32) -> Result<Option<String>, StatuslineError> {
    let Some(Shown::Statusline(plan)) = app.state::<Pending>().take(token) else {
        return Err(StatuslineError::StalePlan);
    };
    let paths = paths().map_err(|message| StatuslineError::Io { message })?;
    let backup = plan.apply(paths.config_path()).map_err(install_error)?;
    tracing::info!(change = ?plan.change, "statusline settings changed");
    Ok(backup.as_deref().map(display))
}

/// Forgets a plan the user cancelled.
pub fn discard(app: &AppHandle) {
    *app.state::<Pending>()
        .plan
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;
}

fn install_error(e: InstallError) -> StatuslineError {
    match e {
        InstallError::NoConfigDir(_) => StatuslineError::NoDirectory,
        InstallError::Invalid(_) => StatuslineError::InvalidSettings,
        InstallError::Changed(_) => StatuslineError::ChangedMeanwhile,
        other => StatuslineError::Io {
            message: other.to_string(),
        },
    }
}

pub(crate) fn diff_lines(diff: &str) -> Vec<DiffLine> {
    diff.lines()
        .map(|line| {
            let kind = if line.starts_with("+++") || line.starts_with("---") {
                DiffKind::Header
            } else if line.starts_with("@@") {
                DiffKind::Hunk
            } else if line.starts_with('+') {
                DiffKind::Added
            } else if line.starts_with('-') {
                DiffKind::Removed
            } else {
                DiffKind::Context
            };
            DiffLine {
                kind,
                text: line.to_owned(),
            }
        })
        .collect()
}

/// Grants or withdraws the usage API consent of the primary account and tells the core.
pub fn set_api_consent(app: &AppHandle, granted: bool) -> Result<(), String> {
    let paths = paths()?;
    let config = load_config(&paths);
    let dir = accounts(&config)
        .into_iter()
        .next()
        .ok_or("no Claude Code directory")?
        .config_dir;
    config::save_api_consent_now(paths.config_path(), &dir, granted).map_err(|e| e.to_string())?;
    core_bridge::with_core(app, |core| core.set_api_consent(granted));
    tracing::info!(granted, "usage API consent changed");
    Ok(())
}

/// Opens a folder picker. A picked directory becomes the primary account and the core
/// restarts to watch it. `Ok(false)`: the user cancelled.
pub fn pick_claude_dir(app: &AppHandle) -> Result<bool, PickError> {
    let Some(picked) = app.dialog().file().blocking_pick_folder() else {
        return Ok(false);
    };
    let dir = picked.into_path().map_err(|_| PickError::NotADirectory)?;
    if !looks_like_claude_dir(&dir) {
        return Err(PickError::NotClaudeDir);
    }
    let paths = paths().map_err(|message| PickError::Io { message })?;
    config::save_claude_config_dir(paths.config_path(), Some(&dir)).map_err(|e| PickError::Io {
        message: e.to_string(),
    })?;
    core_bridge::restart(app);
    Ok(true)
}

/// Goes back to the discovered directories.
pub fn reset_claude_dir(app: &AppHandle) -> Result<(), String> {
    let paths = paths()?;
    config::save_claude_config_dir(paths.config_path(), None).map_err(|e| e.to_string())?;
    core_bridge::restart(app);
    Ok(())
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum PickError {
    NotADirectory,
    /// Neither `projects\` nor `settings.json` inside: probably the wrong folder.
    NotClaudeDir,
    Io {
        message: String,
    },
}

fn looks_like_claude_dir(dir: &Path) -> bool {
    dir.join(claude::PROJECTS_DIR).is_dir() || dir.join("settings.json").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_lines_are_classified() {
        let diff = "--- a\n+++ b\n@@ -1,2 +1,2 @@\n {\n-  \"a\": 1\n+  \"a\": 2\n";
        let kinds: Vec<DiffKind> = diff_lines(diff).iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            [
                DiffKind::Header,
                DiffKind::Header,
                DiffKind::Hunk,
                DiffKind::Context,
                DiffKind::Removed,
                DiffKind::Added
            ]
        );
    }

    #[test]
    fn a_claude_dir_has_projects_or_settings() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(!looks_like_claude_dir(tmp.path()));
        std::fs::write(tmp.path().join("settings.json"), "{}").expect("write");
        assert!(looks_like_claude_dir(tmp.path()));
    }
}
