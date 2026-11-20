//! Step 12t: the old burn rate (30-minute Theil–Sen) against the step-edge rate on a TokenRadar
//! database (ADR-062).
//!
//! ```text
//! cargo run --release -p tokenradar-core --example rate_compare -- --db <copy of tokenradar.db>
//! ```
//!
//! Every stored reading of one source of one limit is replayed in order. At most every 15
//! minutes both rates are taken as the app would have known them at that instant and compared
//! with the rate that actually followed: the running maximum's rise over the next three hours.
//! Projections against the window's reset are compared for how often they flip.
//!
//! Opens the database read-only. Prints ratios and shares only: no percentages, counts, times
//! or costs, which are personal usage data.

use std::path::PathBuf;

use tokenradar_core::domain::{LimitKind, LimitObservation, Scope, Source, Timestamp};
use tokenradar_core::limits::rate::{self, Steps};
use tokenradar_core::limits::{Confidence, RateMethod};
use tokenradar_core::store::{ReadOnlyStore, WINDOW_MATCH_TOLERANCE_MS};

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const EVERY: i64 = 15 * MIN;
const AHEAD: i64 = 3 * HOUR;
/// The reading that stands for `t + 3 h` may be at most this much earlier.
const AHEAD_SLACK: i64 = 30 * MIN;

#[derive(Default)]
struct Tally {
    instants: usize,
    old_some: usize,
    new_some: usize,
    new_sure: usize,
    old_ratio: Vec<f64>,
    new_ratio: Vec<f64>,
    sure_ratio: Vec<f64>,
    uncapped_ratio: Vec<f64>,
    /// Absolute error over the next 3 h in points, a missing rate counting as 0; every instant
    /// with a truth, including those without use.
    old_err: Vec<f64>,
    new_err: Vec<f64>,
    uncapped_err: Vec<f64>,
    old_flips: usize,
    new_flips: usize,
    new_sure_flips: usize,
}

fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some(v[v.len() / 2])
}

/// Median of `estimate / truth` and of `|log2(estimate / truth)|`.
fn summary(v: &[f64]) -> String {
    let mut ratio = v.to_vec();
    let mut err: Vec<f64> = v.iter().map(|r| r.log2().abs()).collect();
    match (median(&mut ratio), median(&mut err)) {
        (Some(r), Some(e)) => format!("× {r:.2} (|log2| {e:.2})"),
        _ => "—".into(),
    }
}

fn points(v: &mut [f64]) -> String {
    median(v).map_or_else(|| "—".into(), |m| format!("{m:.2}"))
}

fn share(part: usize, whole: usize) -> String {
    if whole == 0 {
        return "—".into();
    }
    #[allow(clippy::cast_precision_loss)]
    let s = part as f64 / whole as f64 * 100.0;
    format!("{s:.0} %")
}

fn flips_relative(new: usize, old: usize) -> String {
    if old == 0 {
        return if new == 0 {
            "0 / 0".into()
        } else {
            "old none".into()
        };
    }
    #[allow(clippy::cast_precision_loss)]
    let r = new as f64 / old as f64;
    format!("{r:.2} × old")
}

/// One window's readings of one source, oldest first.
fn replay(readings: &[(Timestamp, f64)], resets_at: Timestamp, tally: &mut Tally) {
    let mut steps = Steps::default();
    let mut last_eval: Option<i64> = None;
    let (mut old_hit, mut new_hit, mut sure_hit) = (None, None, None);
    let mut max = f64::MIN;
    for (i, &(at, percent)) in readings.iter().enumerate() {
        steps.observe(at, percent);
        max = max.max(percent);
        let now = at.as_millis();
        if last_eval.is_some_and(|l| now - l < EVERY) {
            continue;
        }
        last_eval = Some(now);
        // The running maximum three hours later, from a reading close enough to that time.
        let later = readings[i..]
            .iter()
            .take_while(|(t, _)| t.as_millis() <= now + AHEAD)
            .last()
            .filter(|(t, _)| t.as_millis() >= now + AHEAD - AHEAD_SLACK);
        let truth = later.map(|_| {
            let peak = readings[i..]
                .iter()
                .take_while(|(t, _)| t.as_millis() <= now + AHEAD)
                .map(|(_, p)| *p)
                .fold(max, f64::max);
            #[allow(clippy::cast_precision_loss)]
            let minutes = AHEAD as f64 / MIN as f64;
            (peak - max) / minutes
        });
        let window: Vec<(Timestamp, f64)> = readings[..=i]
            .iter()
            .copied()
            .filter(|(t, _)| now - t.as_millis() <= rate::RATE_LOOKBACK_MS)
            .collect();
        let old = rate::theil_sen(&window).map(|r| r.max(0.0));
        let new = steps.rate();
        tally.instants += 1;
        tally.old_some += usize::from(old.is_some());
        tally.new_some += usize::from(new.is_some());
        let sure = new.filter(|r| r.confidence != Confidence::Low);
        // The same edges without the stop cap, to see what the cap does.
        let uncapped = new.map(|_| {
            let e = steps.edges();
            let (a, b) = (e[0], e[e.len() - 1]);
            #[allow(clippy::cast_precision_loss)]
            let minutes = (b.at().as_millis() - a.at().as_millis()) as f64 / MIN as f64;
            (b.percent - a.percent) / minutes
        });
        if let Some(truth) = truth {
            #[allow(clippy::cast_precision_loss)]
            let minutes = AHEAD as f64 / MIN as f64;
            let err = |r: Option<f64>| (r.unwrap_or(0.0) - truth).abs() * minutes;
            tally.old_err.push(err(old));
            tally.new_err.push(err(new.map(|n| n.per_min)));
            tally.uncapped_err.push(err(uncapped));
        }
        tally.new_sure += usize::from(sure.is_some());
        if let Some(truth) = truth.filter(|t| *t > 0.0) {
            if let Some(o) = old.filter(|o| *o > 0.0) {
                tally.old_ratio.push(o / truth);
            }
            if let Some(n) = new.filter(|n| n.per_min > 0.0) {
                tally.new_ratio.push(n.per_min / truth);
            }
            if let Some(n) = sure.filter(|n| n.per_min > 0.0) {
                tally.sure_ratio.push(n.per_min / truth);
            }
            if let Some(n) = uncapped.filter(|n| *n > 0.0) {
                tally.uncapped_ratio.push(n / truth);
            }
        }
        let hit = |r: Option<f64>| {
            r.is_some_and(|r| {
                rate::project(
                    at,
                    max,
                    r,
                    Some(resets_at),
                    Confidence::Low,
                    RateMethod::Steps,
                )
                .will_hit_before_reset
            })
        };
        let flip = |prev: &mut Option<bool>, now: bool| {
            let flipped = prev.is_some_and(|p| p != now);
            *prev = Some(now);
            usize::from(flipped)
        };
        tally.old_flips += flip(&mut old_hit, hit(old));
        tally.new_flips += flip(&mut new_hit, hit(new.map(|n| n.per_min)));
        tally.new_sure_flips += flip(&mut sure_hit, hit(sure.map(|n| n.per_min)));
    }
}

