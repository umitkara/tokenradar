//! `tokenradar open [PAGE]`: starts `tokenradar-app.exe tokenradar://main[/PAGE]`. A running app
//! gets the argument through its single-instance plugin and the new process exits; otherwise the
//! app starts and opens the window itself (`tokenradar_core::deeplink`).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use tokenradar_core::deeplink::{Link, MainRoute};
use tokenradar_i18n::Arg;

use crate::context::Context;

#[cfg(windows)]
const APP_EXE: &str = "tokenradar-app.exe";
#[cfg(not(windows))]
const APP_EXE: &str = "tokenradar-app";

pub fn run(ctx: &Context, route: Option<MainRoute>) -> ExitCode {
    let l = &ctx.l10n;
    let Some(exe) = std::env::current_exe().ok().and_then(|e| find_app(&e)) else {
        eprintln!("{}", l.msg("open-app-missing"));
        return ExitCode::FAILURE;
    };
    match spawn(&exe, &Link::Main(route).uri()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let message = error.to_string();
            eprintln!("{}", l.fmt("error", &[("message", Arg::Str(&message))]));
            ExitCode::FAILURE
        }
    }
}

/// The app beside the CLI: the installer puts the CLI in `bin\` under the app's folder; cargo
/// builds both into the same `target` folder.
fn find_app(cli: &Path) -> Option<PathBuf> {
    let dir = cli.parent()?;
    [Some(dir), dir.parent()]
        .into_iter()
        .flatten()
        .map(|d| d.join(APP_EXE))
        .find(|p| p.is_file())
}

fn spawn(exe: &Path, arg: &str) -> std::io::Result<()> {
    let mut command = Command::new(exe);
    command.arg(arg);
    // Not tied to this console: closing the terminal must not end the app.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    command.spawn().map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_app_beside_or_above_the_cli() {
        let root = tempfile::tempdir().expect("tempdir");
        let bin = root.path().join("bin");
        std::fs::create_dir(&bin).expect("bin");
        let cli = bin.join("tokenradar.exe");
        assert_eq!(find_app(&cli), None);

        // Installed layout: `<app>\bin\tokenradar.exe`.
        std::fs::write(root.path().join(APP_EXE), b"").expect("app");
        assert_eq!(find_app(&cli), Some(root.path().join(APP_EXE)));

        // Cargo layout: both in one folder, which wins.
        std::fs::write(bin.join(APP_EXE), b"").expect("app");
        assert_eq!(find_app(&cli), Some(bin.join(APP_EXE)));
    }
}
