//! Surgical edits of JSONC files: Windows Terminal and VS Code settings (Step 12o, ADR-055).
//!
//! Both files may hold `//` and `/* */` comments and trailing commas. The scanner works on a
//! blanked copy in which every comment byte and every trailing comma is a space (newlines are
//! kept), so its offsets are the original's and the plain-JSON walker of
//! `statusline::settings` applies. Edits are spliced into the original text; the file is never
//! re-serialized, so comments, key order and formatting stay as the user wrote them.

use serde_json::Value;

use crate::statusline::settings::{
    EditError, Member, Object, find, scan_object, skip_string, skip_ws, splice,
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JsoncError {
    #[error("not a valid JSON object")]
    Invalid,
    /// A key on the path holds something other than an object (Windows Terminal's legacy
    /// `profiles` array, for example).
    #[error("`{0}` is not an object")]
    NotAnObject(String),
}

impl From<EditError> for JsoncError {
    fn from(_: EditError) -> Self {
        Self::Invalid
    }
}

/// The document as plain JSON, for reading only.
pub fn parse(text: &str) -> Result<Value, JsoncError> {
    let blank = blank(text)?;
    let body = blank.strip_prefix('\u{feff}').unwrap_or(&blank);
    match serde_json::from_str::<Value>(body) {
        Ok(v @ Value::Object(_)) => Ok(v),
        _ => Err(JsoncError::Invalid),
    }
}

/// The value at `path` exactly as written, or `None` when a key on the path is missing.
pub fn get<'a>(text: &'a str, path: &[&str]) -> Result<Option<&'a str>, JsoncError> {
    let doc = Doc::parse(text)?;
    Ok(match doc.walk(path)? {
        Walk::Found { member, .. } => Some(&text[member.value_start..member.value_end]),
        Walk::Missing { .. } => None,
    })
}

/// Sets the value at `path` to `raw`, a JSON value written verbatim. Missing objects on the
/// path are created, laid out like the rest of the file.
pub fn set(text: &str, path: &[&str], raw: &str) -> Result<String, JsoncError> {
    serde_json::from_str::<Value>(raw).map_err(|_| JsoncError::Invalid)?;
    let doc = Doc::parse(text)?;
    match doc.walk(path)? {
        Walk::Found { member, .. } => Ok(splice(text, member.value_start, member.value_end, raw)),
        Walk::Missing { obj, depth } => Ok(doc.insert(&obj, path[depth], &path[depth + 1..], raw)),
    }
}

/// Removes the member at `path` with its separator. Unchanged when it is absent. Removing a
/// member [`set`] inserted gives back the bytes from before the insertion.
pub fn remove(text: &str, path: &[&str]) -> Result<String, JsoncError> {
    let doc = Doc::parse(text)?;
    let Walk::Found { obj, idx, member } = doc.walk(path)? else {
        return Ok(text.to_owned());
    };
    Ok(doc.remove(&obj, idx, member))
}

struct Doc<'a> {
    text: &'a str,
    /// `text` with comments and trailing commas turned into spaces.
    blank: String,
    root: usize,
}

enum Walk {
    Found {
        obj: Object,
        idx: usize,
        member: Member,
    },
    /// `path[depth]` is not a key of `obj`.
    Missing { obj: Object, depth: usize },
}

impl<'a> Doc<'a> {
    fn parse(text: &'a str) -> Result<Self, JsoncError> {
        let blank = blank(text)?;
        let body = blank.strip_prefix('\u{feff}').unwrap_or(&blank);
        match serde_json::from_str::<Value>(body) {
            Ok(Value::Object(_)) => {}
            _ => return Err(JsoncError::Invalid),
        }
        let root = skip_ws(blank.as_bytes(), blank.len() - body.len());
        Ok(Self { text, blank, root })
    }

    fn walk(&self, path: &[&str]) -> Result<Walk, JsoncError> {
        let mut open = self.root;
        for (depth, key) in path.iter().enumerate() {
            let obj = scan_object(&self.blank, open)?;
            let Some((idx, member)) = find(&obj, key).map(|(i, m)| (i, *m)) else {
                return Ok(Walk::Missing { obj, depth });
            };
            if depth + 1 == path.len() {
                return Ok(Walk::Found { obj, idx, member });
            }
            if self.blank.as_bytes()[member.value_start] != b'{' {
                return Err(JsoncError::NotAnObject(path[..=depth].join(".")));
            }
            open = member.value_start;
        }
        Err(JsoncError::Invalid)
    }

