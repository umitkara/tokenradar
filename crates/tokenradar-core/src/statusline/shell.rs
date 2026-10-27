//! The shell Claude Code runs `statusLine.command` with, and quoting for it (data-sources S1).
//!
//! On Windows Claude Code uses Git Bash when it is installed and PowerShell otherwise. The
//! installed command must survive that shell, and chain mode must run the previous command
//! the same way Claude Code would have.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shell {
    /// `bash.exe` of Git for Windows.
    GitBash(PathBuf),
    PowerShell,
    /// `/bin/sh` on Unix.
    Sh,
}

impl Shell {
    /// The shell Claude Code would pick on this machine.
    pub fn detect() -> Self {
        if !cfg!(windows) {
            return Self::Sh;
        }
        if let Some(p) = std::env::var_os("CLAUDE_CODE_GIT_BASH_PATH").map(PathBuf::from)
            && p.is_file()
        {
            return Self::GitBash(p);
        }
        find_git_bash().map_or(Self::PowerShell, Self::GitBash)
    }

    /// A process that runs `command_line` in this shell.
    pub fn command(&self, command_line: &str) -> Command {
        match self {
            Self::GitBash(bash) => {
                let mut c = Command::new(bash);
                c.arg("-c").arg(command_line);
                c
            }
            Self::PowerShell => {
                let mut c = Command::new("powershell.exe");
                c.args(["-NoProfile", "-NonInteractive", "-Command", command_line]);
                c
            }
            Self::Sh => {
                let mut c = Command::new("/bin/sh");
                c.arg("-c").arg(command_line);
                c
            }
        }
    }

    /// Git Bash eats unquoted backslashes: `C:\Users\x` arrives as `C:Usersx`. True when
    /// `command_line` has one outside quotes and would break here.
    pub fn mangles(&self, command_line: &str) -> bool {
        if !matches!(self, Self::GitBash(_)) {
            return false;
        }
        let mut quote = None;
        for c in command_line.chars() {
            match (quote, c) {
                (None, '"' | '\'') => quote = Some(c),
                (Some(q), c) if c == q => quote = None,
                (None, '\\') => return true,
                _ => {}
            }
        }
        false
    }
}

/// `bash.exe` next to the first `git.exe` on `PATH`, or in the default install location.
fn find_git_bash() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        if !dir.join("git.exe").is_file() {
            continue;
        }
        // <Git>\cmd, <Git>\bin and <Git>\mingw64\bin all hold a git.exe.
        for up in [
            dir.parent(),
            Some(dir.as_path()),
            dir.parent().and_then(Path::parent),
        ]
        .into_iter()
        .flatten()
        {
            let bash = up.join("bin").join("bash.exe");
            if bash.is_file() {
                return Some(bash);
            }
        }
    }
    let default = PathBuf::from(r"C:\Program Files\Git\bin\bash.exe");
    default.is_file().then_some(default)
}

/// The executable part of the installed command, written so both of Claude Code's shells run
/// it: forward slashes, no quotes when possible (Git Bash and PowerShell differ on quoting).
pub fn exe_for_command(exe: &Path, shell: &Shell) -> String {
    let plain = forward_slashes(exe);
    if !needs_quoting(&plain) {
        return plain;
    }
    if let Some(short) = short_path(exe).map(|p| forward_slashes(&p))
        && !needs_quoting(&short)
    {
        return short;
    }
    match shell {
        Shell::PowerShell => format!("& '{}'", plain.replace('\'', "''")),
        Shell::GitBash(_) | Shell::Sh => format!("\"{}\"", plain.replace(['"', '$', '`'], "")),
    }
}

fn forward_slashes(p: &Path) -> String {
    let s = p.to_string_lossy();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    if cfg!(windows) {
        s.replace('\\', "/")
    } else {
        s.to_owned()
    }
}

fn needs_quoting(s: &str) -> bool {
    s.chars().any(|c| {
        c.is_whitespace()
            || matches!(
                c,
                '\'' | '"'
                    | '$'
                    | '`'
                    | '&'
                    | '('
                    | ')'
                    | ';'
                    | '|'
                    | '<'
                    | '>'
                    | '#'
                    | '@'
                    | '{'
                    | '}'
            )
    })
}

/// The 8.3 name of `path`, if the volume keeps them.
#[cfg(windows)]
#[allow(unsafe_code)]
fn short_path(path: &Path) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut buf = vec![0u16; 1024];
    // SAFETY: `wide` is NUL-terminated; `buf` is writable for the length passed.
    let n = unsafe {
        GetShortPathNameW(
            wide.as_ptr(),
            buf.as_mut_ptr(),
            u32::try_from(buf.len()).unwrap_or(u32::MAX),
        )
    };
    let n = usize::try_from(n).ok()?;
    if n == 0 || n >= buf.len() {
        return None;
    }
    Some(PathBuf::from(OsString::from_wide(&buf[..n])))
}

#[cfg(not(windows))]
fn short_path(_: &Path) -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_paths_are_written_with_forward_slashes_and_no_quotes() {
        let bash = Shell::GitBash(PathBuf::from("bash.exe"));
        let exe = Path::new(r"C:\Tools\TokenRadar\tokenradar.exe");
        let expected = if cfg!(windows) {
            "C:/Tools/TokenRadar/tokenradar.exe"
        } else {
            r"C:\Tools\TokenRadar\tokenradar.exe"
        };
        assert_eq!(exe_for_command(exe, &bash), expected);
        assert_eq!(exe_for_command(exe, &Shell::PowerShell), expected);
    }

    #[test]
    fn paths_with_spaces_are_quoted_for_the_shell() {
        // Does not exist, so no 8.3 name: the quoting fallback is used.
        let exe = Path::new("/no such dir/tokenradar.exe");
        let bash = Shell::GitBash(PathBuf::from("bash.exe"));
        assert_eq!(
            exe_for_command(exe, &bash),
            "\"/no such dir/tokenradar.exe\""
        );
        assert_eq!(
            exe_for_command(Path::new("/it's here/t.exe"), &Shell::PowerShell),
            "& '/it''s here/t.exe'"
        );
    }

    #[test]
    fn unquoted_backslashes_are_flagged_only_for_git_bash() {
        let bash = Shell::GitBash(PathBuf::from("bash.exe"));
        assert!(bash.mangles(r"python C:\Users\x\.claude\statusline.py"));
        assert!(!bash.mangles("python C:/Users/x/.claude/statusline.py"));
        assert!(!bash.mangles(r#"python "C:\Users\x\s.py""#));
        assert!(!Shell::PowerShell.mangles(r"python C:\Users\x\s.py"));
    }
}