fn label(kind: &LimitKind, scope: Option<&Scope>, source: Source) -> String {
    let scope = scope.map(|s| format!(" {s}")).unwrap_or_default();
    format!("{}{scope} · {}", kind.as_str(), source.as_str())
}

fn main() {
    let mut db = None;
    let mut p = lexopt::Parser::from_env();
    while let Some(arg) = p.next().expect("arguments") {
        match arg {
            lexopt::Arg::Long("db") => db = Some(PathBuf::from(p.value().expect("--db value"))),
            _ => panic!("usage: rate_compare --db <path>"),
        }
    }
    let store = ReadOnlyStore::open(db.expect("--db is required"))
        .expect("open")
        .expect("database exists");
    println!("# Burn rate: 30-min Theil–Sen (old) vs step edges (new)\n");
    println!(
        "| Limit · source | Has a rate (old / new / new ≥ medium) | Rate vs next 3 h: old | new | new ≥ medium | new, no stop cap | Error over 3 h, median points: old / new / no cap | will_hit flips: new | new ≥ medium |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    for account in store.accounts().expect("accounts") {
        let snaps = store
            .snapshots_between(account.id, Timestamp::from_millis(0), Timestamp::now())
            .expect("snapshots");
        let mut groups: Vec<(LimitKind, Option<Scope>, Source)> = Vec::new();
        for s in &snaps {
            let key = (s.kind.clone(), s.scope.clone(), s.source);
            if matches!(s.source, Source::Api | Source::Statusline) && !groups.contains(&key) {
                groups.push(key);
            }
        }
        for (kind, scope, source) in groups {
            let mine: Vec<&LimitObservation> = snaps
                .iter()
                .filter(|s| s.kind == kind && s.scope == scope && s.source == source)
                .collect();
            let mut windows: Vec<(Timestamp, Vec<(Timestamp, f64)>)> = Vec::new();
            for s in mine {
                let Some(r) = s.resets_at else { continue };
                match windows.iter_mut().find(|(w, _)| {
                    (w.as_millis() - r.as_millis()).abs() <= WINDOW_MATCH_TOLERANCE_MS
                }) {
                    Some((_, v)) => v.push((s.observed_at, s.percent)),
                    None => windows.push((r, vec![(s.observed_at, s.percent)])),
                }
            }
            let mut tally = Tally::default();
            for (resets_at, readings) in &windows {
                replay(readings, *resets_at, &mut tally);
            }
            println!(
                "| {} | {} / {} / {} | {} | {} | {} | {} | {} / {} / {} | {} | {} |",
                label(&kind, scope.as_ref(), source),
                share(tally.old_some, tally.instants),
                share(tally.new_some, tally.instants),
                share(tally.new_sure, tally.instants),
                summary(&tally.old_ratio),
                summary(&tally.new_ratio),
                summary(&tally.sure_ratio),
                summary(&tally.uncapped_ratio),
                points(&mut tally.old_err),
                points(&mut tally.new_err),
                points(&mut tally.uncapped_err),
                flips_relative(tally.new_flips, tally.old_flips),
                flips_relative(tally.new_sure_flips, tally.old_flips),
            );
        }
    }
}
