//! `calibrations` rows (Step 13, ADR-061) and the per-model cost query calibration reads.

use rusqlite::{Connection, params};

use super::Result;
use crate::calibration::{Calibration, ModelCost, StoredDetails};
use crate::domain::{AccountId, LimitKind, MicroUsd, ModelFamily, Scope, Timestamp};

/// Stores `cal` as the account's next version and returns that version.
pub(crate) fn insert(conn: &Connection, account: AccountId, cal: &Calibration) -> Result<i64> {
    let version: i64 = conn
        .prepare_cached(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM calibrations WHERE account_id = ?1",
        )?
        .query_row([account], |r| r.get(0))?;
    let details = StoredDetails {
        alpha: cal.alpha.clone(),
        fitted: cal.fitted.clone(),
        source: cal.source,
        mad_rel: cal.mad_rel,
        drift: cal.drift,
    };
    let json = serde_json::to_string(&details).unwrap_or_else(|_| "{}".to_owned());
    conn.prepare_cached(
        "INSERT INTO calibrations
           (account_id, limit_kind, scope, k, alpha_json, confidence, n_obs, version, computed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?
    .execute(params![
        account,
        cal.kind.as_str(),
        cal.scope.as_ref().map(ToString::to_string),
        cal.k,
        json,
        cal.confidence,
        cal.n_obs,
        version,
        cal.computed_at
    ])?;
    Ok(version)
}

/// The newest row per `(limit_kind, scope)` of an account.
pub(crate) fn latest(conn: &Connection, account: AccountId) -> Result<Vec<Calibration>> {
    let mut stmt = conn.prepare_cached(
        "SELECT c.limit_kind, c.scope, c.k, c.alpha_json, c.confidence, c.n_obs, c.version,
                c.computed_at
         FROM calibrations c
         WHERE c.account_id = ?1
           AND c.version = (SELECT MAX(version) FROM calibrations d
                            WHERE d.account_id = c.account_id AND d.limit_kind = c.limit_kind
                              AND d.scope IS c.scope)
         ORDER BY c.limit_kind, c.scope",
    )?;
    let rows = stmt.query_map([account], |r| {
        let kind: String = r.get(0)?;
        let scope: Option<String> = r.get(1)?;
        let json: Option<String> = r.get(3)?;
        Ok((
            kind,
            scope,
            r.get::<_, f64>(2)?,
            json,
            r.get::<_, f64>(4)?,
            r.get::<_, u32>(5)?,
            r.get::<_, i64>(6)?,
            r.get::<_, Timestamp>(7)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (kind, scope, k, json, confidence, n_obs, version, computed_at) = row?;
        let Some(details) = json.and_then(|j| serde_json::from_str::<StoredDetails>(&j).ok())
        else {
            continue;
        };
        out.push(Calibration {
            kind: LimitKind::from(kind.as_str()),
            scope: scope.and_then(|s| s.parse::<Scope>().ok()),
            source: details.source,
            k,
            alpha: details.alpha,
            fitted: details.fitted,
            confidence,
            n_obs,
            mad_rel: details.mad_rel,
            drift: details.drift,
            computed_at,
            version,
        });
    }
    Ok(out)
}

/// Cost per model of the account's events with `from < ts <= to`.
pub(crate) fn model_costs_between(
    conn: &Connection,
    account: AccountId,
    from: Timestamp,
    to: Timestamp,
) -> Result<Vec<ModelCost>> {
    let mut stmt = conn.prepare_cached(
        "SELECT m.name, m.family, SUM(e.cost_micro_usd) FROM usage_events e
         JOIN models m ON m.id = e.model_id
         WHERE e.account_id = ?1 AND e.ts > ?2 AND e.ts <= ?3
         GROUP BY e.model_id",
    )?;
    let rows = stmt.query_map(params![account, from, to], |r| {
        let family: String = r.get(1)?;
        Ok(ModelCost {
            model: r.get(0)?,
            family: family.parse().unwrap_or(ModelFamily::Unknown),
            cost: MicroUsd(r.get(2)?),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::calibration::{Drift, DriftDirection};
    use crate::domain::Source;
    use crate::store::events;
    use crate::store::testutil::{memory_db, seed};

    fn cal(kind: LimitKind, scope: Option<Scope>, k: f64, at: i64) -> Calibration {
        Calibration {
            kind,
            scope,
            source: Source::Statusline,
            k,
            alpha: BTreeMap::from([(ModelFamily::Fable, 2.2)]),
            fitted: vec![ModelFamily::Fable],
            confidence: 0.8,
            n_obs: 42,
            mad_rel: 0.12,
            drift: Some(Drift {
                detected_at: Timestamp::from_millis(at),
                since: Timestamp::from_millis(at - 1000),
                direction: DriftDirection::Up,
            }),
            computed_at: Timestamp::from_millis(at),
            version: 0,
        }
    }

    #[test]
    fn versions_grow_and_latest_returns_the_newest_row_per_limit() {
        let conn = memory_db();
        let f = seed(&conn);
        let fable = Some(Scope::Model("Fable".into()));
        assert_eq!(
            insert(&conn, f.account, &cal(LimitKind::WeeklyAll, None, 0.04, 1)).expect("store"),
            1
        );
        assert_eq!(
            insert(
                &conn,
                f.account,
                &cal(LimitKind::WeeklyScoped, fable.clone(), 0.17, 2)
            )
            .expect("store"),
            2
        );
        assert_eq!(
            insert(&conn, f.account, &cal(LimitKind::WeeklyAll, None, 0.05, 3)).expect("store"),
            3
        );

        let latest = latest(&conn, f.account).expect("store");
        assert_eq!(latest.len(), 2);
        let weekly = latest
            .iter()
            .find(|c| c.kind == LimitKind::WeeklyAll)
            .expect("store");
        assert_eq!(weekly.version, 3);
        assert!((weekly.k - 0.05).abs() < 1e-12);
        // Everything round-trips through `alpha_json`.
        let mut expected = cal(LimitKind::WeeklyAll, None, 0.05, 3);
        expected.version = 3;
        assert_eq!(weekly, &expected);
        let scoped = latest
            .iter()
            .find(|c| c.kind == LimitKind::WeeklyScoped)
            .expect("store");
        assert_eq!(scoped.scope, fable);
    }

    #[test]
    fn model_costs_cover_the_half_open_range_per_model() {
        let mut conn = memory_db();
        let f = seed(&conn);
        let batch = [
            f.event(1, 10, 1_000, 0, 0),
            f.event(2, 20, 2_000, 0, 0),
            f.event(3, 30, 2_000, 1, 1),
            f.event(4, 40, 3_000, 0, 0),
        ];
        let tx = conn.transaction().expect("store");
        events::upsert(&tx, &batch).expect("store");
        tx.commit().expect("store");
        let mut costs = model_costs_between(
            &conn,
            f.account,
            Timestamp::from_millis(1_000),
            Timestamp::from_millis(2_000),
        )
        .expect("store");
        costs.sort_by(|a, b| a.model.cmp(&b.model));
        // `ts = 1000` is excluded, `ts = 2000` included.
        assert_eq!(costs.len(), 2);
        assert_eq!(costs[0].model, "claude-opus-5-5");
        assert_eq!(costs[0].family, ModelFamily::Opus);
        assert_eq!(costs[0].cost, MicroUsd(20 * 11 + 5));
        assert_eq!(costs[1].family, ModelFamily::Sonnet);
        assert_eq!(costs[1].cost, MicroUsd(30 * 11 + 5));
    }
}
