//! The bundle leaks nothing: no token, no fixture content, no title, project or path.

use std::fmt::Write as _;
use std::path::Path;

use super::*;
use crate::domain::{AccountSpec, Provider};
use crate::indexer::{Indexer, IndexerConfig};
use crate::store::Store;

const TOKEN: &str = "sk-ant-oat01-SECRETSECRET_abc-123";

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("mkdir");
    for entry in std::fs::read_dir(from).expect("read_dir") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("copy");
        }
    }
}

/// A portable data directory with an indexed copy of the fixtures, a config and a log that
/// mention the user's paths and a token.
fn seeded() -> (tempfile::TempDir, Paths) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().join("Users").join("Ada Lovelace");
    let claude_dir = home.join(".claude");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/claude"),
        &claude_dir,
    );
    let paths = Paths::portable(&tmp.path().join("app"));
    std::fs::create_dir_all(paths.logs_dir()).expect("logs");
    {
        let store = Store::open(paths.db_path()).expect("store");
        let account = store
            .writer()
            .upsert_account(AccountSpec {
                provider: Provider::Claude,
                label: "Ada's Claude".into(),
                config_dir: claude_dir.to_string_lossy().into_owned(),
                plan: Some("max".into()),
                tier: None,
                environment: "native".into(),
            })
            .expect("account");
        let config = IndexerConfig {
            workers: 1,
            background: false,
            ..IndexerConfig::default()
        };
        Indexer::new(store.clone(), account, &claude_dir, config)
            .full_scan(&mut |_| {})
            .expect("scan");
        store.writer().shutdown();
    }
    let dir = claude_dir.display();
    std::fs::write(
        paths.config_path(),
        format!(
            "schema_version = 2\n[general]\nclaude_config_dir = '{dir}'\n\n[[accounts]]\n\
             provider = \"claude\"\nconfig_dir = '{dir}'\nlabel = \"Ada at work\"\n\n\
             [accounts.statusline]\ninstalled = true\n\
             previous_command = 'python {dir}\\secret-script.py'\n"
        ),
    )
    .expect("config");
    let mut log = String::new();
    for i in 0..250 {
        let _ = writeln!(log, "2026-10-27T10:00:{:02}Z INFO tick n={i}", i % 60);
    }
    let _ = write!(
        log,
        "2026-10-27T10:05:00Z WARN could not watch directory dir={dir} error=denied\n\
         2026-10-27T10:05:01Z WARN request failed auth=Bearer {TOKEN}\n\
         2026-10-27T10:05:02Z WARN echo {TOKEN}\n"
    );
    std::fs::write(paths.logs_dir().join("tokenradar.2026-10-27.log"), log).expect("log");
    std::fs::write(
        paths.logs_dir().join("tokenradar.2026-10-26.log"),
        "older line\n",
    )
    .expect("old log");
    (tmp, paths)
}

#[test]
fn bundle_holds_no_secret_content_or_path() {
    let (_tmp, paths) = seeded();
    let input = Input {
        producer: "test",
        paths: &paths,
        live: None,
    };
    let out = paths.data_dir().join("bundle.zip");
    write(&out, &input).expect("write");
    let bytes = std::fs::read(&out).expect("read");
    let text = String::from_utf8_lossy(&bytes);
    for needle in [
        "sk-ant-oat",
        "SECRETSECRET",
        "synthetic prompt",
        "synthetic reply",
        "generated title",
        "Alpha",
        "Beta",
        "Gamma",
        "Lovelace",
        "Ada",
        "secret-script",
        "Users",
    ] {
        assert!(!text.contains(needle), "bundle contains {needle:?}");
    }
    assert!(text.contains("sk-ant-***"));
    assert!(text.contains("path#"));
    assert!(text.contains("<hidden, "));
}

#[test]
fn report_counts_the_indexed_fixtures() {
    let (_tmp, paths) = seeded();
    let input = Input {
        producer: "test",
        paths: &paths,
        live: Some(serde_json::json!({ "api_state": "off" })),
    };
    let files = bundle(&input, Timestamp::from_millis(1_800_000_000_000));
    let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["README.txt", "report.json", "config.toml", "log.txt"]
    );
    let report: Value = serde_json::from_slice(&files[1].1).expect("json");
    assert_eq!(report["schema"], REPORT_SCHEMA);
    assert_eq!(report["live"]["api_state"], "off");
    let account = &report["database"]["accounts"][0];
    assert_eq!(account["plan"], "max");
    assert_eq!(account["stats"]["sessions"], 3);
    assert!(account["stats"]["events"].as_u64().unwrap_or(0) > 0);
    assert_eq!(report["database"]["tables"]["accounts"], 1);
    assert_eq!(report["files"]["logs"]["files"], 2);
    assert_eq!(report["errors"], serde_json::json!([]));
}

#[test]
fn log_tail_keeps_the_newest_lines_across_files() {
    let (_tmp, paths) = seeded();
    let tail = log_tail(&paths.logs_dir(), LOG_LINES);
    let lines: Vec<&str> = tail.lines().collect();
    assert_eq!(lines.len(), LOG_LINES);
    assert!(lines[LOG_LINES - 1].contains("echo sk-ant-***"));
    // 253 lines in the newest file: the older file is not needed.
    assert!(!tail.contains("older line"));
    let short = log_tail(&paths.logs_dir(), 300);
    assert!(short.starts_with("older line\n"));
}

#[test]
fn default_name_is_utc_stamped() {
    assert_eq!(
        default_file_name(Timestamp::from_millis(1_793_136_838_000)),
        "tokenradar-doctor-20261027-213358.zip"
    );
}
