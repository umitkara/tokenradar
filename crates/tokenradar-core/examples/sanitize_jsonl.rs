//! Turns a real Claude Code transcript into a fixture that is safe to commit.
//!
//! ```text
//! cargo run -p tokenradar-core --example sanitize_jsonl -- <input.jsonl> [--out <path>] [--max-lines N]
//! ```
//!
//! Allowlist, not denylist: only the fields the indexer reads survive.
//!
//! - Kept: record type, ids (`uuid`, `sessionId`, `requestId`, `message.id`, `agentId`; the
//!   dedup tests depend on them), timestamps, version, entrypoint, sidechain flag, model and
//!   the whole `message.usage` object (numbers and enums only).
//! - Hashed: `cwd` → `C:\work\proj-<hash8>`, branch names other than `main`/`master` →
//!   `branch-<hash8>`, titles → `title-<hash8>`.
//! - Dropped: everything else, including `message.content` (text, thinking, signatures, tool
//!   calls), tool results, attachments, prompts and API request logs.
//!
//! Output defaults to `testdata/claude/sanitized/<hash8>.jsonl`, named by a hash of the input
//! path so the real project name never appears. Unparsable lines are dropped and counted.
//! Review the output before committing it.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

const TOP_LEVEL: &[&str] = &[
    "type",
    "uuid",
    "parentUuid",
    "sessionId",
    "isSidechain",
    "timestamp",
    "version",
    "entrypoint",
    "userType",
    "requestId",
    "agentId",
];
const MESSAGE: &[&str] = &["id", "model", "role", "type", "stop_reason", "usage"];

fn hash8(s: &str) -> String {
    format!("{:08x}", xxhash_rust::xxh3::xxh3_64(s.as_bytes()) >> 32)
}

fn sanitize(line: &Value) -> Option<Value> {
    let obj = line.as_object()?;
    let mut out = Map::new();
    for key in TOP_LEVEL {
        if let Some(v) = obj.get(*key) {
            out.insert((*key).to_owned(), v.clone());
        }
    }
    if let Some(cwd) = obj.get("cwd").and_then(Value::as_str) {
        out.insert("cwd".into(), format!(r"C:\work\proj-{}", hash8(cwd)).into());
    }
    if let Some(branch) = obj.get("gitBranch").and_then(Value::as_str) {
        let kept = matches!(branch, "main" | "master" | "HEAD" | "");
        let value = if kept {
            branch.to_owned()
        } else {
            format!("branch-{}", hash8(branch))
        };
        out.insert("gitBranch".into(), value.into());
    }
    for key in ["aiTitle", "customTitle", "agentName"] {
        if let Some(t) = obj.get(key).and_then(Value::as_str) {
            out.insert(key.into(), format!("title-{}", hash8(t)).into());
        }
    }
    if let Some(message) = obj.get("message").and_then(Value::as_object) {
        let kept: Map<String, Value> = MESSAGE
            .iter()
            .filter_map(|k| message.get(*k).map(|v| ((*k).to_owned(), v.clone())))
            .collect();
        out.insert("message".into(), Value::Object(kept));
    }
    Some(Value::Object(out))
}

/// `(input, out, max_lines)`.
fn parse_args() -> Result<(PathBuf, Option<PathBuf>, usize), lexopt::Error> {
    use lexopt::prelude::*;
    let mut parser = lexopt::Parser::from_env();
    let (mut input, mut out, mut max_lines) = (None::<PathBuf>, None::<PathBuf>, usize::MAX);
    while let Some(arg) = parser.next()? {
        match arg {
            Long("out") => out = Some(parser.value()?.into()),
            Long("max-lines") => max_lines = parser.value()?.parse()?,
            Value(v) if input.is_none() => input = Some(v.into()),
            _ => return Err(arg.unexpected()),
        }
    }
    Ok((input.ok_or("missing <input.jsonl>")?, out, max_lines))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (input, out, max_lines) = parse_args()?;
    let out = out.unwrap_or_else(|| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../testdata/claude/sanitized")
            .join(format!("{}.jsonl", hash8(&input.to_string_lossy())))
    });
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }

    let mut writer = BufWriter::new(File::create(&out)?);
    let (mut kept, mut dropped) = (0usize, 0usize);
    for line in BufReader::new(File::open(&input)?)
        .split(b'\n')
        .take(max_lines)
    {
        let line = line?;
        match serde_json::from_slice::<Value>(&line)
            .ok()
            .as_ref()
            .and_then(sanitize)
        {
            Some(v) => {
                serde_json::to_writer(&mut writer, &v)?;
                writer.write_all(b"\n")?;
                kept += 1;
            }
            None => dropped += 1,
        }
    }
    writer.flush()?;
    println!(
        "wrote {kept} lines to {} ({dropped} unparsable dropped)",
        out.display()
    );
    Ok(())
}
