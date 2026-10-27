//! Step 02 spike: four ways to pull the same usage record out of a Claude Code JSONL line.
//!
//! Shared by `benches/parse.rs` (synthetic data) and `examples/parse_real.rs` (real data,
//! read-only) through `#[path]`. This is measurement code, not the indexer: Step 04 builds the
//! real one on the strategy this spike selects.
//!
//! Privacy: only usage and metadata fields are extracted. Strategy A parses whole lines into a
//! `serde_json::Value` because it is the baseline, but nothing beyond the usage fields leaves
//! this module and nothing is printed or stored.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Read};

use memchr::memmem;
use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    /// Full parse of every line into a `serde_json::Value`.
    A,
    /// Typed structs that borrow from the line; unknown fields such as `content` are skipped.
    B,
    /// `memmem` pre-filter on `"type":"assistant"` and `"usage":`, then B.
    C,
    /// The C pre-filter, then a typed `simd-json` parse of a copy of the line.
    D,
}

impl Strategy {
    pub const ALL: [Self; 4] = [Self::A, Self::B, Self::C, Self::D];

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "a" => Some(Self::A),
            "b" => Some(Self::B),
            "c" => Some(Self::C),
            "d" => Some(Self::D),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }
}

/// The usage fields of one assistant line. Strings borrow from the line when they hold no
/// escapes; `cwd` on Windows always has escaped backslashes, so it is usually owned.
#[derive(Debug, Default)]
pub struct UsageEvent<'a> {
    pub message_id: Cow<'a, str>,
    pub request_id: Cow<'a, str>,
    pub session_id: Cow<'a, str>,
    pub timestamp: Cow<'a, str>,
    pub model: Cow<'a, str>,
    pub cwd: Cow<'a, str>,
    pub is_sidechain: bool,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub ephemeral_5m_input_tokens: u64,
    pub ephemeral_1h_input_tokens: u64,
    pub speed: Option<Cow<'a, str>>,
    pub service_tier: Option<Cow<'a, str>>,
}

// ---------------------------------------------------------------------------------------------
// Typed line model (strategies B, C, D)
// ---------------------------------------------------------------------------------------------

/// A string that borrows when the deserializer can lend it.
///
/// serde only borrows a `Cow<'a, str>` when it is a field's direct type; inside an `Option` it
/// always allocates. This newtype borrows in both positions.
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

#[derive(Deserialize)]
struct Line<'a> {
    #[serde(rename = "type", borrow, default)]
    kind: Option<Str<'a>>,
    #[serde(borrow, default)]
    message: Option<Message<'a>>,
    #[serde(rename = "requestId", borrow, default)]
    request_id: Option<Str<'a>>,
    #[serde(rename = "sessionId", borrow, default)]
    session_id: Option<Str<'a>>,
    #[serde(borrow, default)]
    timestamp: Option<Str<'a>>,
    #[serde(borrow, default)]
    cwd: Option<Str<'a>>,
    #[serde(rename = "isSidechain", default)]
    is_sidechain: Option<bool>,
}

/// `content` is deliberately absent: serde skips unknown fields without building them.
#[derive(Deserialize)]
struct Message<'a> {
    #[serde(borrow, default)]
    id: Option<Str<'a>>,
    #[serde(borrow, default)]
    model: Option<Str<'a>>,
    #[serde(borrow, default)]
    usage: Option<Usage<'a>>,
}

#[derive(Deserialize)]
struct Usage<'a> {
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
    #[serde(borrow, default)]
    speed: Option<Str<'a>>,
    #[serde(borrow, default)]
    service_tier: Option<Str<'a>>,
}

#[derive(Deserialize)]
struct CacheCreation {
    #[serde(default)]
    ephemeral_5m_input_tokens: Option<u64>,
    #[serde(default)]
    ephemeral_1h_input_tokens: Option<u64>,
}

