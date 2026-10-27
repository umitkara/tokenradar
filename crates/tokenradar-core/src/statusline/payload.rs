//! The part of Claude Code's statusline JSON (S1) that TokenRadar keeps.
//!
//! Claude Code sends a large object on stdin; only the fields below are deserialized and every
//! other field is skipped. The raw JSON is never stored or forwarded: the IPC push and the
//! spool line carry this struct, re-serialized.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::{AccountId, LimitKind, LimitObservation, Source, Timestamp};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StatuslinePayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<ContextInfo>,
    /// Pro/Max only, and only after the session's first API response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limits: Option<RateLimits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CostInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContextInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_percentage: Option<f64>,
}

/// Each window can be missing on its own: Claude Code drops a window once its `resets_at` passed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RateLimits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub five_hour: Option<RateWindow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seven_day: Option<RateWindow>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RateWindow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_percentage: Option<f64>,
    /// Epoch seconds. Accepted as a float in case a version sends one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<f64>,
}

impl RateWindow {
    pub fn resets_at(&self) -> Option<Timestamp> {
        #[allow(clippy::cast_possible_truncation)]
        self.resets_at
            .filter(|s| s.is_finite() && *s > 0.0)
            .map(|s| Timestamp::from_millis((s * 1000.0).round() as i64))
    }

    /// The used percentage, if it is a usable number.
    pub fn percent(&self) -> Option<f64> {
        self.used_percentage.filter(|p| p.is_finite() && *p >= 0.0)
    }
}

impl StatuslinePayload {
    /// Parses Claude Code's stdin. Unknown fields are skipped without being kept.
    pub fn parse(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    pub fn five_hour(&self) -> Option<&RateWindow> {
        self.rate_limits.as_ref()?.five_hour.as_ref()
    }

    pub fn seven_day(&self) -> Option<&RateWindow> {
        self.rate_limits.as_ref()?.seven_day.as_ref()
    }

    /// The limit readings in this payload, attributed to `account`.
    pub fn observations(
        &self,
        account: AccountId,
        observed_at: Timestamp,
    ) -> Vec<LimitObservation> {
        let windows = [
            (LimitKind::Session, self.five_hour()),
            (LimitKind::WeeklyAll, self.seven_day()),
        ];
        windows
            .into_iter()
            .filter_map(|(kind, window)| {
                let window = window?;
                Some(LimitObservation {
                    account,
                    kind,
                    scope: None,
                    percent: window.percent()?,
                    resets_at: window.resets_at(),
                    severity: None,
                    source: Source::Statusline,
                    session_uuid: self.session_id.clone(),
                    observed_at,
                })
            })
            .collect()
    }
}

/// A payload with the time the CLI received it: the IPC push and one spool line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatuslinePush {
    pub observed_at: Timestamp,
    pub payload: StatuslinePayload,
}

