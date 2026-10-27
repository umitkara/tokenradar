//! End-to-end indexer tests on the synthetic transcripts in `testdata/claude`.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::*;
use crate::domain::{AccountSpec, Provider};

const ALPHA: &str = "0a1f0000-0000-4000-8000-00000000a001";
const BETA: &str = "0b2e0000-0000-4000-8000-00000000b001";
const GAMMA: &str = "0c3d0000-0000-4000-8000-00000000c001";

struct Env {
    _tmp: tempfile::TempDir,
    config_dir: PathBuf,
    store: Store,
    account: AccountId,
}

impl Env {
    /// A fresh copy of the fixtures and an empty database.
    fn new() -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let config_dir = tmp.path().join("claude");
        copy_dir(&fixtures(), &config_dir);
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

    fn indexer(&self, workers: usize) -> Indexer {
        let config = IndexerConfig {
            workers,
            // Small batches exercise flushing mid-file.
            batch_events: 2,
            background: false,
            ..IndexerConfig::default()
        };
        Indexer::new(self.store.clone(), self.account, &self.config_dir, config)
    }

    fn scan(&self) -> (IndexReport, Vec<IndexEvent>) {
        let mut events = Vec::new();
        let report = self
            .indexer(2)
            .full_scan(&mut |e| events.push(e))
            .expect("scan");
        (report, events)
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.config_dir.join("projects").join(rel)
    }

    fn alpha_main(&self) -> PathBuf {
        self.path(&format!("C--Work-Alpha/{ALPHA}.jsonl"))
    }

