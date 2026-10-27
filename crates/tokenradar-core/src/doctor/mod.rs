//! The diagnostics bundle (`tokenradar doctor`, "Create diagnostics bundle" in the settings).
//!
//! A zip the user creates and shares themselves (ADR-013: no telemetry). It holds versions,
//! counts, sizes, the settings and the last log lines. It never holds a token, conversation
//! content, a session title, a project name or a readable path: paths become `path#<hash>`
//! ([`redact::path_tag`]) and every log line goes through [`redact::line`].

pub mod redact;
mod sys;
mod zip;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};
use toml_edit::{DocumentMut, Item, value};

use crate::config::AppConfig;
use crate::domain::Timestamp;
use crate::ipc::StatusReply;
use crate::limits::PanelSnapshot;
use crate::paths::Paths;
use crate::providers::claude;
use crate::store::ReadOnlyStore;

pub use sys::SystemInfo;

/// Log lines in a bundle (the newest ones).
pub const LOG_LINES: usize = 200;
/// Version of `report.json`.
pub const REPORT_SCHEMA: u32 = 1;

/// What the caller adds to the bundle.
#[derive(Debug, Clone)]
pub struct Input<'a> {
    /// Who made the bundle, e.g. `tokenradar-cli/0.0.1`.
    pub producer: &'a str,
    pub paths: &'a Paths,
    /// State of the running app: polling, index, statusline. Counts, times and enum names
    /// only; callers never put names, titles or paths in here.
    pub live: Option<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum DoctorError {
    #[error("could not write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// The bundle's files: `report.json`, `config.toml`, `log.txt` and `README.txt`.
pub fn bundle(input: &Input<'_>, now: Timestamp) -> Vec<(String, Vec<u8>)> {
    let paths = input.paths;
    let config_text = std::fs::read_to_string(paths.config_path()).ok();
    let config = config_text
        .as_deref()
        .and_then(|t| AppConfig::parse(t).ok())
        .unwrap_or_default();
    let mut errors = Vec::new();
    let report = json!({
        "schema": REPORT_SCHEMA,
        "generated_at": now.to_string(),
        "producer": input.producer,
        "system": sys::info(),
        "files": files(paths),
        "claude_dirs": claude_dirs(&config),
        "database": database(paths, &mut errors),
        "live": input.live,
        "errors": errors,
    });
    let report = serde_json::to_vec_pretty(&report).unwrap_or_default();
    let config =
        config_text.map_or_else(|| "# no config.toml\n".to_owned(), |t| sanitize_config(&t));
    vec![
        ("README.txt".to_owned(), README.as_bytes().to_vec()),
        ("report.json".to_owned(), report),
        ("config.toml".to_owned(), config.into_bytes()),
        (
            "log.txt".to_owned(),
            log_tail(&paths.logs_dir(), LOG_LINES).into_bytes(),
        ),
    ]
}

/// Writes the bundle to `out` (atomically) and returns it.
pub fn write(out: &Path, input: &Input<'_>) -> Result<PathBuf, DoctorError> {
    let now = Timestamp::now();
    let files = bundle(input, now);
    let civil = jiff::Timestamp::from_millisecond(now.as_millis())
        .map(|t| t.to_zoned(jiff::tz::TimeZone::UTC).datetime())
        .unwrap_or_default();
    let mut buf = Vec::new();
    let io = |source| DoctorError::Write {
        path: out.to_path_buf(),
        source,
    };
    zip::write(&mut buf, &files, zip::dos_time(civil)).map_err(io)?;
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    crate::fsutil::atomic_write(out, &buf).map_err(io)?;
    Ok(out.to_path_buf())
}

/// The running app's state as the CLI sees it over IPC. Limits without names; no session.
pub fn live_from_status(s: &StatusReply) -> Value {
    let limits: Vec<Value> = s
        .limits
        .iter()
        .map(|l| {
            json!({
                "kind": l.kind,
                "scope": l.scope,
                "percent": l.percent,
                "source": l.source,
                "freshness": l.freshness,
                "observed_at": l.observed_at,
            })
        })
        .collect();
    json!({
        "origin": "ipc",
        "api_state": s.api_state,
        "statusline_last_seen": s.statusline_last_seen,
        "live_sessions": s.live_sessions.len(),
        "limits": limits,
    })
}

/// The app's own state, richer than [`live_from_status`]: poll counters, index progress and
/// schema drift. Session names, titles and projects are left out.
pub fn live_from_snapshot(s: &PanelSnapshot) -> Value {
    let limits: Vec<Value> = s
        .limits
        .iter()
        .map(|l| {
            json!({
                "kind": l.kind,
                "scope": l.scope,
                "percent": l.percent,
                "source": l.source,
                "freshness": l.freshness,
                "observed_at": l.observed_at,
                "projection": l.projection,
            })
        })
        .collect();
    json!({
        "origin": "app",
        "api": s.api,
        "statusline_last_seen": s.statusline.last_seen,
        "index": s.index_state,
        "live_sessions": s.live_sessions.len(),
        "limits": limits,
    })
}

/// `tokenradar-doctor-20261027-213358.zip`.
pub fn default_file_name(now: Timestamp) -> String {
    let stamp = jiff::Timestamp::from_millisecond(now.as_millis()).map_or_else(
        |_| "unknown".to_owned(),
        |t| t.strftime("%Y%m%d-%H%M%S").to_string(),
    );
    format!("tokenradar-doctor-{stamp}.zip")
}

const README: &str = "\
TokenRadar diagnostics bundle

You created this file yourself; TokenRadar never sends it anywhere. Read it before sharing.

report.json  versions, file sizes, row counts, data-source state
config.toml  your settings; paths and previous statusline commands are hidden
log.txt      the last log lines; tokens are masked, paths are hidden

Paths appear as path#<hash>: the same directory always gets the same tag, but the tag does
not say where it is. No token, conversation content, session title or project name is
included.
";

#[derive(Serialize)]
struct DirInfo {
    files: u64,
    bytes: u64,
}

fn dir_info(dir: &Path, prefix: &str) -> DirInfo {
    let mut info = DirInfo { files: 0, bytes: 0 };
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy().starts_with(prefix)
                && let Ok(m) = e.metadata()
                && m.is_file()
            {
                info.files += 1;
                info.bytes += m.len();
            }
        }
    }
    info
}