    fn newline(&self) -> &'static str {
        if self.text.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        }
    }

    /// One level of indentation, taken from the root object; `None` for a one-line file.
    fn unit(&self) -> Option<&'a str> {
        let root = scan_object(&self.blank, self.root).ok()?;
        match root.members.first() {
            None => Some("    "),
            Some((_, first)) => own_line_indent(self.text, first.key_start),
        }
    }

    fn insert(&self, obj: &Object, key: &str, rest: &[&str], raw: &str) -> String {
        let text = self.text;
        let nl = self.newline();
        let unit = self.unit();
        let indent = match (obj.members.first(), unit) {
            (_, None) => None,
            (Some((_, first)), Some(_)) => {
                own_line_indent(text, first.key_start).map(str::to_owned)
            }
            (None, Some(unit)) => Some(format!("{}{unit}", line_indent(text, obj.open))),
        };
        let Some(indent) = indent else {
            // A one-line object: `, "key": value` or `"key": value`.
            let member = member(key, rest, raw, None);
            return match obj.members.last() {
                Some((_, last)) => {
                    splice(text, last.value_end, last.value_end, &format!(", {member}"))
                }
                None => splice(text, obj.open + 1, obj.open + 1, &member),
            };
        };
        let member = member(key, rest, raw, Some((&indent, unit.unwrap_or("    "), nl)));
        let Some((_, last)) = obj.members.last() else {
            let inner = &text[obj.open + 1..obj.close];
            let outer = line_indent(text, obj.open);
            if inner.bytes().all(is_ws) {
                return splice(
                    text,
                    obj.open + 1,
                    obj.close,
                    &format!("{nl}{indent}{member}{nl}{outer}"),
                );
            }
            // Only comments inside: keep them, add the member on the line of the `}`.
            return if own_line_indent(text, obj.close).is_some() {
                let at = line_start(text, obj.close);
                splice(text, at, at, &format!("{indent}{member}{nl}"))
            } else {
                splice(
                    text,
                    obj.close,
                    obj.close,
                    &format!("{nl}{indent}{member}{nl}{outer}"),
                )
            };
        };
        let eol = text[last.value_end..]
            .find(['\r', '\n'])
            .map_or(text.len(), |i| last.value_end + i);
        let trailing = self.trailing_comma(last.value_end, obj.close);
        if eol < obj.close {
            // A comment after the last value stays on its line: the new member goes below it.
            match trailing {
                Some(t) if t < eol => {
                    return splice(text, eol, eol, &format!("{nl}{indent}{member},"));
                }
                None => {
                    let out = splice(text, eol, eol, &format!("{nl}{indent}{member}"));
                    return splice(&out, last.value_end, last.value_end, ",");
                }
                Some(_) => {}
            }
        }
        splice(
            text,
            last.value_end,
            last.value_end,
            &format!(",{nl}{indent}{member}"),
        )
    }

    fn remove(&self, obj: &Object, idx: usize, m: Member) -> String {
        let text = self.text;
        let b = self.blank.as_bytes();
        let own_line = own_line_indent(text, m.key_start).is_some();
        let from = if own_line {
            before_newline(text, line_start(text, m.key_start))
        } else {
            m.key_start
        };
        let trailing = self.trailing_comma(m.value_end, obj.close);
        if let Some((_, next)) = obj.members.get(idx + 1) {
            let comma = (m.value_end..next.key_start)
                .find(|&p| b[p] == b',')
                .unwrap_or(m.value_end);
            let mut to = comma + 1;
            if !own_line {
                while matches!(text.as_bytes().get(to), Some(b' ' | b'\t')) {
                    to += 1;
                }
            }
            return splice(text, from, to, "");
        }
        if let Some(t) = trailing.filter(|&t| own_line && text[m.value_end..t].bytes().all(is_ws)) {
            // The member carried the trailing comma: it goes with it, the comma before stays.
            return splice(text, from, t + 1, "");
        }
        if idx == 0 {
            let to = trailing.map_or(m.value_end, |t| t + 1);
            let rest_blank = text[obj.open + 1..from].bytes().all(is_ws)
                && text[to..obj.close].bytes().all(is_ws);
            return if rest_blank {
                splice(text, obj.open + 1, obj.close, "")
            } else {
                splice(text, from, to, "")
            };
        }
        let prev = obj.members[idx - 1].1;
        let comma = (prev.value_end..m.key_start)
            .rev()
            .find(|&p| b[p] == b',')
            .unwrap_or(prev.value_end);
        if text[comma + 1..from].bytes().all(is_ws) {
            return splice(text, comma, m.value_end, "");
        }
        let out = splice(text, from, m.value_end, "");
        splice(&out, comma, comma + 1, "")
    }

    /// The first trailing comma (a comma in the text that the blanked copy dropped) in
    /// `from..to`.
    fn trailing_comma(&self, from: usize, to: usize) -> Option<usize> {
        let (t, b) = (self.text.as_bytes(), self.blank.as_bytes());
        (from..to).find(|&p| t[p] == b',' && b[p] == b' ')
    }
}

