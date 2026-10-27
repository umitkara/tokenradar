//! Step 07: runs the core with its IPC server, without the app, for
//! `scripts/measure-statusline.ps1`.
//!
//! ```text
//! ipc_serve <config-dir> <db-path>
//! ```
//!
//! Serves the current user's pipe with one account for `<config-dir>` and a database at
//! `<db-path>`. Prints `READY` once the pipe accepts clients and stops when stdin closes.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use tokenradar_core::ipc::{Client, pipe_name};
use tokenradar_core::providers::claude::AccountDescriptor;
use tokenradar_core::runtime::{CoreConfig, CoreRuntime, IpcConfig};

fn main() {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let (Some(config_dir), Some(db_path)) = (args.next(), args.next()) else {
        eprintln!("usage: ipc_serve <config-dir> <db-path>");
        std::process::exit(2);
    };
    let pipe = pipe_name().expect("pipe name");
    let mut config = CoreConfig::new(
        db_path,
        vec![AccountDescriptor {
            config_dir,
            label: "Claude".into(),
        }],
    );
    config.retention_days = None;
    config.ipc = Some(IpcConfig {
        pipe_name: pipe.clone(),
        server_id: "ipc_serve".into(),
    });
    let handle = CoreRuntime::start(&config).expect("start");

    // The pipe comes up on its own thread; another server (the app) makes this fail.
    let started = Instant::now();
    while Client::connect(&pipe, Duration::from_millis(100)).is_err() {
        if started.elapsed() > Duration::from_secs(10) {
            eprintln!("the pipe did not come up; is the TokenRadar app running?");
            std::process::exit(1);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    println!("READY");
    let _ = std::io::stdout().flush();

    for _ in std::io::stdin().lock().lines() {}
    handle.shutdown(Duration::from_secs(5));
}
