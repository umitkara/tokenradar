//! Step 13: walk-forward backtest of calibration on a TokenRadar database (ADR-061).
//!
//! ```text
//! cargo run --release -p tokenradar-core --example backtest -- --db <tokenradar.db> [--days 21]
//! ```
//!
//! For every test instant `t` (every 30 minutes), `k` **and** the family weights are learned
//! from the intervals that ended before `t` only, exactly as the app would have known them.
//! The percentage at `t + h` is then predicted as `p(t) + k · Σ α · cost(t, t+h]` and compared
//! with the running maximum actually observed. `h` is 6 and 24 hours; the acceptance criterion
//! is MAE ≤ 1.5 points and |bias| ≤ 0.5 at 24 hours.
//!
//! The same run without family weights (one `k`, hypothesis H1) is printed for comparison, and
//! so is the prediction of each complete window's final percentage from the `k` known at its
//! start.
//!
//! `--trace` prints every 24-hour prediction of the fitted model to stderr: day offset,
//! confidence, weights and error.
//!
//! Opens the database read-only. Prints error metrics, weights and confidence only: interval
//! counts, `k` and costs are personal usage data and are not printed.

use std::collections::BTreeMap;
use std::path::PathBuf;

use tokenradar_core::calibration::{self, CalibrationData, Obs, estimator};
use tokenradar_core::domain::{
    AccountId, LimitKind, LimitObservation, ModelFamily, Scope, Source, Timestamp,
};
use tokenradar_core::limits::Confidence;
use tokenradar_core::store::{ReadOnlyStore, WINDOW_MATCH_TOLERANCE_MS};

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const DAY: i64 = 24 * HOUR;
const STEP: i64 = 30 * MIN;
/// The observed value at `t + h` must be at most this late.
const TRUTH_SLACK: i64 = 30 * MIN;
/// Predictions need at least this many past intervals.
const MIN_TRAIN: u32 = 10;

struct Args {
    db: PathBuf,
    days: i64,
    trace: bool,
}

fn args() -> Args {
    use lexopt::prelude::*;
    let mut db = None;
    let mut days = 21;
    let mut trace = false;
    let mut p = lexopt::Parser::from_env();
    while let Some(arg) = p.next().expect("arguments") {
        match arg {
            Long("db") => db = Some(PathBuf::from(p.value().expect("--db value"))),
            Long("days") => days = p.value().expect("--days").parse().expect("number"),
            Long("trace") => trace = true,
            _ => panic!("usage: backtest --db <path> [--days N]"),
        }
    }
    Args {
        db: db.expect("--db is required"),
        days,
        trace,
    }
}

/// `(t, running max, resets_at)` of one source of one limit.
fn truth(
    snaps: &[LimitObservation],
    kind: &LimitKind,
    scope: Option<&Scope>,
    source: Source,
) -> Vec<(i64, f64, i64)> {
    let mut windows: Vec<(i64, f64)> = Vec::new();
    let mut out = Vec::new();
    for s in snaps {
        if &s.kind != kind || s.scope.as_ref() != scope || s.source != source {
            continue;
        }
        let Some(r) = s.resets_at.map(Timestamp::as_millis) else {
            continue;
        };
        let at = windows
            .iter()
            .position(|w| (w.0 - r).abs() <= WINDOW_MATCH_TOLERANCE_MS)
            .unwrap_or_else(|| {
                windows.push((r, s.percent));
                windows.len() - 1
            });
        let w = &mut windows[at];
        w.1 = w.1.max(s.percent);
        out.push((s.observed_at.as_millis(), w.1, w.0));
    }
    out
}

fn weighted_cost(
    costs: &[calibration::ModelCost],
    only: Option<ModelFamily>,
    alpha: &BTreeMap<ModelFamily, f64>,
) -> f64 {
    costs
        .iter()
        .filter(|c| only.is_none_or(|f| f == c.family))
        .map(|c| {
            #[allow(clippy::cast_precision_loss)]
            let usd = c.cost.0 as f64 / 1e6;
            usd * alpha.get(&c.family).copied().unwrap_or(1.0)
        })
        .sum()
}

#[derive(Default)]
struct Stats {
    errs: Vec<f64>,
}

impl Stats {
    fn push(&mut self, e: f64) {
        self.errs.push(e);
    }

    fn line(&self) -> String {
        if self.errs.is_empty() {
            return "| – | – | – | 0 |".into();
        }
        #[allow(clippy::cast_precision_loss)]
        let n = self.errs.len() as f64;
        let mae = self.errs.iter().map(|e| e.abs()).sum::<f64>() / n;
        let bias = self.errs.iter().sum::<f64>() / n;
        let mut abs: Vec<f64> = self.errs.iter().map(|e| e.abs()).collect();
        abs.sort_by(f64::total_cmp);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let p90 = abs[((n * 0.9) as usize).min(abs.len() - 1)];
        format!("| {mae:.2} | {p90:.2} | {bias:+.2} | {} |", self.errs.len())
    }
}

