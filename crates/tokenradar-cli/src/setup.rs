//! `statusline install` and `statusline uninstall`: show the diff, ask, back up, write
//! (ADR-009). The user's settings are never changed without their yes.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use tokenradar_core::statusline::default_config_dir;
use tokenradar_core::statusline::install::{
    Change, InstallError, Plan, Planned, plan_install, plan_uninstall,
};
use tokenradar_core::statusline::shell::{Shell, exe_for_command};
use tokenradar_i18n::{Arg, Localizer};

use crate::context::Context;

/// Exit code, messages already printed.
pub type Outcome = Result<(), ()>;

pub fn install(
    ctx: &Context,
    chain: Option<bool>,
    yes: bool,
    config_dir: Option<PathBuf>,
) -> Outcome {
    let l = &ctx.l10n;
    let (dir, app_config_path) = locations(ctx, config_dir)?;
    let shell = Shell::detect();
    let exe = std::env::current_exe().map_err(|e| fail(l, &e.to_string()))?;
    let command = format!("{} statusline", exe_for_command(&exe, &shell));
    let plan = match plan_install(&dir, &ctx.config, &command, chain) {
        Ok(Planned::Ready(plan)) => plan,
        Ok(Planned::AlreadyInstalled) => {
            println!("{}", path_msg(l, "install-already", &dir));
            return Ok(());
        }
        Ok(Planned::NotInstalled) => return Ok(()),
        Err(e) => {
            install_error(l, &e);
            return Err(());
        }
    };
    let Change::Install {
        previous_command,
        chain,
    } = plan.change.clone()
    else {
        return Ok(());
    };
    println!("{}", plan.diff());
    // Only a chained command keeps running, so only then is its quoting worth a word.
    if let Some(prev) = previous_command.as_ref().filter(|_| chain)
        && shell.mangles(prev)
    {
        println!(
            "{}",
            l.fmt("install-previous-mangled", &[("command", Arg::Str(prev))])
        );
    }
    let backup = confirm_and_apply(l, &plan, yes, &app_config_path)?;
    println!("{}", backup_msg(l, "install-done", backup.as_deref()));
    if chain {
        println!("{}", l.msg("install-chained"));
    } else if previous_command.is_some() {
        println!("{}", l.msg("install-chain-hint"));
    }
    Ok(())
}

pub fn uninstall(ctx: &Context, yes: bool, config_dir: Option<PathBuf>) -> Outcome {
    let l = &ctx.l10n;
    let (dir, app_config_path) = locations(ctx, config_dir)?;
    let plan = match plan_uninstall(&dir, &ctx.config) {
        Ok(Planned::Ready(plan)) => plan,
        Ok(Planned::NotInstalled | Planned::AlreadyInstalled) => {
            println!("{}", path_msg(l, "uninstall-not-installed", &dir));
            return Ok(());
        }
        Err(e) => {
            install_error(l, &e);
            return Err(());
        }
    };
    println!("{}", plan.diff());
    let backup = confirm_and_apply(l, &plan, yes, &app_config_path)?;
    println!("{}", backup_msg(l, "uninstall-done", backup.as_deref()));
    if matches!(
        plan.change,
        Change::Uninstall {
            restores_previous: true
        }
    ) {
        println!("{}", l.msg("uninstall-restored"));
    }
    Ok(())
}

/// Removes the bridge from every Claude Code directory `config.toml` records as installed; the
/// uninstaller runs this. Nothing recorded is a success.
pub fn uninstall_all(ctx: &Context, yes: bool) -> Outcome {
    let dirs: Vec<PathBuf> = ctx
        .config
        .accounts
        .iter()
        .filter(|a| a.statusline.installed)
        .map(|a| a.config_dir.clone())
        .collect();
    if dirs.is_empty() {
        println!("{}", ctx.l10n.msg("uninstall-none"));
        return Ok(());
    }
    // Every directory is tried, even after one failed.
    let mut result = Ok(());
    for dir in dirs {
        if uninstall(ctx, yes, Some(dir)).is_err() {
            result = Err(());
        }
    }
    result
}

/// Claude Code's directory and the app's config file.
fn locations(ctx: &Context, config_dir: Option<PathBuf>) -> Result<(PathBuf, PathBuf), ()> {
    let l = &ctx.l10n;
    let dir = config_dir.or_else(default_config_dir).unwrap_or_default();
    if !dir.is_dir() {
        eprintln!("{}", path_msg(l, "install-no-config-dir", &dir));
        return Err(());
    }
    let Some(paths) = &ctx.paths else {
        fail(l, "no data directory for TokenRadar");
        return Err(());
    };
    Ok((dir, paths.config_path().to_path_buf()))
}

fn confirm_and_apply(
    l: &Localizer,
    plan: &Plan,
    yes: bool,
    app_config_path: &Path,
) -> Result<Option<PathBuf>, ()> {
    if !yes {
        if !std::io::stdin().is_terminal() {
            eprintln!("{}", l.msg("install-not-interactive"));
            return Err(());
        }
        print!("{} ", path_msg(l, "install-confirm", &plan.settings_path));
        let _ = std::io::stdout().flush();
        let mut answer = String::new();
        let _ = std::io::stdin().lock().read_line(&mut answer);
        if !is_yes(&answer) {
            println!("{}", l.msg("install-cancelled"));
            return Err(());
        }
    }
    plan.apply(app_config_path)
        .map_err(|e| install_error(l, &e))
}

/// `y`/`yes` and the Turkish `e`/`evet`.
fn is_yes(answer: &str) -> bool {
    matches!(
        answer.trim().to_lowercase().as_str(),
        "y" | "yes" | "e" | "evet"
    )
}

fn install_error(l: &Localizer, e: &InstallError) {
    let msg = match e {
        InstallError::NoConfigDir(p) => path_msg(l, "install-no-config-dir", p),
        InstallError::Invalid(p) => path_msg(l, "settings-invalid", p),
        InstallError::Changed(p) => path_msg(l, "settings-changed-meanwhile", p),
        other => l.fmt("error", &[("message", Arg::Str(&other.to_string()))]),
    };
    eprintln!("{msg}");
}

fn fail(l: &Localizer, message: &str) {
    eprintln!("{}", l.fmt("error", &[("message", Arg::Str(message))]));
}

fn path_msg(l: &Localizer, key: &str, path: &Path) -> String {
    l.fmt(key, &[("path", Arg::Str(&path.display().to_string()))])
}

fn backup_msg(l: &Localizer, key: &str, backup: Option<&Path>) -> String {
    let shown = backup.map_or_else(|| "—".to_owned(), |b| b.display().to_string());
    l.fmt(key, &[("backup", Arg::Str(&shown))])
}

#[cfg(test)]
mod tests {
    use super::is_yes;

    #[test]
    fn yes_in_both_languages() {
        for a in ["y\n", "Y", " yes ", "e", "Evet\r\n"] {
            assert!(is_yes(a), "{a:?}");
        }
        for a in ["", "\n", "n", "hayır", "no", "yess"] {
            assert!(!is_yes(a), "{a:?}");
        }
    }
}
