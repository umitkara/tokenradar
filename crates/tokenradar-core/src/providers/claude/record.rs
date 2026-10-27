//! Selective parsing of one Claude Code JSONL line (ADR-024, strategy C).
//!
//! Only usage and metadata fields are deserialized. `message.content`, tool output and every
//! other conversation field is skipped by serde without being built (no `IgnoredAny` needed:
//! unknown fields are ignored). Prompts, responses and `last-prompt` records never reach a
//! struct here.

use std::borrow::Cow;
use std::fmt;

use memchr::memmem;
use serde::Deserialize;
use serde::de::{self, Deserializer, SeqAccess, Visitor};

use crate::domain::{MsgKey, Speed, Timestamp, TitleSource, TokenCounts, Usage};

/// What one line contributed.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // returned once per line and consumed at once, never stored
pub enum ParseOutcome<'a> {
    Usage(UsageRecord<'a>),
    Meta(MetaRecord<'a>),
    /// Not relevant to usage: user turns, attachments, API request logs, …
    Skip,
    /// Looked relevant but could not be used. Counted as a schema-drift signal (§15).
    Unknown(UnknownReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnknownReason {
    /// Passed the pre-filter but is not valid JSON or has unexpected field types.
    Malformed,
    /// An assistant record with usage but an unparsable `timestamp`.
    BadTimestamp,
}

/// Usage of one assistant line: the response itself plus any advisor iterations.
#[derive(Debug)]
pub struct UsageRecord<'a> {
    pub key: MsgKey,
    /// `message.id` was missing, so `key` hashes the line's `uuid` (or the line) instead.
    pub key_is_fallback: bool,
    pub ts: Timestamp,
    pub model: Cow<'a, str>,
    pub usage: Usage,
    pub is_sidechain: bool,
    pub cwd: Option<Cow<'a, str>>,
    pub git_branch: Option<Cow<'a, str>>,
    pub version: Option<Cow<'a, str>>,
    pub entrypoint: Option<Cow<'a, str>>,
    /// Advisor iterations. Their tokens are billed to the advisor model and are not part of
    /// the top-level `usage` (data-sources.md S3, ADR-026).
    pub advisors: Vec<AdvisorRecord<'a>>,
    /// Matched only the loose pre-filter: the compact JSON layout the strict filter relies on
    /// has changed (ADR-024). Still indexed, but worth a warning.
    pub loose_match: bool,
}

#[derive(Debug)]
pub struct AdvisorRecord<'a> {
    pub key: MsgKey,
    pub model: Cow<'a, str>,
    pub usage: Usage,
}

/// Session metadata that is not tied to a response.
#[derive(Debug, PartialEq, Eq)]
pub enum MetaRecord<'a> {
    Title {
        source: TitleSource,
        text: Cow<'a, str>,
    },
}

/// Reusable line parser; cheap to create, but holds pre-built substring finders.
pub struct LineParser {
    usage: memmem::Finder<'static>,
    assistant_strict: memmem::Finder<'static>,
    assistant_loose: memmem::Finder<'static>,
}

