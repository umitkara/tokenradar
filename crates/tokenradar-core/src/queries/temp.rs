//! Temporary-folder patterns (Step 16b): the `[data] temp_dirs` of the config, turned into SQL
//! `LIKE` patterns matched against `projects.path_norm`.
//!
//! A pattern is a part of a path matched anywhere in it: `*` stands for any text, `?` for one
//! character, `%NAME%` for an environment variable. Matching ignores case, as `path_norm` is
//! case-folded on Windows and `LIKE` folds ASCII. A pattern naming an unknown variable matches
//! nothing, rather than turning into a wildcard.

/// `LIKE … ESCAPE '\'` patterns for `patterns`; `env` looks up a variable. A pattern with a
/// separator gives a `\` and a `/` version, so it also matches WSL and macOS paths.
pub fn temp_like_patterns(
    patterns: &[String],
    env: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for pattern in patterns {
        let Some(text) = expand(pattern.trim(), &env) else {
            continue;
        };
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        let back = to_like(&text.replace('/', "\\"));
        let slash = to_like(&text.replace('\\', "/"));
        if !out.contains(&back) {
            out.push(back.clone());
        }
        if slash != back && !out.contains(&slash) {
            out.push(slash);
        }
    }
    out
}

/// The SQL condition "`col` matches one of the `LIKE` parameters `?first`…", or `None` for
/// none.
pub(crate) fn like_any(col: &str, first: usize, count: usize) -> Option<String> {
    (count > 0).then(|| {
        let parts: Vec<String> = (first..first + count)
            .map(|i| format!("{col} LIKE ?{i} ESCAPE '\\'"))
            .collect();
        format!("({})", parts.join(" OR "))
    })
}

/// `%NAME%` replaced by the variable's value; `None` when a named variable is unknown. A `%`
/// without a closing one, or around something that is not a variable name, stays as it is.
fn expand(text: &str, env: &impl Fn(&str) -> Option<String>) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if is_var_name(&after[..end]) => {
                out.push_str(&env(&after[..end])?);
                rest = &after[end + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    Some(out)
}

/// Windows variable names as people write them: letters, digits, `_`, and the parentheses of
/// `ProgramFiles(x86)`.
fn is_var_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '(' | ')'))
}

/// A path part → `%part%`, `LIKE`'s own characters escaped, `*` and `?` as wildcards, lower case
/// (`path_norm` is folded with the same `to_lowercase`).
fn to_like(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('%');
    for c in text.to_lowercase().chars() {
        match c {
            '*' => out.push('%'),
            '?' => out.push('_'),
            '%' | '_' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push('%');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str) -> Option<String> {
        match name.to_ascii_uppercase().as_str() {
            "TEMP" => Some(r"C:\Users\Ada\AppData\Local\Temp".into()),
            "PROGRAMFILES(X86)" => Some(r"C:\Program Files (x86)".into()),
            _ => None,
        }
    }

    fn like(patterns: &[&str]) -> Vec<String> {
        let owned: Vec<String> = patterns.iter().map(|&p| p.to_owned()).collect();
        temp_like_patterns(&owned, env)
    }

    #[test]
    fn variables_expand_and_case_folds() {
        assert_eq!(
            like(&["%TEMP%"]),
            [
                r"%c:\\users\\ada\\appdata\\local\\temp%",
                "%c:/users/ada/appdata/local/temp%"
            ]
        );
        assert_eq!(
            like(&["%ProgramFiles(x86)%"])[0],
            r"%c:\\program files (x86)%"
        );
    }

    #[test]
    fn separators_give_both_forms_and_wildcards_translate() {
        assert_eq!(like(&[r"\Temp\"]), [r"%\\temp\\%", "%/temp/%"]);
        assert_eq!(like(&["build-*"]), ["%build-%%"]);
        assert_eq!(like(&["a?c"]), ["%a_c%"]);
        assert_eq!(like(&["100%_done"]), [r"%100\%\_done%"]);
    }

    #[test]
    fn unknown_variables_and_empty_patterns_match_nothing() {
        assert_eq!(like(&["%NOPE%", "  ", r"%NOPE%\x"]), Vec::<String>::new());
        assert_eq!(like(&["%%"]), [r"%\%\%%"]);
        assert_eq!(like(&["50% off"]), [r"%50\% off%"]);
    }

    #[test]
    fn repeated_patterns_are_kept_once() {
        assert_eq!(like(&["temp", "TEMP"]), ["%temp%"]);
    }

    #[test]
    fn like_any_joins_parameters() {
        assert_eq!(like_any("p.x", 3, 0), None);
        assert_eq!(
            like_any("p.x", 3, 2).as_deref(),
            Some(r"(p.x LIKE ?3 ESCAPE '\' OR p.x LIKE ?4 ESCAPE '\')")
        );
    }
}