fn cow(s: Option<Str<'_>>) -> Cow<'_, str> {
    s.map(|s| s.0).unwrap_or_default()
}

impl<'a> Line<'a> {
    /// Converts an assistant line with usage into an event; anything else yields `None`.
    fn into_event(self) -> Option<UsageEvent<'a>> {
        if self.kind.as_ref().is_none_or(|k| k.0 != "assistant") {
            return None;
        }
        let message = self.message?;
        let usage = message.usage?;
        let message_id = message.id?.0;
        let cache = usage.cache_creation;
        Some(UsageEvent {
            message_id,
            request_id: cow(self.request_id),
            session_id: cow(self.session_id),
            timestamp: cow(self.timestamp),
            model: cow(message.model),
            cwd: cow(self.cwd),
            is_sidechain: self.is_sidechain.unwrap_or(false),
            input_tokens: usage.input_tokens.unwrap_or(0),
            output_tokens: usage.output_tokens.unwrap_or(0),
            cache_creation_input_tokens: usage.cache_creation_input_tokens.unwrap_or(0),
            cache_read_input_tokens: usage.cache_read_input_tokens.unwrap_or(0),
            ephemeral_5m_input_tokens: cache
                .as_ref()
                .and_then(|c| c.ephemeral_5m_input_tokens)
                .unwrap_or(0),
            ephemeral_1h_input_tokens: cache
                .as_ref()
                .and_then(|c| c.ephemeral_1h_input_tokens)
                .unwrap_or(0),
            speed: usage.speed.map(|s| s.0),
            service_tier: usage.service_tier.map(|s| s.0),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Untyped extraction (strategy A)
// ---------------------------------------------------------------------------------------------

fn event_from_value(v: &serde_json::Value) -> Option<UsageEvent<'_>> {
    use serde_json::Value;

    fn s<'v>(v: &'v Value, key: &str) -> Cow<'v, str> {
        Cow::Borrowed(v.get(key).and_then(Value::as_str).unwrap_or_default())
    }
    fn opt_s<'v>(v: &'v Value, key: &str) -> Option<Cow<'v, str>> {
        v.get(key).and_then(Value::as_str).map(Cow::Borrowed)
    }
    fn n(v: Option<&Value>, key: &str) -> u64 {
        v.and_then(|v| v.get(key))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    }

    if v.get("type").and_then(Value::as_str) != Some("assistant") {
        return None;
    }
    let message = v.get("message")?;
    let usage = message.get("usage").filter(|u| u.is_object())?;
    let message_id = message.get("id").and_then(Value::as_str)?;
    let cache = usage.get("cache_creation");
    Some(UsageEvent {
        message_id: Cow::Borrowed(message_id),
        request_id: s(v, "requestId"),
        session_id: s(v, "sessionId"),
        timestamp: s(v, "timestamp"),
        model: s(message, "model"),
        cwd: s(v, "cwd"),
        is_sidechain: v
            .get("isSidechain")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        input_tokens: n(Some(usage), "input_tokens"),
        output_tokens: n(Some(usage), "output_tokens"),
        cache_creation_input_tokens: n(Some(usage), "cache_creation_input_tokens"),
        cache_read_input_tokens: n(Some(usage), "cache_read_input_tokens"),
        ephemeral_5m_input_tokens: n(cache, "ephemeral_5m_input_tokens"),
        ephemeral_1h_input_tokens: n(cache, "ephemeral_1h_input_tokens"),
        speed: opt_s(usage, "speed"),
        service_tier: opt_s(usage, "service_tier"),
    })
}

// ---------------------------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------------------------

pub struct Parser {
    strategy: Strategy,
    assistant: memmem::Finder<'static>,
    usage: memmem::Finder<'static>,
    scratch: Vec<u8>,
    buffers: simd_json::Buffers,
}

