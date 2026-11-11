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
    /// The fields below are for the line only (Step 12i) and are never serialized: the push
    /// and the spool stay as they were. All are metadata; no conversation content is read.
    #[serde(default, skip_serializing)]
    pub workspace: Option<WorkspaceInfo>,
    #[serde(default, skip_serializing)]
    pub vim: Option<VimInfo>,
    #[serde(default, skip_serializing)]
    pub agent: Option<AgentInfo>,
    #[serde(default, skip_serializing)]
    pub worktree: Option<WorktreeInfo>,
    /// Step 12m, observed 2026-11-08: the output style's name only.
    #[serde(default, skip_serializing)]
    pub output_style: Option<OutputStyleInfo>,
    /// Step 12q (ADR-059), for the line only. A field of the wrong type is left out on its
    /// own (`lax`): a schema change never drops the limits with it.
    #[serde(default, deserialize_with = "lax", skip_serializing)]
    pub effort: Option<EffortInfo>,
    #[serde(default, deserialize_with = "lax", skip_serializing)]
    pub thinking: Option<ThinkingInfo>,
    #[serde(default, deserialize_with = "lax", skip_serializing)]
    pub fast_mode: Option<bool>,
    #[serde(default, deserialize_with = "lax", skip_serializing)]
    pub prompt_cache: Option<PromptCacheInfo>,
    /// Documented, not observed: only while the branch has an open PR or merge request.
    /// `url` is not read.
    #[serde(default, deserialize_with = "lax", skip_serializing)]
    pub pr: Option<PrInfo>,
}

/// Deserializes `T`, or `None` when the value is of another type (`null` included).
fn lax<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Lax<T> {
        Ok(T),
        Bad(serde::de::IgnoredAny),
    }
    Ok(match Lax::<T>::deserialize(de)? {
        Lax::Ok(v) => Some(v),
        Lax::Bad(_) => None,
    })
}

