use super::*;

const ASSISTANT: &str = r#"{"parentUuid":"p1","isSidechain":false,"message":{"id":"msg_1","model":"claude-opus-5-5","content":[{"type":"text","text":"SECRET"}],"usage":{"input_tokens":2,"cache_creation_input_tokens":300,"cache_read_input_tokens":400,"output_tokens":50,"output_tokens_details":{"thinking_tokens":20},"cache_creation":{"ephemeral_1h_input_tokens":200,"ephemeral_5m_input_tokens":100},"server_tool_use":{"web_search_requests":1,"web_fetch_requests":2},"service_tier":"standard","speed":"fast","inference_geo":"global","iterations":[{"input_tokens":2,"output_tokens":50,"type":"message"}]}},"type":"assistant","uuid":"u1","timestamp":"2026-10-26T20:41:12.345Z","cwd":"C:\\work\\alpha","sessionId":"s1","version":"2.1.310","gitBranch":"main","entrypoint":"cli","requestId":"req_1"}"#;

fn usage_of(line: &str) -> UsageRecord<'_> {
    match LineParser::default().parse(line.as_bytes()) {
        ParseOutcome::Usage(u) => u,
        other => panic!("expected usage, got {other:?}"),
    }
}

#[test]
fn assistant_line_yields_usage_and_metadata() {
    let u = usage_of(ASSISTANT);
    assert_eq!(u.key, MsgKey::from_ids("msg_1", "req_1"));
    assert!(!u.key_is_fallback);
    assert_eq!(
        u.ts,
        Timestamp::parse_iso("2026-10-26T20:41:12.345Z").expect("ts")
    );
    assert_eq!(u.model, "claude-opus-5-5");
    assert_eq!(
        u.usage,
        Usage {
            tokens: TokenCounts {
                input: 2,
                output: 50,
                cache_read: 400,
                cache_w5m: 100,
                cache_w1h: 200,
                thinking: 20,
            },
            web_search: 1,
            web_fetch: 2,
            speed: Speed::Fast,
        }
    );
    assert!(!u.is_sidechain);
    assert_eq!(u.cwd.as_deref(), Some(r"C:\work\alpha"));
    assert_eq!(u.git_branch.as_deref(), Some("main"));
    assert_eq!(u.version.as_deref(), Some("2.1.310"));
    assert_eq!(u.entrypoint.as_deref(), Some("cli"));
    assert!(u.advisors.is_empty());
    assert!(!u.loose_match);
}

#[test]
fn content_is_never_materialized() {
    // Nothing in the record can hold the text; the Debug output proves it is not carried.
    let u = usage_of(ASSISTANT);
    assert!(!format!("{u:?}").contains("SECRET"));
}

#[test]
fn missing_breakdown_counts_cache_writes_as_5m() {
    let line = ASSISTANT.replace(
        r#""cache_creation":{"ephemeral_1h_input_tokens":200,"ephemeral_5m_input_tokens":100},"#,
        "",
    );
    let t = usage_of(&line).usage.tokens;
    assert_eq!((t.cache_w5m, t.cache_w1h), (300, 0));
}

#[test]
fn partial_streaming_record_parses_with_empty_iterations() {
    let line = ASSISTANT
        .replace(
            r#""output_tokens":50,"output"#,
            r#""output_tokens":3,"output"#,
        )
        .replace(
            r#""iterations":[{"input_tokens":2,"output_tokens":50,"type":"message"}]"#,
            r#""iterations":[]"#,
        );
    let u = usage_of(&line);
    assert_eq!(u.usage.tokens.output, 3);
    assert_eq!(u.key, MsgKey::from_ids("msg_1", "req_1"));
}

#[test]
fn advisor_iterations_become_separate_records() {
    let line = ASSISTANT.replace(
        r#""iterations":[{"input_tokens":2,"output_tokens":50,"type":"message"}]"#,
        r#""iterations":[{"input_tokens":1,"output_tokens":20,"type":"message"},{"input_tokens":7000,"output_tokens":400,"cache_read_input_tokens":0,"cache_creation_input_tokens":0,"type":"advisor_message","model":"claude-opus-5-5"},{"input_tokens":1,"output_tokens":30,"type":"message"}]"#,
    );
    let u = usage_of(&line);
    // Top-level usage already sums the `message` iterations; it stays as reported.
    assert_eq!(u.usage.tokens.output, 50);
    assert_eq!(u.advisors.len(), 1);
    let a = &u.advisors[0];
    assert_eq!(a.model, "claude-opus-5-5");
    assert_eq!((a.usage.tokens.input, a.usage.tokens.output), (7000, 400));
    assert_eq!(
        a.key,
        MsgKey::from_ids_tagged("msg_1", "req_1", "advisor#0")
    );
    assert_ne!(a.key, u.key);
}