impl Parser {
    pub fn new(strategy: Strategy) -> Self {
        Self {
            strategy,
            assistant: memmem::Finder::new(br#""type":"assistant""#),
            usage: memmem::Finder::new(br#""usage":"#),
            scratch: Vec::new(),
            buffers: simd_json::Buffers::default(),
        }
    }

    /// Feeds one complete line (without the trailing newline) into `tally`.
    pub fn line(&mut self, line: &[u8], tally: &mut Tally) {
        tally.lines += 1;
        tally.bytes += line.len() as u64 + 1;
        match self.strategy {
            Strategy::A => match serde_json::from_slice::<serde_json::Value>(line) {
                Ok(value) => {
                    if let Some(event) = event_from_value(&value) {
                        tally.add(&event);
                    }
                }
                Err(_) => tally.errors += 1,
            },
            Strategy::B => Self::typed(line, tally),
            Strategy::C => {
                if self.passes_filter(line) {
                    Self::typed(line, tally);
                }
            }
            Strategy::D => {
                if self.passes_filter(line) {
                    self.scratch.clear();
                    self.scratch.extend_from_slice(line);
                    match simd_json::serde::from_slice_with_buffers::<Line<'_>>(
                        &mut self.scratch,
                        &mut self.buffers,
                    ) {
                        Ok(parsed) => {
                            if let Some(event) = parsed.into_event() {
                                tally.add(&event);
                            }
                        }
                        Err(_) => tally.errors += 1,
                    }
                }
            }
        }
    }

    fn passes_filter(&self, line: &[u8]) -> bool {
        self.assistant.find(line).is_some() && self.usage.find(line).is_some()
    }

    fn typed(line: &[u8], tally: &mut Tally) {
        match serde_json::from_slice::<Line<'_>>(line) {
            Ok(parsed) => {
                if let Some(event) = parsed.into_event() {
                    tally.add(&event);
                }
            }
            Err(_) => tally.errors += 1,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Deduplication and totals
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
struct Record {
    input: u64,
    output: u64,
    cache_creation: u64,
    cache_read: u64,
    ephemeral_1h: u64,
    sidechain: bool,
    /// Total length of the metadata strings: proves every strategy extracted the same text.
    meta_len: u64,
}

/// Per-thread counters plus the dedup map: `(message.id, requestId)` → the record with the
/// largest `output_tokens`, the later one winning ties (data-sources.md S3, pitfall 2).
#[derive(Default)]
pub struct Tally {
    pub lines: u64,
    pub bytes: u64,
    pub events: u64,
    pub errors: u64,
    winners: HashMap<u128, Record>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    pub unique: u64,
    pub input: u64,
    pub output: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
    pub ephemeral_1h: u64,
    pub sidechain: u64,
    pub meta_len: u64,
}

impl Tally {
    fn add(&mut self, event: &UsageEvent<'_>) {
        self.events += 1;
        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
        hasher.update(event.message_id.as_bytes());
        hasher.update(&[0]);
        hasher.update(event.request_id.as_bytes());
        let record = Record {
            input: event.input_tokens,
            output: event.output_tokens,
            cache_creation: event.cache_creation_input_tokens,
            cache_read: event.cache_read_input_tokens,
            ephemeral_1h: event.ephemeral_1h_input_tokens,
            sidechain: event.is_sidechain,
            meta_len: [
                &event.session_id,
                &event.timestamp,
                &event.model,
                &event.cwd,
                event.speed.as_ref().unwrap_or(&Cow::Borrowed("")),
                event.service_tier.as_ref().unwrap_or(&Cow::Borrowed("")),
            ]
            .iter()
            .map(|s| s.len() as u64)
            .sum::<u64>()
                + event.ephemeral_5m_input_tokens,
        };
        self.winners
            .entry(hasher.digest128())
            .and_modify(|old| {
                if record.output >= old.output {
                    *old = record;
                }
            })
            .or_insert(record);
    }

    pub fn merge(&mut self, other: Self) {
        self.lines += other.lines;
        self.bytes += other.bytes;
        self.events += other.events;
        self.errors += other.errors;
        for (key, record) in other.winners {
            self.winners
                .entry(key)
                .and_modify(|old| {
                    if record.output > old.output {
                        *old = record;
                    }
                })
                .or_insert(record);
        }
    }

    pub fn totals(&self) -> Totals {
        self.winners.values().fold(
            Totals {
                unique: self.winners.len() as u64,
                ..Totals::default()
            },
            |mut t, r| {
                t.input += r.input;
                t.output += r.output;
                t.cache_creation += r.cache_creation;
                t.cache_read += r.cache_read;
                t.ephemeral_1h += r.ephemeral_1h;
                t.sidechain += u64::from(r.sidechain);
                t.meta_len += r.meta_len;
                t
            },
        )
    }
}

// ---------------------------------------------------------------------------------------------
// Line reader
// ---------------------------------------------------------------------------------------------

/// Read size. A multiple of every common sector size, as `FILE_FLAG_NO_BUFFERING` requires.
pub const CHUNK: usize = 1 << 20;
const ALIGN: usize = 4096;

/// Streams complete lines out of a reader with a fixed, sector-aligned buffer.
///
/// Lines that fit inside one chunk are handed out without copying. A trailing line without a
/// newline is not reported, matching the indexer rule for half-written lines.
pub struct LineReader {
    raw: Vec<u8>,
    carry: Vec<u8>,
}

impl Default for LineReader {
    fn default() -> Self {
        Self {
            raw: vec![0; CHUNK + ALIGN],
            carry: Vec::new(),
        }
    }
}

impl LineReader {
    /// Reads at most `limit` bytes. Files grow while Claude Code runs, so a measurement freezes
    /// their sizes up front to make every run see the same bytes.
    pub fn read_lines(
        &mut self,
        mut src: impl Read,
        limit: u64,
        mut on_line: impl FnMut(&[u8]),
    ) -> io::Result<u64> {
        let Self { raw, carry } = self;
        let start = raw.as_ptr().align_offset(ALIGN);
        let chunk = &mut raw[start..start + CHUNK];
        carry.clear();
        let mut total = 0;
        loop {
            // Always request whole chunks: unbuffered reads must stay sector-sized.
            let n = src.read(chunk)?;
            let n = usize::try_from((n as u64).min(limit - total)).expect("at most CHUNK");
            if n == 0 {
                return Ok(total);
            }
            total += n as u64;
            let mut rest = &chunk[..n];
            if !carry.is_empty() {
                if let Some(i) = memchr::memchr(b'\n', rest) {
                    carry.extend_from_slice(&rest[..i]);
                    on_line(carry);
                    carry.clear();
                    rest = &rest[i + 1..];
                } else {
                    carry.extend_from_slice(rest);
                    continue;
                }
            }
            while let Some(i) = memchr::memchr(b'\n', rest) {
                on_line(&rest[..i]);
                rest = &rest[i + 1..];
            }
            carry.extend_from_slice(rest);
        }
    }
}
