use std::io::{Read, Write};
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread::JoinHandle;

use super::*;
use crate::domain::Timestamp;
use crate::ipc::client::{Client, ClientError};
use crate::ipc::proto::{ERR_UNSUPPORTED, ERR_VERSION, Hello, HelloReply, Op, StatusQuery};
use crate::ipc::security::{canonical_sid, current_user_sid, dacl_sddl};
use crate::ipc::view::{LimitsReply, Origin, SessionsReply, StatusReply};
use crate::statusline::{StatuslinePayload, StatuslinePush};

#[derive(Default)]
struct Fake {
    pushes: Mutex<Vec<StatuslinePush>>,
}

impl Handler for Fake {
    fn push(&self, push: StatuslinePush) {
        self.pushes.lock().expect("lock").push(push);
    }
    fn status(&self, query: &StatusQuery) -> StatusReply {
        let mut r = StatusReply::empty(Timestamp::from_millis(1));
        r.api_state = query.session_uuid.clone();
        r
    }
    fn limits(&self) -> LimitsReply {
        StatusReply::empty(Timestamp::from_millis(2)).limits_reply()
    }
    fn sessions(&self) -> SessionsReply {
        StatusReply::empty(Timestamp::from_millis(3)).sessions_reply()
    }
}

struct Running {
    name: String,
    fake: Arc<Fake>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(t) = self.thread.take() {
            t.join().expect("server thread");
        }
    }
}

fn unique_name() -> String {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!(r"\\.\pipe\tokenradar-test-{}-{n}", std::process::id())
}

fn start_with(config: ServerConfig) -> Running {
    let fake = Arc::new(Fake::default());
    let (stop_tx, stop_rx) = oneshot::channel();
    let (ready_tx, ready_rx) = mpsc::channel();
    let handler: Arc<dyn Handler> = fake.clone();
    let cfg = config.clone();
    let thread = std::thread::spawn(move || {
        run(&cfg, handler, stop_rx, move |r| {
            let _ = ready_tx.send(r);
        });
    });
    ready_rx.recv().expect("ready").expect("server started");
    Running {
        name: config.name,
        fake,
        stop: Some(stop_tx),
        thread: Some(thread),
    }
}

fn start() -> Running {
    start_with(ServerConfig::new(unique_name(), "tokenradar-test/1".into()))
}

fn connect(name: &str) -> Client {
    Client::connect(name, Duration::from_secs(2)).expect("connect")
}