impl Default for LineParser {
    fn default() -> Self {
        Self {
            usage: memmem::Finder::new(br#""usage""#),
            assistant_strict: memmem::Finder::new(br#""type":"assistant""#),
            assistant_loose: memmem::Finder::new(br#""assistant""#),
        }
    }
}

impl LineParser {
    /// Classifies one complete line (without its trailing newline).
    ///
    /// Pre-filter: a usage record must contain `"usage"` and `"type":"assistant"`. Lines that
    /// contain `"usage"` and only a looser `"assistant"` are parsed too, so a change in JSON
    /// spacing degrades into a warning instead of silently losing every event.
    pub fn parse<'a>(&self, line: &'a [u8]) -> ParseOutcome<'a> {
        if self.usage.find(line).is_some() {
            let strict = self.assistant_strict.find(line).is_some();
            if strict || self.assistant_loose.find(line).is_some() {
                return parse_usage(line, !strict);
            }
            return ParseOutcome::Skip;
        }
        match title_source(line) {
            Some(source) => parse_title(line, source),
            None => ParseOutcome::Skip,
        }
    }
}

fn parse_usage(line: &[u8], loose_match: bool) -> ParseOutcome<'_> {
    let Ok(parsed) = serde_json::from_slice::<Line<'_>>(line) else {
        return ParseOutcome::Unknown(UnknownReason::Malformed);
    };
    if parsed.kind.as_ref().is_none_or(|k| k.0 != "assistant") {
        return ParseOutcome::Skip;
    }
    let Some(message) = parsed.message else {
        return ParseOutcome::Skip;
    };
    let Some(usage) = message.usage else {
        return ParseOutcome::Skip;
    };
    let Some(Ok(ts)) = parsed
        .timestamp
        .as_ref()
        .map(|t| Timestamp::parse_iso(&t.0))
    else {
        return ParseOutcome::Unknown(UnknownReason::BadTimestamp);
    };

    let request_id = parsed.request_id.as_ref().map_or("", |r| r.0.as_ref());
    let (key, key_is_fallback) = match &message.id {
        Some(id) => (MsgKey::from_ids(&id.0, request_id), false),
        None => match &parsed.uuid {
            Some(uuid) => (MsgKey::from_fallback(uuid.0.as_bytes()), true),
            None => (MsgKey::from_fallback(line), true),
        },
    };
    let message_id = message.id.as_ref().map_or("", |id| id.0.as_ref());
    let advisors = usage
        .iterations
        .0
        .into_iter()
        .enumerate()
        .map(|(i, it)| AdvisorRecord {
            key: MsgKey::from_ids_tagged(message_id, request_id, &format!("advisor#{i}")),
            model: cow(it.model),
            usage: Usage {
                tokens: tokens(
                    it.input_tokens,
                    it.output_tokens,
                    it.cache_read_input_tokens,
                    it.cache_creation_input_tokens,
                    it.cache_creation.as_ref(),
                    None,
                ),
                ..Usage::default()
            },
        })
        .collect();
    let server = usage.server_tool_use.unwrap_or_default();

    ParseOutcome::Usage(UsageRecord {
        key,
        key_is_fallback,
        ts,
        model: cow(message.model),
        usage: Usage {
            tokens: tokens(
                usage.input_tokens,
                usage.output_tokens,
                usage.cache_read_input_tokens,
                usage.cache_creation_input_tokens,
                usage.cache_creation.as_ref(),
                usage.output_tokens_details.and_then(|d| d.thinking_tokens),
            ),
            web_search: server.web_search_requests.unwrap_or(0),
            web_fetch: server.web_fetch_requests.unwrap_or(0),
            speed: match usage.speed.as_ref().map(|s| s.0.as_ref()) {
                Some("fast") => Speed::Fast,
                _ => Speed::Standard,
            },
        },
        is_sidechain: parsed.is_sidechain.unwrap_or(false),
        cwd: parsed.cwd.map(|s| s.0),
        git_branch: parsed.git_branch.map(|s| s.0),
        version: parsed.version.map(|s| s.0),
        entrypoint: parsed.entrypoint.map(|s| s.0),
        advisors,
        loose_match,
    })
}

fn tokens(
    input: Option<u64>,
    output: Option<u64>,
    cache_read: Option<u64>,
    cache_creation: Option<u64>,
    breakdown: Option<&CacheCreation>,
    thinking: Option<u64>,
) -> TokenCounts {
    let (w5m, w1h) = TokenCounts::split_cache_creation(
        cache_creation.unwrap_or(0),
        breakdown.map(|b| {
            (
                b.ephemeral_5m_input_tokens.unwrap_or(0),
                b.ephemeral_1h_input_tokens.unwrap_or(0),
            )
        }),
    );
    TokenCounts {
        input: input.unwrap_or(0),
        output: output.unwrap_or(0),
        cache_read: cache_read.unwrap_or(0),
        cache_w5m: w5m,
        cache_w1h: w1h,
        thinking: thinking.unwrap_or(0),
    }
}

/// Title records are small and start with their type: `{"type":"ai-title",…`.
fn title_source(line: &[u8]) -> Option<TitleSource> {
    let rest = line.strip_prefix(br#"{"type":""#)?;
    if rest.starts_with(br#"ai-title""#) {
        Some(TitleSource::Ai)
    } else if rest.starts_with(br#"custom-title""#) {
        Some(TitleSource::Custom)
    } else if rest.starts_with(br#"agent-name""#) {
        Some(TitleSource::Agent)
    } else {
        None
    }
}

fn parse_title(line: &[u8], source: TitleSource) -> ParseOutcome<'_> {
    let Ok(parsed) = serde_json::from_slice::<TitleLine<'_>>(line) else {
        return ParseOutcome::Unknown(UnknownReason::Malformed);
    };
    let text = match source {
        TitleSource::Ai => parsed.ai_title,
        TitleSource::Custom => parsed.custom_title,
        TitleSource::Agent => parsed.agent_name,
    };
    match text.map(|t| t.0) {
        Some(text) if !text.trim().is_empty() => {
            ParseOutcome::Meta(MetaRecord::Title { source, text })
        }
        _ => ParseOutcome::Skip,
    }
}

// ---------------------------------------------------------------------------------------------
// Borrowing line model
// ---------------------------------------------------------------------------------------------

