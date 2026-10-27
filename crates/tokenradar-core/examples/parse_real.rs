//! Step 02 spike: measures the parse strategies on a real `~/.claude/projects` tree.
//!
//! Read-only. Files are opened for reading with full sharing, and nothing from them is printed
//! except counts, sizes and token totals.
//!
//! ```text
//! parse_real <projects-dir> [options]      matrix: re-runs itself once per configuration
//! parse_real <projects-dir> --single ...   one run, prints one `RESULT` line (raw totals:
//!                                          for the matrix, never commit it)
//! parse_real <projects-dir> --stat         timings of the metadata scan behind the 500 ms budget
//! parse_real <projects-dir> --profile      line-type histogram (counts and bytes only)
//!
//! options:
//!   --strategies abcd     strategies to run (matrix) or the single one (--single)
//!   --threads 1,4         thread counts (matrix) or the single count (--single)
//!   --repeat 3            runs per configuration (matrix)
//!   --uncached            read with FILE_FLAG_NO_BUFFERING, bypassing the OS file cache
//!   --background          run workers in THREAD_MODE_BACKGROUND_BEGIN (low CPU and I/O priority)
//!   --limit-mb N          only the newest files, up to N MiB in total
//!   --manifest FILE       read exactly the files and sizes listed (`<len>TAB<path>` lines);
//!                         the matrix writes one so all runs see the same bytes
//! ```

// Win32 calls for peak memory, CPU time and background mode.
#![allow(unsafe_code)]
// Measurement arithmetic on byte counts and durations; precision loss is irrelevant here.
#![allow(clippy::cast_precision_loss)]

#[path = "../spike/parse_strategies.rs"]
mod strategies;

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::{Instant, SystemTime};

use serde::Deserialize;
use strategies::{LineReader, Parser, Strategy, Tally};

const MIB: f64 = 1024.0 * 1024.0;

#[derive(Clone)]
struct Options {
    dir: PathBuf,
    mode: Mode,
    strategies: Vec<Strategy>,
    threads: Vec<usize>,
    repeat: usize,
    uncached: bool,
    background: bool,
    limit_mb: Option<u64>,
    manifest: Option<PathBuf>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Matrix,
    Single,
    Stat,
    Profile,
}

fn parse_args() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let mut opts = Options {
        dir: PathBuf::new(),
        mode: Mode::Matrix,
        strategies: Strategy::ALL.to_vec(),
        threads: vec![1, 4],
        repeat: 3,
        uncached: false,
        background: false,
        limit_mb: None,
        manifest: None,
    };
    let mut dir = None;
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--single" => opts.mode = Mode::Single,
            "--stat" => opts.mode = Mode::Stat,
            "--profile" => opts.mode = Mode::Profile,
            "--uncached" => opts.uncached = true,
            "--background" => opts.background = true,
            "--strategies" => {
                opts.strategies = value()?
                    .chars()
                    .map(|c| Strategy::from_name(&c.to_string()))
                    .collect::<Option<_>>()
                    .ok_or("unknown strategy")?;
            }
            "--threads" => {
                opts.threads = value()?
                    .split(',')
                    .map(str::parse)
                    .collect::<Result<_, _>>()
                    .map_err(|e| format!("--threads: {e}"))?;
            }
            "--repeat" => opts.repeat = value()?.parse().map_err(|e| format!("--repeat: {e}"))?,
            "--manifest" => opts.manifest = Some(PathBuf::from(value()?)),
            "--limit-mb" => {
                opts.limit_mb = Some(value()?.parse().map_err(|e| format!("--limit-mb: {e}"))?);
            }
            _ if arg.starts_with("--") => return Err(format!("unknown option {arg}")),
            _ => dir = Some(PathBuf::from(arg)),
        }
    }
    opts.dir = dir.ok_or("missing <projects-dir>")?;
    Ok(opts)
}

