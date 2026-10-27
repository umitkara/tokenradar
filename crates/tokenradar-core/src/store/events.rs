//! Usage events: the "max output wins" upsert and the `usage_5m` rollup it maintains.
//!
//! Claude Code writes one line per content block, so the same response appears several
//! times, sometimes with partial usage (`docs/data-sources.md` S3). Per `(account, msg_key)`
//! the stored event is the one with the largest `output`. On ties:
//!
//! - **Same session:** the latest line wins (later lines of one file are more complete).
//! - **Different sessions:** the same response also appears in resumed sessions and in forked
//!   subagents that copy their parent's context. Batches from parallel workers arrive in any
//!   order, so "latest wins" would make the owner random. Instead a main session beats a
//!   subagent, and otherwise the smaller session uuid wins. This is a total order, so the
//!   result does not depend on processing order (ADR-026).
//!
//! The rollup is kept exact by applying differences: when an event replaces an older version,
//! the old values leave the old row's bucket and the new values enter the new bucket (session,
//! model or timestamp may all have changed).

use std::collections::HashMap;

use rusqlite::{OptionalExtension, Transaction, params};

use super::Result;
use super::sql::tokens_to_sql;
use crate::domain::{AccountId, ModelId, SessionId, Timestamp, UsageEvent};

/// What one batch did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UpsertStats {
    /// New keys.
    pub inserted: usize,
    /// Existing keys replaced by an event with `output >=` the stored one.
    pub updated: usize,
    /// Events with a smaller `output` than the stored one; ignored.
    pub kept_existing: usize,
    /// New keys older than the account's prune watermark. Their raw event was deleted by
    /// retention but the rollup still counts it, so inserting again would double count.
    pub skipped_pruned: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BucketKey {
    account: AccountId,
    session: SessionId,
    model: ModelId,
    bucket: Timestamp,
}

/// Signed change to one `usage_5m` row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Delta {
    input: i64,
    output: i64,
    cache_read: i64,
    cache_w5m: i64,
    cache_w1h: i64,
    cost: i64,
    events: i64,
}

impl Delta {
    fn add(&mut self, r: &Values, sign: i64) {
        self.input += sign * r.input;
        self.output += sign * r.output;
        self.cache_read += sign * r.cache_read;
        self.cache_w5m += sign * r.cache_w5m;
        self.cache_w1h += sign * r.cache_w1h;
        self.cost += sign * r.cost;
        self.events += sign;
    }
}

/// The rollup-relevant part of an event, as stored.
struct Values {
    session: SessionId,
    model: ModelId,
    ts: Timestamp,
    input: i64,
    output: i64,
    cache_read: i64,
    cache_w5m: i64,
    cache_w1h: i64,
    cost: i64,
}

impl Values {
    fn of(e: &UsageEvent) -> Self {
        let t = &e.usage.tokens;
        Self {
            session: e.session_id,
            model: e.model_id,
            ts: e.ts,
            input: tokens_to_sql(t.input),
            output: tokens_to_sql(t.output),
            cache_read: tokens_to_sql(t.cache_read),
            cache_w5m: tokens_to_sql(t.cache_w5m),
            cache_w1h: tokens_to_sql(t.cache_w1h),
            cost: e.cost.0,
        }
    }

    fn key(&self, account: AccountId) -> BucketKey {
        BucketKey {
            account,
            session: self.session,
            model: self.model,
            bucket: self.ts.bucket_5m(),
        }
    }
}

/// Upserts `batch` inside `tx`. Order within the batch matters only for ties.
pub(crate) fn upsert(tx: &Transaction<'_>, batch: &[UsageEvent]) -> Result<UpsertStats> {
    let mut stats = UpsertStats::default();
    let mut deltas: HashMap<BucketKey, Delta> = HashMap::new();
    let mut watermarks: HashMap<AccountId, Option<Timestamp>> = HashMap::new();
    let mut ranks: HashMap<SessionId, SessionRank> = HashMap::new();
    let mut rank_stmt =
        tx.prepare_cached("SELECT kind = 'subagent', uuid FROM sessions WHERE id = ?1")?;

    let mut select = tx.prepare_cached(
        "SELECT id, session_id, model_id, ts, input, output, cache_read, cache_w5m, cache_w1h,
                cost_micro_usd
         FROM usage_events WHERE account_id = ?1 AND msg_key = ?2",
    )?;
    let mut insert = tx.prepare_cached(
        "INSERT INTO usage_events (account_id, session_id, model_id, ts, msg_key, input, output,
                                   cache_read, cache_w5m, cache_w1h, thinking, web_search,
                                   web_fetch, speed, cost_micro_usd)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
    )?;
    let mut update = tx.prepare_cached(
        "UPDATE usage_events SET session_id = ?2, model_id = ?3, ts = ?4, input = ?5, output = ?6,
                cache_read = ?7, cache_w5m = ?8, cache_w1h = ?9, thinking = ?10, web_search = ?11,
                web_fetch = ?12, speed = ?13, cost_micro_usd = ?14
         WHERE id = ?1",
    )?;

    for event in batch {
        let new = Values::of(event);
        let t = &event.usage.tokens;
        let old = select
            .query_row(params![event.account_id, event.msg_key], stored_row)
            .optional()?;

        match old {
            None => {
                let watermark = cached_watermark(tx, &mut watermarks, event.account_id)?;
                if watermark.is_some_and(|w| event.ts < w) {
                    stats.skipped_pruned += 1;
                    continue;
                }
                insert.execute(params![
                    event.account_id,
                    event.session_id,
                    event.model_id,
                    event.ts,
                    event.msg_key,
                    new.input,
                    new.output,
                    new.cache_read,
                    new.cache_w5m,
                    new.cache_w1h,
                    tokens_to_sql(t.thinking),
                    event.usage.web_search,
                    event.usage.web_fetch,
                    event.usage.speed.as_i64(),
                    new.cost,
                ])?;
                deltas
                    .entry(new.key(event.account_id))
                    .or_default()
                    .add(&new, 1);
                stats.inserted += 1;
            }
            Some((id, old))
                if new.output > old.output
                    || (new.output == old.output
                        && tie_prefers(&mut ranks, &mut rank_stmt, new.session, old.session)?) =>
            {
                update.execute(params![
                    id,
                    event.session_id,
                    event.model_id,
                    event.ts,
                    new.input,
                    new.output,
                    new.cache_read,
                    new.cache_w5m,
                    new.cache_w1h,
                    tokens_to_sql(t.thinking),
                    event.usage.web_search,
                    event.usage.web_fetch,
                    event.usage.speed.as_i64(),
                    new.cost,
                ])?;
                deltas
                    .entry(old.key(event.account_id))
                    .or_default()
                    .add(&old, -1);
                deltas
                    .entry(new.key(event.account_id))
                    .or_default()
                    .add(&new, 1);
                stats.updated += 1;
            }
            Some(_) => stats.kept_existing += 1,
        }
    }

    apply_deltas(tx, &deltas)?;
    Ok(stats)
}