fn level_name(c: Confidence) -> &'static str {
    match c {
        Confidence::Low => "low",
        Confidence::Medium => "medium",
        Confidence::High => "high",
    }
}

#[allow(clippy::too_many_lines)]
fn run(store: &ReadOnlyStore, account: AccountId, days: i64, trace: bool) {
    let end = store
        .snapshots(
            account,
            Timestamp::from_millis(0),
            Timestamp::from_millis(i64::MAX),
        )
        .expect("snapshots")
        .last()
        .map_or(0, |s| s.observed_at.as_millis());
    let start = end - days * DAY;
    let snaps = store
        .snapshots(
            account,
            Timestamp::from_millis(start),
            Timestamp::from_millis(end + 1),
        )
        .expect("snapshots");
    println!("## Account {}\n", account.0);
    for series in calibration::series(&snaps) {
        let only = series.scope.as_ref().and_then(|s| match s {
            Scope::Model(m) => m.to_ascii_lowercase().parse::<ModelFamily>().ok(),
            Scope::Surface(_) => None,
        });
        let fit = only.is_none();
        let obs: Vec<Obs> = calibration::observations(store, account, &series).expect("costs");
        let points = truth(&snaps, &series.kind, series.scope.as_ref(), series.source);
        let label = match &series.scope {
            Some(s) => format!("{} {s}", series.kind.as_str()),
            None => series.kind.as_str().to_owned(),
        };
        println!("### `{label}` (series: {})\n", series.source.as_str());

        let mut table: BTreeMap<String, Stats> = BTreeMap::new();
        let mut alphas: Vec<f64> = Vec::new();
        let first = points.first().map_or(end, |p| p.0);
        let mut t = first;
        while t <= end {
            let past: Vec<Obs> = obs
                .iter()
                .filter(|o| o.at.as_millis() <= t)
                .cloned()
                .collect();
            let now = Timestamp::from_millis(t);
            let models = [("model", fit), ("H1", false)];
            let estimates: Vec<_> = models
                .iter()
                .map(|&(_, f)| estimator::estimate(past.clone(), now, f))
                .collect();
            let Some(&(_, p_now, reset)) = points.iter().rev().find(|p| p.0 <= t) else {
                t += STEP;
                continue;
            };
            for h_hours in [6i64, 24] {
                let target = t + h_hours * HOUR;
                if target > reset {
                    continue;
                }
                let Some(&(at, p_then, _)) = points.iter().find(|p| p.0 >= target && p.2 == reset)
                else {
                    continue;
                };
                if at - target > TRUTH_SLACK {
                    continue;
                }
                let costs = store
                    .model_costs(account, now, Timestamp::from_millis(target))
                    .expect("costs");
                let total = weighted_cost(&costs, only, &BTreeMap::new());
                let special = weighted_cost(
                    &costs
                        .iter()
                        .filter(|c| !matches!(c.family, ModelFamily::Opus))
                        .cloned()
                        .collect::<Vec<_>>(),
                    only,
                    &BTreeMap::new(),
                );
                let mix = if total <= 0.0 {
                    "idle"
                } else if special / total >= 0.3 {
                    "other families ≥ 30 %"
                } else {
                    "mostly Opus"
                };
                for ((name, _), est) in models.iter().zip(&estimates) {
                    if !fit && *name == "H1" {
                        continue;
                    }
                    let Some(e) = est.as_ref().filter(|e| e.n_obs >= MIN_TRAIN) else {
                        continue;
                    };
                    let pred = p_now + e.k * 1e6 * weighted_cost(&costs, only, &e.alpha);
                    // What the app would show: the confidence of applying k to this usage.
                    let unknown: f64 = costs
                        .iter()
                        .filter(|c| {
                            only.is_none()
                                && estimator::needs_own_weight(c.family)
                                && !e.fitted.contains(&c.family)
                        })
                        .map(|c| {
                            #[allow(clippy::cast_precision_loss)]
                            let usd = c.cost.0 as f64 / 1e6;
                            usd
                        })
                        .sum();
                    let use_conf = if total > 0.0 {
                        e.confidence * (1.0 - unknown / total)
                    } else {
                        e.confidence
                    };
                    let err = pred - p_then;
                    table
                        .entry(format!("{h_hours:>2} h · {name} · all"))
                        .or_default()
                        .push(err);
                    if *name == "model" && trace && h_hours == 24 {
                        #[allow(clippy::cast_precision_loss)]
                        let day = (t - first) as f64 / DAY as f64;
                        eprintln!(
                            "{label} day {day:5.2} n {:3} conf {:.2} use {use_conf:.2} alpha {:?} drift {} err {err:+.2}",
                            e.n_obs,
                            e.confidence,
                            e.alpha
                                .iter()
                                .map(|(f, a)| format!("{}={a:.2}", f.as_str()))
                                .collect::<Vec<_>>(),
                            e.drift.is_some(),
                        );
                    }
                    if *name == "model" {
                        table
                            .entry(format!("{h_hours:>2} h · {name} · {mix}"))
                            .or_default()
                            .push(err);
                        if estimator::level(use_conf) != Confidence::Low {
                            table
                                .entry(format!("{h_hours:>2} h · {name} · used (≥ medium)"))
                                .or_default()
                                .push(err);
                        }
                        table
                            .entry(format!(
                                "{h_hours:>2} h · {name} · confidence {}",
                                level_name(estimator::level(use_conf))
                            ))
                            .or_default()
                            .push(err);
                        if h_hours == 24
                            && let Some(&a) = e.alpha.get(&ModelFamily::Fable)
                        {
                            alphas.push(a);
                        }
                    }
                }
            }
            t += STEP;
        }
        println!(
            "| Horizon · model · subset | MAE (pt) | p90 abs (pt) | bias (pt) | test points |"
        );
        println!("|---|---|---|---|---|");
        for (k, s) in &table {
            println!("| {k} {}", s.line());
        }
        if let (Some(lo), Some(hi)) = (
            alphas.iter().copied().reduce(f64::min),
            alphas.iter().copied().reduce(f64::max),
        ) {
            println!("\nα_fable learned walk-forward: {lo:.2} – {hi:.2}");
        }
        if let Some(e) = estimator::estimate(obs.clone(), Timestamp::from_millis(end), fit) {
            let alpha: Vec<String> = e
                .alpha
                .iter()
                .map(|(f, a)| format!("{}={a:.2}", f.as_str()))
                .collect();
            println!(
                "Final: α {{{}}}, confidence {:.2} ({}), MAD/k {:.2}, drift {}",
                alpha.join(", "),
                e.confidence,
                level_name(estimator::level(e.confidence)),
                e.mad_rel,
                e.drift
                    .map_or("none".to_owned(), |d| format!("{:?}", d.direction)),
            );
        }

        // Each complete window's final value from the k known at its start.
        let mut resets: Vec<i64> = points.iter().map(|p| p.2).collect();
        resets.dedup();
        for r in resets {
            let Some(&(_, p_end, _)) = points.iter().rev().find(|p| p.2 == r) else {
                continue;
            };
            let length = if series.kind == LimitKind::Session {
                5 * HOUR
            } else {
                7 * DAY
            };
            let w_start = r - length;
            if r > end || w_start < first {
                continue; // incomplete, or recording began mid-window
            }
            let past: Vec<Obs> = obs
                .iter()
                .filter(|o| o.at.as_millis() <= w_start)
                .cloned()
                .collect();
            let Some(e) = estimator::estimate(past, Timestamp::from_millis(w_start), fit)
                .filter(|e| e.n_obs >= MIN_TRAIN)
            else {
                println!("Window ending {r}: not enough intervals before it");
                continue;
            };
            let costs = store
                .model_costs(
                    account,
                    Timestamp::from_millis(w_start),
                    Timestamp::from_millis(r),
                )
                .expect("costs");
            let pred = e.k * 1e6 * weighted_cost(&costs, only, &e.alpha);
            println!(
                "Window ending {r}: final-value error {:+.1} pt",
                pred - p_end
            );
        }
        println!();
    }
}

fn main() {
    let a = args();
    let store = ReadOnlyStore::open(&a.db)
        .expect("open")
        .expect("database exists");
    println!("# Calibration backtest\n");
    for account in store.accounts().expect("accounts") {
        run(&store, account.id, a.days, a.trace);
        // What one recompute in the app costs: `calibrate` over the full lookback.
        let mut ms: Vec<f64> = (0..7)
            .map(|_| {
                let started = std::time::Instant::now();
                let _ = calibration::calibrate(&store, account.id, Timestamp::now());
                started.elapsed().as_secs_f64() * 1000.0
            })
            .collect();
        ms.sort_by(f64::total_cmp);
        println!(
            "
One `calibrate` run: median {:.1} ms, max {:.1} ms (7 runs)",
            ms[3], ms[6]
        );
    }
}
