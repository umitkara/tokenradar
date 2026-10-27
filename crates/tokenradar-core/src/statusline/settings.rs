//! Surgical edits of Claude Code's `settings.json` (ADR-009, ADR-029).
//!
//! The file is never re-serialized. Only the byte range of one top-level member changes, so
//! the user's formatting, key order and every other setting stay exactly as they were.
//! Insertion and removal mirror each other: installing into a file without `statusLine` and
//! uninstalling again gives back the original bytes.

use std::fmt::Write as _;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EditError {
    #[error("not a valid JSON object")]
    Invalid,
}

/// Byte offsets of one top-level member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Member {
    key_start: usize,
    value_start: usize,
    value_end: usize,
}

struct Object {
    open: usize,
    close: usize,
    members: Vec<(String, Member)>,
}

/// The value of top-level `key` exactly as written, or `None` without that key.
pub fn raw_member<'a>(text: &'a str, key: &str) -> Result<Option<&'a str>, EditError> {
    let obj = scan(text)?;
    Ok(find(&obj, key).map(|(_, m)| &text[m.value_start..m.value_end]))
}

/// Sets top-level `key` to `value`, formatted like the rest of the file. Fields are written
/// in `value`'s own order (a struct's declaration order).
pub fn set_member(text: &str, key: &str, value: &impl Serialize) -> Result<String, EditError> {
    let obj = scan(text)?;
    let layout = Layout::detect(text, &obj);
    set_raw(text, &obj, key, &layout.render(value), &layout)
}

/// Sets top-level `key` to `raw`, a JSON value written verbatim (to restore a saved value).
pub fn set_member_raw(text: &str, key: &str, raw: &str) -> Result<String, EditError> {
    serde_json::from_str::<Value>(raw).map_err(|_| EditError::Invalid)?;
    let obj = scan(text)?;
    let layout = Layout::detect(text, &obj);
    set_raw(text, &obj, key, raw, &layout)
}

/// Removes top-level `key` with its separator. Unchanged if the key is absent.
pub fn remove_member(text: &str, key: &str) -> Result<String, EditError> {
    let obj = scan(text)?;
    let Some((idx, m)) = find(&obj, key) else {
        return Ok(text.to_owned());
    };
    let (from, to) = if obj.members.len() == 1 {
        (obj.open + 1, obj.close)
    } else if idx > 0 {
        // `,⏎  "key": value` after the previous member: the mirror of an insertion.
        (obj.members[idx - 1].1.value_end, m.value_end)
    } else {
        (m.key_start, obj.members[1].1.key_start)
    };
    Ok(splice(text, from, to, ""))
}

fn set_raw(
    text: &str,
    obj: &Object,
    key: &str,
    raw: &str,
    layout: &Layout,
) -> Result<String, EditError> {
    if let Some((_, m)) = find(obj, key) {
        return Ok(splice(text, m.value_start, m.value_end, raw));
    }
    let key = serde_json::to_string(key).map_err(|_| EditError::Invalid)?;
    Ok(match obj.members.last() {
        None => {
            let nl = &layout.newline;
            let member = match &layout.indent {
                Some(indent) => format!("{nl}{indent}{key}: {raw}{nl}"),
                None => format!("{key}: {raw}"),
            };
            splice(text, obj.open + 1, obj.close, &member)
        }
        Some((_, last)) => {
            let member = match &layout.indent {
                Some(indent) => format!(",{}{indent}{key}: {raw}", layout.newline),
                None => format!(", {key}: {raw}"),
            };
            splice(text, last.value_end, last.value_end, &member)
        }
    })
}

fn find<'o>(obj: &'o Object, key: &str) -> Option<(usize, &'o Member)> {
    // Duplicate keys: the last one wins, as in every JSON parser Claude Code could use.
    obj.members
        .iter()
        .enumerate()
        .rev()
        .find(|(_, (k, _))| k == key)
        .map(|(i, (_, m))| (i, m))
}

fn splice(text: &str, from: usize, to: usize, with: &str) -> String {
    let mut out = String::with_capacity(text.len() - (to - from) + with.len());
    out.push_str(&text[..from]);
    out.push_str(with);
    out.push_str(&text[to..]);
    out
}

/// How the file is written: one member per line with an indent, or all on one line.
struct Layout {
    /// `None`: compact, single line.
    indent: Option<String>,
    newline: String,
}