/// A string that borrows when the deserializer can lend it.
///
/// serde only borrows a `Cow<'a, str>` when it is a field's direct type; inside an `Option` it
/// always allocates. This newtype borrows in both positions. Strings with escapes (Windows
/// `cwd` values) are owned either way.
#[derive(Debug, Default)]
struct Str<'a>(Cow<'a, str>);

impl<'de: 'a, 'a> Deserialize<'de> for Str<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StrVisitor;

        impl<'de> Visitor<'de> for StrVisitor {
            type Value = Str<'de>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a string")
            }

            fn visit_borrowed_str<E: de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
                Ok(Str(Cow::Borrowed(v)))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Str(Cow::Owned(v.to_owned())))
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(Str(Cow::Owned(v)))
            }
        }

        deserializer.deserialize_str(StrVisitor)
    }
}

fn cow(s: Option<Str<'_>>) -> Cow<'_, str> {
    s.map(|s| s.0).unwrap_or_default()
}

#[derive(Deserialize)]
struct Line<'a> {
    #[serde(rename = "type", borrow, default)]
    kind: Option<Str<'a>>,
    #[serde(borrow, default)]
    uuid: Option<Str<'a>>,
    #[serde(rename = "isSidechain", default)]
    is_sidechain: Option<bool>,
    #[serde(borrow, default)]
    timestamp: Option<Str<'a>>,
    #[serde(borrow, default)]
    cwd: Option<Str<'a>>,
    #[serde(rename = "gitBranch", borrow, default)]
    git_branch: Option<Str<'a>>,
    #[serde(borrow, default)]
    version: Option<Str<'a>>,
    #[serde(borrow, default)]
    entrypoint: Option<Str<'a>>,
    #[serde(rename = "requestId", borrow, default)]
    request_id: Option<Str<'a>>,
    #[serde(borrow, default)]
    message: Option<Message<'a>>,
}

/// `content` is deliberately absent: serde skips unknown fields without building them.
#[derive(Deserialize)]
struct Message<'a> {
    #[serde(borrow, default)]
    id: Option<Str<'a>>,
    #[serde(borrow, default)]
    model: Option<Str<'a>>,
    #[serde(borrow, default)]
    usage: Option<RawUsage<'a>>,
}

#[derive(Deserialize)]
struct RawUsage<'a> {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    cache_creation: Option<CacheCreation>,
    #[serde(default)]
    output_tokens_details: Option<OutputDetails>,
    #[serde(default)]
    server_tool_use: Option<ServerToolUse>,
    #[serde(borrow, default)]
    speed: Option<Str<'a>>,
    #[serde(borrow, default)]
    iterations: Advisors<'a>,
}

#[derive(Deserialize)]
struct CacheCreation {
    #[serde(default)]
    ephemeral_5m_input_tokens: Option<u64>,
    #[serde(default)]
    ephemeral_1h_input_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct OutputDetails {
    #[serde(default)]
    thinking_tokens: Option<u64>,
}

#[derive(Deserialize, Default, Clone, Copy)]
struct ServerToolUse {
    #[serde(default)]
    web_search_requests: Option<u32>,
    #[serde(default)]
    web_fetch_requests: Option<u32>,
}

#[derive(Deserialize)]
struct Iteration<'a> {
    #[serde(rename = "type", borrow, default)]
    kind: Option<Str<'a>>,
    #[serde(borrow, default)]
    model: Option<Str<'a>>,
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    cache_creation: Option<CacheCreation>,
}

/// The `advisor_message` entries of `usage.iterations`. `message` entries are already summed
/// into the top-level usage and are dropped while deserializing, so the common case (one
/// `message` entry, or none) allocates nothing. `null` reads as empty.
#[derive(Default)]
struct Advisors<'a>(Vec<Iteration<'a>>);

impl<'de: 'a, 'a> Deserialize<'de> for Advisors<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SeqVisitor;

        impl<'de> Visitor<'de> for SeqVisitor {
            type Value = Advisors<'de>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a list of iterations")
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(Advisors::default())
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(it) = seq.next_element::<Iteration<'de>>()? {
                    if it.kind.as_ref().is_some_and(|k| k.0 == "advisor_message") {
                        out.push(it);
                    }
                }
                Ok(Advisors(out))
            }
        }

        deserializer.deserialize_any(SeqVisitor)
    }
}

#[derive(Deserialize)]
struct TitleLine<'a> {
    #[serde(rename = "aiTitle", borrow, default)]
    ai_title: Option<Str<'a>>,
    #[serde(rename = "customTitle", borrow, default)]
    custom_title: Option<Str<'a>>,
    #[serde(rename = "agentName", borrow, default)]
    agent_name: Option<Str<'a>>,
}

#[cfg(test)]
mod tests;
