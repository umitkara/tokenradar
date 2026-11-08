//! Step 12n measurement: cost of a compact-only backfill scan over every transcript.
//!
//! ```text
//! cargo run --release -p tokenradar-core --example compact_scan -- <config_dir>
//! ```
//!
//! Reads each `projects/**/*.jsonl` file whole and counts lines that contain
//! `"compact_boundary"`, without parsing anything else. Prints bytes, wall time and
//! throughput only; record counts are personal usage data and are not printed.
//!
//! Read-only on the transcripts.

use std::path::{Path, PathBuf};
use std::time::Instant;

use memchr::memmem;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "jsonl") {
            out.push(p);
        }
    }
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .expect("usage: compact_scan <config_dir>");
    let started = Instant::now();
    let mut files = Vec::new();
    walk(&dir.join("projects"), &mut files);
    let inventory = started.elapsed();
    let finder = memmem::Finder::new(br#""compact_boundary""#);
    let (mut bytes, mut hits) = (0u64, 0u64);
    let scan = Instant::now();
    for f in &files {
        let Ok(data) = std::fs::read(f) else { continue };
        bytes += data.len() as u64;
        hits += finder.find_iter(&data).count() as u64;
    }
    let scan = scan.elapsed();
    #[allow(clippy::cast_precision_loss)]
    let gib = bytes as f64 / f64::from(1u32 << 30);
    println!(
        "files {} · {gib:.2} GiB · inventory {} ms · scan {} ms · {:.2} GiB/s · hits>0: {}",
        files.len(),
        inventory.as_millis(),
        scan.as_millis(),
        gib / scan.as_secs_f64(),
        hits > 0
    );
}