/// Tie-break order of a session: main before subagent, then by uuid.
type SessionRank = (bool, String);

/// Whether an event of session `new` replaces a stored event of session `old` with the same
/// `output`. Ties across sessions are rare, so ranks are looked up lazily and cached per batch.
fn tie_prefers(
    cache: &mut HashMap<SessionId, SessionRank>,
    stmt: &mut rusqlite::CachedStatement<'_>,
    new: SessionId,
    old: SessionId,
) -> Result<bool> {
    if new == old {
        return Ok(true);
    }
    let mut rank = |id: SessionId| -> Result<SessionRank> {
        if let Some(r) = cache.get(&id) {
            return Ok(r.clone());
        }
        let r: SessionRank = stmt.query_row([id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        cache.insert(id, r.clone());
        Ok(r)
    };
    Ok(rank(new)? < rank(old)?)
}

/// `(id, values)` of a stored event, from the `select` statement in [`upsert`].
fn stored_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<(i64, Values)> {
    Ok((
        r.get(0)?,
        Values {
            session: r.get(1)?,
            model: r.get(2)?,
            ts: r.get(3)?,
            input: r.get(4)?,
            output: r.get(5)?,
            cache_read: r.get(6)?,
            cache_w5m: r.get(7)?,
            cache_w1h: r.get(8)?,
            cost: r.get(9)?,
        },
    ))
}

/// The account's prune watermark, read once per batch.
fn cached_watermark(
    tx: &Transaction<'_>,
    cache: &mut HashMap<AccountId, Option<Timestamp>>,
    account: AccountId,
) -> Result<Option<Timestamp>> {
    if let Some(w) = cache.get(&account) {
        return Ok(*w);
    }
    let w = super::retention::watermark(tx, account)?;
    cache.insert(account, w);
    Ok(w)
}

fn apply_deltas(tx: &Transaction<'_>, deltas: &HashMap<BucketKey, Delta>) -> Result<()> {
    let mut upsert = tx.prepare_cached(
        "INSERT INTO usage_5m (account_id, session_id, model_id, bucket_ts, input, output,
                               cache_read, cache_w5m, cache_w1h, cost_micro_usd, events)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT (account_id, session_id, model_id, bucket_ts) DO UPDATE SET
           input = input + excluded.input,
           output = output + excluded.output,
           cache_read = cache_read + excluded.cache_read,
           cache_w5m = cache_w5m + excluded.cache_w5m,
           cache_w1h = cache_w1h + excluded.cache_w1h,
           cost_micro_usd = cost_micro_usd + excluded.cost_micro_usd,
           events = events + excluded.events",
    )?;
    let mut delete_empty = tx.prepare_cached(
        "DELETE FROM usage_5m
         WHERE account_id = ?1 AND session_id = ?2 AND model_id = ?3 AND bucket_ts = ?4
           AND events = 0",
    )?;
    for (k, d) in deltas {
        if *d == Delta::default() {
            continue; // an update that changed nothing the rollup tracks
        }
        upsert.execute(params![
            k.account,
            k.session,
            k.model,
            k.bucket,
            d.input,
            d.output,
            d.cache_read,
            d.cache_w5m,
            d.cache_w1h,
            d.cost,
            d.events
        ])?;
        if d.events < 0 {
            // The last event left this bucket (it moved to another one).
            delete_empty.execute(params![k.account, k.session, k.model, k.bucket])?;
        }
    }
    Ok(())
}

/// Sum of all rollup rows of an account, for tests and diagnostics:
/// `(input, output, cache_read, cache_w5m, cache_w1h, cost, events)`.
#[cfg(test)]
pub(crate) fn rollup_totals(conn: &rusqlite::Connection, account: AccountId) -> Result<[u64; 7]> {
    let row = conn.query_row(
        "SELECT COALESCE(SUM(input),0), COALESCE(SUM(output),0), COALESCE(SUM(cache_read),0),
                COALESCE(SUM(cache_w5m),0), COALESCE(SUM(cache_w1h),0),
                COALESCE(SUM(cost_micro_usd),0), COALESCE(SUM(events),0)
         FROM usage_5m WHERE account_id = ?1",
        [account],
        |r| {
            Ok([
                r.get::<_, i64>(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ])
        },
    )?;
    Ok(row.map(super::sql::tokens_from_sql))
}
