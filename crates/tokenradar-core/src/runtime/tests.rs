//! Runtime tests against a temporary copy of the synthetic fixtures, with the real watcher.

use std::fs::{self, OpenOptions};
use std::io::Write as _;

use super::*;
use crate::events::Delivery;
use crate::live_sessions::{Liveness, SESSIONS_DIR};
use crate::statusline::StatuslinePush;

const ALPHA: &str = "0a1f0000-0000-4000-8000-00000000a001";

/// Event latency budget (Step 05: `UsageAppended` within 2 s). CI machines get slack.
fn latency_budget() -> Duration {
    if std::env::var_os("CI").is_some() {
        Duration::from_secs(10)
    } else {
        Duration::from_secs(2)
    }
}

struct Env {
    _tmp: tempfile::TempDir,
    config_dir: PathBuf,
    db_path: PathBuf,
}

impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let config_dir = tmp.path().join("claude");
        copy_dir(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/claude"),
            &config_dir,
        );
        fs::create_dir_all(config_dir.join(SESSIONS_DIR)).expect("mkdir");
        let env = Self {
            db_path: tmp.path().join("db.sqlite"),
            config_dir,
            _tmp: tmp,
        };
        // The Alpha fixture ends in a half-written line on purpose (indexer tests). Finish it,
        // so lines appended here start on a line of their own.
        append(
            &env.alpha_main(),
            "
",
        );
        env
    }

    fn config(&self) -> CoreConfig {
        let mut config = CoreConfig::new(
            self.db_path.clone(),
            vec![AccountDescriptor {
                config_dir: self.config_dir.clone(),
                label: "Claude".into(),
            }],
        );
        config.timings.startup_delay = Duration::from_millis(50);
        config.indexer.background = false;
        config.retention_days = None;
        config
    }

    fn alpha_main(&self) -> PathBuf {
        self.config_dir
            .join("projects/C--Work-Alpha")
            .join(format!("{ALPHA}.jsonl"))
    }
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("mkdir");
    for entry in fs::read_dir(from).expect("read_dir") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy");
        }
    }
}

/// A synthetic assistant line in the fixture format.
fn assistant_line(n: u32) -> String {
    format!(
        concat!(
            r#"{{"parentUuid":null,"isSidechain":false,"message":{{"id":"msg_rt{n}","model":"claude-opus-5-5","#,
            r#""role":"assistant","type":"message","content":[],"usage":{{"input_tokens":1,"#,
            r#""cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":{n}}}}},"#,
            r#""type":"assistant","uuid":"u-rt{n}","timestamp":"2026-10-27T10:00:00.000Z","#,
            r#""sessionId":"{ALPHA}","cwd":"C:\\Work\\Alpha","requestId":"req_rt{n}"}}"#,
            "\n"
        ),
        n = n,
        ALPHA = ALPHA
    )
}

fn append(path: &Path, text: &str) {
    let mut f = OpenOptions::new().append(true).open(path).expect("open");
    f.write_all(text.as_bytes()).expect("append");
}

/// Forwards a subscription to a channel so tests can wait with a timeout.
fn events(handle: &CoreHandle) -> mpsc::Receiver<Delivery> {
    let mut sub = handle.subscribe();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        while let Some(d) = sub.blocking_recv() {
            if tx.send(d).is_err() {
                break;
            }
        }
    });
    rx
}

/// Waits for the first event that `pick` accepts.
fn wait_for<T>(
    rx: &mpsc::Receiver<Delivery>,
    timeout: Duration,
    mut pick: impl FnMut(&CoreEvent) -> Option<T>,
) -> Option<T> {
    let deadline = Instant::now() + timeout;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(Delivery::Event(e)) => {
                if let Some(t) = pick(&e) {
                    return Some(t);
                }
            }
            Ok(Delivery::Resync { .. }) => {}
            Err(_) => return None,
        }
    }
}

fn wait_initial(rx: &mpsc::Receiver<Delivery>) {
    wait_for(rx, Duration::from_secs(30), |e| {
        matches!(
            e,
            CoreEvent::IndexMilestone {
                milestone: IndexMilestone::Complete,
                ..
            }
        )
        .then_some(())
    })
    .expect("initial scan completes");
}