impl Layout {
    fn detect(text: &str, obj: &Object) -> Self {
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" }.to_owned();
        let indent = match obj.members.first() {
            None => Some("  ".to_owned()),
            Some((_, first)) => {
                let before = &text[obj.open + 1..first.key_start];
                before.rfind('\n').map(|nl| before[nl + 1..].to_owned())
            }
        };
        Self { indent, newline }
    }

    fn render(&self, value: &impl Serialize) -> String {
        let compact = || serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned());
        let Some(indent) = &self.indent else {
            return compact();
        };
        let mut buf = Vec::new();
        let fmt = serde_json::ser::PrettyFormatter::with_indent(indent.as_bytes());
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
        if value.serialize(&mut ser).is_err() {
            return compact();
        }
        let pretty = String::from_utf8(buf).unwrap_or_else(|_| compact());
        // Nested lines sit one level deeper than the member itself.
        pretty.replace('\n', &format!("{}{indent}", self.newline))
    }
}

/// Validates the whole document, then records where each top-level member sits.
fn scan(text: &str) -> Result<Object, EditError> {
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(_)) => {}
        _ => return Err(EditError::Invalid),
    }
    let b = text.as_bytes();
    let mut i = text.len() - body.len();
    i = skip_ws(b, i);
    if b.get(i) != Some(&b'{') {
        return Err(EditError::Invalid);
    }
    let open = i;
    i += 1;
    let mut members = Vec::new();
    loop {
        i = skip_ws(b, i);
        match b.get(i) {
            Some(b'}') => {
                return Ok(Object {
                    open,
                    close: i,
                    members,
                });
            }
            Some(b',') => {
                i += 1;
                continue;
            }
            Some(b'"') => {}
            _ => return Err(EditError::Invalid),
        }
        let key_start = i;
        let key_end = skip_string(b, i)?;
        let key: String =
            serde_json::from_str(&text[key_start..key_end]).map_err(|_| EditError::Invalid)?;
        i = skip_ws(b, key_end);
        if b.get(i) != Some(&b':') {
            return Err(EditError::Invalid);
        }
        let value_start = skip_ws(b, i + 1);
        let value_end = skip_value(b, value_start)?;
        members.push((
            key,
            Member {
                key_start,
                value_start,
                value_end,
            },
        ));
        i = value_end;
    }
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while matches!(b.get(i), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        i += 1;
    }
    i
}

/// `b[i]` is an opening quote; returns the index after the closing one.
fn skip_string(b: &[u8], mut i: usize) -> Result<usize, EditError> {
    i += 1;
    while let Some(&c) = b.get(i) {
        match c {
            b'\\' => i += 2,
            b'"' => return Ok(i + 1),
            _ => i += 1,
        }
    }
    Err(EditError::Invalid)
}