    fn summary(&self) -> Summary {
        summarize(&self.store)
    }
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/claude")
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

fn append(path: &Path, text: &str) {
    let mut f = OpenOptions::new().append(true).open(path).expect("open");
    f.write_all(text.as_bytes()).expect("append");
}

// --- Database summary -----------------------------------------------------------------------

#[derive(Debug, Serialize, PartialEq)]
struct Summary {
    sessions: Vec<SessionRow>,
    models: Vec<(String, String)>,
    total: Totals,
}

#[derive(Debug, Serialize, PartialEq)]
struct SessionRow {
    uuid: String,
    kind: String,
    parent: Option<String>,
    root: Option<String>,
    project: Option<(String, String)>,
    title: Option<(String, String)>,
    git_branch: Option<String>,
    cc_version: Option<String>,
    entrypoint: Option<String>,
    first_ts: Option<i64>,
    last_ts: Option<i64>,
    totals: Totals,
}

#[derive(Debug, Serialize, PartialEq, Default)]
struct Totals {
    events: i64,
    input: i64,
    output: i64,
    cache_read: i64,
    cache_w5m: i64,
    cache_w1h: i64,
    thinking: i64,
    cost_micro_usd: i64,
}

fn summarize(store: &Store) -> Summary {
    store
        .read(|c| {
            let totals_sql = "COUNT(e.id), COALESCE(SUM(e.input),0), COALESCE(SUM(e.output),0),
                 COALESCE(SUM(e.cache_read),0), COALESCE(SUM(e.cache_w5m),0),
                 COALESCE(SUM(e.cache_w1h),0), COALESCE(SUM(e.thinking),0),
                 COALESCE(SUM(e.cost_micro_usd),0)";
            let totals = |r: &rusqlite::Row<'_>, at: usize| -> rusqlite::Result<Totals> {
                Ok(Totals {
                    events: r.get(at)?,
                    input: r.get(at + 1)?,
                    output: r.get(at + 2)?,
                    cache_read: r.get(at + 3)?,
                    cache_w5m: r.get(at + 4)?,
                    cache_w1h: r.get(at + 5)?,
                    thinking: r.get(at + 6)?,
                    cost_micro_usd: r.get(at + 7)?,
                })
            };
            let mut stmt = c.prepare(&format!(
                "SELECT s.uuid, s.kind, p.uuid, r.uuid, pr.path_norm, pr.display_name, s.title,
                        s.title_source, s.git_branch, s.cc_version, s.entrypoint, s.first_ts,
                        s.last_ts, {totals_sql}
                 FROM sessions s
                 LEFT JOIN sessions p ON p.id = s.parent_id
                 LEFT JOIN sessions r ON r.id = s.root_id
                 LEFT JOIN projects pr ON pr.id = s.project_id
                 LEFT JOIN usage_events e ON e.session_id = s.id
                 GROUP BY s.id ORDER BY s.uuid"
            ))?;
            let sessions = stmt
                .query_map([], |r| {
                    let pair = |a: Option<String>, b: Option<String>| a.zip(b);
                    Ok(SessionRow {
                        uuid: r.get(0)?,
                        kind: r.get(1)?,
                        parent: r.get(2)?,
                        root: r.get(3)?,
                        project: pair(r.get(4)?, r.get(5)?),
                        title: pair(r.get(6)?, r.get(7)?),
                        git_branch: r.get(8)?,
                        cc_version: r.get(9)?,
                        entrypoint: r.get(10)?,
                        first_ts: r.get(11)?,
                        last_ts: r.get(12)?,
                        totals: totals(r, 13)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let models = c
                .prepare("SELECT name, family FROM models ORDER BY name")?
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let total = c.query_row(
                &format!("SELECT {totals_sql} FROM usage_events e"),
                [],
                |r| totals(r, 0),
            )?;
            Ok(Summary {
                sessions,
                models,
                total,
            })
        })
        .expect("summary")
}

fn session<'a>(s: &'a Summary, uuid: &str) -> &'a SessionRow {
    s.sessions
        .iter()
        .find(|r| r.uuid == uuid)
        .unwrap_or_else(|| panic!("no session {uuid}"))
}

// --- Tests ------------------------------------------------------------------------------------

#[test]
fn full_scan_indexes_the_fixture_set() {
    let env = Env::new();
    let (report, events) = env.scan();
    let summary = env.summary();
    insta::assert_yaml_snapshot!("fixture_summary", summary);

    // Main session: partial and duplicate lines of msg_a1 count once at full size; the
    // half-written msg_a4 is not indexed yet; the advisor iteration is its own event.
    let alpha = session(&summary, ALPHA);
    assert_eq!(alpha.totals.events, 4); // a1, a2, a3, a3's advisor
    assert_eq!(alpha.totals.output, 50 + 70 + 188 + 400);
    assert_eq!(
        alpha.title,
        Some(("Alpha work".into(), "custom".into())),
        "a later generated title must not replace the user's"
    );
    assert_eq!(
        alpha.project,
        Some((r"C:\work\alpha".into(), "Alpha".into()))
    );
    // msg_a2 has no cache breakdown: all 900 cache-write tokens count as 5-minute writes.
    assert_eq!(alpha.totals.cache_w5m, 900);

    // The subagent's copy of msg_a1 has equal output; the main session keeps it.
    let sub = session(&summary, "agent-a1b2c3");
    assert_eq!(sub.kind, "subagent");
    assert_eq!(sub.parent.as_deref(), Some(ALPHA));
    assert_eq!(sub.totals.events, 2);

    // No cwd anywhere: the project falls back to the directory name.
    let beta = session(&summary, BETA);
    assert_eq!(
        beta.project,
        Some(("unresolved:C--Work-Beta".into(), "C--Work-Beta".into()))
    );
    assert_eq!(beta.title, Some(("beta-reviewer".into(), "agent".into())));

    // A subagent whose main file is missing gets a placeholder parent.
    let gamma = session(&summary, GAMMA);
    assert_eq!((gamma.kind.as_str(), gamma.totals.events), ("main", 0));
    assert_eq!(
        session(&summary, "agent-ffee01").project,
        Some((r"C:\work\gamma".into(), "Gamma".into()))
    );

    // `tool-results` is never read; bad lines are counted, not fatal.
    assert!(!summary.models.iter().any(|(m, _)| m == "msg_tool"));
    assert_eq!(summary.total.output, 708 + 70 + 15 + 20);
    assert_eq!(report.files_seen, 4);
    assert_eq!(report.files_failed, 0);
    assert_eq!(report.parse.malformed, 1);
    assert_eq!(report.parse.advisor_records, 1);
    assert_eq!(report.parse.prefilter_misses, 0);

    assert!(events.contains(&IndexEvent::Milestone(IndexMilestone::RecentReady)));
    assert!(matches!(
        events.last(),
        Some(IndexEvent::Progress(IndexProgress {
            phase: IndexPhase::Done,
            files_done: 4,
            files_total: 4,
            ..
        }))
    ));
}

#[test]
fn worker_count_and_batch_size_do_not_change_the_result() {
    let reference = {
        let env = Env::new();
        env.indexer(1).full_scan(&mut |_| {}).expect("scan");
        env.summary()
    };
    for workers in [2, 4] {
        let env = Env::new();
        env.indexer(workers).full_scan(&mut |_| {}).expect("scan");
        assert_eq!(env.summary(), reference, "workers = {workers}");
    }
}

#[test]
fn rescanning_unchanged_files_reads_nothing() {
    let env = Env::new();
    env.scan();
    let before = env.summary();
    let (report, _) = env.scan();
    assert_eq!(report.files_unchanged, report.files_seen);
    assert_eq!(report.bytes_read, 0);
    assert_eq!(report.events, UpsertStats::default());
    assert_eq!(env.summary(), before);
}

#[test]
fn appended_lines_are_read_incrementally() {
    let env = Env::new();
    env.scan();
    let path = env.alpha_main();
    let size_before = fs::metadata(&path).expect("stat").len();

    // Claude Code finishes the half-written line and appends a new response.
    let new_line = fs::read_to_string(&path)
        .expect("read")
        .lines()
        .last()
        .expect("last line")
        .replace("msg_a4", "msg_a5")
        .replace("req_a4", "req_a5");
    append(&path, &format!("\n{new_line}\n"));

    let mut indexer = env.indexer(1);
    let report = indexer
        .index_files(std::slice::from_ref(&path), &mut |_| {})
        .expect("index");
    assert_eq!(report.files_seen, 1);
    assert_eq!(report.events.inserted, 2); // msg_a4 and msg_a5 only
    assert_eq!(report.parse.usage_records, 2);
    let alpha = session(&env.summary(), ALPHA).totals.output;
    assert_eq!(alpha, 708 + 10 + 10);

    // The finished last line was re-read from its start, nothing before it.
    let half_line = fs::read_to_string(&path)
        .expect("read")
        .lines()
        .nth(13)
        .map(str::len);
    assert_eq!(
        report.bytes_read,
        fs::metadata(&path).expect("stat").len() - size_before + half_line.expect("line 14") as u64
    );
    assert_eq!(
        session(&env.summary(), ALPHA).git_branch.as_deref(),
        Some("feature-x")
    );
}

#[test]
fn truncated_file_is_reread_without_double_counting() {
    let env = Env::new();
    env.scan();
    let before = env.summary();
    let sub = env.path(&format!(
        "C--Work-Alpha/{ALPHA}/subagents/agent-a1b2c3.jsonl"
    ));
    let text = fs::read_to_string(&sub).expect("read");
    let first_line = text.lines().next().expect("line");
    fs::write(&sub, format!("{first_line}\n")).expect("truncate");

    let (report, _) = env.scan();
    assert_eq!(report.files_truncated, 1);
    assert_eq!(report.events.inserted, 0);
    // Data from the lost lines stays: the database keeps what Claude Code removed.
    assert_eq!(env.summary(), before);
}

#[test]
fn deleted_files_are_marked_and_their_data_kept() {
    let env = Env::new();
    env.scan();
    let before = env.summary();
    let beta = env.path(&format!("C--Work-Beta/{BETA}.jsonl"));
    fs::remove_file(&beta).expect("delete");

    let (report, _) = env.scan();
    assert_eq!(report.files_deleted, 1);
    assert_eq!(env.summary(), before);
    let files = env.store.source_files(env.account).expect("files");
    let row = &files[&path_key(&beta)];
    assert!(row.deleted_at.is_some());

    // A second scan does not mark it again; restoring it re-reads it idempotently.
    assert_eq!(env.scan().0.files_deleted, 0);
    copy_dir(
        &fixtures().join("projects/C--Work-Beta"),
        &env.path("C--Work-Beta"),
    );
    let (report, _) = env.scan();
    assert_eq!((report.files_indexed, report.events.inserted), (1, 0));
    let files = env.store.source_files(env.account).expect("files");
    assert!(files[&path_key(&beta)].deleted_at.is_none());
}

#[test]
fn subagent_indexed_before_its_main_session_is_linked_later() {
    let env = Env::new();
    let sub = env.path(&format!(
        "C--Work-Alpha/{ALPHA}/subagents/agent-a1b2c3.jsonl"
    ));
    let mut indexer = env.indexer(1);
    indexer.index_files(&[sub], &mut |_| {}).expect("subagent");

    let early = env.summary();
    let parent = session(&early, ALPHA);
    assert_eq!(
        (parent.kind.as_str(), parent.title.as_ref()),
        ("main", None)
    );
    // Until the main file arrives, the subagent's copy of msg_a1 is all there is.
    assert_eq!(session(&early, "agent-a1b2c3").totals.events, 3);

    // A new indexer (as after a restart) finds the placeholder by uuid and fills it in.
    env.indexer(1)
        .index_files(&[env.alpha_main()], &mut |_| {})
        .expect("main");
    let late = env.summary();
    assert_eq!(
        session(&late, ALPHA).title,
        Some(("Alpha work".into(), "custom".into()))
    );
    assert_eq!(session(&late, "agent-a1b2c3").totals.events, 2);
    assert_eq!(session(&late, ALPHA).totals.events, 4);
}

#[test]
fn paths_outside_the_layout_are_ignored() {
    let env = Env::new();
    let tool = env.path(&format!("C--Work-Alpha/{ALPHA}/tool-results/toolu_1.jsonl"));
    let report = env
        .indexer(1)
        .index_files(&[tool, env.config_dir.join("history.jsonl")], &mut |_| {})
        .expect("index");
    assert_eq!(report.files_seen, 0);
    assert_eq!(env.summary().total, Totals::default());
}

#[test]
fn spaced_json_is_indexed_and_reported_as_drift() {
    let env = Env::new();
    let path = env.path("C--Work-Delta/0d4e0000-0000-4000-8000-00000000d001.jsonl");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    let line = r#"{"type": "assistant", "timestamp": "2026-10-20T11:00:00.000Z", "cwd": "D:\\delta", "requestId": "req_d1", "message": {"id": "msg_d1", "model": "claude-opus-5-5", "usage": {"input_tokens": 1, "output_tokens": 9}}}"#;
    fs::write(&path, format!("{line}\n")).expect("write");

    let report = env
        .indexer(1)
        .index_files(&[path], &mut |_| {})
        .expect("index");
    assert_eq!(report.parse.prefilter_misses, 1);
    assert_eq!(report.events.inserted, 1);
}

#[test]
fn retention_runs_once_per_interval() {
    let env = Env::new();
    env.scan();
    let w = env.store.writer();
    let day = 24 * 60 * 60 * 1000;
    let now = Timestamp::parse_iso("2026-12-31T00:00:00Z").expect("ts");
    // All fixture events are from 2026-10-20: a 30-day retention prunes every raw event.
    let first = w.prune_if_due(now, 30).expect("prune").expect("due");
    assert_eq!(first.deleted, 9);
    assert_eq!(
        w.prune_if_due(now.saturating_add_millis(day / 2), 30)
            .expect("prune"),
        None
    );
    assert!(
        w.prune_if_due(now.saturating_add_millis(day), 30)
            .expect("prune")
            .is_some()
    );
}
