//! Re-pricing stored events when the embedded price table changes (ADR-057).
//!
//! `usage_events.cost_micro_usd` is computed when a line is indexed. A model that the table did
//! not know then was priced with a stand-in (ADR-025), and that cost stays wrong after an update
//! adds the model. So the store remembers which table priced its events (its
//! [`Pricing::fingerprint`]); when the running build carries a different one, every raw event
//! is priced again and `usage_5m` receives the difference.
//!
//! Events already pruned by retention cannot be priced again (the rollup has no per-event
//! speed, web searches or prompt size), so their share of `usage_5m` keeps the old cost.

use std::collections::HashMap;

use rusqlite::{Connection, params};

use super::sql::tokens_from_sql;
use super::{Result, kv};
use crate::domain::{AccountId, ModelId, SessionId, Speed, Timestamp, TokenCounts, Usage};
use crate::pricing::Pricing;

/// Fingerprint of the price table that priced the stored events.
pub(super) const TABLE_KEY: &str = "pricing.table";

/// Events read per transaction, so the writer never holds the lock for long.
pub(super) const CHUNK: i64 = 10_000;

/// What one re-pricing pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RepriceStats {
    /// Raw events looked at.
    pub scanned: usize,
    /// Events whose stored cost differed from the table's.
    pub repriced: usize,
}

/// Prices every raw event again if `pricing` is not the table that priced them. Returns `None`
/// when the fingerprints match.
///
/// An interrupted pass leaves the fingerprint unset and simply runs again; events that already
/// carry the right cost are left alone.
pub(crate) fn reprice_if_table_changed(
    conn: &mut Connection,
    pricing: &Pricing,
) -> Result<Option<RepriceStats>> {
    if kv::get(conn, TABLE_KEY)?.as_deref() == Some(pricing.fingerprint()) {
        return Ok(None);
    }
    let stats = reprice(conn, pricing)?;
    kv::set(conn, TABLE_KEY, pricing.fingerprint())?;
    tracing::info!(
        table = pricing.fingerprint(),
        scanned = stats.scanned,
        repriced = stats.repriced,
        "re-priced stored events with the current price table"
    );
    Ok(Some(stats))
}

fn reprice(conn: &mut Connection, pricing: &Pricing) -> Result<RepriceStats> {
    let models: HashMap<ModelId, String> = conn
        .prepare("SELECT id, name FROM models")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;

    let mut stats = RepriceStats::default();
    let mut after = 0_i64;
    loop {
        let tx = conn.transaction()?;
        let rows: Vec<Stored> = tx
            .prepare_cached(
                "SELECT id, account_id, session_id, model_id, ts, input, output, cache_read,
                        cache_w5m, cache_w1h, web_search, speed, cost_micro_usd
                 FROM usage_events WHERE id > ?1 ORDER BY id LIMIT ?2",
            )?
            .query_map(params![after, CHUNK], Stored::from_row)?
            .collect::<rusqlite::Result<_>>()?;
        let Some(last) = rows.last() else {
            break;
        };
        after = last.id;
        stats.scanned += rows.len();

        // `(account, session, model, bucket) → cost difference` for the rollup.
        let mut rollup: HashMap<(AccountId, SessionId, ModelId, Timestamp), i64> = HashMap::new();
        {
            let mut update =
                tx.prepare_cached("UPDATE usage_events SET cost_micro_usd = ?2 WHERE id = ?1")?;
            for row in &rows {
                // Every event references a model row; a missing name would be a broken database.
                let Some(model) = models.get(&row.model) else {
                    continue;
                };
                let cost = pricing.cost(model, &row.usage).cost.0;
                if cost == row.cost {
                    continue;
                }
                update.execute(params![row.id, cost])?;
                *rollup
                    .entry((row.account, row.session, row.model, row.ts.bucket_5m()))
                    .or_default() += cost - row.cost;
                stats.repriced += 1;
            }
            // An event inside retention is always counted in its bucket, so the row exists.
            let mut shift = tx.prepare_cached(
                "UPDATE usage_5m SET cost_micro_usd = cost_micro_usd + ?5
                 WHERE account_id = ?1 AND session_id = ?2 AND model_id = ?3 AND bucket_ts = ?4",
            )?;
            for ((account, session, model, bucket), diff) in rollup {
                if diff != 0 {
                    shift.execute(params![account, session, model, bucket, diff])?;
                }
            }
        }
        tx.commit()?;
    }
    Ok(stats)
}

/// The part of a stored event that determines its cost, plus its rollup key.
struct Stored {
    id: i64,
    account: AccountId,
    session: SessionId,
    model: ModelId,
    ts: Timestamp,
    usage: Usage,
    cost: i64,
}

impl Stored {
    fn from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get(0)?,
            account: r.get(1)?,
            session: r.get(2)?,
            model: r.get(3)?,
            ts: r.get(4)?,
            usage: Usage {
                tokens: TokenCounts {
                    input: tokens_from_sql(r.get(5)?),
                    output: tokens_from_sql(r.get(6)?),
                    cache_read: tokens_from_sql(r.get(7)?),
                    cache_w5m: tokens_from_sql(r.get(8)?),
                    cache_w1h: tokens_from_sql(r.get(9)?),
                    // Part of `output` and never priced on its own.
                    thinking: 0,
                },
                web_search: r.get(10)?,
                // Free, so the cost does not depend on it.
                web_fetch: 0,
                speed: Speed::from_i64(r.get(11)?),
            },
            cost: r.get(12)?,
        })
    }
}