fn main() -> ExitCode {
    let opts = match parse_args() {
        Ok(opts) => opts,
        Err(err) => {
            eprintln!("parse_real: {err}");
            return ExitCode::from(2);
        }
    };
    let result = match opts.mode {
        Mode::Matrix => matrix(&opts),
        Mode::Single => single(&opts),
        Mode::Stat => stat(&opts),
        Mode::Profile => profile(&opts),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("parse_real: {err}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------------------------
// File discovery
// ---------------------------------------------------------------------------------------------

struct JsonlFile {
    path: PathBuf,
    len: u64,
    modified: SystemTime,
}

/// Collects every `*.jsonl` below `dir`, newest first (the order the indexer will use).
fn discover(opts: &Options) -> std::io::Result<Vec<JsonlFile>> {
    if let Some(manifest) = &opts.manifest {
        return fs::read_to_string(manifest)?
            .lines()
            .map(|line| {
                let (len, path) = line
                    .split_once('\t')
                    .ok_or(std::io::ErrorKind::InvalidData)?;
                Ok(JsonlFile {
                    path: PathBuf::from(path),
                    len: len.parse().map_err(|_| std::io::ErrorKind::InvalidData)?,
                    modified: SystemTime::UNIX_EPOCH,
                })
            })
            .collect();
    }
    let mut files = Vec::new();
    walk(&opts.dir, &mut |entry| {
        let meta = entry.metadata()?;
        files.push(JsonlFile {
            path: entry.path(),
            len: meta.len(),
            modified: meta.modified()?,
        });
        Ok(())
    })?;
    files.sort_by_key(|f| std::cmp::Reverse(f.modified));
    if let Some(limit) = opts.limit_mb {
        let mut budget = limit * 1024 * 1024;
        files.retain(|f| {
            let keep = budget > 0;
            budget = budget.saturating_sub(f.len);
            keep
        });
    }
    Ok(files)
}

/// Calls `on_file` for every `*.jsonl` entry below `dir`.
fn walk(
    dir: &Path,
    on_file: &mut dyn FnMut(&fs::DirEntry) -> std::io::Result<()>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            walk(&entry.path(), on_file)?;
        } else if kind.is_file() && entry.path().extension().is_some_and(|e| e == "jsonl") {
            on_file(&entry)?;
        }
    }
    Ok(())
}

fn open(path: &Path, uncached: bool) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    if uncached {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_NO_BUFFERING: every read goes to the disk, like a cold cache.
        options.custom_flags(0x2000_0000);
    }
    #[cfg(not(windows))]
    let _ = uncached;
    options.open(path)
}

// ---------------------------------------------------------------------------------------------
// Single run
// ---------------------------------------------------------------------------------------------

fn single(opts: &Options) -> Result<(), String> {
    let [strategy] = opts.strategies[..] else {
        return Err("--single needs exactly one strategy".into());
    };
    let [threads] = opts.threads[..] else {
        return Err("--single needs exactly one thread count".into());
    };
    let files = discover(opts).map_err(|e| format!("scan: {e}"))?;
    let io_errors = AtomicU64::new(0);
    let next = AtomicUsize::new(0);
    let cpu_before = win::cpu_time();
    let started = Instant::now();
    let tallies: Vec<Tally> = thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    if opts.background {
                        win::enter_background_mode();
                    }
                    let mut parser = Parser::new(strategy);
                    let mut reader = LineReader::default();
                    let mut tally = Tally::default();
                    while let Some(file) = files.get(next.fetch_add(1, Ordering::Relaxed)) {
                        let read = open(&file.path, opts.uncached).and_then(|f| {
                            reader.read_lines(f, file.len, |line| parser.line(line, &mut tally))
                        });
                        if read.is_err() {
                            io_errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    tally
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().expect("worker panicked"))
            .collect()
    });
    let mut tally = Tally::default();
    for t in tallies {
        tally.merge(t);
    }
    let totals = tally.totals();
    let wall = started.elapsed();
    let cpu = win::cpu_time().saturating_sub(cpu_before);
    let mem = win::memory();
    println!(
        "RESULT strategy={} threads={threads} uncached={} background={} files={} io_errors={} \
         bytes={} lines={} events={} errors={} wall_ms={} cpu_ms={} peak_ws={} peak_private={} \
         unique={} input={} output={} cache_creation={} cache_read={} e1h={} sidechain={} meta={}",
        strategy.name(),
        u8::from(opts.uncached),
        u8::from(opts.background),
        files.len(),
        io_errors.load(Ordering::Relaxed),
        tally.bytes,
        tally.lines,
        tally.events,
        tally.errors,
        wall.as_millis(),
        cpu.as_millis(),
        mem.peak_working_set,
        mem.peak_private,
        totals.unique,
        totals.input,
        totals.output,
        totals.cache_creation,
        totals.cache_read,
        totals.ephemeral_1h,
        totals.sidechain,
        totals.meta_len,
    );
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Matrix: one child process per run so peak memory is per configuration
// ---------------------------------------------------------------------------------------------

