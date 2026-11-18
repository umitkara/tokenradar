//! Synthetic end-to-end tests of calibration (Step 13): a simulator with a known `k` and
//! family weight, Gaussian noise, external usage in about 10 % of the hours and whole-percent
//! readings drives [`calibrate`] through an in-memory [`CalibrationData`].

use std::cell::RefCell;

use super::*;
use crate::domain::AccountId;

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;
const T0: i64 = 1_800_000_000_000;
const ACCOUNT: AccountId = AccountId(1);

/// xorshift64* with Box–Muller; deterministic per seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        #[allow(clippy::cast_precision_loss)]
        let v = (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64;
        v / 9_007_199_254_740_992.0
    }

    fn gauss(&mut self) -> f64 {
        let (u1, u2) = (self.next().max(1e-12), self.next());
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

#[derive(Default)]
struct Fake {
    snaps: Vec<LimitObservation>,
    /// `(ts, model, family, micro-USD)`, sorted by time.
    events: Vec<(i64, &'static str, ModelFamily, i64)>,
    queries: RefCell<u32>,
}

impl CalibrationData for Fake {
    fn snapshots(
        &self,
        _: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<Vec<LimitObservation>, StoreError> {
        Ok(self
            .snaps
            .iter()
            .filter(|s| s.observed_at >= from && s.observed_at < to)
            .cloned()
            .collect())
    }

    fn model_costs(
        &self,
        _: AccountId,
        from: Timestamp,
        to: Timestamp,
    ) -> Result<Vec<ModelCost>, StoreError> {
        *self.queries.borrow_mut() += 1;
        let mut by: BTreeMap<&str, (ModelFamily, i64)> = BTreeMap::new();
        let lo = self.events.partition_point(|e| e.0 <= from.as_millis());
        let hi = self.events.partition_point(|e| e.0 <= to.as_millis());
        for e in &self.events[lo..hi] {
            by.entry(e.1).or_insert((e.2, 0)).1 += e.3;
        }
        Ok(by
            .into_iter()
            .map(|(m, (f, c))| ModelCost {
                model: m.to_owned(),
                family: f,
                cost: MicroUsd(c),
            })
            .collect())
    }
}

struct Sim {
    days: i64,
    /// `k` in percent per USD of weighted cost, as a function of time.
    k: Box<dyn Fn(i64) -> f64>,
    alpha_fable: f64,
    seed: u64,
}

/// Simulates `days` of usage: active 08–24 h, 5-minute steps, Opus most of the time and Fable
/// in about 30 % of the hours. Each step's true use carries 10 % Gaussian noise; about 10 % of
/// the hours add external usage nobody sees locally. The statusline reports `floor(p)` every
/// 5 minutes while active; the API `round(p)` every 15 minutes.
fn simulate(sim: &Sim) -> Fake {
    let mut rng = Rng(sim.seed);
    let mut fake = Fake::default();
    let mut p = 0.0f64;
    let mut reset = T0 + WEEK;
    let mut fable_hour = false;
    let mut external_hour = false;
    let steps = sim.days * DAY / (5 * MIN);
    for i in 0..steps {
        let t = T0 + i * 5 * MIN;
        if t >= reset {
            reset += WEEK;
            p = 0.0;
        }
        let hour_of_day = (t - T0) % DAY / HOUR;
        let active = hour_of_day >= 8;
        if (t - T0) % HOUR == 0 {
            fable_hour = rng.next() < 0.3;
            external_hour = rng.next() < 0.1;
        }
        if active {
            let (model, family, mean) = if fable_hour {
                ("claude-fable-5-2", ModelFamily::Fable, 0.25)
            } else {
                ("claude-opus-5-5", ModelFamily::Opus, 0.60)
            };
            let usd = (mean * (1.0 + 0.5 * rng.gauss())).max(0.01);
            #[allow(clippy::cast_possible_truncation)]
            fake.events
                .push((t + 2 * MIN, model, family, (usd * 1e6) as i64));
            let weight = if family == ModelFamily::Fable {
                sim.alpha_fable
            } else {
                1.0
            };
            let noise = (1.0 + 0.1 * rng.gauss()).max(0.0);
            p += (sim.k)(t) * usd * weight * noise;
            if external_hour {
                // About half again the hour's local use, invisible locally.
                p += 0.012;
            }
            let observed = t + 4 * MIN;
            let snap = |percent: f64, source: Source| LimitObservation {
                account: ACCOUNT,
                kind: LimitKind::WeeklyAll,
                scope: None,
                percent,
                resets_at: Some(Timestamp::from_millis(reset)),
                severity: None,
                source,
                session_uuid: None,
                observed_at: Timestamp::from_millis(observed),
            };
            fake.snaps.push(snap(p.floor(), Source::Statusline));
            if i % 3 == 0 {
                fake.snaps.push(snap(p.round(), Source::Api));
            }
        }
    }
    fake
}

fn weekly(cals: &[Calibration]) -> &Calibration {
    cals.iter()
        .find(|c| c.kind == LimitKind::WeeklyAll)
        .expect("weekly_all calibrated")
}

#[test]
fn recovers_a_known_k_and_family_weight_within_five_percent() {
    for seed in [1, 7, 42, 2026] {
        let fake = simulate(&Sim {
            days: 14,
            k: Box::new(|_| 0.045),
            alpha_fable: 2.2,
            seed,
        });
        let now = Timestamp::from_millis(T0 + 14 * DAY);
        let cals = calibrate(&fake, ACCOUNT, now).expect("calibrate");
        let w = weekly(&cals);
        assert_eq!(w.source, Source::Statusline);
        let k_err = (w.k / 0.045 - 1.0).abs();
        assert!(k_err < 0.05, "seed {seed}: k {} ({k_err:.3})", w.k);
        let a = w.alpha.get(&ModelFamily::Fable).copied().unwrap_or(1.0);
        assert!((a / 2.2 - 1.0).abs() < 0.10, "seed {seed}: alpha {a}");
        assert!(w.drift.is_none(), "seed {seed}: false drift {:?}", w.drift);
        assert_eq!(w.level(), Confidence::High, "seed {seed}: {}", w.confidence);
    }
}

#[test]
fn a_single_k_holds_when_families_cost_alike() {
    let fake = simulate(&Sim {
        days: 14,
        k: Box::new(|_| 0.045),
        alpha_fable: 1.0,
        seed: 3,
    });
    let cals = calibrate(&fake, ACCOUNT, Timestamp::from_millis(T0 + 14 * DAY)).expect("calibrate");
    let w = weekly(&cals);
    assert!(w.alpha.is_empty(), "H1 holds, yet alpha {:?}", w.alpha);
    assert!((w.k / 0.045 - 1.0).abs() < 0.05);
}

#[test]
fn detects_an_abrupt_change_of_k_within_a_day() {
    let change = T0 + 10 * DAY + 9 * HOUR;
    for seed in [5, 11, 99] {
        let fake = simulate(&Sim {
            days: 14,
            k: Box::new(move |t| if t < change { 0.045 } else { 0.045 * 1.5 }),
            alpha_fable: 2.2,
            seed,
        });
        let mut found = None;
        let mut t = change;
        while t <= change + 2 * DAY {
            let cals = calibrate(&fake, ACCOUNT, Timestamp::from_millis(t)).expect("calibrate");
            if let Some(d) = weekly(&cals).drift
                && d.detected_at.as_millis() >= change - 6 * HOUR
            {
                assert_eq!(d.direction, DriftDirection::Up);
                found = Some(t);
                break;
            }
            t += HOUR;
        }
        let at = found.unwrap_or_else(|| panic!("seed {seed}: no drift within 2 days"));
        // Only active hours produce intervals; 24 h of wall time is the plan's bound.
        assert!(
            at - change <= DAY,
            "seed {seed}: drift after {} h",
            (at - change) / HOUR
        );
        // After the alarm, k relearns toward the new level within a day.
        let later = calibrate(&fake, ACCOUNT, Timestamp::from_millis(at + DAY)).expect("calibrate");
        let k = weekly(&later).k;
        assert!(
            (k / (0.045 * 1.5) - 1.0).abs() < 0.10,
            "seed {seed}: relearned k {k}"
        );
    }
}

#[test]
fn a_constant_k_raises_no_drift_over_time() {
    let fake = simulate(&Sim {
        days: 14,
        k: Box::new(|_| 0.045),
        alpha_fable: 2.2,
        seed: 17,
    });
    for day in 4..=14 {
        let cals =
            calibrate(&fake, ACCOUNT, Timestamp::from_millis(T0 + day * DAY)).expect("calibrate");
        let w = weekly(&cals);
        assert!(
            w.visible_drift(Timestamp::from_millis(T0 + day * DAY))
                .is_none(),
            "day {day}: {:?}",
            w.drift
        );
    }
}

#[test]
fn few_intervals_give_low_confidence() {
    let fake = simulate(&Sim {
        days: 1,
        k: Box::new(|_| 0.045),
        alpha_fable: 2.2,
        seed: 8,
    });
    let cals = calibrate(&fake, ACCOUNT, Timestamp::from_millis(T0 + DAY)).expect("calibrate");
    let w = weekly(&cals);
    assert!(w.n_obs < 30);
    assert_ne!(w.level(), Confidence::High);
}

#[test]
fn the_api_series_is_used_when_the_statusline_has_too_few_intervals() {
    let mut fake = simulate(&Sim {
        days: 7,
        k: Box::new(|_| 0.045),
        alpha_fable: 1.0,
        seed: 21,
    });
    fake.snaps.retain(|s| s.source == Source::Api);
    let cals = calibrate(&fake, ACCOUNT, Timestamp::from_millis(T0 + 7 * DAY)).expect("calibrate");
    let w = weekly(&cals);
    assert_eq!(w.source, Source::Api);
    assert!((w.k / 0.045 - 1.0).abs() < 0.10, "k {}", w.k);
}

#[test]
fn a_scoped_limit_counts_only_its_family() {
    let mut fake = simulate(&Sim {
        days: 7,
        k: Box::new(|_| 0.045),
        alpha_fable: 2.0,
        seed: 4,
    });
    // A Fable-only limit at twice the weekly rate per Fable dollar (half the allowance).
    let mut p = 0.0;
    let mut scoped = Vec::new();
    let mut i = 0usize;
    for s in fake.snaps.iter().filter(|s| s.source == Source::Api) {
        while i < fake.events.len() && fake.events[i].0 <= s.observed_at.as_millis() {
            if fake.events[i].2 == ModelFamily::Fable {
                #[allow(clippy::cast_precision_loss)]
                let usd = fake.events[i].3 as f64 / 1e6;
                p += 0.18 * usd;
            }
            i += 1;
        }
        scoped.push(LimitObservation {
            kind: LimitKind::WeeklyScoped,
            scope: Some(Scope::Model("Fable".into())),
            percent: p.round(),
            ..s.clone()
        });
    }
    fake.snaps.extend(scoped);
    fake.snaps.sort_by_key(|s| s.observed_at);
    let cals = calibrate(&fake, ACCOUNT, Timestamp::from_millis(T0 + 7 * DAY)).expect("calibrate");
    let s = cals
        .iter()
        .find(|c| c.kind == LimitKind::WeeklyScoped)
        .expect("scoped calibrated");
    assert_eq!(s.family(), Some(ModelFamily::Fable));
    assert!(s.alpha.is_empty());
    assert!((s.k / 0.18 - 1.0).abs() < 0.10, "k {}", s.k);
}

#[test]
fn intervals_dominated_by_unpriced_models_are_left_out() {
    let costs = [
        ModelCost {
            model: "claude-opus-5-5".into(),
            family: ModelFamily::Opus,
            cost: MicroUsd(800),
        },
        ModelCost {
            model: "claude-next-9".into(),
            family: ModelFamily::Unknown,
            cost: MicroUsd(200),
        },
    ];
    let unpriced: BTreeSet<String> = ["claude-next-9".to_owned()].into();
    assert!(family_costs(&costs, None, &unpriced).is_none());
    assert!(family_costs(&costs[..1], None, &unpriced).is_some());
    // A scoped limit ignores other families entirely.
    assert!(family_costs(&costs, Some(ModelFamily::Opus), &unpriced).is_some());
}

#[test]
fn a_new_row_is_stored_only_on_a_meaningful_change_or_once_a_day() {
    let base = Calibration {
        kind: LimitKind::WeeklyAll,
        scope: None,
        source: Source::Statusline,
        k: 0.045,
        alpha: BTreeMap::from([(ModelFamily::Fable, 2.2)]),
        fitted: vec![ModelFamily::Fable],
        confidence: 0.8,
        n_obs: 90,
        mad_rel: 0.2,
        drift: None,
        computed_at: Timestamp::from_millis(T0),
        version: 3,
    };
    let at = |ms: i64, k: f64| Calibration {
        k,
        computed_at: Timestamp::from_millis(T0 + ms),
        ..base.clone()
    };
    assert!(should_store(None, &base));
    assert!(!should_store(Some(&base), &at(HOUR, 0.0455)));
    assert!(should_store(Some(&base), &at(HOUR, 0.046)));
    assert!(should_store(Some(&base), &at(DAY, 0.045)));
    let lower = Calibration {
        confidence: 0.5,
        ..at(HOUR, 0.045)
    };
    assert!(should_store(Some(&base), &lower));
    let alpha = Calibration {
        alpha: BTreeMap::from([(ModelFamily::Fable, 2.5)]),
        ..at(HOUR, 0.045)
    };
    assert!(should_store(Some(&base), &alpha));
}

#[test]
fn confidence_factors_behave() {
    // Plenty of tight, fresh, fully fitted data → high.
    assert!(estimator::confidence(90, 0.2, 60.0, 0, 0.0) > 0.7);
    // Few intervals → low.
    assert!(estimator::confidence(8, 0.2, 8.0, 0, 0.0) < 0.4);
    // A week without new intervals halves it about twice.
    let fresh = estimator::confidence(90, 0.2, 60.0, 0, 0.0);
    let stale = estimator::confidence(90, 0.2, 60.0, 7 * DAY, 0.0);
    assert!(stale < fresh * 0.3);
    // Most cost on a family without its own α → uncertain.
    assert!(estimator::confidence(90, 0.2, 60.0, 0, 0.7) < 0.4);
}

#[test]
fn weighted_median_respects_weights() {
    let mut v = [(1.0, 1.0), (2.0, 1.0), (10.0, 5.0)];
    assert_eq!(estimator::weighted_median(&mut v), Some(10.0));
    let mut empty: [(f64, f64); 0] = [];
    assert_eq!(estimator::weighted_median(&mut empty), None);
}

#[test]
fn usage_of_an_unlearned_family_lowers_the_confidence_of_applying_k() {
    let cal = Calibration {
        kind: LimitKind::WeeklyAll,
        scope: None,
        source: Source::Statusline,
        k: 0.045,
        alpha: BTreeMap::new(),
        fitted: Vec::new(),
        confidence: 0.9,
        n_obs: 60,
        mad_rel: 0.1,
        drift: None,
        computed_at: Timestamp::from_millis(T0),
        version: 1,
    };
    let cost = |family, micro| ModelCost {
        model: String::new(),
        family,
        cost: MicroUsd(micro),
    };
    let opus = [cost(ModelFamily::Opus, 1000)];
    assert!((cal.confidence_for(&opus) - 0.9).abs() < 1e-9);
    let mixed = [cost(ModelFamily::Opus, 250), cost(ModelFamily::Fable, 750)];
    assert!((cal.confidence_for(&mixed) - 0.225).abs() < 1e-9);
    let learned = Calibration {
        fitted: vec![ModelFamily::Fable],
        ..cal.clone()
    };
    assert!((learned.confidence_for(&mixed) - 0.9).abs() < 1e-9);
    // Nothing used: the learned confidence stands.
    assert!((cal.confidence_for(&[]) - 0.9).abs() < 1e-9);
}
