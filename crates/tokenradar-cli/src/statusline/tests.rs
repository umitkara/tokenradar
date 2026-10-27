use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use tokenradar_core::ipc::{Handler, LimitsReply, SessionInfo, SessionsReply};
use tokenradar_i18n::{Domain, Locale};

use super::*;

fn unique_pipe() -> String {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!(r"\\.\pipe\tokenradar-cli-test-{}-{n}", std::process::id())
}

/// A payload whose 5-hour window resets in two hours.
fn input(percent: f64) -> Vec<u8> {
    let resets = Timestamp::now().as_millis() / 1000 + 2 * 3600;
    format!(
        r#"{{"session_id":"s1","rate_limits":{{"five_hour":{{"used_percentage":{percent},"resets_at":{resets}}}}}}}"#
    )
    .into_bytes()
}

fn deps<'a>(l10n: &'a Localizer, pipe: &str, spool: &Path) -> Deps<'a> {
    Deps {
        l10n,
        pipe_name: Some(pipe.to_owned()),
        spool_dir: Some(spool.to_path_buf()),
        chain: None,
        style: Style {
            color: false,
            width: None,
        },
    }
}

fn spooled_lines(dir: &Path) -> usize {
    std::fs::read_dir(dir).map_or(0, |rd| {
        rd.flatten()
            .map(|e| {
                std::fs::read_to_string(e.path())
                    .expect("read")
                    .lines()
                    .count()
            })
            .sum()
    })
}

#[test]
fn without_the_app_the_reading_is_spooled_and_the_line_uses_stdin() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let l10n = Localizer::new(Locale::En, &[Domain::Cli]);
    let out = respond(Some(&input(42.0)), &deps(&l10n, &unique_pipe(), tmp.path()));
    assert!(out.starts_with("◑ 5h 42% · 1h59m"), "{out}");
    assert_eq!(spooled_lines(tmp.path()), 1);
}

#[test]
fn garbage_on_stdin_still_prints_a_line_and_spools_nothing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let l10n = Localizer::new(Locale::Tr, &[Domain::Cli]);
    let d = deps(&l10n, &unique_pipe(), tmp.path());
    assert_eq!(respond(Some(b"not json"), &d), "henüz limit verisi yok");
    assert_eq!(respond(None, &d), "henüz limit verisi yok");
    assert_eq!(spooled_lines(tmp.path()), 0);
}

struct App {
    pushes: std::sync::Mutex<usize>,
}

impl Handler for App {
    fn push(&self, _push: StatuslinePush) {
        *self.pushes.lock().expect("lock") += 1;
    }
    fn status(&self, query: &StatusQuery) -> StatusReply {
        let mut r = StatusReply::empty(Timestamp::now());
        r.session = Some(SessionInfo {
            session_uuid: query.session_uuid.clone(),
            name: None,
            title: None,
            project: None,
            status: None,
            share_percent: Some(9.0),
            cost_usd: 0.0,
            tokens_per_min: 0.0,
        });
        r
    }
    fn limits(&self) -> LimitsReply {
        StatusReply::empty(Timestamp::now()).limits_reply()
    }
    fn sessions(&self) -> SessionsReply {
        StatusReply::empty(Timestamp::now()).sessions_reply()
    }
}

#[test]
fn with_the_app_the_reading_is_pushed_and_the_line_is_enriched() {
    use tokenradar_core::ipc::server::{ServerConfig, run, stop_channel};

    let pipe = unique_pipe();
    let app = Arc::new(App {
        pushes: std::sync::Mutex::new(0),
    });
    let (stop, stop_rx) = stop_channel();
    let (ready_tx, ready_rx) = mpsc::channel();
    let handler: Arc<dyn Handler> = app.clone();
    let config = ServerConfig::new(pipe.clone(), "test".into());
    let server = std::thread::spawn(move || {
        run(&config, handler, stop_rx, move |r| {
            let _ = ready_tx.send(r.is_ok());
        });
    });
    assert!(ready_rx.recv().expect("ready"));

    let tmp = tempfile::tempdir().expect("tempdir");
    let l10n = Localizer::new(Locale::En, &[Domain::Cli]);
    let out = respond(Some(&input(42.0)), &deps(&l10n, &pipe, tmp.path()));
    assert!(out.ends_with("│ ≈9% this session"), "{out}");
    assert_eq!(*app.pushes.lock().expect("lock"), 1);
    assert_eq!(
        spooled_lines(tmp.path()),
        0,
        "nothing spooled while the app runs"
    );
    drop(stop);
    server.join().expect("server");
}

fn chained_deps<'a>(l10n: &'a Localizer, spool: &Path, cmd: &str, pos: ChainPosition) -> Deps<'a> {
    let mut d = deps(l10n, &unique_pipe(), spool);
    d.chain = Some((cmd.to_owned(), pos));
    d
}

#[test]
fn the_previous_command_goes_before_or_after_our_line() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let l10n = Localizer::new(Locale::En, &[Domain::Cli]);
    let before = respond(
        Some(&input(10.0)),
        &chained_deps(&l10n, tmp.path(), "echo previous", ChainPosition::Before),
    );
    let lines: Vec<&str> = before.lines().collect();
    assert_eq!(lines.len(), 2, "{before}");
    assert_eq!(lines[0], "previous");
    assert!(lines[1].starts_with("○ 5h 10%"), "{before}");
    let after = respond(
        Some(&input(10.0)),
        &chained_deps(&l10n, tmp.path(), "echo previous", ChainPosition::After),
    );
    assert_eq!(after.lines().last(), Some("previous"), "{after}");
}

#[test]
fn the_previous_command_reads_the_same_stdin() {
    if !matches!(Shell::detect(), Shell::GitBash(_) | Shell::Sh) {
        return; // the command below is POSIX shell
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    let l10n = Localizer::new(Locale::En, &[Domain::Cli]);
    let stdin = input(10.0);
    let out = respond(
        Some(&stdin),
        &chained_deps(&l10n, tmp.path(), "wc -c", ChainPosition::Before),
    );
    let count: usize = out
        .lines()
        .next()
        .expect("line")
        .trim()
        .parse()
        .expect("count");
    assert_eq!(count, stdin.len());
}

#[test]
fn a_slow_previous_command_is_dropped_after_the_timeout() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let l10n = Localizer::new(Locale::En, &[Domain::Cli]);
    let started = Instant::now();
    let out = respond(
        Some(&input(10.0)),
        &chained_deps(&l10n, tmp.path(), "sleep 5", ChainPosition::Before),
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(out.lines().count(), 1, "only our line: {out}");
    assert!(out.starts_with("○ 5h 10%"), "{out}");
}
