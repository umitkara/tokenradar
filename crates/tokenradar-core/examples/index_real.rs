//! Step 04 measurement: index a real Claude Code config directory into a scratch database.
//!
//! ```text
//! cargo run --release -p tokenradar-core --example index_real -- <config_dir> --db <path>
//!     [--keep] [--workers N] [--foreground] [--daily-since YYYYMMDD]
//! ```
//!
//! - Deletes the database first unless `--keep` (so the first pass is a first index), runs a
//!   full scan, then a second scan of the unchanged set.
//! - Prints timings, `RecentReady`, peak memory, database size and counters, plus an xxh3
//!   fingerprint of the stored totals so runs can be compared without revealing them.
//! - Event counts and token totals are personal usage data and never go into a commit. Totals
//!   are printed only with `--daily-since` (UTC days, for comparing with `ccusage daily`).
//!
//! Read-only on the transcripts.

// Win32 FFI for memory counters; same helper as `parse_real.rs`.
#![allow(unsafe_code)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use tokenradar_core::domain::{AccountSpec, Provider};
use tokenradar_core::indexer::{IndexEvent, IndexMilestone, Indexer, IndexerConfig};
use tokenradar_core::store::Store;

struct Args {
    config_dir: PathBuf,
    db: PathBuf,
    keep: bool,
    workers: Option<usize>,
    foreground: bool,
    daily_since: Option<String>,
}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;
    let mut parser = lexopt::Parser::from_env();
    let (mut config_dir, mut db) = (None, None);
    let mut args = Args {
        config_dir: PathBuf::new(),
        db: PathBuf::new(),
        keep: false,
        workers: None,
        foreground: false,
        daily_since: None,
    };
    while let Some(arg) = parser.next()? {
        match arg {
            Long("db") => db = Some(PathBuf::from(parser.value()?)),
            Long("keep") => args.keep = true,
            Long("workers") => args.workers = Some(parser.value()?.parse()?),
            Long("foreground") => args.foreground = true,
            Long("daily-since") => args.daily_since = Some(parser.value()?.string()?),
            Value(v) if config_dir.is_none() => config_dir = Some(PathBuf::from(v)),
            _ => return Err(arg.unexpected()),
        }
    }
    args.config_dir = config_dir.ok_or("missing <config_dir>")?;
    args.db = db.ok_or("missing --db <path>")?;
    Ok(args)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    if !args.keep {
        for suffix in ["", "-wal", "-shm"] {
            let mut p = args.db.clone().into_os_string();
            p.push(suffix);
            let _ = std::fs::remove_file(p);
        }
    }

    let store = Store::open(&args.db)?;
    let account = store.writer().upsert_account(AccountSpec {
        provider: Provider::Claude,
        label: "Claude".into(),
        config_dir: args.config_dir.to_string_lossy().into_owned(),
        plan: None,
        tier: None,
        environment: "native".into(),
    })?;
    let mut config = IndexerConfig {
        background: !args.foreground,
        ..IndexerConfig::default()
    };
    if let Some(w) = args.workers {
        config.workers = w;
    }
    println!(
        "workers: {} · background: {}",
        config.workers, config.background
    );
    let mut indexer = Indexer::new(store.clone(), account, &args.config_dir, config);
    // Ten minutes back so that sessions still streaming do not change the fingerprint.
    let cutoff = tokenradar_core::domain::Timestamp::now().as_millis() - 10 * 60 * 1000;

    for pass in ["first", "second (unchanged)"] {
        let cpu_before = win::cpu_time();
        let start = Instant::now();
        let mut recent_ready = None;
        let report = indexer.full_scan(&mut |e| {
            if e == IndexEvent::Milestone(IndexMilestone::RecentReady) && recent_ready.is_none() {
                recent_ready = Some(start.elapsed());
            }
        })?;
        let elapsed = start.elapsed();
        let mem = win::memory();
        println!("\n== {pass} pass");
        println!("elapsed:        {}", ms(elapsed));
        println!("recent ready:   {}", recent_ready.map_or("-".into(), ms));
        println!(
            "cpu:            {}",
            ms(win::cpu_time().saturating_sub(cpu_before))
        );
        println!(
            "files:          seen {} · unchanged {} · indexed {} · truncated {} · deleted {} · failed {}",
            report.files_seen,
            report.files_unchanged,
            report.files_indexed,
            report.files_truncated,
            report.files_deleted,
            report.files_failed
        );
        println!(
            "bytes read:     {:.2} GiB ({:.0} MiB/s)",
            gib(report.bytes_read),
            gib(report.bytes_read) * 1024.0 / elapsed.as_secs_f64().max(1e-9)
        );
        println!("events:         {:?}", report.events);
        println!("parse:          {:?}", report.parse);
        println!(
            "peak memory:    working set {:.1} MiB · private {:.1} MiB",
            mib(mem.peak_working_set),
            mib(mem.peak_private)
        );
        println!("db size:        {:.1} MiB", mib(db_size(&args.db)));
        println!("fingerprint:    {}", fingerprint(&store, cutoff)?);
    }

    if let Some(since) = args.daily_since {
        print_daily(&store, &since)?;
    }
    Ok(())
}

