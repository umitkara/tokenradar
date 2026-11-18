//! The cost fallback with a calibration (Step 13, ADR-061).

use std::collections::BTreeMap;

use super::*;
use crate::calibration::Calibration;
use crate::limits::RateMethod;

const MIN_MS: i64 = 60 * SEC;
const RESET: i64 = 3 * 24 * 3600 * SEC;
const NOW: i64 = 2 * 3600 * SEC;

fn spend_model(fx: &Fx, key: u32, model: &str, family: ModelFamily, cost: i64, at: i64) {
    let model = fx
        .store
        .writer()
        .upsert_model(model.into(), family)
        .expect("model");
    fx.store
        .writer()
        .upsert_events(vec![UsageEvent {
            account_id: fx.account,
            session_id: fx.sessions[0],
            model_id: model,
            ts: t(at),
            msg_key: MsgKey::from_ids(&format!("c{key}"), "r"),
            usage: Usage::default(),
            cost: MicroUsd(cost),
        }])
        .expect("events");
}

fn calibration(confidence: f64) -> Calibration {
    Calibration {
        kind: LimitKind::WeeklyAll,
        scope: None,
        source: Source::Statusline,
        k: 0.05,
        alpha: BTreeMap::new(),
        fitted: Vec::new(),
        confidence,
        n_obs: 60,
        mad_rel: 0.1,
        drift: None,
        computed_at: t(NOW),
        version: 1,
    }
}

fn weekly(engine: &mut LimitEngine) -> LimitView {
    current(engine, t(NOW))
        .into_iter()
        .find(|l| l.kind == LimitKind::WeeklyAll)
        .expect("weekly view")
}

#[test]
fn a_trusted_calibration_replaces_the_prior_in_the_cost_fallback() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    // One reading: no observed slope, so the cost fallback decides.
    e.ingest(
        &fx.obs(
            LimitKind::WeeklyAll,
            40.0,
            RESET,
            Source::Statusline,
            NOW - MIN_MS,
        ),
        t(NOW - MIN_MS),
    );
    spend_model(
        &fx,
        1,
        "claude-opus-5-5",
        ModelFamily::Opus,
        2_000_000,
        NOW - 5 * MIN_MS,
    );

    let v = weekly(&mut e);
    assert_eq!(v.projection.expect("prior").method, RateMethod::CostRate);
    assert!(v.calibration.is_none());

    e.set_calibrations(fx.account, vec![calibration(0.9)]);
    let v = weekly(&mut e);
    let p = v.projection.expect("calibrated");
    assert_eq!(p.method, RateMethod::Calibrated);
    // User decision: still low confidence.
    assert_eq!(p.confidence, Confidence::Low);
    // 0.05 %/$ × $2 over 15 min.
    assert!((p.rate_per_min - 0.05 * 2.0 / 15.0).abs() < 1e-9);
    let c = v.calibration.expect("view");
    assert_eq!(c.level, Confidence::High);
    assert_eq!(c.n_obs, 60);

    // A weak calibration does not replace the prior.
    e.set_calibrations(fx.account, vec![calibration(0.3)]);
    let v = weekly(&mut e);
    assert_eq!(v.projection.expect("prior").method, RateMethod::CostRate);
    assert_eq!(v.calibration.expect("still shown").level, Confidence::Low);
}

#[test]
fn recent_usage_of_an_unlearned_family_falls_back_to_the_prior() {
    let fx = fixture();
    let mut e = fx.engine(t(0));
    e.ingest(
        &fx.obs(
            LimitKind::WeeklyAll,
            40.0,
            RESET,
            Source::Statusline,
            NOW - MIN_MS,
        ),
        t(NOW - MIN_MS),
    );
    spend_model(
        &fx,
        1,
        "claude-opus-5-5",
        ModelFamily::Opus,
        1_000_000,
        NOW - 5 * MIN_MS,
    );
    spend_model(
        &fx,
        2,
        "claude-fable-5-2",
        ModelFamily::Fable,
        3_000_000,
        NOW - 4 * MIN_MS,
    );
    e.set_calibrations(fx.account, vec![calibration(0.9)]);
    assert_eq!(
        weekly(&mut e).projection.expect("prior").method,
        RateMethod::CostRate
    );

    // Once Fable has its own weight, the calibration applies, weighted.
    let mut learned = calibration(0.9);
    learned.fitted = vec![ModelFamily::Fable];
    learned.alpha = BTreeMap::from([(ModelFamily::Fable, 2.0)]);
    e.set_calibrations(fx.account, vec![learned]);
    let p = weekly(&mut e).projection.expect("calibrated");
    assert_eq!(p.method, RateMethod::Calibrated);
    assert!((p.rate_per_min - 0.05 * (1.0 + 2.0 * 3.0) / 15.0).abs() < 1e-9);
}
