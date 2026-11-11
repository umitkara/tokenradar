//! The git branch for the statusline's `git_branch` item (Step 12i, ADR-049).
//!
//! Only `.git/HEAD` is read; no git process is started, so the cost stays within the
//! statusline's latency budget. `git status` runs in the app instead ([`super::git_status`]).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::line::clean;

/// Folders walked up from the start before giving up.
const MAX_DEPTH: usize = 32;
/// `HEAD` and a `.git` file are a line each; anything longer is not one.
const MAX_FILE: u64 = 4096;

/// The branch checked out in the repository holding `start`, or the first 7 characters of
/// the commit on a detached HEAD. `None` outside a repository and on any read error.
pub fn branch(start: &Path) -> Option<String> {
    head(&locate(start)?.1)
}

/// The working tree holding `start` and its git directory: `.git` itself, or where a `.git`
/// file points (a linked worktree's `.git/worktrees/<name>`, a submodule's
/// `.git/modules/<name>`). Each has its own `HEAD` and `index`.
pub fn locate(start: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut dir = Some(start);
    for _ in 0..MAX_DEPTH {
        let d = dir?;
        let dotgit = d.join(".git");
        match fs::metadata(&dotgit) {
            Ok(m) if m.is_dir() => return Some((d.to_path_buf(), dotgit)),
            // A linked worktree or a submodule: `gitdir: <path>`.
            Ok(m) if m.is_file() => return Some((d.to_path_buf(), gitdir(&dotgit, d)?)),
            _ => dir = d.parent(),
        }
    }
    None
}

fn gitdir(file: &Path, dir: &Path) -> Option<PathBuf> {
    let text = read_small(file)?;
    let path = text.lines().find_map(|l| l.strip_prefix("gitdir:"))?.trim();
    let path = Path::new(path);
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        dir.join(path)
    })
}

fn head(gitdir: &Path) -> Option<String> {
    let text = read_small(&gitdir.join("HEAD"))?;
    let text = text.trim();
    let name = if let Some(r) = text.strip_prefix("ref:") {
        let r = r.trim();
        r.strip_prefix("refs/heads/").unwrap_or(r).to_owned()
    } else if text.len() >= 7 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
        text[..7].to_owned()
    } else {
        return None;
    };
    Some(clean(&name, 64)).filter(|n| !n.is_empty())
}

fn read_small(path: &Path) -> Option<String> {
    let mut text = String::new();
    fs::File::open(path)
        .ok()?
        .take(MAX_FILE)
        .read_to_string(&mut text)
        .ok()?;
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, text).expect("write");
    }

    #[test]
    fn reads_the_branch_from_any_folder_below_the_repository() {
        let tmp = tempfile::tempdir().expect("tmp");
        write(&tmp.path().join(".git/HEAD"), "ref: refs/heads/feature/x\n");
        let deep = tmp.path().join("src/a/b");
        fs::create_dir_all(&deep).expect("mkdir");
        assert_eq!(branch(tmp.path()).as_deref(), Some("feature/x"));
        assert_eq!(branch(&deep).as_deref(), Some("feature/x"));
    }

    #[test]
    fn a_detached_head_shows_the_short_commit() {
        let tmp = tempfile::tempdir().expect("tmp");
        write(
            &tmp.path().join(".git/HEAD"),
            "3f2a9c1d0e8b7a6f5e4d3c2b1a0f9e8d7c6b5a49\n",
        );
        assert_eq!(branch(tmp.path()).as_deref(), Some("3f2a9c1"));
    }

    #[test]
    fn a_worktree_follows_its_gitdir_file() {
        let tmp = tempfile::tempdir().expect("tmp");
        let main = tmp.path().join("main/.git/worktrees/wt");
        write(&main.join("HEAD"), "ref: refs/heads/wt-branch\n");
        let wt = tmp.path().join("wt");
        write(&wt.join(".git"), &format!("gitdir: {}\n", main.display()));
        assert_eq!(branch(&wt).as_deref(), Some("wt-branch"));
        // A relative path is relative to the worktree.
        let rel = tmp.path().join("rel");
        write(&rel.join(".git"), "gitdir: ../main/.git/worktrees/wt\n");
        assert_eq!(branch(&rel).as_deref(), Some("wt-branch"));
    }

    #[test]
    fn outside_a_repository_or_with_a_broken_head_there_is_none() {
        let tmp = tempfile::tempdir().expect("tmp");
        let dir = tmp.path().join("plain");
        fs::create_dir_all(&dir).expect("mkdir");
        // The temp folder itself may sit inside a repository on some machines; only the
        // broken HEAD below is checked strictly.
        let broken = tmp.path().join("broken");
        write(&broken.join(".git/HEAD"), "garbage\x1b[31m\n");
        assert_eq!(branch(&broken), None);
        // Control characters in a branch name never reach the terminal.
        let odd = tmp.path().join("odd");
        write(&odd.join(".git/HEAD"), "ref: refs/heads/a\x1b[2Jb\n");
        assert_eq!(branch(&odd).as_deref(), Some("a[2Jb"));
    }
}