type Run = HashMap<String, String>;

fn matrix(opts: &Options) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let files = discover(opts).map_err(|e| format!("scan: {e}"))?;
    let total: u64 = files.iter().map(|f| f.len).sum();
    println!(
        "dataset: {} files, {:.2} GiB{}{}",
        files.len(),
        total as f64 / MIB / 1024.0,
        if opts.uncached { ", uncached" } else { "" },
        if opts.background {
            ", background mode"
        } else {
            ""
        },
    );
    if !opts.uncached {
        // Pull the data into the file cache so the first configuration is not the only cold one.
        let started = Instant::now();
        let mut reader = LineReader::default();
        for file in &files {
            let f = open(&file.path, false).map_err(|e| e.to_string())?;
            reader
                .read_lines(f, file.len, |_| ())
                .map_err(|e| e.to_string())?;
        }
        let secs = started.elapsed().as_secs_f64();
        println!(
            "warm-up read: {secs:.2} s ({:.0} MiB/s)",
            total as f64 / MIB / secs
        );
    }
    let manifest = std::env::temp_dir().join(format!("parse_real-{}.manifest", std::process::id()));
    let mut list = String::new();
    for f in &files {
        writeln!(list, "{}\t{}", f.len, f.path.display()).expect("write to String");
    }
    fs::write(&manifest, list).map_err(|e| e.to_string())?;
    drop(files);
    let result = run_rounds(opts, &exe, &manifest);
    let _ = fs::remove_file(&manifest);
    let runs = result?;
    print_table(&runs);
    check_totals(opts, &runs)
}