/// Per-UTC-day totals in `ccusage daily --json` field names. Local only.
fn print_daily(store: &Store, since: &str) -> Result<(), Box<dyn std::error::Error>> {
    let since = format!(
        "{}-{}-{}T00:00:00Z",
        &since[..4],
        &since[4..6],
        &since[6..8]
    );
    let from = tokenradar_core::domain::Timestamp::parse_iso(&since)?;
    let rows = store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT date(ts / 1000, 'unixepoch') AS day, SUM(input), SUM(output),
                    SUM(cache_w5m + cache_w1h), SUM(cache_read), COUNT(*)
             FROM usage_events WHERE ts >= ?1 GROUP BY day ORDER BY day",
        )?;
        let rows = stmt
            .query_map([from.as_millis()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })?;
    println!("\n== daily (UTC; local only, do not commit)");
    for (day, input, output, create, read, events) in rows {
        println!(
            r#"{{"date":"{day}","inputTokens":{input},"outputTokens":{output},"cacheCreationTokens":{create},"cacheReadTokens":{read},"events":{events}}}"#
        );
    }
    Ok(())
}

/// xxh3 over every event and rollup row older than `cutoff`, keyed by stable values (message
/// key, session uuid, model name) instead of row ids, which depend on worker timing. Rows from
/// transcripts still being written after `cutoff` are left out, so runs over a live config
/// directory stay comparable. Equal fingerprints mean equal results.
fn fingerprint(store: &Store, cutoff: i64) -> Result<String, Box<dyn std::error::Error>> {
    let digest = store.read(|c| {
        let mut h = xxhash_rust::xxh3::Xxh3::new();
        for sql in [
            "SELECT hex(e.msg_key) || s.uuid || m.name || e.ts || ',' || e.input || ',' ||
                    e.output || ',' || e.cache_read || ',' || e.cache_w5m || ',' || e.cache_w1h ||
                    ',' || e.cost_micro_usd
             FROM usage_events e JOIN sessions s ON s.id = e.session_id
                                 JOIN models m ON m.id = e.model_id
             WHERE e.ts < ?1 ORDER BY e.msg_key",
            "SELECT s.uuid || m.name || r.bucket_ts || ',' || r.output || ',' || r.events
             FROM usage_5m r JOIN sessions s ON s.id = r.session_id
                             JOIN models m ON m.id = r.model_id
             WHERE r.bucket_ts + 300000 <= ?1 ORDER BY 1",
        ] {
            let mut stmt = c.prepare(sql)?;
            let mut rows = stmt.query([cutoff])?;
            while let Some(row) = rows.next()? {
                h.update(row.get_ref(0)?.as_str().unwrap_or_default().as_bytes());
                h.update(
                    b"
",
                );
            }
        }
        Ok(h.digest())
    })?;
    Ok(format!("{digest:016x}"))
}

fn db_size(db: &std::path::Path) -> u64 {
    ["", "-wal"]
        .iter()
        .filter_map(|suffix| {
            let mut p = db.to_path_buf().into_os_string();
            p.push(suffix);
            std::fs::metadata(p).ok().map(|m| m.len())
        })
        .sum()
}

fn ms(d: Duration) -> String {
    format!("{:.0} ms", d.as_secs_f64() * 1000.0)
}

#[allow(clippy::cast_precision_loss)] // display only
fn gib(b: u64) -> f64 {
    b as f64 / f64::from(1u32 << 30)
}

#[allow(clippy::cast_precision_loss)] // display only
fn mib(b: u64) -> f64 {
    b as f64 / f64::from(1u32 << 20)
}

#[cfg(windows)]
mod win {
    use std::time::Duration;

    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    pub struct Memory {
        pub peak_working_set: u64,
        /// Peak commit charge of the process: its peak private bytes.
        pub peak_private: u64,
    }

    pub fn memory() -> Memory {
        let mut counters = PROCESS_MEMORY_COUNTERS::default();
        let size = u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS>()).expect("small struct");
        counters.cb = size;
        // SAFETY: the pseudo handle is always valid; `counters` is a writable struct of `size`.
        let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &raw mut counters, size) };
        assert!(ok != 0, "GetProcessMemoryInfo failed");
        Memory {
            peak_working_set: counters.PeakWorkingSetSize as u64,
            peak_private: counters.PeakPagefileUsage as u64,
        }
    }

    /// User plus kernel CPU time of the whole process.
    pub fn cpu_time() -> Duration {
        let mut times = [FILETIME::default(); 4];
        let [creation, exit, kernel, user] = &mut times;
        // SAFETY: the pseudo handle is always valid; all four out-pointers are writable.
        let ok = unsafe { GetProcessTimes(GetCurrentProcess(), creation, exit, kernel, user) };
        assert!(ok != 0, "GetProcessTimes failed");
        let ticks = |t: &FILETIME| u64::from(t.dwHighDateTime) << 32 | u64::from(t.dwLowDateTime);
        Duration::from_nanos((ticks(&times[2]) + ticks(&times[3])) * 100)
    }
}

#[cfg(not(windows))]
mod win {
    use std::time::Duration;

    pub struct Memory {
        pub peak_working_set: u64,
        pub peak_private: u64,
    }

    pub fn memory() -> Memory {
        Memory {
            peak_working_set: 0,
            peak_private: 0,
        }
    }

    pub fn cpu_time() -> Duration {
        Duration::ZERO
    }
}
