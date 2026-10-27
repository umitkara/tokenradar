//! `tokenradar doctor`: writes the diagnostics bundle (see `tokenradar_core::doctor`). Works
//! with the app closed; when it runs, its data-source state is added over IPC.

use std::path::PathBuf;

use tokenradar_core::doctor::{self, Input};
use tokenradar_core::domain::Timestamp;
use tokenradar_i18n::Arg;

use crate::context::Context;
use crate::report;
use crate::setup::Outcome;

pub fn run(ctx: &Context, out: Option<PathBuf>) -> Outcome {
    let l = &ctx.l10n;
    let Some(paths) = &ctx.paths else {
        eprintln!(
            "{}",
            l.fmt(
                "error",
                &[("message", Arg::Str("no data directory for TokenRadar"))]
            )
        );
        return Err(());
    };
    let out = out.unwrap_or_else(|| PathBuf::from(doctor::default_file_name(Timestamp::now())));
    let producer = format!("tokenradar-cli/{}", env!("CARGO_PKG_VERSION"));
    let input = Input {
        producer: &producer,
        paths,
        live: report::from_app(ctx).as_ref().map(doctor::live_from_status),
    };
    match doctor::write(&out, &input) {
        Ok(path) => {
            let shown = std::path::absolute(&path).unwrap_or(path);
            let shown = shown.display().to_string();
            println!("{}", l.fmt("doctor-done", &[("path", Arg::Str(&shown))]));
            Ok(())
        }
        Err(e) => {
            let message = e.to_string();
            eprintln!("{}", l.fmt("error", &[("message", Arg::Str(&message))]));
            Err(())
        }
    }
}