/// `"key": value`, with `rest` turned into nested objects around `raw`. `layout` is
/// `(indent of this member, one level, newline)`; `None` writes everything on one line.
fn member(key: &str, rest: &[&str], raw: &str, layout: Option<(&str, &str, &str)>) -> String {
    let key = serde_json::to_string(key).unwrap_or_default();
    let Some((&next, deeper)) = rest.split_first() else {
        return format!("{key}: {raw}");
    };
    match layout {
        None => format!("{key}: {{{}}}", member(next, deeper, raw, None)),
        Some((indent, unit, nl)) => {
            let inner = format!("{indent}{unit}");
            let child = member(next, deeper, raw, Some((&inner, unit, nl)));
            format!("{key}: {{{nl}{inner}{child}{nl}{indent}}}")
        }
    }
}

fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n')
}

fn line_start(text: &str, pos: usize) -> usize {
    text[..pos].rfind('\n').map_or(0, |i| i + 1)
}

/// The indentation before `pos` when only spaces and tabs precede it on its line.
fn own_line_indent(text: &str, pos: usize) -> Option<&str> {
    let indent = &text[line_start(text, pos)..pos];
    indent
        .bytes()
        .all(|c| c == b' ' || c == b'\t')
        .then_some(indent)
}

/// The leading whitespace of the line holding `pos`.
fn line_indent(text: &str, pos: usize) -> &str {
    let line = &text[line_start(text, pos)..];
    let len = line
        .bytes()
        .take_while(|&c| c == b' ' || c == b'\t')
        .count();
    &line[..len]
}

/// `at` moved back over the line break before it, if there is one.
fn before_newline(text: &str, mut at: usize) -> usize {
    let b = text.as_bytes();
    if at > 0 && b[at - 1] == b'\n' {
        at -= 1;
        if at > 0 && b[at - 1] == b'\r' {
            at -= 1;
        }
    }
    at
}