fn skip_value(b: &[u8], i: usize) -> Result<usize, EditError> {
    match b.get(i) {
        Some(b'"') => skip_string(b, i),
        Some(b'{' | b'[') => {
            let mut depth = 0usize;
            let mut j = i;
            while let Some(&c) = b.get(j) {
                match c {
                    b'"' => {
                        j = skip_string(b, j)?;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return Ok(j + 1);
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            Err(EditError::Invalid)
        }
        Some(_) => {
            let mut j = i;
            while let Some(&c) = b.get(j) {
                if matches!(c, b',' | b'}' | b']' | b' ' | b'\t' | b'\r' | b'\n') {
                    break;
                }
                j += 1;
            }
            Ok(j)
        }
        None => Err(EditError::Invalid),
    }
}

/// A unified diff of a change that touches one contiguous region, as the edits above do.
pub fn unified_diff(old: &str, new: &str, label: &str) -> String {
    const CONTEXT: usize = 3;
    let a: Vec<&str> = old.split_inclusive('\n').collect();
    let b: Vec<&str> = new.split_inclusive('\n').collect();
    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let max_suffix = a.len().min(b.len()) - prefix;
    let suffix = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(max_suffix)
        .take_while(|(x, y)| x == y)
        .count();
    if prefix == a.len() && prefix == b.len() {
        return String::new();
    }
    let start = prefix.saturating_sub(CONTEXT);
    let a_end = (a.len() - suffix + CONTEXT).min(a.len());
    let b_end = (b.len() - suffix + CONTEXT).min(b.len());
    let line = |s: &str| s.trim_end_matches(['\r', '\n']).to_owned();
    let mut out = format!("--- {label}\n+++ {label} (TokenRadar)\n");
    let _ = writeln!(
        out,
        "@@ -{},{} +{},{} @@",
        start + 1,
        a_end - start,
        start + 1,
        b_end - start
    );
    for l in &a[start..prefix] {
        let _ = writeln!(out, " {}", line(l));
    }
    for l in &a[prefix..a.len() - suffix] {
        let _ = writeln!(out, "-{}", line(l));
    }
    for l in &b[prefix..b.len() - suffix] {
        let _ = writeln!(out, "+{}", line(l));
    }
    for l in &a[a.len() - suffix..a_end] {
        let _ = writeln!(out, " {}", line(l));
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const KEY: &str = "statusLine";

    #[derive(Serialize)]
    struct Command {
        #[serde(rename = "type")]
        kind: &'static str,
        command: &'static str,
    }

    fn ours() -> Command {
        Command {
            kind: "command",
            command: "C:/Tools/tokenradar.exe statusline",
        }
    }

    fn ours_value() -> Value {
        json!({"type": "command", "command": "C:/Tools/tokenradar.exe statusline"})
    }

    /// Install into `text`, then remove: the result must be the original bytes.
    fn round_trip(text: &str) {
        let installed = set_member(text, KEY, &ours()).expect("install");
        let parsed: Value =
            serde_json::from_str(installed.trim_start_matches('\u{feff}')).expect("valid JSON");
        assert_eq!(parsed[KEY], ours_value(), "{installed}");
        assert_eq!(remove_member(&installed, KEY).expect("remove"), text);
    }

    #[test]
    fn insertion_and_removal_are_exact_mirrors() {
        round_trip("{\n  \"model\": \"opus\",\n  \"env\": {\n    \"A\": \"1\"\n  }\n}\n");
        round_trip("{\r\n    \"a\": [1, 2],\r\n    \"b\": true\r\n}");
        round_trip("{\"a\":1,\"b\":{\"c\":\"}\"}}");
        round_trip("\u{feff}{\n\t\"a\": null\n}\n");
        round_trip("{}");
        round_trip("{\n  \"weird \\\" key\": \"x, }\"\n}\n");
    }

    #[test]
    fn insertion_matches_the_files_indentation() {
        let text = "{\n    \"model\": \"opus\"\n}\n";
        let out = set_member(text, KEY, &ours()).expect("install");
        assert_eq!(
            out,
            "{\n    \"model\": \"opus\",\n    \"statusLine\": {\n        \"type\": \"command\",\n        \
             \"command\": \"C:/Tools/tokenradar.exe statusline\"\n    }\n}\n"
        );
    }

    #[test]
    fn replacing_and_restoring_keeps_the_previous_value_verbatim() {
        let text = "{\n  \"statusLine\": {\"type\":\"command\",   \"command\": \"python s.py\"},\n  \"z\": 1\n}\n";
        let previous = raw_member(text, KEY).expect("scan").expect("present");
        assert_eq!(
            previous,
            "{\"type\":\"command\",   \"command\": \"python s.py\"}"
        );
        let installed = set_member(text, KEY, &ours()).expect("install");
        assert!(installed.ends_with("  },\n  \"z\": 1\n}\n"), "{installed}");
        let restored = set_member_raw(&installed, KEY, previous).expect("restore");
        assert_eq!(restored, text);
    }

    #[test]
    fn removing_the_first_of_several_members() {
        let text = "{\n  \"statusLine\": 1,\n  \"b\": 2\n}";
        assert_eq!(
            remove_member(text, KEY).expect("remove"),
            "{\n  \"b\": 2\n}"
        );
        assert_eq!(
            remove_member("{\"a\":1}", KEY).expect("absent"),
            "{\"a\":1}"
        );
    }

    #[test]
    fn invalid_documents_are_refused() {
        for bad in ["", "[]", "{\"a\":}", "{\"a\":1,}", "{\"a\":1} x", "\"s\""] {
            assert_eq!(
                set_member(bad, KEY, &ours()),
                Err(EditError::Invalid),
                "{bad}"
            );
        }
        assert_eq!(
            set_member_raw("{}", KEY, "{not json"),
            Err(EditError::Invalid)
        );
    }

    #[test]
    fn diff_shows_one_hunk_with_context() {
        let old = "{\n  \"a\": 1,\n  \"b\": 2\n}\n";
        let new = set_member(old, KEY, &json!(true)).expect("set");
        let diff = unified_diff(old, &new, "settings.json");
        assert_eq!(
            diff,
            "--- settings.json\n+++ settings.json (TokenRadar)\n@@ -1,4 +1,5 @@\n {\n   \"a\": 1,\n\
             -  \"b\": 2\n+  \"b\": 2,\n+  \"statusLine\": true\n }\n"
        );
        assert_eq!(unified_diff(old, old, "x"), "");
    }
}
