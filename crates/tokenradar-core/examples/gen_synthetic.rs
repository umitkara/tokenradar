//! Writes the synthetic benchmark database (Step 14b): a year of made-up usage, 3,000 sessions,
//! ~150k events a month, weekly windows with readings, a calibration and attributions.
//!
//! ```text
//! cargo run --release -p tokenradar-core --example gen_synthetic -- --out <path.db> [--days N]
//!     [--end-now] [--config-dir <dir>]
//! ```
//!
//! `--end-now` ends the data at the current time instead of the fixed benchmark date, and
//! `--config-dir` names the account's Claude folder. Together they make a database the app
//! can open as its own (screenshots and measurements with sample data, Step 17b).

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

#[path = "synthetic/mod.rs"]
mod synthetic;

use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let mut out: Option<PathBuf> = None;
    let mut params = synthetic::Params::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out = args.next().map(PathBuf::from),
            "--days" => {
                params.days = args.next().and_then(|d| d.parse().ok()).expect("--days N");
                params.sessions = (params.sessions as i64 * params.days / 365).max(10) as usize;
            }
            "--end-now" => {
                // Whole minutes, so the windows' reset times stay readable.
                let now = tokenradar_core::domain::Timestamp::now().as_millis();
                params.end = tokenradar_core::domain::Timestamp::from_millis(now - now % 60_000);
            }
            "--config-dir" => params.config_dir = args.next().expect("--config-dir <dir>"),
            other => panic!("unknown argument {other}"),
        }
    }
    let out = out.expect("--out <path.db>");
    for suffix in ["", "-wal", "-shm"] {
        let mut p = out.clone().into_os_string();
        p.push(suffix);
        let _ = std::fs::remove_file(p);
    }
    let started = Instant::now();
    let s = synthetic::generate(&out, &params);
    println!(
        "{}: {} main sessions, {} subagents, {} events ({} raw kept), {} snapshots, {} windows, {} attributed, {:.1} s",
        out.display(),
        s.roots,
        s.subagents,
        s.events,
        s.raw_kept,
        s.snapshots,
        s.windows,
        s.attributed,
        started.elapsed().as_secs_f64()
    );
}
