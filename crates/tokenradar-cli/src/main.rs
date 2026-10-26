//! `tokenradar`: the console companion of the TokenRadar app.

use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
TokenRadar - usage limits for AI coding agents

Usage: tokenradar [OPTIONS]

Options:
  -h, --help       Print help
  -V, --version    Print version
";

enum Command {
    Help,
    Version,
}

fn parse_args() -> Result<Command, lexopt::Error> {
    use lexopt::prelude::*;

    let mut parser = lexopt::Parser::from_env();
    let mut command = Command::Help;
    while let Some(arg) = parser.next()? {
        match arg {
            Short('h') | Long("help") => command = Command::Help,
            Short('V') | Long("version") => command = Command::Version,
            _ => return Err(arg.unexpected()),
        }
    }
    Ok(command)
}

fn main() -> ExitCode {
    match parse_args() {
        Ok(Command::Help) => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("tokenradar {VERSION}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}\n\n{HELP}");
            ExitCode::from(2)
        }
    }
}
