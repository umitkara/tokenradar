use std::fmt::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::*;
use crate::clock::ManualClock;
use crate::domain::{LimitKind, Scope, Source, Timestamp};
use crate::providers::claude::credentials::CREDENTIALS_FILE;
use crate::secret::Secret;

const ACCOUNT: AccountId = AccountId(1);
const NOW: Timestamp = Timestamp::from_millis(1_793_050_000_000);
const TOKEN: &str = "sk-ant-oat01-FAKE-token-used-only-in-tests";

fn real_fixture() -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/api/claude-usage-2026-10-26-limits-array.json");
    std::fs::read(path).expect("fixture")
}

fn parse(body: &str) -> UsageReport {
    parse_usage(body.as_bytes(), ACCOUNT, NOW).expect("parse")
}

// --- Parser ---------------------------------------------------------------------------------

#[test]
fn real_response_uses_the_limits_array() {
    let report = parse_usage(&real_fixture(), ACCOUNT, NOW).expect("parse");
    assert_eq!(report.schema, UsageSchema::LimitsArray);
    let summary: Vec<_> = report
        .observations
        .iter()
        .map(|o| {
            (
                o.kind.clone(),
                o.scope.clone(),
                o.percent,
                o.severity.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![
            (LimitKind::Session, None, 5.0, Some("normal")),
            (LimitKind::WeeklyAll, None, 66.0, Some("normal")),
            (
                LimitKind::WeeklyScoped,
                Some(Scope::Model("Fable".into())),
                0.0,
                Some("normal")
            ),
        ]
    );
    let first = &report.observations[0];
    assert_eq!(first.source, Source::Api);
    assert_eq!(first.observed_at, NOW);
    assert_eq!(
        first.resets_at,
        Some(Timestamp::parse_iso("2026-10-26T22:29:59.942Z").expect("iso"))
    );

    let breakdown = report.breakdown.expect("breakdown");
    assert_eq!(breakdown.rows.len(), 4);
    assert_eq!(breakdown.rows[0], ("claude_code".to_owned(), Some(99.0)));
    assert!(breakdown.window_started_at.is_some());
    assert_eq!(report.extra.enabled(), Some(false));
}

#[test]
fn flat_fields_are_the_fallback() {
    let report = parse(
        r#"{"five_hour":{"utilization":12.0,"resets_at":"2026-10-26T22:29:59.9+00:00"},
            "seven_day":{"utilization":40.5,"resets_at":"2026-10-28T08:00:00+00:00"},
            "seven_day_opus":{"utilization":3.0,"resets_at":"2026-10-28T08:00:00+00:00"},
            "seven_day_sonnet":null,
            "seven_day_cowork":{"utilization":1.0},
            "iguana_necktie":{"utilization":0.0,"limit_dollars":250},
            "extra_usage":{"is_enabled":true}}"#,
    );
    assert_eq!(report.schema, UsageSchema::FlatFields);
    let kinds: Vec<_> = report
        .observations
        .iter()
        .map(|o| (o.kind.clone(), o.scope.clone(), o.percent))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (LimitKind::Session, None, 12.0),
            (LimitKind::WeeklyAll, None, 40.5),
            (
                LimitKind::WeeklyScoped,
                Some(Scope::Model("Opus".into())),
                3.0
            ),
        ]
    );
    assert_eq!(report.extra.enabled(), Some(true));
    assert_eq!(report.breakdown, None);
}

#[test]
fn an_empty_limits_array_falls_back_to_flat_fields() {
    let report = parse(r#"{"limits":[],"five_hour":{"utilization":7.0,"resets_at":null}}"#);
    assert_eq!(report.schema, UsageSchema::FlatFields);
    assert_eq!(report.observations.len(), 1);
    assert_eq!(report.observations[0].resets_at, None);
}

#[test]
fn unknown_kinds_are_kept_and_broken_entries_skipped() {
    let report = parse(
        r#"{"limits":[
            {"kind":"monthly_opus","percent":9,"resets_at":"2026-11-01T00:00:00Z","scope":null},
            {"kind":"session","percent":"lots"},
            {"percent":4},
            "nonsense",
            {"kind":"weekly_scoped","percent":2,"scope":{"model":null,"surface":"cowork"}}
        ]}"#,
    );
    assert_eq!(report.schema, UsageSchema::LimitsArray);
    assert_eq!(report.observations.len(), 2);
    assert_eq!(
        report.observations[0].kind,
        LimitKind::Other("monthly_opus".into())
    );
    assert_eq!(
        report.observations[1].scope,
        Some(Scope::Surface("cowork".into()))
    );
}

