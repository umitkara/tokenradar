//! The statusline settings' terminal font helper (Step 12o, ADR-055).
//!
//! The view says whether a Nerd Font is installed, which terminals the statusline was called
//! from and which font each terminal's settings select. Downloading the pinned font happens
//! only on the user's click after the size, source and licence were shown; writing a terminal's
//! settings goes through the same token-guarded diff dialog as `settings.json` (ADR-009).

use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokenradar_core::terminal_font::download::{self, DownloadError};
use tokenradar_core::terminal_font::install::{self, InstallError};
use tokenradar_core::terminal_font::terminals::{self, FontChange, TargetError, TargetState};
use tokenradar_core::terminal_font::{FontScan, NerdTier, TargetId, TerminalKind, fonts};

use crate::core_bridge;
use crate::sources::{self, DiffLine, Pending, Shown};

fn env(key: &str) -> Option<OsString> {
    std::env::var_os(key)
}

/// The best Nerd Font installed.
#[derive(Debug, Clone, Serialize, Type)]
pub struct NerdFontView {
    pub family: String,
    pub tier: NerdTier,
    /// Icons take one terminal cell, as the statusline counts them.
    pub mono: bool,
}

/// What the download button fetches, shown before the user agrees.
#[derive(Debug, Clone, Serialize, Type)]
pub struct FontOffer {
    pub family: String,
    pub version: String,
    pub size_bytes: u32,
    pub license: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct TerminalFontView {
    pub font: Option<NerdFontView>,
    /// Terminals the statusline was called from since the app started.
    pub terminals: Vec<TerminalKind>,
    pub last_terminal: Option<TerminalKind>,
    /// The terminal settings files on this machine.
    pub targets: Vec<TargetState>,
    pub offer: FontOffer,
    /// Installing is implemented for this platform.
    pub can_install: bool,
    /// A download is running.
    pub installing: bool,
}

/// Download progress, sent to the settings window.
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct FontInstallProgress {
    pub done: u32,
    pub total: u32,
}

/// Why installing or selecting the font failed; the window words it.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum FontError {
    /// Another download is running.
    Busy,
    Cancelled,
    Connect,
    Timeout,
    Status {
        code: u16,
    },
    Request,
    /// The download did not match the pinned checksum; nothing was written.
    Checksum,
    Archive,
    Unsupported,
    /// No Nerd Font is installed to select.
    NoFont,
    InvalidSettings,
    /// Windows Terminal's old format: `profiles` is a list.
    LegacySettings,
    ChangedMeanwhile,
    StalePlan,
    Io {
        message: String,
    },
}

/// A planned change to one terminal's settings, shown before anything is written.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum FontPlanView {
    Ready {
        /// Pass back to `apply_terminal_font`; only this plan can be applied.
        token: u32,
        settings_path: String,
        family: String,
        diff: Vec<DiffLine>,
    },
    /// The font is already selected, or there is nothing to undo.
    Unchanged,
}

/// The running download. Managed by Tauri.
#[derive(Default)]
pub struct FontJob {
    busy: AtomicBool,
    cancel: AtomicBool,
}

pub fn init(app: &AppHandle) {
    app.manage(FontJob::default());
}

fn scan() -> FontScan {
    fonts::scan(&fonts::font_dirs(&env))
}

pub fn view(app: &AppHandle) -> TerminalFontView {
    let scan = scan();
    let status = core_bridge::core_status(app);
    TerminalFontView {
        font: scan.best().map(|f| NerdFontView {
            family: f.family.clone(),
            tier: f.tier,
            mono: f.mono,
        }),
        terminals: status
            .as_ref()
            .map(|s| s.terminals.clone())
            .unwrap_or_default(),
        last_terminal: status.and_then(|s| s.last_terminal),
        targets: terminals::targets(&env)
            .into_iter()
            .map(|(id, path)| terminals::read_state(id, &path, &scan))
            .collect(),
        offer: FontOffer {
            family: download::FAMILY.to_owned(),
            version: download::VERSION.to_owned(),
            size_bytes: u32::try_from(download::SIZE).unwrap_or(u32::MAX),
            license: "OFL-1.1".to_owned(),
            url: download::URL.to_owned(),
        },
        can_install: cfg!(windows),
        installing: app.state::<FontJob>().busy.load(Ordering::Relaxed),
    }
}