fn run_rounds(
    opts: &Options,
    exe: &Path,
    manifest: &Path,
) -> Result<BTreeMap<(usize, &'static str), Vec<Run>>, String> {
    // Rounds interleave the configurations so drifting cache state hits all of them alike.
    let mut runs: BTreeMap<(usize, &'static str), Vec<Run>> = BTreeMap::new();
    for round in 1..=opts.repeat {
        for &threads in &opts.threads {
            for &strategy in &opts.strategies {
                let mut cmd = Command::new(exe);
                cmd.arg(&opts.dir)
                    .arg("--single")
                    .arg("--manifest")
                    .arg(manifest)
                    .args(["--strategies", strategy.name()])
                    .args(["--threads", &threads.to_string()]);
                if opts.uncached {
                    cmd.arg("--uncached");
                }
                if opts.background {
                    cmd.arg("--background");
                }
                let output = cmd.output().map_err(|e| e.to_string())?;
                let stdout = String::from_utf8_lossy(&output.stdout);
                let line = stdout
                    .lines()
                    .find_map(|l| l.strip_prefix("RESULT "))
                    .ok_or_else(|| {
                        format!("run failed: {}", String::from_utf8_lossy(&output.stderr))
                    })?;
                let run: Run = line
                    .split(' ')
                    .filter_map(|kv| kv.split_once('='))
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect();
                eprintln!(
                    "round {round}: {} × {threads} thread(s): {} ms",
                    strategy.name(),
                    run["wall_ms"]
                );
                runs.entry((threads, strategy.name()))
                    .or_default()
                    .push(run);
            }
        }
    }

    Ok(runs)
}

fn print_table(runs: &BTreeMap<(usize, &'static str), Vec<Run>>) {
    println!();
    println!(
        "| Strategy | Threads | Wall s (median) | Wall s (min–max) | MiB/s | lines/s | CPU s | \
         Peak WS MiB | Peak private MiB |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    for ((threads, strategy), runs) in runs {
        let wall = median(runs, "wall_ms") / 1000.0;
        let walls: Vec<f64> = runs.iter().map(|r| num(r, "wall_ms") / 1000.0).collect();
        let bytes = num(&runs[0], "bytes");
        let lines = num(&runs[0], "lines");
        println!(
            "| {strategy} | {threads} | {wall:.2} | {:.2}–{:.2} | {:.0} | {:.0} | {:.2} | {:.1} | {:.1} |",
            walls.iter().copied().fold(f64::INFINITY, f64::min),
            walls.iter().copied().fold(0.0, f64::max),
            bytes / MIB / wall,
            lines / wall,
            median(runs, "cpu_ms") / 1000.0,
            median(runs, "peak_ws") / MIB,
            median(runs, "peak_private") / MIB,
        );
    }
}

/// Every run must agree on the deduplicated events and token totals.
fn check_totals(
    opts: &Options,
    runs: &BTreeMap<(usize, &'static str), Vec<Run>>,
) -> Result<(), String> {
    // Token totals must match in every run. Metadata (`sidechain`, `meta`) only has to match
    // when files are merged in a fixed order: the same key can appear in several files with equal
    // `output_tokens` but different metadata, and threads finish in a random order.
    let token_keys = [
        "unique",
        "input",
        "output",
        "cache_creation",
        "cache_read",
        "e1h",
    ];
    let meta_keys = ["sidechain", "meta"];
    let all: Vec<&Run> = runs.values().flatten().collect();
    let reference = all[0];
    let same = |a: &Run, b: &Run, keys: &[&str]| keys.iter().all(|k| a[*k] == b[*k]);
    let tokens_agree = all.iter().all(|run| same(run, reference, &token_keys));
    let ordered: Vec<&Run> = all
        .iter()
        .copied()
        .filter(|r| r["threads"] == "1")
        .collect();
    let meta_agree = ordered.iter().all(|run| same(run, ordered[0], &meta_keys));
    let meta_varies = !all.iter().all(|run| same(run, reference, &meta_keys));
    // Personal usage totals never reach the output (it is committed under docs/perf/raw/);
    // a fingerprint of them is enough to show that runs agree.
    let fingerprint = |run: &Run, keys: &[&str]| {
        let values: Vec<&str> = keys.iter().map(|k| run[*k].as_str()).collect();
        format!(
            "{:016x}",
            xxhash_rust::xxh3::xxh3_64(values.join(",").as_bytes())
        )
    };

    println!();
    println!(
        "lines={} bytes={} io_errors={}",
        reference["lines"], reference["bytes"], reference["io_errors"]
    );
    for strategy in &opts.strategies {
        if let Some(run) = all.iter().find(|r| r["strategy"] == strategy.name()) {
            println!("parse errors {}: {}", strategy.name(), run["errors"]);
        }
    }
    println!(
        "all runs agree on token totals: {} (fingerprint {})",
        yes_no(tokens_agree),
        fingerprint(reference, &token_keys)
    );
    if !ordered.is_empty() {
        println!(
            "single-thread runs agree on metadata: {} (fingerprint {})",
            yes_no(meta_agree),
            fingerprint(ordered[0], &meta_keys)
        );
    }
    if meta_varies {
        println!("metadata of tied duplicates differs between merge orders:");
        for run in &all {
            println!(
                "  {} × {}: {}",
                run["strategy"],
                run["threads"],
                fingerprint(run, &meta_keys)
            );
        }
    }
    if tokens_agree && meta_agree {
        Ok(())
    } else {
        Err("strategies disagree".into())
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "NO" }
}

fn num(run: &Run, key: &str) -> f64 {
    run[key].parse().unwrap_or(f64::NAN)
}

fn median(runs: &[Run], key: &str) -> f64 {
    let mut values: Vec<f64> = runs.iter().map(|r| num(r, key)).collect();
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        f64::midpoint(values[mid - 1], values[mid])
    } else {
        values[mid]
    }
}

// ---------------------------------------------------------------------------------------------
// Stat walk: the cost behind "incremental scan after restart ≤ 500 ms"
// ---------------------------------------------------------------------------------------------

fn stat(opts: &Options) -> Result<(), String> {
    let err = |e: std::io::Error| e.to_string();
    let paths: Vec<PathBuf> = discover(opts)
        .map_err(err)?
        .into_iter()
        .map(|f| f.path)
        .collect();
    println!("{} files", paths.len());
    println!("| Method | Round | ms |");
    println!("|---|---|---|");
    for round in 1..=opts.repeat {
        // 1. Walk the tree; size and mtime come with the directory listing (FindNextFileW).
        let started = Instant::now();
        let mut n = 0u64;
        walk(&opts.dir, &mut |entry| {
            let meta = entry.metadata()?;
            n += meta.len() & 1;
            Ok(())
        })
        .map_err(err)?;
        println!(
            "| walk + DirEntry::metadata | {round} | {} |",
            started.elapsed().as_millis()
        );

        // 2. Walk the tree and stat every file by path (opens a handle per file on Windows).
        let started = Instant::now();
        walk(&opts.dir, &mut |entry| {
            let meta = fs::metadata(entry.path())?;
            n += meta.len() & 1;
            Ok(())
        })
        .map_err(err)?;
        println!(
            "| walk + fs::metadata(path) | {round} | {} |",
            started.elapsed().as_millis()
        );

        // 3. No walk: stat the paths the database already knows.
        let started = Instant::now();
        for path in &paths {
            n += fs::metadata(path).map_err(err)?.len() & 1;
        }
        println!(
            "| known paths + fs::metadata | {round} | {} |",
            started.elapsed().as_millis()
        );
        std::hint::black_box(n);
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Profile: shape of the data, used to model the synthetic benchmark input
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize)]
struct Kind<'a> {
    #[serde(rename = "type", borrow, default)]
    kind: Option<std::borrow::Cow<'a, str>>,
}

#[derive(Default)]
struct KindStats {
    lines: u64,
    bytes: u64,
    max: u64,
    with_usage: u64,
}

fn profile(opts: &Options) -> Result<(), String> {
    let files = discover(opts).map_err(|e| e.to_string())?;
    let subagents = files
        .iter()
        .filter(|f| f.path.components().any(|c| c.as_os_str() == "subagents"))
        .count();
    let usage = memchr::memmem::Finder::new(br#""usage":"#);
    let mut stats: BTreeMap<String, KindStats> = BTreeMap::new();
    let mut reader = LineReader::default();
    let mut over_chunk = 0u64;
    for file in &files {
        let f = open(&file.path, false).map_err(|e| e.to_string())?;
        reader
            .read_lines(f, file.len, |line| {
                let kind = serde_json::from_slice::<Kind<'_>>(line)
                    .ok()
                    .and_then(|k| k.kind)
                    .map_or_else(|| "<none/invalid>".to_owned(), std::borrow::Cow::into_owned);
                let s = stats.entry(kind).or_default();
                s.lines += 1;
                s.bytes += line.len() as u64 + 1;
                s.max = s.max.max(line.len() as u64);
                s.with_usage += u64::from(usage.find(line).is_some());
                over_chunk += u64::from(line.len() > strategies::CHUNK);
            })
            .map_err(|e| e.to_string())?;
    }
    let total_lines: u64 = stats.values().map(|s| s.lines).sum();
    let total_bytes: u64 = stats.values().map(|s| s.bytes).sum();
    println!("{} files ({subagents} subagent files)", files.len());
    println!(
        "| type | lines | % lines | MiB | % bytes | avg KiB | max KiB | contains \"usage\": |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    for (kind, s) in &stats {
        println!(
            "| {kind} | {} | {:.1} | {:.0} | {:.1} | {:.1} | {:.0} | {} |",
            s.lines,
            100.0 * s.lines as f64 / total_lines as f64,
            s.bytes as f64 / MIB,
            100.0 * s.bytes as f64 / total_bytes as f64,
            s.bytes as f64 / s.lines as f64 / 1024.0,
            s.max as f64 / 1024.0,
            s.with_usage,
        );
    }
    println!("lines longer than one read chunk: {over_chunk}");
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Win32 helpers
// ---------------------------------------------------------------------------------------------

#[cfg(windows)]
mod win {
    use std::time::Duration;

    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentThread, GetProcessTimes, SetThreadPriority,
        THREAD_MODE_BACKGROUND_BEGIN,
    };

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

    /// Lowers the calling thread's CPU *and* I/O priority, as the indexer will (architecture §2).
    pub fn enter_background_mode() {
        // SAFETY: the pseudo handle always refers to the calling thread.
        let ok = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN) };
        assert!(ok != 0, "SetThreadPriority failed");
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

    pub fn enter_background_mode() {}
}
