use std::fs;

use super::*;
use crate::domain::{AccountSpec, Provider};

const UUID: &str = "0a1f0000-0000-4000-8000-00000000a001";

struct Env {
    _tmp: tempfile::TempDir,
    config_dir: PathBuf,
    store: Store,
    account: AccountId,
}

impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let config_dir = tmp.path().join("claude");
        fs::create_dir_all(config_dir.join(SESSIONS_DIR)).expect("mkdir");
        let store = Store::open(tmp.path().join("db.sqlite")).expect("store");
        let account = store
            .writer()
            .upsert_account(AccountSpec {
                provider: Provider::Claude,
                label: "Claude".into(),
                config_dir: config_dir.to_string_lossy().into_owned(),
                plan: None,
                tier: None,
                environment: "native".into(),
            })
            .expect("account");
        Self {
            _tmp: tmp,
            config_dir,
            store,
            account,
        }
    }

    fn tracker(&self, probe: fn(u32) -> Liveness) -> LiveSessionTracker {
        LiveSessionTracker::with_probe(self.account, &self.config_dir, self.store.clone(), probe)
    }

    fn write(&self, pid: u32, json: &serde_json::Value) {
        let path = self
            .config_dir
            .join(SESSIONS_DIR)
            .join(format!("{pid}.json"));
        fs::write(path, serde_json::to_vec(json).expect("json")).expect("write");
    }
}

/// A session file shaped like the ones Claude Code 2.1.311 writes, with synthetic values.
fn session_json(pid: u32, status: &str, proc_start: u64) -> serde_json::Value {
    serde_json::json!({
        "pid": pid,
        "sessionId": UUID,
        "cwd": "C:\\Work\\Alpha",
        "startedAt": 1_793_082_859_896_i64,
        "procStart": proc_start.to_string(),
        "version": "2.1.311",
        "kind": "interactive",
        "entrypoint": "cli",
        "pidDomain": "win32:tester",
        "messagingSocketPath": "\\\\.\\pipe\\LOCAL\\cc-msg-synthetic",
        "name": "alpha-1",
        "nameSource": "derived",
        "status": status,
        "updatedAt": 1_793_090_172_041_i64,
        "statusUpdatedAt": 1_793_090_172_041_i64,
    })
}

const CREATED: u64 = 134_375_564_581_785_791;

fn alive(_: u32) -> Liveness {
    Liveness::Alive { created: CREATED }
}

fn dead(_: u32) -> Liveness {
    Liveness::Dead
}

#[test]
fn parses_numbers_and_strings_and_ignores_the_rest() {
    let mut json = session_json(4396, "busy", CREATED);
    json["pid"] = serde_json::json!("4396");
    json["procStart"] = serde_json::json!(CREATED);
    json["futureField"] = serde_json::json!({ "nested": [1, 2, 3] });
    let file = parse_session_file(&serde_json::to_vec(&json).expect("json"), 1).expect("parse");
    assert_eq!(file.pid, 4396);
    assert_eq!(file.proc_start, Some(CREATED));
    assert_eq!(file.status, LiveStatus::Busy);
    assert_eq!(file.name.as_deref(), Some("alpha-1"));

    // No pid in the content: the file name decides. No session id: not a session file.
    json.as_object_mut().expect("object").remove("pid");
    let file = parse_session_file(&serde_json::to_vec(&json).expect("json"), 77).expect("parse");
    assert_eq!(file.pid, 77);
    json.as_object_mut().expect("object").remove("sessionId");
    assert_eq!(
        parse_session_file(&serde_json::to_vec(&json).expect("json"), 77),
        None
    );
    assert_eq!(parse_session_file(b"{not json", 1), None);
}

#[test]
fn liveness_requires_a_matching_creation_time() {
    let file = parse_session_file(
        &serde_json::to_vec(&session_json(1, "idle", CREATED)).expect("json"),
        1,
    )
    .expect("parse");
    assert!(is_live(&file, Liveness::Alive { created: CREATED }));
    assert!(is_live(&file, Liveness::Unknown));
    assert!(!is_live(&file, Liveness::Dead));
    // Same pid, different process: the pid was reused after a crash.
    assert!(!is_live(
        &file,
        Liveness::Alive {
            created: CREATED + 10_000_000
        }
    ));

    // Without `procStart`, the process must have started shortly before `startedAt`.
    let no_start = SessionFile {
        proc_start: None,
        ..file
    };
    assert!(is_live(&no_start, Liveness::Alive { created: CREATED }));
    let hour = 36_000_000_000;
    assert!(!is_live(
        &no_start,
        Liveness::Alive {
            created: CREATED - hour
        }
    ));
}

