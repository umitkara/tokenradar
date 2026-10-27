//! Claude Code: config-directory discovery, transcript layout and line parsing.
//!
//! ```text
//! <config>/projects/
//!   <dir>/                                  lossy encoding of the cwd
//!     <sessionId>.jsonl                     main session (file name = sessionId)
//!     <sessionId>/subagents/agent-<id>.jsonl   its subagents (sessionId = the main session)
//!     <sessionId>/tool-results/…            large tool outputs, never indexed
//! ```

pub mod credentials;
mod project;
mod record;
pub mod usage_api;

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

pub use project::{ProjectKey, UNRESOLVED_PREFIX};
pub use record::{AdvisorRecord, LineParser, MetaRecord, ParseOutcome, UnknownReason, UsageRecord};

/// Name of the transcript directory inside a config directory.
pub const PROJECTS_DIR: &str = "projects";

/// A Claude Code configuration directory that exists on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountDescriptor {
    pub config_dir: PathBuf,
    /// `Claude` for the default directory, otherwise the directory name.
    pub label: String,
}

impl AccountDescriptor {
    pub fn projects_dir(&self) -> PathBuf {
        self.config_dir.join(PROJECTS_DIR)
    }
}

/// Config directories to track: `~/.claude`, `CLAUDE_CONFIG_DIR` and user-configured extras,
/// in that order, without duplicates and only if they exist.
pub fn discover(extra: &[PathBuf]) -> Vec<AccountDescriptor> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    let env = std::env::var_os("CLAUDE_CONFIG_DIR");
    discover_from(home.as_deref(), env.as_deref(), extra)
}

/// [`discover`] with explicit inputs, for tests.
pub fn discover_from(
    home: Option<&Path>,
    claude_config_dir: Option<&OsStr>,
    extra: &[PathBuf],
) -> Vec<AccountDescriptor> {
    let default = home.map(|h| h.join(".claude"));
    let candidates = default
        .iter()
        .cloned()
        .chain(
            claude_config_dir
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
        )
        .chain(extra.iter().cloned());

    let mut seen = Vec::new();
    let mut out = Vec::new();
    for dir in candidates {
        if !dir.is_dir() {
            continue;
        }
        let identity = identity(&dir);
        if seen.contains(&identity) {
            continue;
        }
        seen.push(identity);
        let label = if default.as_ref() == Some(&dir) {
            "Claude".to_owned()
        } else {
            dir.file_name().map_or_else(
                || dir.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            )
        };
        out.push(AccountDescriptor {
            config_dir: dir,
            label,
        });
    }
    out
}

/// Comparable identity of a directory: canonical when possible, case-folded on Windows.
fn identity(dir: &Path) -> String {
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let s = canonical.to_string_lossy().into_owned();
    if cfg!(windows) { s.to_lowercase() } else { s }
}

/// What a transcript file is, from its position under `<config>/projects`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    /// `<dir>/<session>.jsonl`.
    Main {
        project_dir: String,
        session: String,
    },
    /// `<dir>/<parent>/subagents/agent-<id>.jsonl`; `agent` is `agent-<id>`.
    Subagent {
        project_dir: String,
        parent: String,
        agent: String,
    },
}

impl SourceKind {
    pub fn project_dir(&self) -> &str {
        match self {
            Self::Main { project_dir, .. } | Self::Subagent { project_dir, .. } => project_dir,
        }
    }
}

/// Classifies a path under `projects_dir`. Anything else (tool results, other files) is
/// `None` and never read.
pub fn classify_path(projects_dir: &Path, path: &Path) -> Option<SourceKind> {
    let rel = path.strip_prefix(projects_dir).ok()?;
    let parts: Vec<&str> = rel
        .components()
        .map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Option<_>>()?;
    match parts.as_slice() {
        [dir, file] => Some(SourceKind::Main {
            project_dir: (*dir).to_owned(),
            session: jsonl_stem(file)?.to_owned(),
        }),
        [dir, parent, "subagents", file] => {
            let agent = jsonl_stem(file)?;
            agent.starts_with("agent-").then(|| SourceKind::Subagent {
                project_dir: (*dir).to_owned(),
                parent: (*parent).to_owned(),
                agent: agent.to_owned(),
            })
        }
        _ => None,
    }
}

fn jsonl_stem(file: &str) -> Option<&str> {
    file.strip_suffix(".jsonl").filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_main_subagent_and_ignored_paths() {
        let root = Path::new("/c/projects");
        assert_eq!(
            classify_path(root, &root.join("C--work-alpha/s1.jsonl")),
            Some(SourceKind::Main {
                project_dir: "C--work-alpha".into(),
                session: "s1".into()
            })
        );
        assert_eq!(
            classify_path(
                root,
                &root.join("C--work-alpha/s1/subagents/agent-a7.jsonl")
            ),
            Some(SourceKind::Subagent {
                project_dir: "C--work-alpha".into(),
                parent: "s1".into(),
                agent: "agent-a7".into()
            })
        );
        for ignored in [
            "C--work-alpha/s1/tool-results/x.jsonl",
            "C--work-alpha/s1/subagents/notes.jsonl",
            "C--work-alpha/s1.json",
            "C--work-alpha/.jsonl",
            "loose.jsonl",
        ] {
            assert_eq!(classify_path(root, &root.join(ignored)), None, "{ignored}");
        }
        assert_eq!(
            classify_path(root, Path::new("/elsewhere/a/s1.jsonl")),
            None
        );
    }

    #[test]
    fn discovery_keeps_existing_unique_dirs_in_order() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        let default = home.join(".claude");
        let other = tmp.path().join("work-claude");
        std::fs::create_dir_all(&default).expect("mkdir");
        std::fs::create_dir_all(&other).expect("mkdir");

        let found = discover_from(
            Some(&home),
            Some(other.as_os_str()),
            &[default.clone(), tmp.path().join("missing")],
        );
        assert_eq!(
            found,
            vec![
                AccountDescriptor {
                    config_dir: default,
                    label: "Claude".into()
                },
                AccountDescriptor {
                    config_dir: other,
                    label: "work-claude".into()
                },
            ]
        );
        assert_eq!(discover_from(None, Some(OsStr::new("")), &[]), Vec::new());
    }
}
