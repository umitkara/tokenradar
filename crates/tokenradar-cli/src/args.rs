//! Command-line parsing.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    /// Claude Code's statusline command: JSON on stdin, one line out.
    Statusline {
        chain: bool,
    },
    Install {
        /// `None`: chain whenever there is a previous command.
        chain: Option<bool>,
        yes: bool,
        config_dir: Option<PathBuf>,
    },
    Uninstall {
        yes: bool,
        config_dir: Option<PathBuf>,
    },
    Status {
        json: bool,
    },
    Limits {
        json: bool,
    },
    Sessions {
        json: bool,
    },
    /// A diagnostics bundle; `out` defaults to a stamped name in the current directory.
    Doctor {
        out: Option<PathBuf>,
    },
}

#[derive(Default)]
#[allow(clippy::struct_excessive_bools)] // command-line switches
struct Flags {
    help: bool,
    version: bool,
    json: bool,
    chain: bool,
    no_chain: bool,
    yes: bool,
    config_dir: Option<PathBuf>,
    out: Option<PathBuf>,
}

pub fn parse(mut parser: lexopt::Parser) -> Result<Command, lexopt::Error> {
    use lexopt::prelude::*;

    let mut words: Vec<String> = Vec::new();
    let mut f = Flags::default();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('h') | Long("help") => f.help = true,
            Short('V') | Long("version") => f.version = true,
            Long("json") => f.json = true,
            Long("chain") => f.chain = true,
            Long("no-chain") => f.no_chain = true,
            Short('y') | Long("yes") => f.yes = true,
            Long("config-dir") => f.config_dir = Some(parser.value()?.into()),
            Short('o') | Long("out") => f.out = Some(parser.value()?.into()),
            Value(v) if words.len() < 2 => words.push(v.string()?),
            _ => return Err(arg.unexpected()),
        }
    }
    if f.help {
        return Ok(Command::Help);
    }
    if f.version {
        return Ok(Command::Version);
    }
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let command = match words.as_slice() {
        [] => Command::Help,
        ["statusline"] => Command::Statusline { chain: f.chain },
        ["statusline", "install"] => {
            if f.chain && f.no_chain {
                return Err("--chain and --no-chain exclude each other".into());
            }
            Command::Install {
                chain: (f.chain || f.no_chain).then_some(f.chain),
                yes: f.yes,
                config_dir: f.config_dir.take(),
            }
        }
        ["statusline", "uninstall"] => Command::Uninstall {
            yes: f.yes,
            config_dir: f.config_dir.take(),
        },
        ["status"] => Command::Status { json: f.json },
        ["limits"] => Command::Limits { json: f.json },
        ["sessions"] => Command::Sessions { json: f.json },
        ["doctor"] => Command::Doctor { out: f.out.take() },
        [..] => return Err(format!("unknown command '{}'", words.join(" ")).into()),
    };
    check_flags(&command, &f)?;
    Ok(command)
}

/// Rejects flags the command does not take, so a typo is not silently ignored.
fn check_flags(command: &Command, f: &Flags) -> Result<(), lexopt::Error> {
    let (json, chain, no_chain, yes, dir, out) = match command {
        Command::Statusline { .. } => (false, true, false, false, false, false),
        Command::Install { .. } => (false, true, true, true, true, false),
        Command::Uninstall { .. } => (false, false, false, true, true, false),
        Command::Status { .. } | Command::Limits { .. } | Command::Sessions { .. } => {
            (true, false, false, false, false, false)
        }
        Command::Doctor { .. } => (false, false, false, false, false, true),
        Command::Help | Command::Version => return Ok(()),
    };
    let used = [
        (f.json && !json, "--json"),
        (f.chain && !chain, "--chain"),
        (f.no_chain && !no_chain, "--no-chain"),
        (f.yes && !yes, "--yes"),
        (f.config_dir.is_some() && !dir, "--config-dir"),
        (f.out.is_some() && !out, "--out"),
    ];
    match used.iter().find(|(bad, _)| *bad) {
        Some((_, flag)) => Err(format!("{flag} does not apply to this command").into()),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Command, String> {
        parse(lexopt::Parser::from_args(args)).map_err(|e| e.to_string())
    }

    #[test]
    fn commands_and_flags() {
        assert_eq!(parse_args(&[]), Ok(Command::Help));
        assert_eq!(parse_args(&["-V"]), Ok(Command::Version));
        assert_eq!(parse_args(&["status", "--help"]), Ok(Command::Help));
        assert_eq!(
            parse_args(&["statusline"]),
            Ok(Command::Statusline { chain: false })
        );
        assert_eq!(
            parse_args(&["statusline", "--chain"]),
            Ok(Command::Statusline { chain: true })
        );
        assert_eq!(
            parse_args(&[
                "statusline",
                "install",
                "--no-chain",
                "-y",
                "--config-dir",
                "D:/c"
            ]),
            Ok(Command::Install {
                chain: Some(false),
                yes: true,
                config_dir: Some("D:/c".into()),
            })
        );
        assert_eq!(
            parse_args(&["statusline", "install"]),
            Ok(Command::Install {
                chain: None,
                yes: false,
                config_dir: None,
            })
        );
        assert_eq!(
            parse_args(&["statusline", "uninstall", "--yes"]),
            Ok(Command::Uninstall {
                yes: true,
                config_dir: None,
            })
        );
        assert_eq!(
            parse_args(&["limits", "--json"]),
            Ok(Command::Limits { json: true })
        );
        assert_eq!(parse_args(&["doctor"]), Ok(Command::Doctor { out: None }));
        assert_eq!(
            parse_args(&["doctor", "--out", "D:/x.zip"]),
            Ok(Command::Doctor {
                out: Some("D:/x.zip".into())
            })
        );
    }

    #[test]
    fn mistakes_are_errors() {
        assert!(parse_args(&["stats"]).is_err());
        assert!(parse_args(&["status", "--chain"]).is_err());
        assert!(parse_args(&["statusline", "--json"]).is_err());
        assert!(parse_args(&["statusline", "install", "--chain", "--no-chain"]).is_err());
        assert!(parse_args(&["status", "extra", "words"]).is_err());
        assert!(parse_args(&["--frobnicate"]).is_err());
        assert!(parse_args(&["status", "--out", "x.zip"]).is_err());
        assert!(parse_args(&["doctor", "--json"]).is_err());
    }
}
