//! Step 14 probe: how much of each window the "Other" bucket would take at different
//! aggregation levels. Read-only; prints shares only.
//!
//! ```text
//! cargo run --release -p tokenradar-core --example attr_probe -- <copy.db>
//! ```

use std::collections::BTreeMap;

use tokenradar_core::calibration::{self, CalibrationData};
use tokenradar_core::domain::Timestamp;
use tokenradar_core::store::ReadOnlyStore;

const HOUR: i64 = 3_600_000;

fn main() {
    let path = std::env::args().nth(1).expect("db path");
    let store = ReadOnlyStore::open(&path).expect("open").expect("exists");
    let now = Timestamp::now();
    for account in store.accounts().expect("accounts") {
        let cals = calibration::calibrate(&store, account.id, now).expect("calibrate");
        let snaps = store
            .snapshots(account.id, Timestamp::from_millis(0), now)
            .expect("snaps");
        for s in calibration::series(&snaps) {
            let Some(cal) = cals.iter().find(|c| c.kind == s.kind && c.scope == s.scope) else {
                continue;
            };
            println!(
                "\n## {} {:?} ({:?}) conf {:.2} mad {:.3}",
                s.kind.as_str(),
                s.scope.as_ref().map(ToString::to_string),
                s.source,
                cal.confidence,
                cal.mad_rel
            );
            // window → (to, dp, k·ΔC)
            let mut by_window: BTreeMap<i64, Vec<(i64, f64, f64)>> = BTreeMap::new();
            for iv in &s.intervals {
                let costs = store
                    .model_costs(account.id, iv.from, iv.to)
                    .expect("costs");
                let kc = cal.k * cal.weighted_cost(&costs) / 1e6;
                by_window
                    .entry(iv.resets_at.as_millis())
                    .or_default()
                    .push((iv.to.as_millis(), iv.dp, kc));
            }
            println!("window_end | dp_sum/kC_sum | other% @interval | @6h | @24h | @window");
            for (r, ivs) in &by_window {
                let dp: f64 = ivs.iter().map(|x| x.1).sum();
                let kc: f64 = ivs.iter().map(|x| x.2).sum();
                let other_at = |bucket: i64| -> f64 {
                    let mut g: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
                    for &(to, d, c) in ivs {
                        let e = g.entry(to.div_euclid(bucket)).or_default();
                        e.0 += d;
                        e.1 += c;
                    }
                    g.values().map(|(d, c)| (d - c).max(0.0)).sum::<f64>() / dp * 100.0
                };
                println!(
                    "{} | {:.2} | {:.1} | {:.1} | {:.1} | {:.1}",
                    Timestamp::from_millis(*r),
                    dp / kc,
                    other_at(1),
                    other_at(6 * HOUR),
                    other_at(24 * HOUR),
                    other_at(i64::MAX / 4),
                );
            }
        }
    }
}
