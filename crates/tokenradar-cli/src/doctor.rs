//! `tokenradar doctor`: writes the diagnostics bundle (see `tokenradar_core::doctor`). Works
//! with the app closed; when it runs, its data-source state is added over IPC.

use std::path::PathBuf;

use tokenradar_core::calibration::CalibrationStatus;
use tokenradar_core::doctor::{self, Input};
use tokenradar_core::domain::{Scope, Timestamp};
use tokenradar_core::ipc::StatusQuery;
use tokenradar_core::limits::Confidence;
use tokenradar_i18n::{Arg, Localizer};

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
        // The working folder, so the report says whether the app gave up counting untracked
        // files there (Step 12r); the folder itself stays out of the bundle.
        live: report::from_app_with(
            ctx,
            StatusQuery {
                cwd: std::env::current_dir()
                    .ok()
                    .map(|d| d.to_string_lossy().into_owned()),
                ..StatusQuery::default()
            },
        )
        .as_ref()
        .map(doctor::live_from_status),
    };
    match doctor::write(&out, &input) {
        Ok(path) => {
            let shown = std::path::absolute(&path).unwrap_or(path);
            let shown = shown.display().to_string();
            println!("{}", l.fmt("doctor-done", &[("path", Arg::Str(&shown))]));
            let unpriced = doctor::unpriced_models(paths, Timestamp::now());
            if !unpriced.is_empty() {
                let models = unpriced.join(", ");
                println!(
                    "{}",
                    l.fmt("doctor-unpriced", &[("models", Arg::Str(&models))])
                );
            }
            for c in doctor::calibrations(paths, Timestamp::now()) {
                println!("{}", calibration_line(l, &c));
            }
            Ok(())
        }
        Err(e) => {
            let message = e.to_string();
            eprintln!("{}", l.fmt("error", &[("message", Arg::Str(&message))]));
            Err(())
        }
    }
}

/// One line per calibrated limit: level, confidence, intervals and a drift if one shows.
fn calibration_line(l: &Localizer, c: &CalibrationStatus) -> String {
    let scope = c.scope.as_ref().map(|s| match s {
        Scope::Model(m) | Scope::Surface(m) => m.clone(),
    });
    let limit = report::limit_name(l, c.kind.as_str(), scope.as_deref());
    let level = match c.level {
        Confidence::High => "high",
        Confidence::Medium => "medium",
        Confidence::Low => "low",
    };
    let drift = if c.drift.is_some() { "yes" } else { "no" };
    l.fmt(
        "doctor-calibration",
        &[
            ("limit", Arg::Str(&limit)),
            ("level", Arg::Str(level)),
            ("confidence", Arg::Num((c.confidence * 100.0).round())),
            ("intervals", Arg::Num(f64::from(c.n_obs))),
            ("drift", Arg::Str(drift)),
        ],
    )
}

#[cfg(test)]
mod tests {
    use tokenradar_core::calibration::{Drift, DriftDirection};
    use tokenradar_core::domain::{AccountId, LimitKind, Source};
    use tokenradar_i18n::{Domain, Locale};

    use super::*;

    fn status(drift: bool) -> CalibrationStatus {
        let at = Timestamp::from_millis(1_795_000_000_000);
        CalibrationStatus {
            account: AccountId(1),
            kind: LimitKind::WeeklyScoped,
            scope: Some(Scope::Model("Fable".into())),
            source: Source::Api,
            k: 0.17,
            alpha: Vec::new(),
            confidence: 0.82,
            level: Confidence::High,
            n_obs: 61,
            drift: drift.then_some(Drift {
                detected_at: at,
                since: at,
                direction: DriftDirection::Up,
            }),
            computed_at: at,
            version: 3,
        }
    }

    #[test]
    fn the_calibration_line_names_the_limit_level_and_drift() {
        let en = Localizer::new(Locale::En, &[Domain::Cli]);
        let line = calibration_line(&en, &status(false));
        assert!(line.contains("Weekly (Fable)"), "{line}");
        assert!(line.contains("high confidence (82"), "{line}");
        assert!(line.contains("61 intervals"), "{line}");
        assert!(!line.contains("changed"), "{line}");
        let line = calibration_line(&en, &status(true));
        assert!(
            line.contains("usage-to-limit ratio changed recently"),
            "{line}"
        );

        let tr = Localizer::new(Locale::Tr, &[Domain::Cli]);
        let line = calibration_line(&tr, &status(true));
        assert!(line.contains("yüksek güven"), "{line}");
        assert!(line.contains("61 aralık"), "{line}");
        assert!(line.contains("yakın zamanda değişti"), "{line}");
    }
}