/// The account whose config directory holds `transcript_path`: the longest matching prefix
/// wins. Without a transcript path the first (primary) account is used; a path outside every
/// known directory belongs to none, so another account's readings are never mixed in.
pub fn account_for<'a, I>(transcript_path: Option<&str>, accounts: I) -> Option<AccountId>
where
    I: IntoIterator<Item = (AccountId, &'a Path)>,
{
    let mut accounts = accounts.into_iter().peekable();
    let Some(path) = transcript_path else {
        return accounts.peek().map(|(id, _)| *id);
    };
    let path = normalize(Path::new(path));
    accounts
        .filter_map(|(id, dir)| {
            let dir = normalize(dir);
            let rest = path.strip_prefix(&dir)?;
            (rest.is_empty() || rest.starts_with(SEP)).then_some((dir.len(), id))
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, id)| id)
}

const SEP: char = if cfg!(windows) { '\\' } else { '/' };

/// Comparable form: one separator style, no trailing separator, case-folded on Windows.
fn normalize(path: &Path) -> String {
    let s = path.to_string_lossy();
    let s = if cfg!(windows) {
        s.replace('/', "\\").to_lowercase()
    } else {
        s.into_owned()
    };
    s.trim_end_matches(SEP).to_owned()
}

/// Exposed for the CLI, which picks the config directory before the app is involved.
pub fn path_in_dir(path: &str, dir: &Path) -> bool {
    account_for(Some(path), [(AccountId(1), dir)]).is_some()
}

/// The same directory, spelled with either separator and, on Windows, in any case.
pub fn same_dir(a: &Path, b: &Path) -> bool {
    normalize(a) == normalize(b)
}

/// The config directory of the default account: `CLAUDE_CONFIG_DIR` or `~/.claude`.
pub fn default_config_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|home| PathBuf::from(home).join(".claude"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../../../../testdata/statusline/sample.json");

    #[test]
    fn parses_the_sample_and_keeps_only_known_fields() {
        let p = StatuslinePayload::parse(SAMPLE.as_bytes()).expect("parse");
        assert_eq!(
            p.session_id.as_deref(),
            Some("00000000-0000-4000-8000-000000000001")
        );
        assert_eq!(
            p.model.as_ref().and_then(|m| m.id.as_deref()),
            Some("claude-opus-5-5")
        );
        assert_eq!(p.five_hour().and_then(|w| w.used_percentage), Some(23.5));
        let out = serde_json::to_string(&p).expect("serialize");
        for dropped in [
            "workspace",
            "prompt_cache",
            "current_usage",
            "display_name",
            "effort",
        ] {
            assert!(!out.contains(dropped), "{dropped} must not be forwarded");
        }
    }

    #[test]
    fn windows_become_statusline_observations() {
        let p = StatuslinePayload::parse(SAMPLE.as_bytes()).expect("parse");
        let at = Timestamp::from_millis(1_738_420_000_000);
        let obs = p.observations(AccountId(3), at);
        assert_eq!(obs.len(), 2);
        assert_eq!(obs[0].kind, LimitKind::Session);
        assert_eq!(obs[0].percent, 23.5);
        assert_eq!(
            obs[0].resets_at,
            Some(Timestamp::from_millis(1_738_425_600_000))
        );
        assert_eq!(obs[0].source, Source::Statusline);
        assert_eq!(obs[0].session_uuid, p.session_id);
        assert_eq!(obs[1].kind, LimitKind::WeeklyAll);
        assert!(
            obs.iter()
                .all(|o| o.account == AccountId(3) && o.observed_at == at)
        );
    }

    #[test]
    fn missing_or_partial_rate_limits_are_tolerated() {
        let p = StatuslinePayload::parse(br#"{"session_id":"x"}"#).expect("parse");
        assert_eq!(
            p.observations(AccountId(1), Timestamp::from_millis(0)),
            Vec::new()
        );
        let p = StatuslinePayload::parse(
            br#"{"rate_limits":{"seven_day":{"used_percentage":41.2,"resets_at":1738857600},
                 "five_hour":{"used_percentage":null}}}"#,
        )
        .expect("parse");
        let obs = p.observations(AccountId(1), Timestamp::from_millis(0));
        assert_eq!(obs.len(), 1);
        assert_eq!(obs[0].kind, LimitKind::WeeklyAll);
    }

    #[cfg(windows)]
    #[test]
    fn transcripts_map_to_the_longest_matching_config_dir() {
        let default = Path::new(r"C:\Users\Ada\.claude");
        let work = Path::new(r"C:\Users\Ada\.claude-work");
        let nested = Path::new(r"C:\Users\Ada\.claude\nested");
        let accounts = [
            (AccountId(1), default),
            (AccountId(2), work),
            (AccountId(3), nested),
        ];
        let t = |p: &str| account_for(Some(p), accounts);
        assert_eq!(
            t(r"C:\Users\Ada\.claude\projects\x\s.jsonl"),
            Some(AccountId(1))
        );
        assert_eq!(
            t("c:/users/ada/.CLAUDE-work/projects/x/s.jsonl"),
            Some(AccountId(2))
        );
        assert_eq!(
            t(r"C:\Users\Ada\.claude\nested\projects\s.jsonl"),
            Some(AccountId(3))
        );
        assert_eq!(t(r"D:\elsewhere\s.jsonl"), None);
        assert_eq!(account_for(None, accounts), Some(AccountId(1)));
        assert_eq!(account_for(None, []), None);
    }
}