/// `text` with every comment byte (except line breaks) and every trailing comma replaced by a
/// space, so byte offsets stay the same and the result is plain JSON.
fn blank(text: &str) -> Result<String, JsoncError> {
    let src = text.as_bytes();
    let mut out = src.to_vec();
    let mut i = 0;
    while i < src.len() {
        match (src[i], src.get(i + 1)) {
            (b'"', _) => i = skip_string(src, i)?,
            (b'/', Some(b'/')) => {
                while i < src.len() && src[i] != b'\n' {
                    if src[i] != b'\r' {
                        out[i] = b' ';
                    }
                    i += 1;
                }
            }
            (b'/', Some(b'*')) => {
                let end = text[i + 2..]
                    .find("*/")
                    .map(|e| i + 2 + e + 2)
                    .ok_or(JsoncError::Invalid)?;
                for c in &mut out[i..end] {
                    if !matches!(*c, b'\r' | b'\n') {
                        *c = b' ';
                    }
                }
                i = end;
            }
            _ => i += 1,
        }
    }
    let mut i = 0;
    while i < out.len() {
        match out[i] {
            b'"' => i = skip_string(&out, i)?,
            b',' => {
                if matches!(out.get(skip_ws(&out, i + 1)), Some(b'}' | b']')) {
                    out[i] = b' ';
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    // Every replaced byte belonged to a comment or was ASCII, so whole characters went.
    String::from_utf8(out).map_err(|_| JsoncError::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FACE: [&str; 4] = ["profiles", "defaults", "font", "face"];
    const NAME: &str = "\"CaskaydiaMono Nerd Font Mono\"";

    /// Set, check the value parses back, then remove: the original bytes must return.
    fn mirror(text: &str, path: &[&str]) -> String {
        let out = set(text, path, NAME).expect("set");
        let v = parse(&out).expect("still valid");
        let got = path.iter().fold(&v, |v, k| &v[*k]);
        assert_eq!(got, "CaskaydiaMono Nerd Font Mono", "{out}");
        assert_eq!(get(&out, path).expect("get"), Some(NAME));
        if path.len() == 1 || get(text, &path[..path.len() - 1]).expect("get").is_some() {
            assert_eq!(remove(&out, path).expect("remove"), text, "{out}");
        }
        out
    }

    #[test]
    fn comments_and_trailing_commas_are_read_and_kept() {
        let text = "// generated by Windows Terminal\n{\n    \"$help\": \"https://aka.ms/x\", /* a // b */\n    \"profiles\":\n    {\n        \"defaults\": {}, // here\n        \"list\": [ { \"name\": \"pwsh\", }, ],\n    },\n}\n";
        let out = mirror(text, &FACE);
        assert_eq!(
            out,
            "// generated by Windows Terminal\n{\n    \"$help\": \"https://aka.ms/x\", /* a // b */\n    \"profiles\":\n    {\n        \"defaults\": {\n            \"font\": {\n                \"face\": \"CaskaydiaMono Nerd Font Mono\"\n            }\n        }, // here\n        \"list\": [ { \"name\": \"pwsh\", }, ],\n    },\n}\n"
        );
    }

    #[test]
    fn existing_values_are_replaced_in_place() {
        let text = "{\r\n\t\"profiles\": {\r\n\t\t\"defaults\": {\r\n\t\t\t\"font\": { \"face\": \"Consolas\", \"size\": 11 } // mine\r\n\t\t}\r\n\t}\r\n}";
        let out = set(text, &FACE, NAME).expect("set");
        assert_eq!(out, text.replace("\"Consolas\"", NAME));
        assert_eq!(get(text, &FACE).expect("get"), Some("\"Consolas\""));
    }

    #[test]
    fn a_new_member_lands_below_a_comment_after_the_last_value() {
        let text = "{\n    \"editor.fontSize\": 14 // big\n}\n";
        let out = mirror(text, &["terminal.integrated.fontFamily"]);
        assert_eq!(
            out,
            "{\n    \"editor.fontSize\": 14, // big\n    \"terminal.integrated.fontFamily\": \"CaskaydiaMono Nerd Font Mono\"\n}\n"
        );
        let trailing = "{\r\n    \"a\": 1, // c\r\n}";
        let out = mirror(trailing, &["k"]);
        assert_eq!(
            out,
            "{\r\n    \"a\": 1, // c\r\n    \"k\": \"CaskaydiaMono Nerd Font Mono\",\r\n}"
        );
    }

    #[test]
    fn layouts_round_trip() {
        for text in [
            "{}",
            "{\"a\":1}",
            "{\"a\":1,}",
            "{\n  \"a\": [1, 2]\n}\n",
            "\u{feff}{\n\t\"a\": null\n}",
            "{\n    \"profiles\": {\n        \"defaults\": { /* empty */ }\n    }\n}",
            "{\n    \"profiles\": {\n        \"defaults\": {\n            // nothing yet\n        }\n    }\n}",
            "{\n  \"s\": \"// not a comment, }\"\n}",
        ] {
            mirror(text, &["k"]);
            if text.contains("defaults") {
                mirror(text, &FACE);
            }
        }
    }

    #[test]
    fn removal_handles_neighbours() {
        let text = "{\n    \"a\": 1,\n    \"k\": 2, // two\n    \"b\": 3\n}";
        assert_eq!(
            remove(text, &["k"]).expect("remove"),
            "{\n    \"a\": 1, // two\n    \"b\": 3\n}"
        );
        assert_eq!(
            remove("{\"k\":1, \"b\":2}", &["k"]).expect("remove"),
            "{\"b\":2}"
        );
        assert_eq!(remove("{\"a\":1}", &["k"]).expect("absent"), "{\"a\":1}");
    }

    #[test]
    fn wrong_shapes_are_refused() {
        let legacy = "{ \"profiles\": [ { \"name\": \"cmd\" } ] }";
        assert_eq!(
            set(legacy, &FACE, NAME),
            Err(JsoncError::NotAnObject("profiles".into()))
        );
        assert_eq!(
            get(legacy, &FACE),
            Err(JsoncError::NotAnObject("profiles".into()))
        );
        for bad in ["", "[]", "{\"a\":}", "{\"a\":1} x", "{ /* open"] {
            assert_eq!(set(bad, &["k"], NAME), Err(JsoncError::Invalid), "{bad}");
        }
        assert_eq!(set("{}", &["k"], "{nope"), Err(JsoncError::Invalid));
    }

    #[test]
    fn blanking_keeps_offsets_and_multibyte_text() {
        let text = "{ \"a\": \"ç//ş\", // ğüış\n /* İ */ \"b\": 1, }";
        let b = blank(text).expect("blank");
        assert_eq!(b.len(), text.len());
        assert!(b.contains("\"ç//ş\""), "{b}");
        assert_eq!(b.matches('/').count(), 2, "{b}");
        assert!(!b.contains(['*', 'İ', 'ğ', ']']), "{b}");
        let v = parse(text).expect("parse");
        assert_eq!((v["a"].as_str(), v["b"].as_i64()), (Some("ç//ş"), Some(1)));
    }
}