fn file_size(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

fn files(paths: &Paths) -> Value {
    let db = paths.db_path();
    let mut wal = db.clone().into_os_string();
    wal.push("-wal");
    json!({
        "data_dir": redact::path_tag(&paths.data_dir().to_string_lossy()),
        "portable": paths.is_portable(),
        "config_exists": paths.config_path().is_file(),
        "db_bytes": file_size(&db),
        "wal_bytes": file_size(Path::new(&wal)),
        "logs": dir_info(&paths.logs_dir(), "tokenradar."),
        "spool": dir_info(&paths.spool_dir(), "statusline-"),
    })
}

fn claude_dirs(config: &AppConfig) -> Value {
    let chosen = config.general.claude_config_dir.as_deref();
    let dirs: Vec<Value> = claude::discover_configured(chosen)
        .iter()
        .enumerate()
        .map(|(i, a)| {
            json!({
                "dir": redact::path_tag(&a.config_dir.to_string_lossy()),
                "primary": i == 0,
                "chosen": chosen.is_some_and(|c| crate::statusline::same_dir(c, &a.config_dir)),
                "has_projects": a.projects_dir().is_dir(),
                "has_credentials": a.config_dir.join(".credentials.json").is_file(),
                "has_settings": a.config_dir.join("settings.json").is_file(),
            })
        })
        .collect();
    json!({
        "chosen": chosen.map(|c| redact::path_tag(&c.to_string_lossy())),
        "chosen_exists": chosen.map(Path::is_dir),
        "env_set": std::env::var_os("CLAUDE_CONFIG_DIR").is_some(),
        "found": dirs,
    })
}

fn database(paths: &Paths, errors: &mut Vec<String>) -> Value {
    let store = match ReadOnlyStore::open(paths.db_path()) {
        Ok(Some(store)) => store,
        Ok(None) => return Value::Null,
        Err(e) => {
            errors.push(format!("database: {}", redact::line(&e.to_string())));
            return Value::Null;
        }
    };
    let mut note = |what: &str, e: &dyn std::fmt::Display| {
        errors.push(format!("{what}: {}", redact::line(&e.to_string())));
    };
    let tables: BTreeMap<String, u64> = store
        .table_counts()
        .map_err(|e| note("table counts", &e))
        .unwrap_or_default()
        .into_iter()
        .collect();
    let accounts: Vec<Value> = store
        .accounts()
        .map_err(|e| note("accounts", &e))
        .unwrap_or_default()
        .into_iter()
        .map(|a| {
            let stats = store
                .account_stats(a.id)
                .map_err(|e| note("stats", &e))
                .ok();
            json!({
                "id": a.id,
                "provider": a.spec.provider.as_str(),
                "dir": redact::path_tag(&a.spec.config_dir),
                "environment": a.spec.environment,
                "plan": a.spec.plan,
                "tier": a.spec.tier,
                "stats": stats,
            })
        })
        .collect();
    json!({
        "schema_version": store.schema_version().ok(),
        "tables": tables,
        "accounts": accounts,
        "muted_until": store.kv_get(crate::notifier::KV_MUTED_UNTIL).ok().flatten(),
    })
}

/// `config.toml` with paths tagged and the previous statusline commands hidden (they may name
/// the user's scripts and folders). Account labels are the user's words, so they go too.
pub fn sanitize_config(text: &str) -> String {
    let Ok(mut doc) = text.parse::<DocumentMut>() else {
        return "# config.toml is not valid TOML; not included\n".to_owned();
    };
    if let Some(general) = doc.get_mut("general").and_then(Item::as_table_mut)
        && let Some(dir) = general.get("claude_config_dir").and_then(Item::as_str)
    {
        let tag = redact::path_tag(dir);
        general["claude_config_dir"] = value(tag);
    }
    if let Some(accounts) = doc
        .get_mut("accounts")
        .and_then(Item::as_array_of_tables_mut)
    {
        for account in accounts.iter_mut() {
            if let Some(dir) = account.get("config_dir").and_then(Item::as_str) {
                let tag = redact::path_tag(dir);
                account["config_dir"] = value(tag);
            }
            if account.contains_key("label") {
                account["label"] = value("<hidden>");
            }
            if let Some(sl) = account.get_mut("statusline").and_then(Item::as_table_mut) {
                for key in ["previous", "previous_command"] {
                    if let Some(len) = sl.get(key).and_then(Item::as_str).map(str::len) {
                        sl[key] = value(format!("<hidden, {len} characters>"));
                    }
                }
            }
        }
    }
    // Anything this build does not know about still gets its paths and secrets masked.
    doc.to_string()
        .lines()
        .map(redact::line)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// The last `max` lines of the app's log files, newest file last, each redacted.
pub fn log_tail(logs_dir: &Path, max: usize) -> String {
    let mut files: Vec<PathBuf> = std::fs::read_dir(logs_dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with("tokenradar."))
                })
                .collect()
        })
        .unwrap_or_default();
    // `tokenradar.YYYY-MM-DD.log`: the name sorts by day.
    files.sort();
    let mut lines: Vec<String> = Vec::new();
    for file in files.iter().rev() {
        let Ok(bytes) = std::fs::read(file) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let mut chunk: Vec<String> = text.lines().map(str::to_owned).collect();
        let take = (max - lines.len()).min(chunk.len());
        let mut tail = chunk.split_off(chunk.len() - take);
        tail.append(&mut lines);
        lines = tail;
        if lines.len() >= max {
            break;
        }
    }
    if lines.is_empty() {
        return "# no log lines\n".to_owned();
    }
    let mut out: String = lines
        .iter()
        .map(|l| redact::line(l))
        .collect::<Vec<_>>()
        .join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests;