#[test]
fn a_session_with_a_missing_pid_is_not_listed() {
    let env = Env::new();
    // A pid that is never handed out on Windows, checked with the real probe.
    env.write(0xFFFF_FFFD, &session_json(0xFFFF_FFFD, "busy", CREATED));
    let mut tracker = env.tracker(probe);
    assert!(!tracker.refresh(), "empty before and after");
    assert_eq!(tracker.sessions(), &[] as &[LiveSession]);

    let mut tracker = env.tracker(dead);
    tracker.refresh();
    assert_eq!(tracker.sessions(), &[] as &[LiveSession]);
}

#[cfg(windows)]
#[test]
fn the_running_process_is_listed_with_the_real_probe() {
    let env = Env::new();
    let Liveness::Alive { created } = probe(std::process::id()) else {
        panic!("own process must be alive");
    };
    let pid = std::process::id();
    env.write(pid, &session_json(pid, "busy", created));
    let mut tracker = env.tracker(probe);
    assert!(tracker.refresh());
    assert_eq!(tracker.sessions().len(), 1);
    assert_eq!(tracker.sessions()[0].pid, pid);

    // The same pid with another start time is a reused pid.
    env.write(pid, &session_json(pid, "busy", created - 1_000_000_000));
    assert!(tracker.refresh());
    assert_eq!(tracker.sessions(), &[] as &[LiveSession]);
}

#[test]
fn reports_only_real_changes() {
    let env = Env::new();
    let mut tracker = env.tracker(alive);
    env.write(10, &session_json(10, "busy", CREATED));
    assert!(tracker.refresh(), "new session");
    assert!(!tracker.refresh(), "nothing changed");
    assert_eq!(tracker.sessions()[0].status, LiveStatus::Busy);

    // A rewrite that only moves timestamps is not a change …
    let mut json = session_json(10, "busy", CREATED);
    json["updatedAt"] = serde_json::json!(1_793_090_999_999_i64);
    env.write(10, &json);
    assert!(!tracker.refresh());

    // … a status flip is.
    env.write(10, &session_json(10, "idle", CREATED));
    assert!(tracker.refresh());
    assert_eq!(tracker.sessions()[0].status, LiveStatus::Idle);

    // Removing the file removes the session.
    fs::remove_file(env.config_dir.join(SESSIONS_DIR).join("10.json")).expect("rm");
    assert!(tracker.refresh());
    assert_eq!(tracker.sessions(), &[] as &[LiveSession]);
}

#[test]
fn skips_key_files_foreign_pid_domains_and_junk() {
    let env = Env::new();
    let dir = env.config_dir.join(SESSIONS_DIR);
    // Never opened: the name does not match `<pid>.json`.
    fs::write(dir.join("10.8f1e.key"), b"secret").expect("write");
    fs::write(dir.join("notes.json"), b"{}").expect("write");
    fs::write(dir.join("11.json"), b"{ half a write").expect("write");
    let mut json = session_json(12, "busy", CREATED);
    json["pidDomain"] = serde_json::json!("linux:wsl-ubuntu");
    env.write(12, &json);
    env.write(13, &session_json(13, "idle", CREATED));

    let mut tracker = env.tracker(alive);
    tracker.refresh();
    let pids: Vec<u32> = tracker.sessions().iter().map(|s| s.pid).collect();
    if cfg!(windows) {
        assert_eq!(pids, [13]);
    } else {
        assert_eq!(pids, [12, 13]);
    }
    assert!(is_session_file(&dir, &dir.join("13.json")));
    assert!(!is_session_file(&dir, &dir.join("10.8f1e.key")));
    assert!(!is_session_file(&dir, &env.config_dir.join("13.json")));
}

#[test]
fn maps_to_the_indexed_session_once_it_exists() {
    let env = Env::new();
    env.write(10, &session_json(10, "busy", CREATED));
    let mut tracker = env.tracker(alive);
    tracker.refresh();
    assert!(tracker.has_unresolved());

    let id = env
        .store
        .writer()
        .upsert_session(crate::domain::SessionSpec::new(
            env.account,
            UUID,
            crate::domain::SessionKind::Main,
        ))
        .expect("session");
    assert!(tracker.refresh(), "the mapping counts as a change");
    assert_eq!(tracker.sessions()[0].session_id, Some(id));
    assert!(!tracker.has_unresolved());
}
