//! The log file: `<data>/logs/tokenradar.YYYY-MM-DD.log`, one per day, the last
//! [`KEEP_FILES`] kept and at most [`MAX_TOTAL_BYTES`] together (architecture §9).
//!
//! The level comes from `advanced.log_level` and can change while the app runs. Until
//! `config.toml` has been read, `info` applies. Secrets never reach the log: tokens are
//! `Secret<T>`, which prints `***`.

use std::path::Path;
use std::sync::OnceLock;

use tokenradar_core::config::LogLevel;
use tokenradar_core::paths::Paths;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{Registry, reload};

/// Daily files kept.
pub const KEEP_FILES: usize = 7;
/// Upper bound for all log files together; the oldest go first.
pub const MAX_TOTAL_BYTES: u64 = 20 * 1024 * 1024;
const PREFIX: &str = "tokenradar";
const SUFFIX: &str = "log";

struct Logging {
    level: reload::Handle<LevelFilter, Registry>,
    /// Flushes the writer thread when dropped; kept for the life of the process.
    _guard: WorkerGuard,
}

static LOGGING: OnceLock<Logging> = OnceLock::new();

/// Starts writing the log file. Without a data directory the app runs without one.
pub fn init() {
    let dir = match Paths::resolve() {
        Ok(paths) => paths.logs_dir(),
        Err(error) => {
            eprintln!("TokenRadar: no log file: {error}");
            return;
        }
    };
    // The appender lists the directory to prune old files before it would create it.
    if let Err(error) = std::fs::create_dir_all(&dir) {
        eprintln!("TokenRadar: no log file: {error}");
        return;
    }
    trim(&dir, MAX_TOTAL_BYTES);
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(PREFIX)
        .filename_suffix(SUFFIX)
        .max_log_files(KEEP_FILES)
        .build(&dir);
    let appender = match appender {
        Ok(appender) => appender,
        Err(error) => {
            eprintln!("TokenRadar: no log file: {error}");
            return;
        }
    };
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let (filter, level) = reload::Layer::new(LevelFilter::INFO);
    let installed = tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false)
                .with_thread_names(true),
        )
        .try_init();
    if installed.is_ok() {
        let _ = LOGGING.set(Logging {
            level,
            _guard: guard,
        });
        tracing::info!(version = env!("CARGO_PKG_VERSION"), "TokenRadar started");
    }
}

/// Applies `advanced.log_level`.
pub fn set_level(level: LogLevel) {
    if let Some(logging) = LOGGING.get() {
        let _ = logging.level.reload(filter(level));
    }
}

const fn filter(level: LogLevel) -> LevelFilter {
    match level {
        LogLevel::Error => LevelFilter::ERROR,
        LogLevel::Warn => LevelFilter::WARN,
        LogLevel::Info => LevelFilter::INFO,
        LogLevel::Debug => LevelFilter::DEBUG,
        LogLevel::Trace => LevelFilter::TRACE,
    }
}

/// Deletes the oldest log files until the rest fit in `budget` bytes. The appender only
/// counts files, so one noisy `trace` day could otherwise fill the disk.
fn trim(dir: &Path, budget: u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(String, u64)> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let ours = name.starts_with(PREFIX) && name.ends_with(SUFFIX);
            let meta = e.metadata().ok()?;
            (ours && meta.is_file()).then_some((name, meta.len()))
        })
        .collect();
    // The date in the name sorts oldest first.
    files.sort();
    let mut total: u64 = files.iter().map(|(_, len)| len).sum();
    for (name, len) in files {
        if total <= budget {
            break;
        }
        if std::fs::remove_file(dir.join(&name)).is_ok() {
            total -= len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_deletes_the_oldest_files_over_the_budget() {
        let dir = tempfile::tempdir().expect("tempdir");
        for (day, size) in [("01", 400), ("02", 400), ("03", 400)] {
            let name = format!("{PREFIX}.2026-10-{day}.{SUFFIX}");
            std::fs::write(dir.path().join(name), vec![b'x'; size]).expect("write");
        }
        std::fs::write(dir.path().join("other.txt"), vec![b'x'; 5000]).expect("write");
        trim(dir.path(), 900);
        let mut left: Vec<String> = std::fs::read_dir(dir.path())
            .expect("read")
            .map(|e| e.expect("entry").file_name().into_string().expect("utf-8"))
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "other.txt",
                "tokenradar.2026-10-02.log",
                "tokenradar.2026-10-03.log"
            ]
        );
    }
}