fn push() -> StatuslinePush {
    StatuslinePush {
        observed_at: Timestamp::from_millis(42),
        payload: StatuslinePayload::parse(br#"{"session_id":"s1"}"#).expect("payload"),
    }
}

/// Opens the pipe as a raw file for byte-level tests.
fn raw(name: &str) -> std::fs::File {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(name)
        .expect("open pipe")
}

fn read_line(f: &mut std::fs::File) -> Vec<u8> {
    let mut out = Vec::new();
    let mut b = [0u8; 1];
    while f.read(&mut b).expect("read") == 1 {
        out.push(b[0]);
        if b[0] == b'\n' {
            break;
        }
    }
    out
}

#[test]
fn hello_push_and_status_in_one_exchange() {
    let s = start();
    let mut c = connect(&s.name);
    let replies = c
        .call(&[
            Request::new(
                1,
                Op::Hello(Hello {
                    client: "test".into(),
                    version: 1,
                }),
            ),
            Request::new(2, Op::StatuslinePush(Box::new(push()))),
            Request::new(
                3,
                Op::StatusGet(StatusQuery {
                    session_uuid: Some("s1".into()),
                }),
            ),
            Request::new(4, Op::LimitsGet),
            Request::new(5, Op::SessionsLive),
        ])
        .expect("call");
    assert_eq!(
        replies.iter().map(|r| r.id).collect::<Vec<_>>(),
        [1, 3, 4, 5]
    );
    let hello: HelloReply = replies[0].clone().into_data().expect("hello");
    assert_eq!(hello.protocol, 1);
    assert_eq!(hello.server, "tokenradar-test/1");
    let status: StatusReply = replies[1].clone().into_data().expect("status");
    assert_eq!(
        status.api_state.as_deref(),
        Some("s1"),
        "the query reached the handler"
    );
    assert_eq!(status.origin, Origin::App);
    let limits: LimitsReply = replies[2].clone().into_data().expect("limits");
    assert_eq!(limits.generated_at, Timestamp::from_millis(2));
    let sessions: SessionsReply = replies[3].clone().into_data().expect("sessions");
    assert_eq!(sessions.generated_at, Timestamp::from_millis(3));
    assert_eq!(*s.fake.pushes.lock().expect("lock"), vec![push()]);
}

#[test]
fn unknown_ops_and_newer_versions_get_an_error_and_the_connection_stays() {
    let s = start();
    let mut f = raw(&s.name);
    f.write_all(b"{\"v\":1,\"id\":7,\"op\":\"windows.list\"}\n")
        .expect("write");
    let r = Response::decode(&read_line(&mut f)).expect("reply");
    assert_eq!((r.id, r.result), (7, Err(ERR_UNSUPPORTED.to_owned())));
    f.write_all(b"{\"v\":9,\"id\":8,\"op\":\"limits.get\"}\n")
        .expect("write");
    let r = Response::decode(&read_line(&mut f)).expect("reply");
    assert_eq!((r.id, r.result), (8, Err(ERR_VERSION.to_owned())));
    f.write_all(&Request::new(9, Op::LimitsGet).encode())
        .expect("write");
    let r = Response::decode(&read_line(&mut f)).expect("reply");
    assert!(r.result.is_ok());
}

#[test]
fn a_malformed_message_closes_only_its_connection() {
    let s = start();
    let mut bad = raw(&s.name);
    bad.write_all(b"this is not json\n").expect("write");
    let mut buf = [0u8; 16];
    // EOF or a broken pipe: either way the server hung up.
    assert!(matches!(bad.read(&mut buf), Ok(0) | Err(_)));
    let mut c = connect(&s.name);
    assert_eq!(
        c.call(&[Request::new(1, Op::LimitsGet)])
            .expect("call")
            .len(),
        1
    );
}

#[test]
fn an_oversized_message_is_refused() {
    let s = start();
    let mut f = raw(&s.name);
    let big = vec![b'x'; MAX_MESSAGE_BYTES + 10];
    // The server stops reading at the limit and closes; the write may fail part way.
    let _ = f.write_all(&big);
    let mut buf = [0u8; 16];
    assert!(matches!(f.read(&mut buf), Ok(0) | Err(_)));
}

#[test]
fn a_silent_client_is_dropped_after_the_read_timeout() {
    let mut cfg = ServerConfig::new(unique_name(), "t".into());
    cfg.read_timeout = Duration::from_millis(100);
    let s = start_with(cfg);
    let mut f = raw(&s.name);
    let started = std::time::Instant::now();
    let mut buf = [0u8; 16];
    assert!(matches!(f.read(&mut buf), Ok(0) | Err(_)));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn many_clients_are_served_concurrently() {
    let s = start();
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let name = s.name.clone();
            std::thread::spawn(move || {
                let mut c = connect(&name);
                // Keep every connection open at the same time for a moment.
                std::thread::sleep(Duration::from_millis(50));
                c.call(&[Request::new(i, Op::StatusGet(StatusQuery::default()))])
                    .expect("call")[0]
                    .id
            })
        })
        .collect();
    let mut ids: Vec<u32> = threads
        .into_iter()
        .map(|t| t.join().expect("join"))
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, (0..8).collect::<Vec<_>>());
}

#[test]
fn only_the_current_user_may_open_the_pipe() {
    let s = start();
    let f = raw(&s.name);
    let sddl = dacl_sddl(&f).expect("dacl");
    let sid = current_user_sid().expect("sid");
    assert!(sddl.starts_with("D:P"), "protected DACL: {sddl}");
    assert_eq!(sddl.matches('(').count(), 1, "exactly one ACE: {sddl}");
    // The ACE's trustee may be an SDDL alias (`LA` when running as the built-in Administrator).
    let trustee = sddl
        .rsplit(';')
        .next()
        .and_then(|t| t.strip_suffix(')'))
        .expect("trustee");
    assert_eq!(canonical_sid(trustee).expect("trustee sid"), sid, "{sddl}");
    for group in [";;;WD)", ";;;AN)", ";;;BU)", ";;;AU)"] {
        assert!(
            !sddl.contains(group),
            "{group} must not have access: {sddl}"
        );
    }
}

#[test]
fn a_second_server_cannot_take_over_the_name() {
    let s = start();
    let (_tx, rx) = oneshot::channel();
    let (ready_tx, ready_rx) = mpsc::channel();
    let cfg = ServerConfig::new(s.name.clone(), "intruder".into());
    run(&cfg, Arc::new(Fake::default()), rx, move |r| {
        let _ = ready_tx.send(r);
    });
    assert!(ready_rx.recv().expect("ready").is_err());
}

#[test]
fn no_server_means_not_running() {
    assert!(matches!(
        Client::connect(&unique_name(), Duration::from_millis(50)),
        Err(ClientError::NotRunning)
    ));
}
