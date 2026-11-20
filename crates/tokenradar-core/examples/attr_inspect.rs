//! Step 14a: attribution of every window of a TokenRadar database, for inspection (ADR-064).
//!
//! ```text
//! cargo run --release -p tokenradar-core --example attr_inspect -- --db <copy.db> [--names]
//! ```
//!
//! Opens the database read-only and writes nothing. Calibrations are computed in memory as the
//! app would now. Estimated weekly windows are derived the way the app adds them.
//!
//! Prints percentages, methods and confidences only. Costs, counts and session ids are
//! personal usage data and are not printed; `--names` adds session titles for a look on
//! screen and must not be pasted into the repository. `--trace` prints each measured
//! window's segments to stderr: hours from the start, `Δp` and `Δp / (k · Σ α · cost)`, and
//! the cumulative `k · Σ α · cost` at the first edges (the rounding offset, ADR-064).

use std::path::PathBuf;

use tokenradar_core::attribution::{self, compute, estimated_weekly_windows};
use tokenradar_core::calibration;
use tokenradar_core::domain::{LimitWindow, Timestamp};
use tokenradar_core::store::ReadOnlyStore;

fn main() {
    let mut db = None;
    let mut names = false;
    let mut trace = false;
    let mut p = lexopt::Parser::from_env();
    while let Some(arg) = p.next().expect("arguments") {
        use lexopt::prelude::*;
        match arg {
            Long("db") => db = Some(PathBuf::from(p.value().expect("--db value"))),
            Long("names") => names = true,
            Long("trace") => trace = true,
            _ => panic!("usage: attr_inspect --db <path> [--names] [--trace]"),
        }
    }
    let store = ReadOnlyStore::open(db.expect("--db is required"))
        .expect("open")
        .expect("database exists");
    let now = Timestamp::now();
    println!("# Attribution inspection\n");
    for account in store.accounts().expect("accounts") {
        let cals = calibration::calibrate(&store, account.id, now).expect("calibrate");
        let mut windows = store.limit_windows(account.id).expect("windows");
        let earliest = store.earliest_bucket(account.id).expect("earliest");
        for (i, (starts_at, resets_at)) in estimated_weekly_windows(&windows, earliest)
            .into_iter()
            .enumerate()
        {
            windows.push(LimitWindow {
                id: -1 - i64::try_from(i).expect("small"),
                account: account.id,
                kind: tokenradar_core::domain::LimitKind::WeeklyAll,
                scope: None,
                starts_at,
                resets_at,
                peak_percent: None,
                hit_limit: false,
                estimated: true,
            });
        }
        windows.sort_by_key(|w| (w.kind.as_str().to_owned(), w.resets_at));
        println!(
            "| limit | window end | method | conf | total | other | other/total | breakdown | flag | top shares |"
        );
        println!("|---|---|---|---|---|---|---|---|---|---|");
        for w in &windows {
            let started = std::time::Instant::now();
            let (run, shares) = compute(&store, w, &cals, now).expect("compute");
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            if trace && !w.estimated {
                trace_window(&store, w, &cals, now);
            }
            let share_of_total = match (run.other_percent, run.total_percent) {
                (Some(o), Some(t)) if t > 0.0 => format!("{:.1}", o / t * 100.0),
                _ => "–".to_owned(),
            };
            let flag = if run.suspect {
                "suspect"
            } else if run.other_excess {
                "excess"
            } else {
                ""
            };
            let sum: f64 = shares.iter().filter_map(|s| s.percent).sum::<f64>()
                + run.other_percent.unwrap_or(0.0);
            let top = top_shares(&store, &shares, names);
            println!(
                "| {} {} | {} | {}{} | {} | {} | {} | {} | {} | {} | {} | Σ−total {:+.4} · {:.1} ms",
                w.kind.as_str(),
                w.scope
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                w.resets_at,
                run.method.as_str(),
                if w.estimated { " (est.)" } else { "" },
                run.confidence.map_or("–".to_owned(), |c| format!("{c:.2}")),
                pct(run.total_percent),
                pct(run.other_percent),
                share_of_total,
                pct(run.breakdown_other),
                flag,
                top,
                sum - run.total_percent.unwrap_or(0.0),
                ms,
            );
        }
    }
}

fn pct(x: Option<f64>) -> String {
    x.map_or("–".to_owned(), |v| format!("{v:.1}"))
}

/// The five largest shares, with session titles when asked.
fn top_shares(store: &ReadOnlyStore, shares: &[attribution::Share], names: bool) -> String {
    shares
        .iter()
        .take(5)
        .map(|s| {
            let label = if names {
                let l = store
                    .session_labels(&[s.root])
                    .ok()
                    .and_then(|v| v.into_iter().next());
                format!(
                    " {}",
                    l.and_then(|l| l.title.or(l.project))
                        .unwrap_or_else(|| "?".to_owned())
                )
            } else {
                String::new()
            };
            format!("{}{label}", pct(s.percent))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Per segment: hours from the window start, `Δp` and `Δp / (k · Σ α · cost)`.
fn trace_window(
    store: &ReadOnlyStore,
    w: &LimitWindow,
    cals: &[calibration::Calibration],
    now: Timestamp,
) {
    let Some(cal) = cals.iter().find(|c| c.kind == w.kind && c.scope == w.scope) else {
        return;
    };
    let end = now.min(w.resets_at);
    let edges = attribution::window_edges(store, w).expect("edges");
    let buckets = store
        .bucket_costs(w.account, w.starts_at, end)
        .expect("buckets");
    let peak = w.peak_percent.unwrap_or(0.0);
    let segs = attribution::segments(w.starts_at, end, &edges, peak);
    let family = cal.family();
    eprintln!(
        "{} {:?}: {} edges",
        w.kind.as_str(),
        w.resets_at.to_string(),
        edges.len()
    );
    let kc_until = |t: Timestamp| -> f64 {
        buckets
            .iter()
            .filter(|b| b.bucket < t && family.is_none_or(|f| f == b.family))
            .map(|b| {
                #[allow(clippy::cast_precision_loss)]
                let c = b.cost.0 as f64;
                cal.k * c * cal.alpha.get(&b.family).copied().unwrap_or(1.0) / 1e6
            })
            .sum()
    };
    for e in edges.iter().take(4) {
        eprintln!(
            "  edge {:4.1}: cumulative k·C {:.2}",
            e.percent,
            kc_until(e.at())
        );
    }
    for s in segs {
        let wc: f64 = buckets
            .iter()
            .filter(|b| b.bucket >= s.from.bucket_5m() && b.bucket < s.to)
            .filter(|b| family.is_none_or(|f| f == b.family))
            .map(|b| {
                #[allow(clippy::cast_precision_loss)]
                let c = b.cost.0 as f64;
                c * cal.alpha.get(&b.family).copied().unwrap_or(1.0)
            })
            .sum();
        #[allow(clippy::cast_precision_loss)]
        let h = |t: Timestamp| (t.as_millis() - w.starts_at.as_millis()) as f64 / 3.6e6;
        eprintln!(
            "  {:6.1}h → {:6.1}h  dp {:4.1}  dp/kC {:.2}",
            h(s.from),
            h(s.to),
            s.dp,
            s.dp / (cal.k * wc / 1e6)
        );
    }
}