#[test]
fn missing_ids_fall_back_and_are_flagged() {
    let no_req = ASSISTANT.replace(r#","requestId":"req_1""#, "");
    let u = usage_of(&no_req);
    assert_eq!(u.key, MsgKey::from_ids("msg_1", ""));
    assert!(!u.key_is_fallback);

    let no_id = ASSISTANT.replace(r#""id":"msg_1","#, "");
    let u = usage_of(&no_id);
    assert_eq!(u.key, MsgKey::from_fallback(b"u1"));
    assert!(u.key_is_fallback);
}

#[test]
fn non_assistant_lines_are_skipped_even_when_they_carry_usage() {
    let parser = LineParser::default();
    // A Task tool result reports the subagent's usage; counting it would double count.
    let user = r#"{"type":"user","message":{"role":"user","content":"x"},"toolUseResult":{"usage":{"input_tokens":5,"output_tokens":5},"note":"assistant"},"timestamp":"2026-10-26T20:41:12.345Z"}"#;
    assert!(matches!(parser.parse(user.as_bytes()), ParseOutcome::Skip));
    let attachment = r#"{"type":"attachment","attachment":{"usage":{"output_tokens":9}}}"#;
    assert!(matches!(
        parser.parse(attachment.as_bytes()),
        ParseOutcome::Skip
    ));
    let api = r#"{"type":"api-request","sessionId":"s1","params":{}}"#;
    assert!(matches!(parser.parse(api.as_bytes()), ParseOutcome::Skip));
}

#[test]
fn malformed_and_bad_timestamps_are_unknown() {
    let parser = LineParser::default();
    // A line cut short after the usage and type fields, as a crash mid-write leaves it.
    let cut = &ASSISTANT[..ASSISTANT.len() - 10];
    assert!(matches!(
        parser.parse(cut.as_bytes()),
        ParseOutcome::Unknown(UnknownReason::Malformed)
    ));
    let bad_ts = ASSISTANT.replace("2026-10-26T20:41:12.345Z", "yesterday");
    assert!(matches!(
        parser.parse(bad_ts.as_bytes()),
        ParseOutcome::Unknown(UnknownReason::BadTimestamp)
    ));
    // Malformed lines without usage are not ours to judge.
    assert!(matches!(parser.parse(b"{not json"), ParseOutcome::Skip));
}

#[test]
fn spaced_json_is_still_indexed_but_flagged() {
    // If Claude Code ever pretty-prints, the strict filter misses; the loose one catches it.
    let spaced = ASSISTANT.replace(r#""type":"assistant""#, r#""type": "assistant""#);
    let u = usage_of(&spaced);
    assert!(u.loose_match);
    assert_eq!(u.usage.tokens.output, 50);
}

#[test]
fn title_records_are_meta() {
    let parser = LineParser::default();
    let cases = [
        (
            r#"{"type":"ai-title","aiTitle":"Fix the parser","sessionId":"s1"}"#,
            TitleSource::Ai,
            "Fix the parser",
        ),
        (
            r#"{"type":"custom-title","customTitle":"mine","sessionId":"s1"}"#,
            TitleSource::Custom,
            "mine",
        ),
        (
            r#"{"type":"agent-name","agentName":"reviewer","sessionId":"s1"}"#,
            TitleSource::Agent,
            "reviewer",
        ),
    ];
    for (line, source, text) in cases {
        match parser.parse(line.as_bytes()) {
            ParseOutcome::Meta(MetaRecord::Title { source: s, text: t }) => {
                assert_eq!((s, t.as_ref()), (source, text));
            }
            other => panic!("expected a title, got {other:?}"),
        }
    }
    let blank = r#"{"type":"ai-title","aiTitle":"  ","sessionId":"s1"}"#;
    assert!(matches!(parser.parse(blank.as_bytes()), ParseOutcome::Skip));
    // `last-prompt` holds the user's prompt and must never be read.
    let prompt = r#"{"type":"last-prompt","lastPrompt":"secret","sessionId":"s1"}"#;
    assert!(matches!(
        parser.parse(prompt.as_bytes()),
        ParseOutcome::Skip
    ));
}