#[test]
fn an_appended_line_is_indexed_within_the_latency_budget() {
    let env = Env::new();
    let handle = CoreRuntime::start(&env.config()).expect("start");
    let rx = events(&handle);
    wait_initial(&rx);
    let status = handle.status();
    assert!(status.initial_scan_done);
    // The panel reads the indexing state from the status, not from the events it missed.
    assert!(status.recent_ready);
    assert_eq!(status.index_progress, None);
    if let Some(snapshot) = status.panel_snapshot() {
        assert!(snapshot.index_state.initial_scan_done);
        assert!(snapshot.index_state.recent_ready);
    }

    let written = Instant::now();
    append(&env.alpha_main(), &assistant_line(41));
    let sessions = wait_for(&rx, latency_budget(), |e| match e {
        CoreEvent::UsageAppended {
            sessions, to_ts, ..
        } => {
            assert_eq!(to_ts.to_string(), "2026-10-27T10:00:00Z");
            Some(sessions.clone())
        }
        _ => None,
    })
    .unwrap_or_else(|| panic!("no UsageAppended within {:?}", latency_budget()));
    eprintln!("UsageAppended after {:?}", written.elapsed());
    let alpha = handle
        .store()
        .session_id_by_uuid(AccountId(1), ALPHA)
        .expect("query")
        .expect("indexed");
    assert_eq!(sessions, [alpha]);
    assert!(handle.status().last_indexed_at.is_some());

    let report = handle.shutdown(SHUTDOWN_TIMEOUT);
    assert!(report.timed_out.is_empty(), "{report:?}");
}

#[test]
fn a_burst_of_writes_is_one_indexing_pass() {
    let env = Env::new();
    let handle = CoreRuntime::start(&env.config()).expect("start");
    let rx = events(&handle);
    wait_initial(&rx);
    let before = handle.status().incremental_passes;

    let path = env.alpha_main();
    let started = Instant::now();
    for n in 0..100 {
        append(&path, &assistant_line(1000 + n));
    }
    // The burst must fit inside one debounce window for this test to mean anything.
    assert!(started.elapsed() < Duration::from_millis(500));

    wait_for(&rx, latency_budget(), |e| {
        matches!(e, CoreEvent::UsageAppended { .. }).then_some(())
    })
    .expect("UsageAppended");
    // Let any straggling batch through before counting.
    thread::sleep(watcher::DEBOUNCE * 2);
    assert_eq!(handle.status().incremental_passes - before, 1);
    handle.shutdown(SHUTDOWN_TIMEOUT);
}

#[test]
fn live_sessions_follow_the_session_directory() {
    let env = Env::new();
    let handle = CoreRuntime::start(&env.config()).expect("start");
    let rx = events(&handle);
    let sessions = env.config_dir.join(SESSIONS_DIR);

    // A session whose process does not exist never shows up.
    let ghost = 0xFFFF_FFFD_u32;
    let file = |pid: u32, status: &str, proc_start: u64| {
        serde_json::json!({
            "pid": pid, "sessionId": ALPHA, "cwd": "C:\\Work\\Alpha",
            "procStart": proc_start.to_string(), "status": status,
            "startedAt": 1_793_082_859_896_i64, "statusUpdatedAt": 1_793_082_859_896_i64,
        })
        .to_string()
    };
    fs::write(
        sessions.join(format!("{ghost}.json")),
        file(ghost, "busy", 1),
    )
    .expect("write");

    #[cfg(windows)]
    {
        let pid = std::process::id();
        let Liveness::Alive { created } = live_sessions::probe(pid) else {
            panic!("own process is alive");
        };
        let path = sessions.join(format!("{pid}.json"));
        fs::write(&path, file(pid, "busy", created)).expect("write");
        let list = wait_for(&rx, latency_budget(), |e| match e {
            CoreEvent::LiveSessionsChanged(list) => Some(list.clone()),
            _ => None,
        })
        .expect("our session appears");
        assert_eq!(list.len(), 1, "{list:?}");
        assert_eq!(list[0].pid, pid);
        assert_eq!(list[0].status, live_sessions::LiveStatus::Busy);

        // busy → idle within the budget.
        let flipped = Instant::now();
        fs::write(&path, file(pid, "idle", created)).expect("write");
        wait_for(&rx, latency_budget(), |e| match e {
            CoreEvent::LiveSessionsChanged(list)
                if list.first().map(|s| s.status) == Some(live_sessions::LiveStatus::Idle) =>
            {
                Some(())
            }
            _ => None,
        })
        .expect("status change");
        eprintln!("status change after {:?}", flipped.elapsed());

        fs::remove_file(&path).expect("rm");
        wait_for(&rx, latency_budget(), |e| match e {
            CoreEvent::LiveSessionsChanged(list) if list.is_empty() => Some(()),
            _ => None,
        })
        .expect("session gone");
    }
    assert!(handle.status().live_sessions.iter().all(|s| s.pid != ghost));
    handle.shutdown(SHUTDOWN_TIMEOUT);
}

