//! `tokenradar://` launch links: what a toast click (ADR-040) and `tokenradar open` (Step 15)
//! pass to `tokenradar-app.exe`. The installer registers the scheme; the single-instance plugin
//! hands a second launch's arguments to the running app.
//!
//! Only these forms are accepted, case-insensitively and with one optional trailing slash
//! (Windows normalizes a toast's `launch` value to `tokenradar://panel/`):
//! - `tokenradar://panel`
//! - `tokenradar://main` and `tokenradar://main/<route>`
//!
//! Anything else (another host, a query, an unknown route, extra segments) is not a link.

use serde::{Deserialize, Serialize};
use specta::Type;

pub const SCHEME: &str = "tokenradar://";

/// A page of the main window, in navigation order (`Ctrl+1..5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum MainRoute {
    Overview,
    Windows,
    Sessions,
    Projects,
    Patterns,
}

impl MainRoute {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Windows,
        Self::Sessions,
        Self::Projects,
        Self::Patterns,
    ];

    /// The name in links and in the page's URL fragment (`main.html#sessions`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Windows => "windows",
            Self::Sessions => "sessions",
            Self::Projects => "projects",
            Self::Patterns => "patterns",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|r| r.as_str().eq_ignore_ascii_case(name))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Panel,
    /// `None`: the main window as it is (or its default page when it opens).
    Main(Option<MainRoute>),
}

impl Link {
    /// The launch argument for this link.
    pub fn uri(self) -> String {
        match self {
            Self::Panel => format!("{SCHEME}panel"),
            Self::Main(None) => format!("{SCHEME}main"),
            Self::Main(Some(route)) => format!("{SCHEME}main/{}", route.as_str()),
        }
    }

    /// Parses one launch argument; `None` when it is not one of the accepted forms.
    pub fn parse(arg: &str) -> Option<Self> {
        let scheme = arg.get(..SCHEME.len())?;
        if !scheme.eq_ignore_ascii_case(SCHEME) {
            return None;
        }
        let rest = &arg[SCHEME.len()..];
        let rest = rest.strip_suffix('/').unwrap_or(rest);
        let (host, path) = match rest.split_once('/') {
            Some((host, path)) => (host, Some(path)),
            None => (rest, None),
        };
        if host.eq_ignore_ascii_case("panel") {
            return path.is_none().then_some(Self::Panel);
        }
        if host.eq_ignore_ascii_case("main") {
            return match path {
                None => Some(Self::Main(None)),
                Some(name) => MainRoute::parse(name).map(|r| Self::Main(Some(r))),
            };
        }
        None
    }

    /// The first link among a process's arguments (the program name is skipped by the caller).
    pub fn find<I, S>(args: I) -> Option<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        args.into_iter().find_map(|a| Self::parse(a.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_accepts_the_normalized_launch_only() {
        for arg in [
            "tokenradar://panel",
            "tokenradar://panel/",
            "TokenRadar://Panel/",
        ] {
            assert_eq!(Link::parse(arg), Some(Link::Panel), "{arg}");
        }
        for arg in [
            "tokenradar://panel//",
            "tokenradar://panel/x",
            "tokenradar://panel?x=1",
            "tokenradar://settings",
            "tokenradar:panel",
            "",
        ] {
            assert_eq!(Link::parse(arg), None, "{arg}");
        }
    }

    #[test]
    fn main_takes_an_optional_known_route() {
        assert_eq!(Link::parse("tokenradar://main"), Some(Link::Main(None)));
        assert_eq!(Link::parse("tokenradar://main/"), Some(Link::Main(None)));
        assert_eq!(
            Link::parse("TOKENRADAR://MAIN/Sessions/"),
            Some(Link::Main(Some(MainRoute::Sessions)))
        );
        for arg in [
            "tokenradar://main//",
            "tokenradar://main/settings",
            "tokenradar://main/sessions/1",
            "tokenradar://main?x=1",
            "tokenradar://mainx",
        ] {
            assert_eq!(Link::parse(arg), None, "{arg}");
        }
    }

    #[test]
    fn uri_round_trips() {
        let mut links = vec![Link::Panel, Link::Main(None)];
        links.extend(MainRoute::ALL.map(|r| Link::Main(Some(r))));
        for link in links {
            assert_eq!(Link::parse(&link.uri()), Some(link));
        }
    }

    #[test]
    fn find_skips_other_arguments() {
        assert_eq!(
            Link::find(["--flag", "tokenradar://main/windows"]),
            Some(Link::Main(Some(MainRoute::Windows)))
        );
        assert_eq!(Link::find(["--flag"]), None);
    }
}
