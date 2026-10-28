//! `tokenradar`: the console companion of the TokenRadar app.
//!
//! - `statusline` is Claude Code's statusline command: it hands the reading to the app (or
//!   spools it while the app is closed) and prints one line.
//! - `status`, `limits` and `sessions` ask the app, or read its database when it is closed.
//! - `statusline install|uninstall` edit Claude Code's `settings.json`, with a diff and the
//!   user's yes.
//! - `doctor` writes a diagnostics bundle the user can read and share.
//! - `path add|remove` puts this binary's directory on the user's `PATH` (the installer uses it).

mod args;
mod context;
mod doctor;
mod render;
mod report;
mod setup;
mod statusline;
mod user_path;

use std::io::Write;
use std::process::ExitCode;

use args::Command;
use context::Context;
use report::Report;
use tokenradar_i18n::Arg;

fn main() -> ExitCode {
    let command = args::parse(lexopt::Parser::from_env());
    let ctx = Context::load();
    match command {
        Ok(command) => run(&ctx, command),
        Err(err) => {
            let message = err.to_string();
            eprintln!(
                "{}",
                ctx.l10n.fmt("error", &[("message", Arg::Str(&message))])
            );
            eprintln!();
            eprintln!("{}", ctx.l10n.msg("cli-help"));
            ExitCode::from(2)
        }
    }
}

fn run(ctx: &Context, command: Command) -> ExitCode {
    match command {
        Command::Help => {
            println!("{}", ctx.l10n.msg("cli-help"));
            ExitCode::SUCCESS
        }
        Command::Version => {
            println!("tokenradar {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Command::Statusline { chain } => {
            let input = statusline::read_stdin();
            let deps = statusline::Deps {
                l10n: &ctx.l10n,
                pipe_name: ctx.pipe_name.clone(),
                spool_dir: ctx
                    .paths
                    .as_ref()
                    .map(tokenradar_core::paths::Paths::spool_dir),
                chain: chain
                    .then(|| statusline::chain_from_config(&ctx.config, input.as_deref()))
                    .flatten(),
                chain_timeout: statusline::CHAIN_TIMEOUT,
                style: render::Style::from_env(),
            };
            let out = statusline::respond(input.as_deref(), &deps);
            let mut stdout = std::io::stdout().lock();
            let _ = writeln!(stdout, "{out}");
            // Whatever happened, the statusline must not show an error.
            ExitCode::SUCCESS
        }
        Command::Install {
            chain,
            yes,
            config_dir,
        } => exit(setup::install(ctx, chain, yes, config_dir)),
        Command::Uninstall { yes, all: true, .. } => exit(setup::uninstall_all(ctx, yes)),
        Command::Uninstall {
            yes, config_dir, ..
        } => exit(setup::uninstall(ctx, yes, config_dir)),
        Command::Status { json } => print_report(ctx, Report::Status, json),
        Command::Limits { json } => print_report(ctx, Report::Limits, json),
        Command::Sessions { json } => print_report(ctx, Report::Sessions, json),
        Command::Doctor { out } => exit(doctor::run(ctx, out)),
        Command::Path { add } => path(ctx, add),
    }
}

/// Exit codes: 0 done or nothing to do, 1 failed, 3 the `PATH` would be too long (the installer
/// tells the user then).
#[cfg(windows)]
fn path(ctx: &Context, add: bool) -> ExitCode {
    use user_path::Edit;

    let l = &ctx.l10n;
    let fail = |message: &str| {
        eprintln!("{}", l.fmt("error", &[("message", Arg::Str(message))]));
        ExitCode::FAILURE
    };
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.display().to_string()))
    else {
        return fail("could not locate tokenradar.exe");
    };
    let (current, expandable) = match user_path::read() {
        Ok(value) => value,
        Err(e) => return fail(&e.to_string()),
    };
    let shown = [("path", Arg::Str(&dir))];
    let (edit, done, unchanged) = if add {
        match user_path::add(&current, &dir, &user_path::expand) {
            Ok(edit) => (edit, "path-added", "path-already"),
            Err(length) => {
                #[allow(clippy::cast_precision_loss)]
                let args = [
                    ("path", Arg::Str(&dir)),
                    ("length", Arg::Num(length as f64)),
                    ("max", Arg::Num(user_path::MAX_LEN as f64)),
                ];
                eprintln!("{}", l.fmt("path-too-long", &args));
                return ExitCode::from(3);
            }
        }
    } else {
        (
            user_path::remove(&current, &dir, &user_path::expand),
            "path-removed",
            "path-absent",
        )
    };
    match edit {
        Edit::Unchanged => println!("{}", l.fmt(unchanged, &shown)),
        Edit::Changed(value) => {
            if let Err(e) = user_path::write(&value, expandable) {
                return fail(&e.to_string());
            }
            user_path::broadcast();
            println!("{}", l.fmt(done, &shown));
        }
    }
    ExitCode::SUCCESS
}

#[cfg(not(windows))]
fn path(ctx: &Context, _add: bool) -> ExitCode {
    eprintln!("{}", ctx.l10n.msg("path-unsupported"));
    ExitCode::FAILURE
}

fn print_report(ctx: &Context, report: Report, json: bool) -> ExitCode {
    match report::run(ctx, report, json) {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!(
                "{}",
                ctx.l10n.fmt("error", &[("message", Arg::Str(&message))])
            );
            ExitCode::FAILURE
        }
    }
}

fn exit(outcome: setup::Outcome) -> ExitCode {
    if outcome.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