#[test]
fn credentials_changes_are_seen_by_mtime_only() {
    let env = Env::new();
    let creds = env.config_dir.join(CREDENTIALS_FILE);
    fs::write(&creds, b"{}").expect("write");
    let handle = CoreRuntime::start(&env.config()).expect("start");
    let rx = events(&handle);
    // Give the watcher a moment to attach, then change the file.
    thread::sleep(Duration::from_millis(200));
    let file = OpenOptions::new().write(true).open(&creds).expect("open");
    file.set_modified(SystemTime::now() + Duration::from_secs(60))
        .expect("set mtime");
    drop(file);
    wait_for(&rx, latency_budget(), |e| {
        matches!(e, CoreEvent::CredentialsChanged { .. }).then_some(())
    })
    .expect("CredentialsChanged");
    handle.shutdown(SHUTDOWN_TIMEOUT);
}

#[test]
fn shutdown_stops_every_thread_in_time() {
    let env = Env::new();
    let mut config = env.config();
    config.retention_days = Some(indexer::DEFAULT_KEEP_DAYS);
    let handle = CoreRuntime::start(&config).expect("start");
    let mut sub = handle.subscribe();

    let started = Instant::now();
    let report = handle.shutdown(SHUTDOWN_TIMEOUT);
    assert!(started.elapsed() < SHUTDOWN_TIMEOUT);
    assert!(report.timed_out.is_empty(), "{report:?}");
    // core-watch, core-index-1, core-api-1, core-limits and retention.
    assert_eq!(report.joined, 5);
    // Every thread held a bus sender; the bus closing proves they are all gone.
    while sub.blocking_recv().is_some() {}
    // The store's writer thread stopped too: the database can be opened exclusively again.
    let conn = rusqlite::Connection::open(&env.db_path).expect("open");
    conn.execute_batch("PRAGMA locking_mode = EXCLUSIVE; BEGIN EXCLUSIVE; COMMIT;")
        .expect("exclusive lock");
}

#[test]
fn a_limit_reading_reaches_the_panel_snapshot() {
    use crate::domain::{LimitKind, Source};
    use crate::providers::claude::usage_api::ApiState;

    let env = Env::new();
    let handle = CoreRuntime::start(&env.config()).expect("start");
    let rx = events(&handle);
    let now = Timestamp::now();
    handle.ingest_limit(LimitObservation {
        account: AccountId(1),
        kind: LimitKind::Session,
        scope: None,
        percent: 42.0,
        resets_at: Some(Timestamp::from_millis(now.as_millis() + 3 * 3_600_000)),
        severity: None,
        source: Source::Statusline,
        session_uuid: None,
        observed_at: now,
    });
    let snapshot = wait_for(&rx, Duration::from_secs(5), |e| match e {
        CoreEvent::LimitsUpdated(s) if !s.limits.is_empty() => Some(s.clone()),
        _ => None,
    })
    .expect("LimitsUpdated");
    let view = &snapshot.limits[0];
    assert_eq!(view.kind, LimitKind::Session);
    assert!((view.percent - 42.0).abs() < 1e-9);
    assert_eq!(view.source, Source::Statusline);
    // Without consent the API reports itself off and the poller never ran a request.
    assert_eq!(snapshot.api.state, ApiState::Off);
    assert_eq!(snapshot.api.diagnostics.requests, 0);
    let status = handle.status();
    assert!(status.limits.is_some_and(|s| !s.limits.is_empty()));
    handle.shutdown(SHUTDOWN_TIMEOUT);
}

#[test]
fn shutdown_interrupts_the_startup_delay() {
    let env = Env::new();
    let mut config = env.config();
    config.timings.startup_delay = Duration::from_secs(60);
    let handle = CoreRuntime::start(&config).expect("start");
    let started = Instant::now();
    let report = handle.shutdown(SHUTDOWN_TIMEOUT);
    assert_eq!(report.timed_out, Vec::<String>::new());
    assert!(started.elapsed() < Duration::from_millis(500));
}

/// A statusline reading for the Alpha project of `env`, as the CLI would send it.
fn statusline_push(env: &Env, observed_at: Timestamp, percent: f64) -> StatuslinePush {
    let resets_s = observed_at.as_millis() / 1000 + 3 * 3600;
    let payload = serde_json::json!({
        "session_id": ALPHA,
        "transcript_path": env.alpha_main(),
        "rate_limits": {
            "five_hour": { "used_percentage": percent, "resets_at": resets_s },
        },
    });
    StatuslinePush {
        observed_at,
        payload: crate::statusline::StatuslinePayload::parse(
            &serde_json::to_vec(&payload).expect("json"),
        )
        .expect("payload"),
    }
}

