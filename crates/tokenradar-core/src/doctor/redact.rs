//! Removes secrets and paths from text that goes into a diagnostics bundle.
//!
//! Paths become `path#<8 hex>`: the first bytes of the SHA-256 of the case-folded, `/`-separated
//! path. The same directory gets the same tag in the log, the config and the account list, so
//! a bundle can still be read ("the watcher failed on the account directory") without saying
//! where anything is.

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

/// Lines mentioning these are dropped whole: a credential field must never be quoted, even
/// redacted.
const CREDENTIAL_FIELDS: [&str; 3] = ["accessToken", "refreshToken", "access_token"];

/// The tag for `path`.
pub fn path_tag(path: &str) -> String {
    let norm = path
        .trim_end_matches(['\\', '/'])
        .replace("\\\\", "/")
        .replace('\\', "/")
        .to_lowercase();
    let digest = Sha256::digest(norm.as_bytes());
    let mut tag = String::from("path#");
    for b in &digest[..4] {
        let _ = write!(tag, "{b:02x}");
    }
    tag
}

/// One log line, made safe to share.
pub fn line(text: &str) -> String {
    if CREDENTIAL_FIELDS.iter().any(|f| text.contains(f)) {
        return "<line removed: credential field>".to_owned();
    }
    paths(&secrets(text))
}

/// `sk-ant-…` keys and `Bearer …` tokens become `***`.
fn secrets(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let next = [("sk-ant-", "sk-ant-***"), ("Bearer ", "Bearer ***")]
            .into_iter()
            .filter_map(|(needle, mask)| rest.find(needle).map(|i| (i, needle, mask)))
            .min_by_key(|(i, ..)| *i);
        let Some((i, needle, mask)) = next else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..i]);
        out.push_str(mask);
        let after = &rest[i + needle.len()..];
        let token_end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || "-_.~+/=".contains(c)))
            .unwrap_or(after.len());
        rest = &after[token_end..];
    }
}

/// Replaces every absolute path with its tag.
fn paths(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if let Some(len) = path_at(text, i) {
            out.push_str(&path_tag(&text[i..i + len]));
            i += len;
        } else {
            let c = text[i..].chars().next().unwrap_or(' ');
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

/// The byte length of a path starting at `i`, if one does.
fn path_at(text: &str, i: usize) -> Option<usize> {
    let rest = &text[i..];
    let bytes = rest.as_bytes();
    // A path only starts at a word boundary: `abc:\` is not a drive.
    if text[..i]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '_')
    {
        return None;
    }
    let prefix = if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
    {
        3
    } else if rest.starts_with("\\\\") {
        2
    } else if ["/home/", "/Users/", "/mnt/", "/root/", "/tmp/", "/var/"]
        .iter()
        .any(|p| rest.starts_with(p))
    {
        1
    } else {
        return None;
    };
    Some(prefix + path_len(&rest[prefix..]))
}

/// How far a path runs. Spaces are allowed inside it (`Ada Lovelace`), so it ends at a
/// character no file name can hold, or at the next `key=` field of a log line. Taking too
/// much only redacts more.
fn path_len(rest: &str) -> usize {
    let mut end = 0;
    for (i, c) in rest.char_indices() {
        if matches!(
            c,
            '"' | '\'' | '<' | '>' | '|' | '*' | '?' | ':' | '\n' | '\r' | '\t'
        ) {
            return i;
        }
        if c == ' ' && starts_field(&rest[i + 1..]) {
            return i;
        }
        end = i + c.len_utf8();
    }
    end
}

/// `key=` as `tracing` writes its fields.
fn starts_field(s: &str) -> bool {
    let key_len = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
        .unwrap_or(s.len());
    key_len > 0 && s[key_len..].starts_with('=')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_bearer_tokens_are_masked() {
        assert_eq!(
            line("auth failed for sk-ant-oat01-AbC_d-9 status=401"),
            "auth failed for sk-ant-*** status=401"
        );
        assert_eq!(
            line("header Bearer eyJhbGciOi.x.y sent"),
            "header Bearer *** sent"
        );
    }

    #[test]
    fn credential_fields_drop_the_line() {
        assert_eq!(
            line(r#"parse {"accessToken":"abc"}"#),
            "<line removed: credential field>"
        );
    }

    #[test]
    fn paths_become_stable_tags() {
        let tag = path_tag(r"C:\Users\Ada Lovelace\.claude");
        assert_eq!(tag, path_tag("c:/users/ada lovelace/.claude/"));
        assert_eq!(tag.len(), "path#".len() + 8);
        let out =
            line(r"WARN could not watch directory dir=C:\Users\Ada Lovelace\.claude error=denied");
        assert_eq!(
            out,
            format!("WARN could not watch directory dir={tag} error=denied")
        );
    }

    #[test]
    fn quoted_unc_unix_and_escaped_paths_are_tagged() {
        for (input, path) in [
            (
                r#"file="D:\Projects\secret\a.jsonl""#,
                r"D:\Projects\secret\a.jsonl",
            ),
            (
                r"open \\wsl$\Ubuntu\home\ada: denied",
                r"\\wsl$\Ubuntu\home\ada",
            ),
            (
                "read /home/ada/.claude/x.json: gone",
                "/home/ada/.claude/x.json",
            ),
            (r#"path: "C:\\Users\\Ada\\x""#, r"C:\\Users\\Ada\\x"),
        ] {
            let out = line(input);
            assert!(out.contains(&path_tag(path)), "{input} → {out}");
            assert!(!out.to_lowercase().contains("ada"), "{input} → {out}");
            assert!(!out.contains("secret"), "{input} → {out}");
        }
    }

    #[test]
    fn plain_text_is_untouched() {
        for s in [
            "index_progress Recent files=12/40",
            "limits_updated api=Active requests=3 at 10:59",
            "ratio 3/4 and a/b",
            "Ünicode çğış",
        ] {
            assert_eq!(line(s), s);
        }
    }
}
