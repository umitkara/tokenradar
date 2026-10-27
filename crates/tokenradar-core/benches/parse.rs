//! Step 02 spike: parse strategies A–D on synthetic JSONL.
//!
//! The corpus mimics the line mix measured on the reference data set (docs/perf/2026-10-27-spike.md):
//! by bytes, about half are `user` lines with tool output, a tenth are `assistant` lines, and the
//! rest are attachments, request blobs and small bookkeeping lines. All text is generated.

// The file reader and thread merge in the shared module are only used by `parse_real`.
#[allow(dead_code)]
#[path = "../spike/parse_strategies.rs"]
mod strategies;

use std::fmt::Write as _;
use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use strategies::{Parser, Strategy, Tally};

/// Corpus size; large enough that per-iteration overhead is noise.
const CORPUS_BYTES: usize = 16 << 20;

/// Deterministic xorshift generator, so every run measures the same bytes.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Appends roughly `len` bytes of JSON string body: words with occasional escapes.
fn text(rng: &mut Rng, len: usize, out: &mut String) {
    const WORDS: [&str; 8] = [
        "lorem", "ipsum", "dolor", "sit", "amet", "radar", "token", "fn",
    ];
    let start = out.len();
    while out.len() - start < len {
        match rng.below(50) {
            0 => out.push_str("\\n"),
            1 => out.push_str("\\\""),
            2 => out.push_str("\\\\"),
            _ => out.push_str(WORDS[usize::try_from(rng.below(8)).expect("small")]),
        }
        out.push(' ');
    }
}

fn header(out: &mut String, n: u64) {
    write!(
        out,
        r#"{{"parentUuid":"00000000-0000-4000-8000-{n:012}","isSidechain":false,"userType":"external","cwd":"D:\\Projects\\Example\\repo","sessionId":"11111111-2222-4333-8444-555555555555","version":"2.1.310","gitBranch":"main","#
    )
    .expect("write to String");
}

fn assistant(rng: &mut Rng, n: u64, partial: bool, out: &mut String) {
    header(out, n);
    out.push_str(r#""message":{"id":"msg_"#);
    write!(out, "{:020}", n / 2).expect("write to String");
    out.push_str(r#"","type":"message","role":"assistant","model":"claude-opus-5-5","content":[{"type":"text","text":""#);
    let len = usize::try_from(500 + rng.below(6000)).expect("small");
    text(rng, len, out);
    let output = if partial { 1 } else { 1 + rng.below(4000) };
    write!(
        out,
        r#""}}],"usage":{{"input_tokens":{},"cache_creation_input_tokens":{},"cache_read_input_tokens":{},"output_tokens":{output},"cache_creation":{{"ephemeral_1h_input_tokens":0,"ephemeral_5m_input_tokens":0}},"service_tier":"standard","speed":"standard","iterations":[]}}}},"requestId":"req_{:020}","type":"assistant","uuid":"u-{n}","timestamp":"2026-10-26T20:41:12.345Z"}}"#,
        rng.below(10),
        rng.below(20_000),
        rng.below(200_000),
        n / 2,
    )
    .expect("write to String");
}

fn user(rng: &mut Rng, n: u64, out: &mut String) {
    header(out, n);
    out.push_str(r#""message":{"role":"user","content":[{"tool_use_id":"toolu_1","type":"tool_result","content":""#);
    let len = usize::try_from(2_000 + rng.below(55_000)).expect("small");
    text(rng, len, out);
    write!(
        out,
        r#""}}]}},"type":"user","uuid":"u-{n}","timestamp":"2026-10-26T20:41:13.000Z"}}"#
    )
    .expect("write to String");
}

fn other(rng: &mut Rng, kind: &str, len: usize, out: &mut String) {
    out.push_str(r#"{"type":""#);
    out.push_str(kind);
    out.push_str(r#"","data":""#);
    text(rng, len, out);
    out.push_str(r#"","timestamp":"2026-10-26T20:41:14.000Z"}"#);
}

/// Builds newline-separated lines until the corpus reaches `CORPUS_BYTES`.
fn corpus() -> String {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut out = String::with_capacity(CORPUS_BYTES + (1 << 20));
    let mut n = 0;
    while out.len() < CORPUS_BYTES {
        n += 1;
        // Line mix by count: 26 % assistant, 17 % user, 26 % attachment, 8 % blob, 23 % small.
        match rng.below(100) {
            0..26 => {
                // About 55 % of assistant lines repeat a message with partial usage.
                let partial = rng.below(100) < 55;
                assistant(&mut rng, n, partial, &mut out);
            }
            26..43 => user(&mut rng, n, &mut out),
            43..69 => {
                let len = usize::try_from(1_000 + rng.below(12_000)).expect("small");
                other(&mut rng, "attachment", len, &mut out);
            }
            69..77 => {
                let len = usize::try_from(2_000 + rng.below(30_000)).expect("small");
                other(&mut rng, "api-request-blob", len, &mut out);
            }
            _ => other(&mut rng, "last-prompt", 100, &mut out),
        }
        out.push('\n');
    }
    out
}

fn parse(c: &mut Criterion) {
    let corpus = corpus();
    let lines: Vec<&[u8]> = corpus.as_bytes().split(|&b| b == b'\n').collect();
    let mut group = c.benchmark_group("parse");
    group.throughput(Throughput::Bytes(corpus.len() as u64));
    group.sample_size(20);
    for strategy in Strategy::ALL {
        group.bench_function(strategy.name(), |b| {
            let mut parser = Parser::new(strategy);
            b.iter(|| {
                let mut tally = Tally::default();
                for line in &lines {
                    parser.line(black_box(line), &mut tally);
                }
                black_box(tally.totals())
            });
        });
    }
    group.finish();
}

criterion_group!(benches, parse);
criterion_main!(benches);