/// The length of an array, its items skipped unread (never buffered); `None` for anything
/// else.
fn seq_len<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Option<u32>, D::Error> {
    use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};
    struct Count;
    impl<'de> Visitor<'de> for Count {
        type Value = Option<u32>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an array")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut n = 0u32;
            while seq.next_element::<IgnoredAny>()?.is_some() {
                n = n.saturating_add(1);
            }
            Ok(Some(n))
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
            Ok(None)
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_str<E>(self, _: &str) -> Result<Self::Value, E> {
            Ok(None)
        }
    }
    de.deserialize_any(Count)
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct OutputStyleInfo {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct WorkspaceInfo {
    #[serde(default)]
    pub current_dir: Option<String>,
    #[serde(default)]
    pub project_dir: Option<String>,
    /// Step 12q: how many folders `/add-dir` added; the paths are never read.
    #[serde(default, deserialize_with = "seq_len")]
    pub added_dirs: Option<u32>,
    /// The linked worktree's name; absent in the main working tree.
    #[serde(default, deserialize_with = "lax")]
    pub git_worktree: Option<String>,
    /// From the `origin` remote; absent without one.
    #[serde(default, deserialize_with = "lax")]
    pub repo: Option<RepoInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct RepoInfo {
    #[serde(default, deserialize_with = "lax")]
    pub host: Option<String>,
    #[serde(default, deserialize_with = "lax")]
    pub owner: Option<String>,
    #[serde(default, deserialize_with = "lax")]
    pub name: Option<String>,
}

/// `effort.level`: absent when the model does not take the effort parameter.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct EffortInfo {
    #[serde(default, deserialize_with = "lax")]
    pub level: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct ThinkingInfo {
    #[serde(default, deserialize_with = "lax")]
    pub enabled: Option<bool>,
}

/// `prompt_cache`: counts and times only. Times are epoch seconds, as `resets_at`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct PromptCacheInfo {
    #[serde(default, deserialize_with = "lax")]
    pub warm: Option<bool>,
    #[serde(default, deserialize_with = "lax")]
    pub caching_observed: Option<bool>,
    #[serde(default, deserialize_with = "lax")]
    pub ttl: Option<String>,
    #[serde(default, deserialize_with = "lax")]
    pub expires_at: Option<f64>,
    #[serde(default, deserialize_with = "lax")]
    pub misses: Option<f64>,
    #[serde(default, deserialize_with = "lax")]
    pub hit_ratio: Option<f64>,
    /// Documented, not observed (null until the first diagnosed miss).
    #[serde(default, deserialize_with = "lax")]
    pub last_miss_cause: Option<MissCauseInfo>,
    #[serde(default, deserialize_with = "lax")]
    pub recache_tokens_if_cold: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct MissCauseInfo {
    #[serde(default, deserialize_with = "lax")]
    pub causes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct PrInfo {
    #[serde(default, deserialize_with = "lax")]
    pub number: Option<f64>,
    #[serde(default, deserialize_with = "lax")]
    pub review_state: Option<String>,
    /// `mr` for a GitLab merge request; absent for a GitHub pull request.
    #[serde(default, deserialize_with = "lax")]
    pub kind: Option<String>,
}

/// `rate_limits.spend_limit`, behind a Claude apps gateway only: documented, not observed.
/// The dollar amounts come about every five minutes, apart from the percentage.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct SpendLimitInfo {
    #[serde(default, deserialize_with = "lax")]
    pub used_percentage: Option<f64>,
    #[serde(default, deserialize_with = "lax")]
    pub resets_at: Option<f64>,
    #[serde(default, deserialize_with = "lax")]
    pub used_usd: Option<f64>,
    #[serde(default, deserialize_with = "lax")]
    pub limit_usd: Option<f64>,
    #[serde(default, deserialize_with = "lax")]
    pub period: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct VimInfo {
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct AgentInfo {
    #[serde(default)]
    pub name: Option<String>,
}

/// Present only while the session is in a Claude Code worktree session.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct WorktreeInfo {
    #[serde(default)]
    pub branch: Option<String>,
    /// Step 12q, observed 2026-11-11.
    #[serde(default, deserialize_with = "lax")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// For the line only (`Opus 5.5`). Never serialized: the push and the spool stay as they
    /// were before Step 12c.
    #[serde(default, skip_serializing)]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CostInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
    /// For the line only, never serialized (Step 12i).
    #[serde(default, skip_serializing)]
    pub total_duration_ms: Option<f64>,
    #[serde(default, skip_serializing)]
    pub total_api_duration_ms: Option<f64>,
    #[serde(default, skip_serializing)]
    pub total_lines_added: Option<f64>,
    #[serde(default, skip_serializing)]
    pub total_lines_removed: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContextInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_percentage: Option<f64>,
    /// For the line only, never serialized (Step 12i): tokens in the context window now.
    #[serde(default, skip_serializing)]
    pub total_input_tokens: Option<f64>,
    #[serde(default, skip_serializing)]
    pub total_output_tokens: Option<f64>,
    #[serde(default, skip_serializing)]
    pub context_window_size: Option<f64>,
    /// The last request's token counts (Step 12m, observed 2026-11-08); for the line only.
    #[serde(default, skip_serializing)]
    pub current_usage: Option<CurrentUsage>,
}

/// `context_window.current_usage`: counts only. `output_tokens` is not read.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct CurrentUsage {
    #[serde(default)]
    pub input_tokens: Option<f64>,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<f64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<f64>,
}

/// Each window can be missing on its own: Claude Code drops a window once its `resets_at` passed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RateLimits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub five_hour: Option<RateWindow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seven_day: Option<RateWindow>,
    /// For the line only (Step 12q): the limit engine does not take it.
    #[serde(default, deserialize_with = "lax", skip_serializing)]
    pub spend_limit: Option<SpendLimitInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RateWindow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_percentage: Option<f64>,
    /// Epoch seconds. Accepted as a float in case a version sends one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<f64>,
}

/// Epoch seconds as a timestamp; `None` for zero, negative or not a number.
pub fn epoch(seconds: Option<f64>) -> Option<Timestamp> {
    #[allow(clippy::cast_possible_truncation)]
    seconds
        .filter(|s| s.is_finite() && *s > 0.0)
        .map(|s| Timestamp::from_millis((s * 1000.0).round() as i64))
}

impl RateWindow {
    pub fn resets_at(&self) -> Option<Timestamp> {
        epoch(self.resets_at)
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

    pub fn spend_limit(&self) -> Option<&SpendLimitInfo> {
        self.rate_limits.as_ref()?.spend_limit.as_ref()
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

    const DOCUMENTED: &str = include_str!("../../../../testdata/statusline/documented.json");

    #[test]
    fn the_documented_fields_are_read_for_the_line_and_never_forwarded() {
        let p = StatuslinePayload::parse(DOCUMENTED.as_bytes()).expect("parse");
        assert_eq!(
            p.effort.as_ref().and_then(|e| e.level.as_deref()),
            Some("max")
        );
        assert_eq!(p.thinking.as_ref().and_then(|t| t.enabled), Some(false));
        assert_eq!(p.fast_mode, Some(true));
        let cache = p.prompt_cache.clone().expect("cache");
        assert_eq!(cache.ttl.as_deref(), Some("1h"));
        assert_eq!(cache.misses, Some(2.0));
        assert_eq!(
            cache.last_miss_cause.and_then(|c| c.causes),
            Some(vec!["tools_changed".into(), "system_prompt_changed".into()])
        );
        let spend = p.spend_limit().expect("spend");
        assert_eq!(spend.limit_usd, Some(500.0));
        assert_eq!(spend.period.as_deref(), Some("monthly"));
        let pr = p.pr.clone().expect("pr");
        assert_eq!((pr.number, pr.kind.as_deref()), (Some(45.0), Some("mr")));
        let ws = p.workspace.clone().expect("workspace");
        assert_eq!(ws.added_dirs, Some(2));
        assert_eq!(ws.git_worktree.as_deref(), Some("feature-xyz"));
        assert_eq!(
            ws.repo.and_then(|r| r.owner).as_deref(),
            Some("group/subgroup")
        );
        assert_eq!(
            p.worktree.as_ref().and_then(|w| w.name.as_deref()),
            Some("my-feature")
        );
        // The push and the spool line stay as they were: none of it is serialized.
        let out = serde_json::to_string(&p).expect("serialize");
        for dropped in [
            "effort",
            "thinking",
            "fast_mode",
            "prompt_cache",
            "spend_limit",
            "\"pr\"",
            "merge_requests",
            "worktree",
            "added_dirs",
            "Shared",
            "repo",
        ] {
            assert!(
                !out.contains(dropped),
                "{dropped} must not be forwarded: {out}"
            );
        }
        // The limits still are.
        assert!(out.contains("seven_day"), "{out}");
    }

    #[test]
    fn a_new_field_of_the_wrong_type_leaves_out_only_itself() {
        let p = StatuslinePayload::parse(
            br#"{"rate_limits":{"seven_day":{"used_percentage":41,"resets_at":1738857600},
                 "spend_limit":"soon"},
                 "effort":{"level":3},"fast_mode":"yes","thinking":null,
                 "prompt_cache":{"warm":"maybe","misses":"two","ttl":"2h","expires_at":null,
                                 "last_miss_cause":{"causes":"tools_changed"}},
                 "pr":{"number":"12","review_state":"merged_soon"},
                 "workspace":{"added_dirs":"C:\\x","git_worktree":7,"repo":[]}}"#,
        )
        .expect("a wrong type never fails the payload");
        assert_eq!(p.seven_day().and_then(RateWindow::percent), Some(41.0));
        assert!(p.spend_limit().is_none());
        assert_eq!(p.effort.as_ref().map(|e| e.level.clone()), Some(None));
        assert_eq!((p.fast_mode, p.thinking.clone()), (None, None));
        let cache = p.prompt_cache.expect("cache");
        assert_eq!((cache.warm, cache.misses), (None, None));
        // An unknown text value is kept as it is.
        assert_eq!(cache.ttl.as_deref(), Some("2h"));
        assert_eq!(cache.last_miss_cause.map(|c| c.causes), Some(None));
        let pr = p.pr.expect("pr");
        assert_eq!(pr.number, None);
        assert_eq!(pr.review_state.as_deref(), Some("merged_soon"));
        let ws = p.workspace.expect("workspace");
        assert_eq!((ws.added_dirs, ws.git_worktree), (None, None));
        // serde reads a struct from an array too: an empty one has no name, so no repo shows.
        assert_eq!(ws.repo.and_then(|r| r.name), None);
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
