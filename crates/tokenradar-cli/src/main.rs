//! `tokenradar`: the console companion of the TokenRadar app.
//!
//! - `statusline` is Claude Code's statusline command: it hands the reading to the app (or
//!   spools it while the app is closed) and prints one line.
//! - `status`, `limits` and `sessions` ask the app, or read its database when it is closed.
//! - `statusline install|uninstall` edit Claude Code's `settings.json`, with a diff and the
//!   user's yes.
//! - `doctor` writes a diagnostics bundle the user can read and share.

mod args;
mod context;
mod doctor;
mod render;
mod report;
mod setup;
mod statusline;

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
        Command::Uninstall { yes, config_dir } => exit(setup::uninstall(ctx, yes, config_dir)),
        Command::Status { json } => print_report(ctx, Report::Status, json),
        Command::Limits { json } => print_report(ctx, Report::Limits, json),
        Command::Sessions { json } => print_report(ctx, Report::Sessions, json),
        Command::Doctor { out } => exit(doctor::run(ctx, out)),
    }
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