#[test]
fn missing_resets_at_is_not_an_error() {
    let report = parse(r#"{"limits":[{"kind":"session","percent":50,"severity":"warning"}]}"#);
    assert_eq!(report.observations[0].resets_at, None);
    assert_eq!(report.observations[0].severity.as_deref(), Some("warning"));
}

#[test]
fn nothing_usable_is_reported_as_empty() {
    assert_eq!(parse(r#"{"pace":null}"#).schema, UsageSchema::Empty);
    assert_eq!(
        parse_usage(b"[1,2]", ACCOUNT, NOW),
        Err(UsageParseError::NotAnObject)
    );
    assert_eq!(
        parse_usage(b"<html>", ACCOUNT, NOW),
        Err(UsageParseError::NotAnObject)
    );
}

// --- Poller ---------------------------------------------------------------------------------

/// Returns canned replies in order and records what it was asked.
#[derive(Clone, Default)]
struct MockTransport {
    calls: Arc<AtomicUsize>,
    replies: Arc<Mutex<Vec<Result<HttpReply, TransportError>>>>,
    seen_tokens: Arc<Mutex<Vec<String>>>,
}

impl MockTransport {
    fn push(&self, status: u16, body: &[u8]) {
        self.replies.lock().expect("lock").push(Ok(HttpReply {
            status,
            retry_after_ms: None,
            body: body.to_vec(),
        }));
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl UsageTransport for MockTransport {
    fn get_usage(&mut self, token: &Secret<String>) -> Result<HttpReply, TransportError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen_tokens
            .lock()
            .expect("lock")
            .push(token.expose().clone());
        let mut replies = self.replies.lock().expect("lock");
        if replies.is_empty() {
            Err(TransportError::Connect)
        } else {
            replies.remove(0)
        }
    }
}

fn credentials_dir(token: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    write_token(dir.path(), token);
    dir
}

fn write_token(dir: &Path, token: &str) {
    std::fs::write(
        dir.join(CREDENTIALS_FILE),
        format!(
            r#"{{"claudeAiOauth":{{"accessToken":"{token}","refreshToken":"sk-ant-ort01-FAKE","subscriptionType":"max"}}}}"#
        ),
    )
    .expect("write credentials");
}

#[test]
fn without_consent_no_request_is_ever_sent() {
    let dir = credentials_dir(TOKEN);
    let transport = MockTransport::default();
    let clock = ManualClock::at(NOW);
    let poller = UsageApiPoller::new(
        ACCOUNT,
        dir.path().to_path_buf(),
        false,
        transport.clone(),
        clock.clone(),
    );
    let (tx, rx) = mpsc::channel();
    let updates = Arc::new(Mutex::new(Vec::new()));
    let sink = updates.clone();
    let thread = std::thread::spawn(move || {
        poller.run(&rx, |u| sink.lock().expect("lock").push(u));
    });
    for _ in 0..3 {
        tx.send(PollerMsg::PanelOpened).expect("send");
        tx.send(PollerMsg::CredentialsChanged).expect("send");
        clock.advance_ms(60 * 60 * 1000);
        tx.send(PollerMsg::Inputs(PollInputs {
            activity: Activity::Busy,
            ..PollInputs::default()
        }))
        .expect("send");
    }
    tx.send(PollerMsg::Stop).expect("send");
    thread.join().expect("join");
    assert_eq!(transport.calls(), 0);
    let updates = updates.lock().expect("lock");
    assert!(updates.iter().all(|u| u.state == ApiState::Off));
}

#[test]
fn a_401_waits_for_new_credentials_then_recovers() {
    let dir = credentials_dir("sk-ant-oat01-FAKE-old");
    let transport = MockTransport::default();
    transport.push(401, b"{}");
    transport.push(200, &real_fixture());
    let clock = ManualClock::at(NOW);
    let mut poller = UsageApiPoller::new(
        ACCOUNT,
        dir.path().to_path_buf(),
        true,
        transport.clone(),
        clock.clone(),
    );
    assert!(poller.request().is_none());
    assert_eq!(poller.update(None).state, ApiState::TokenStale);
    clock.advance_ms(10 * 60 * 1000);
    assert_eq!(poller.scheduler.due_at(clock.now(), &poller.inputs), None);

    write_token(dir.path(), "sk-ant-oat01-FAKE-new");
    assert_eq!(poller.handle(PollerMsg::CredentialsChanged), Some(true));
    let due = poller
        .scheduler
        .due_at(clock.now(), &poller.inputs)
        .expect("due");
    assert!(due <= clock.now());
    let report = poller.request().expect("report");
    assert_eq!(report.observations.len(), 3);
    assert_eq!(poller.update(None).state, ApiState::Active);
    // The token is read fresh for every request.
    assert_eq!(
        *transport.seen_tokens.lock().expect("lock"),
        vec!["sk-ant-oat01-FAKE-old", "sk-ant-oat01-FAKE-new"]
    );
}

#[test]
fn statuses_map_to_states() {
    type Case = (u16, &'static [u8], fn(ApiState) -> bool);
    let dir = credentials_dir(TOKEN);
    let cases: [Case; 5] = [
        (429, b"", |s| matches!(s, ApiState::Backoff { .. })),
        (503, b"", |s| matches!(s, ApiState::Backoff { .. })),
        (200, b"{\"pace\":null}", |s| {
            matches!(s, ApiState::Error { .. })
        }),
        (200, b"not json", |s| matches!(s, ApiState::Error { .. })),
        (404, b"", |s| matches!(s, ApiState::Error { .. })),
    ];
    for (status, body, check) in cases {
        let transport = MockTransport::default();
        transport.push(status, body);
        let mut poller = UsageApiPoller::new(
            ACCOUNT,
            dir.path().to_path_buf(),
            true,
            transport,
            ManualClock::at(NOW),
        );
        poller.request();
        let state = poller.update(None).state;
        assert!(check(state), "{status}: {state:?}");
    }
    // No reply at all: offline.
    let mut poller = UsageApiPoller::new(
        ACCOUNT,
        dir.path().to_path_buf(),
        true,
        MockTransport::default(),
        ManualClock::at(NOW),
    );
    poller.request();
    assert!(matches!(
        poller.update(None).state,
        ApiState::Offline { .. }
    ));
}

#[test]
fn missing_credentials_mean_a_stale_token_and_no_request() {
    let dir = tempfile::tempdir().expect("tempdir");
    let transport = MockTransport::default();
    let mut poller = UsageApiPoller::new(
        ACCOUNT,
        dir.path().to_path_buf(),
        true,
        transport.clone(),
        ManualClock::at(NOW),
    );
    poller.request();
    assert_eq!(transport.calls(), 0);
    assert_eq!(poller.update(None).state, ApiState::TokenStale);
}

/// A tracing subscriber that keeps every formatted line.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("lock").extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The acceptance check "the token never appears in a log line, error or snapshot", run
/// against what is actually logged: every event at TRACE level, through all outcomes.
#[test]
fn the_token_never_reaches_logs_errors_or_updates() {
    let captured = Captured::default();
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();

    let dir = credentials_dir(TOKEN);
    let mut rendered = String::new();
    tracing::subscriber::with_default(subscriber, || {
        let transport = MockTransport::default();
        transport.push(200, &real_fixture());
        transport.push(401, b"{}");
        transport.push(429, b"");
        transport.push(500, b"");
        transport.push(200, b"garbage");
        let clock = ManualClock::at(NOW);
        let mut poller = UsageApiPoller::new(
            ACCOUNT,
            dir.path().to_path_buf(),
            true,
            transport,
            clock.clone(),
        );
        for _ in 0..6 {
            let report = poller.request();
            let update = poller.update(report);
            let _ = writeln!(rendered, "{update:?}");
            let _ = writeln!(rendered, "{:?}", update.diagnostics);
            poller.scheduler.on_credentials_changed();
            clock.advance_ms(60 * 60 * 1000);
        }
        // Corrupt credentials: the parse error must not quote the file either.
        std::fs::write(
            dir.path().join(CREDENTIALS_FILE),
            format!(r#"{{"claudeAiOauth":"{TOKEN}"}}"#),
        )
        .expect("write");
        poller.request();
        let _ = writeln!(rendered, "{:?}", poller.update(None));
    });
    let logs = String::from_utf8(captured.0.lock().expect("lock").clone()).expect("utf8");
    assert!(logs.contains("usage api"), "nothing was logged: {logs}");
    for text in [&logs, &rendered] {
        assert!(!text.contains("sk-ant-"), "token leaked:\n{text}");
    }
}