/// Downloads, verifies and installs the pinned font for the current user. Blocking.
pub fn install(app: &AppHandle) -> Result<TerminalFontView, FontError> {
    let job = app.state::<FontJob>();
    if job.busy.swap(true, Ordering::AcqRel) {
        return Err(FontError::Busy);
    }
    job.cancel.store(false, Ordering::Relaxed);
    let result = run_install(app, &job);
    job.busy.store(false, Ordering::Release);
    match &result {
        Ok(()) => tracing::info!(version = download::VERSION, "nerd font installed"),
        Err(error) => tracing::warn!(?error, "nerd font install failed"),
    }
    result.map(|()| view(app))
}

fn run_install(app: &AppHandle, job: &FontJob) -> Result<(), FontError> {
    let local = env("LOCALAPPDATA").ok_or(FontError::Unsupported)?;
    let mut last = u64::MAX;
    let mut progress = |done: u64, total: u64| {
        // About a hundred events for the whole download.
        let step = (done * 100).checked_div(total).unwrap_or(0);
        if step != last {
            last = step;
            let _ = FontInstallProgress {
                done: u32::try_from(done).unwrap_or(u32::MAX),
                total: u32::try_from(total).unwrap_or(u32::MAX),
            }
            .emit(app);
        }
    };
    let files = download::fetch(&mut progress, &job.cancel).map_err(download_error)?;
    let dir = install::user_font_dir(std::path::Path::new(&local));
    install::install(&files, &dir).map_err(|e| match e {
        InstallError::Unsupported => FontError::Unsupported,
        other => FontError::Io {
            message: other.to_string(),
        },
    })?;
    Ok(())
}

pub fn cancel(app: &AppHandle) {
    app.state::<FontJob>().cancel.store(true, Ordering::Relaxed);
}

fn download_error(e: DownloadError) -> FontError {
    match e {
        DownloadError::Connect => FontError::Connect,
        DownloadError::Timeout => FontError::Timeout,
        DownloadError::Status(code) => FontError::Status { code },
        DownloadError::Request | DownloadError::Setup => FontError::Request,
        DownloadError::Checksum => FontError::Checksum,
        DownloadError::Archive => FontError::Archive,
        DownloadError::Cancelled => FontError::Cancelled,
    }
}

/// Plans selecting the best installed Nerd Font in `target`, or undoing the last change.
pub fn plan(
    app: &AppHandle,
    target: TargetId,
    change: FontChange,
) -> Result<FontPlanView, FontError> {
    let path = target.path(&env).ok_or(FontError::Unsupported)?;
    let (planned, family) = match change {
        FontChange::Select => {
            let scan = scan();
            let family = scan.best().ok_or(FontError::NoFont)?.family.clone();
            (terminals::plan_select(target, &path, &family), family)
        }
        FontChange::Undo => (terminals::plan_undo(target, &path), String::new()),
    };
    let Some(plan) = planned.map_err(target_error)? else {
        return Ok(FontPlanView::Unchanged);
    };
    let diff = sources::diff_lines(&plan.diff());
    let settings_path = plan.path.display().to_string();
    let token = app.state::<Pending>().show(Shown::Font(plan));
    Ok(FontPlanView::Ready {
        token,
        settings_path,
        family,
        diff,
    })
}

/// Applies the plan the user saw; returns the backup's path.
pub fn apply(app: &AppHandle, token: u32) -> Result<String, FontError> {
    let Some(Shown::Font(plan)) = app.state::<Pending>().take(token) else {
        return Err(FontError::StalePlan);
    };
    let backup = plan.apply().map_err(target_error)?;
    tracing::info!(target = ?plan.id, change = ?plan.change, "terminal font settings changed");
    Ok(backup.display().to_string())
}

#[allow(clippy::needless_pass_by_value)] // used as `map_err(target_error)`
fn target_error(e: TargetError) -> FontError {
    match e {
        TargetError::Invalid(_) => FontError::InvalidSettings,
        TargetError::Legacy(_) => FontError::LegacySettings,
        TargetError::Changed(_) => FontError::ChangedMeanwhile,
        TargetError::Io { .. } => FontError::Io {
            message: e.to_string(),
        },
    }
}
