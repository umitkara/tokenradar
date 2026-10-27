//! Project identity from a session's `cwd` (data-sources.md S3, pitfall 5).
//!
//! The `projects/<dir>` name is a lossy encoding of the cwd (`:`, `\` and `/` all become `-`),
//! so the cwd recorded in the transcript is the identity. The directory name is only a
//! fallback for files that carry no cwd at all.

/// `path_norm` prefix of a project known only by its lossy directory name.
pub const UNRESOLVED_PREFIX: &str = "unresolved:";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectKey {
    /// Unique per account. Windows paths are case-folded so `C:\Work` and `c:\work` match.
    pub path_norm: String,
    pub display_name: String,
}

impl ProjectKey {
    /// Normalizes a recorded cwd. Returns `None` for an empty one.
    ///
    /// - Windows paths (`C:\…`, `\\server\…`): `/` becomes `\`, the drive letter is upper
    ///   case, the rest is lower case (Windows compares paths case-insensitively).
    /// - Other paths (WSL, macOS, Linux) keep their case and use `/`.
    /// - A trailing separator is removed, except for a root such as `C:\` or `/`.
    pub fn from_cwd(cwd: &str) -> Option<Self> {
        let cwd = cwd.trim();
        if cwd.is_empty() {
            return None;
        }
        if is_windows_path(cwd) {
            let mut path = cwd.replace('/', "\\");
            trim_trailing(&mut path, '\\');
            if let Some(drive) = path
                .get(..1)
                .filter(|_| path.as_bytes().get(1) == Some(&b':'))
            {
                path.replace_range(..1, &drive.to_ascii_uppercase());
            }
            let display_name = last_component(&path, '\\');
            let mut path_norm = String::with_capacity(path.len());
            path_norm.push_str(&path[..drive_len(&path)]);
            path_norm.push_str(&path[drive_len(&path)..].to_lowercase());
            Some(Self {
                path_norm,
                display_name,
            })
        } else {
            let mut path = cwd.to_owned();
            trim_trailing(&mut path, '/');
            let display_name = last_component(&path, '/');
            Some(Self {
                path_norm: path,
                display_name,
            })
        }
    }

    /// Fallback from the `projects/<dir>` name when no record has a cwd. Marked unresolved
    /// with [`UNRESOLVED_PREFIX`]; the name cannot be decoded back into a path.
    pub fn from_dir_name(dir: &str) -> Self {
        Self {
            path_norm: format!("{UNRESOLVED_PREFIX}{dir}"),
            display_name: dir.to_owned(),
        }
    }

    pub fn is_resolved(&self) -> bool {
        !self.path_norm.starts_with(UNRESOLVED_PREFIX)
    }
}

fn is_windows_path(p: &str) -> bool {
    let b = p.as_bytes();
    (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
        || p.starts_with("\\\\")
        || p.starts_with("//")
}

/// Length of a `C:` prefix, or 0.
fn drive_len(p: &str) -> usize {
    if p.as_bytes().get(1) == Some(&b':') {
        2
    } else {
        0
    }
}

/// Removes trailing separators but keeps a root (`C:\`, `\\`, `/`).
fn trim_trailing(path: &mut String, sep: char) {
    while path.ends_with(sep) {
        let without = &path[..path.len() - 1];
        let is_root = without.is_empty()
            || (without.len() == 2 && drive_len(without) == 2)
            || without == "\\";
        if is_root {
            break;
        }
        path.pop();
    }
}

fn last_component(path: &str, sep: char) -> String {
    path.rsplit(sep)
        .find(|c| !c.is_empty())
        .unwrap_or(path)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(cwd: &str) -> (String, String) {
        let k = ProjectKey::from_cwd(cwd).expect("non-empty");
        (k.path_norm, k.display_name)
    }

    #[test]
    fn windows_paths_fold_case_and_separators() {
        let expected = (
            r"D:\projects\araclar\tokenradar".to_owned(),
            "TokenRadar".to_owned(),
        );
        assert_eq!(key(r"D:\Projects\Araclar\TokenRadar"), expected);
        assert_eq!(
            key(r"d:\projects\araclar\TokenRadar\"),
            (expected.0.clone(), "TokenRadar".into())
        );
        assert_eq!(key("D:/Projects/Araclar/TokenRadar/"), expected);
    }

    #[test]
    fn roots_and_unc_paths_survive() {
        assert_eq!(key(r"c:\"), (r"C:\".into(), r"C:".into()));
        assert_eq!(
            key(r"\\Server\Share\Repo\"),
            (r"\\server\share\repo".into(), "Repo".into())
        );
        assert_eq!(key("/"), ("/".into(), "/".into()));
    }

    #[test]
    fn posix_paths_keep_case() {
        assert_eq!(
            key("/home/Ada/Repo/"),
            ("/home/Ada/Repo".into(), "Repo".into())
        );
    }

    #[test]
    fn non_ascii_names_fold_too() {
        assert_eq!(key(r"C:\Kullanıcılar\ÜMİT\Proje").1, "Proje");
        assert_eq!(key(r"C:\Çalışma").0, key(r"C:\çalışma").0);
    }

    #[test]
    fn empty_cwd_and_dir_fallback() {
        assert!(ProjectKey::from_cwd("  ").is_none());
        let k = ProjectKey::from_dir_name("D--Projects-Araclar-TokenRadar");
        assert!(!k.is_resolved());
        assert_eq!(k.path_norm, "unresolved:D--Projects-Araclar-TokenRadar");
        assert!(ProjectKey::from_cwd(r"C:\x").expect("cwd").is_resolved());
    }
}