fn session_percent(snapshot: &PanelSnapshot) -> Option<f64> {
    snapshot
        .limits
        .iter()
        .find(|l| l.kind == crate::domain::LimitKind::Session)
        .map(|l| l.percent)
}

#[cfg(windows)]
#[test]
fn a_statusline_push_over_ipc_reaches_the_panel_snapshot() {
    use crate::ipc::{Client, Op, Request, StatusQuery, StatusReply};

    let env = Env::new();
    let mut config = env.config();
    let pipe = format!(r"\\.\pipe\tokenradar-runtime-test-{}", std::process::id());
    config.ipc = Some(IpcConfig {
        pipe_name: pipe.clone(),
        server_id: "tokenradar-test/1".into(),
    });
    let handle = CoreRuntime::start(&config).expect("start");
    let rx = events(&handle);

    // The pipe comes up on its own thread; retry briefly.
    let started = Instant::now();
    let mut client = loop {
        match Client::connect(&pipe, Duration::from_millis(200)) {
            Ok(c) => break c,
            Err(e) if started.elapsed() > Duration::from_secs(5) => panic!("connect: {e:?}"),
            Err(_) => thread::sleep(Duration::from_millis(20)),
        }
    };
    let push = statusline_push(&env, Timestamp::now(), 61.5);
    let replies = client
        .call(&[
            Request::new(1, Op::StatuslinePush(Box::new(push))),
            Request::new(2, Op::StatusGet(StatusQuery::default())),
        ])
        .expect("call");
    assert_eq!(replies.len(), 1);
    let _: StatusReply = replies[0].clone().into_data().expect("status");

    let snapshot = wait_for(&rx, Duration::from_secs(5), |e| match e {
        CoreEvent::LimitsUpdated(s) if session_percent(s).is_some() => Some(s.clone()),
        _ => None,
    })
    .expect("LimitsUpdated");
    assert_eq!(session_percent(&snapshot), Some(61.5));

    // The next status answer reflects it.
    let mut client = Client::connect(&pipe, Duration::from_secs(2)).expect("connect");
    let replies = client
        .call(&[Request::new(3, Op::StatusGet(StatusQuery::default()))])
        .expect("call");
    let status: StatusReply = replies[0].clone().into_data().expect("status");
    assert!(
        status
            .limits
            .iter()
            .any(|l| (l.percent - 61.5).abs() < 1e-9),
        "{status:?}"
    );
    let report = handle.shutdown(SHUTDOWN_TIMEOUT);
    assert!(report.timed_out.is_empty(), "{report:?}");
}

#[test]
fn spooled_readings_are_imported_once_at_start() {
    let env = Env::new();
    let spool_dir = env.db_path.with_file_name("spool");
    let now = Timestamp::now();
    let pushes: Vec<_> = (0..10u32)
        .map(|i| {
            let at = Timestamp::from_millis(now.as_millis() - i64::from(10 - i) * 60_000);
            statusline_push(&env, at, 10.0 + f64::from(i))
        })
        .collect();
    for p in &pushes {
        spool::append(&spool_dir, p).expect("spool");
    }

    let mut config = env.config();
    config.spool_dir = Some(spool_dir.clone());
    let handle = CoreRuntime::start(&config).expect("start");
    let rx = events(&handle);
    let snapshot = wait_for(&rx, Duration::from_secs(5), |e| match e {
        CoreEvent::LimitsUpdated(s) if session_percent(s).is_some() => Some(s.clone()),
        _ => None,
    })
    .expect("LimitsUpdated");
    assert_eq!(
        session_percent(&snapshot),
        Some(19.0),
        "the newest reading wins"
    );

    let account = AccountId(1);
    let stored = handle
        .store()
        .snapshots_between(account, Timestamp::from_millis(0), Timestamp::now())
        .expect("snapshots");
    assert_eq!(stored.len(), 10, "every spooled reading is kept");
    assert_eq!(
        fs::read_dir(&spool_dir).expect("spool dir").count(),
        0,
        "imported files are deleted"
    );

    // A file that comes back (a crash between ingest and delete) adds nothing.
    for p in &pushes {
        spool::append(&spool_dir, p).expect("spool");
    }
    let accounts = vec![(account, env.config_dir.clone())];
    let mut again = 0;
    let report = bridge::import_spool(&spool_dir, handle.store(), &accounts, |_| again += 1);
    assert_eq!((report.ingested, report.duplicates, again), (0, 10, 0));
    handle.shutdown(SHUTDOWN_TIMEOUT);
}
