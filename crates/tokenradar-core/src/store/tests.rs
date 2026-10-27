// Synthetic counts are tiny; sign and precision casts cannot lose information here.
#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::collections::BTreeMap;

use proptest::prelude::*;
use rusqlite::Connection;

use super::testutil::{Fixture, account_spec, memory_db, seed};
use super::*;
use crate::domain::{BUCKET_MS, LimitKind, Scope, SessionKind, Source};

fn upsert(conn: &mut Connection, batch: &[UsageEvent]) -> UpsertStats {
    let tx = conn.transaction().expect("tx");
    let stats = events::upsert(&tx, batch).expect("upsert");
    tx.commit().expect("commit");
    stats
}

/// `usage_5m` as `(session, model, bucket) → [input, output, cache_read, w5m, w1h, cost, events]`.
type Rollup = BTreeMap<(i64, i64, i64), [i64; 7]>;

fn rollup(conn: &Connection) -> Rollup {
    let mut stmt = conn
        .prepare(
            "SELECT session_id, model_id, bucket_ts, input, output, cache_read, cache_w5m,
                    cache_w1h, cost_micro_usd, events FROM usage_5m",
        )
        .expect("prepare");
    stmt.query_map([], |r| {
        Ok((
            (r.get(0)?, r.get(1)?, r.get(2)?),
            [
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
                r.get(9)?,
            ],
        ))
    })
    .expect("query")
    .collect::<rusqlite::Result<_>>()
    .expect("rows")
}

/// The rollup computed from scratch out of the final event per key.
fn expected_rollup(finals: impl IntoIterator<Item = UsageEvent>) -> Rollup {
    let mut out = Rollup::new();
    for e in finals {
        let t = e.usage.tokens;
        let row = out
            .entry((e.session_id.0, e.model_id.0, e.ts.bucket_5m().as_millis()))
            .or_default();
        for (slot, v) in row.iter_mut().zip([
            t.input as i64,
            t.output as i64,
            t.cache_read as i64,
            t.cache_w5m as i64,
            t.cache_w1h as i64,
            e.cost.0,
            1,
        ]) {
            *slot += v;
        }
    }
    out
}

fn event_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM usage_events", [], |r| r.get(0))
        .expect("count")
}

// --- Migrations and PRAGMAs ---------------------------------------------------------------

#[test]
fn migrations_run_once_and_reopen_is_idempotent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("nested").join("tokenradar.db");
    let account = {
        let store = Store::open(&path).expect("first open");
        store
            .writer()
            .upsert_account(account_spec("C:/a"))
            .expect("account")
    };
    let store = Store::open(&path).expect("second open");
    let (version, tables): (i64, i64) = store
        .read(|c| {
            Ok((
                c.query_row("PRAGMA user_version", [], |r| r.get(0))?,
                c.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
                    [],
                    |r| r.get(0),
                )?,
            ))
        })
        .expect("read");
    assert_eq!(version, migrate::LATEST);
    assert_eq!(tables, 14);
    // Data survives and the account keeps its id.
    assert_eq!(
        store
            .writer()
            .upsert_account(account_spec("C:/a"))
            .expect("account"),
        account
    );
}

#[test]
fn newer_schema_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("db");
    drop(Store::open(&path).expect("open"));
    Connection::open(&path)
        .expect("raw")
        .pragma_update(None, "user_version", 99)
        .expect("bump");
    assert!(matches!(
        Store::open(&path),
        Err(StoreError::SchemaTooNew { found: 99, supported }) if supported == migrate::LATEST
    ));
}

#[test]
fn writer_pragmas_are_applied() {
    use rusqlite::types::Value::{Integer, Text};

    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path().join("db")).expect("open");
    let pragmas = store
        .writer()
        .call(|c| {
            let get = |p: &str| {
                c.query_row(&format!("PRAGMA {p}"), [], |r| {
                    r.get::<_, rusqlite::types::Value>(0)
                })
            };
            Ok([
                get("journal_mode")?,
                get("synchronous")?,
                get("foreign_keys")?,
                get("busy_timeout")?,
                get("temp_store")?,
                get("cache_size")?,
                get("auto_vacuum")?,
            ])
        })
        .expect("pragmas");
    assert_eq!(
        pragmas,
        [
            Text("wal".into()),
            Integer(1),
            Integer(1),
            Integer(5000),
            Integer(2),
            Integer(-8000),
            Integer(2)
        ]
    );
}

#[test]
fn foreign_keys_are_enforced() {
    let mut conn = memory_db();
    let fx = seed(&conn);
    let mut e = fx.event(1, 10, 0, 0, 0);
    e.session_id = SessionId(9999);
    let tx = conn.transaction().expect("tx");
    assert!(events::upsert(&tx, &[e]).is_err());
}

// --- Deduplication ------------------------------------------------------------------------

#[test]
fn same_key_growing_output_replaces_and_diffs_the_rollup() {
    let mut conn = memory_db();
    let fx = seed(&conn);
    let partial = fx.event(1, 5, 1_000, 0, 0);
    let complete = fx.event(1, 900, 1_000, 0, 0);

    assert_eq!(upsert(&mut conn, &[partial]).inserted, 1);
    let stats = upsert(&mut conn, std::slice::from_ref(&complete));
    assert_eq!(
        stats,
        UpsertStats {
            updated: 1,
            ..UpsertStats::default()
        }
    );

    assert_eq!(event_count(&conn), 1);
    assert_eq!(rollup(&conn), expected_rollup([complete]));
}

#[test]
fn same_key_shrinking_output_keeps_the_stored_event() {
    let mut conn = memory_db();
    let fx = seed(&conn);
    let complete = fx.event(1, 900, 1_000, 0, 0);
    let partial = fx.event(1, 5, 1_000, 0, 0);

    upsert(&mut conn, std::slice::from_ref(&complete));
    let stats = upsert(&mut conn, &[partial]);
    assert_eq!(
        stats,
        UpsertStats {
            kept_existing: 1,
            ..UpsertStats::default()
        }
    );
    assert_eq!(rollup(&conn), expected_rollup([complete]));
}

#[test]
fn equal_output_takes_the_latest_line() {
    let mut conn = memory_db();
    let fx = seed(&conn);
    let first = fx.event(1, 50, 1_000, 0, 0);
    let later = fx.event(1, 50, 2_000, 1, 0); // e.g. a sidechain copy in another file
    upsert(&mut conn, &[first, later.clone()]);
    assert_eq!(rollup(&conn), expected_rollup([later]));
}

#[test]
fn same_key_in_another_bucket_moves_between_buckets() {
    let mut conn = memory_db();
    let fx = seed(&conn);
    let early = fx.event(1, 5, BUCKET_MS - 1, 0, 0);
    let late = fx.event(1, 500, BUCKET_MS + 1, 0, 1);
    // A second event keeps the old bucket alive to check it is decremented, not dropped.
    let neighbour = fx.event(2, 7, 10, 0, 0);

    upsert(&mut conn, &[early, neighbour.clone()]);
    upsert(&mut conn, std::slice::from_ref(&late));
    assert_eq!(rollup(&conn), expected_rollup([neighbour, late.clone()]));

    // When the moved event was alone in its bucket, the emptied row disappears.
    let mut conn = memory_db();
    let fx = seed(&conn);
    upsert(&mut conn, &[fx.event(1, 5, 0, 0, 0)]);
    upsert(&mut conn, &[fx.event(1, 6, BUCKET_MS * 3, 0, 0)]);
    let rows = rollup(&conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows.keys().next().map(|k| k.2), Some(BUCKET_MS * 3));
}

#[test]
fn rollup_totals_match_the_raw_events() {
    let mut conn = memory_db();
    let fx = seed(&conn);
    let batch: Vec<_> = (0..20)
        .map(|i| fx.event(i, u64::from(i) * 10, i64::from(i) * 60_000, 0, 0))
        .collect();
    upsert(&mut conn, &batch);
    let totals = events::rollup_totals(&conn, fx.account).expect("totals");
    let raw: [i64; 7] = conn
        .query_row(
            "SELECT SUM(input), SUM(output), SUM(cache_read), SUM(cache_w5m), SUM(cache_w1h),
                    SUM(cost_micro_usd), COUNT(*) FROM usage_events",
            [],
            |r| {
                Ok([
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ])
            },
        )
        .expect("raw");
    assert_eq!(totals.map(|v| v as i64), raw);
}

/// One generated line: `(key, output, ts, session, model)`.
type Line = (u8, u64, i64, usize, usize);

fn line() -> impl Strategy<Value = Line> {
    (
        0u8..6,
        0u64..40,
        0i64..(4 * BUCKET_MS),
        0usize..2,
        0usize..2,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Duplicate and partial lines in any order and any batch split give the same rollup as
    /// summing only the final event of each key.
    #[test]
    fn rollup_equals_rollup_of_final_events(
        lines in prop::collection::vec(line(), 1..40),
        splits in prop::collection::vec(1usize..8, 1..10),
    ) {
        let mut conn = memory_db();
        let fx: Fixture = seed(&conn);
        let events: Vec<UsageEvent> =
            lines.iter().map(|&(k, out, ts, s, m)| fx.event(k, out, ts, s, m)).collect();

        let mut rest = &events[..];
        let mut sizes = splits.iter().cycle();
        while !rest.is_empty() {
            let n = (*sizes.next().unwrap_or(&1)).min(rest.len());
            upsert(&mut conn, &rest[..n]);
            rest = &rest[n..];
        }

        // Oracle: the latest event with the largest output per key.
        let mut finals: BTreeMap<u8, (u64, UsageEvent)> = BTreeMap::new();
        for (&(key, output, ..), e) in lines.iter().zip(&events) {
            match finals.get(&key) {
                Some((best, _)) if output < *best => {}
                _ => { finals.insert(key, (output, e.clone())); }
            }
        }
        prop_assert_eq!(event_count(&conn), finals.len() as i64);
        prop_assert_eq!(rollup(&conn), expected_rollup(finals.into_values().map(|(_, e)| e)));
    }
}

// --- Retention ----------------------------------------------------------------------------

#[test]
fn pruning_deletes_raw_events_but_keeps_the_rollup() {
    let mut conn = memory_db();
    let fx = seed(&conn);
    let batch: Vec<_> = (0..30)
        .map(|i| fx.event(i, 100 + u64::from(i), i64::from(i) * BUCKET_MS, 0, 0))
        .collect();
    upsert(&mut conn, &batch);
    let before = rollup(&conn);

    let cutoff = Timestamp::from_millis(20 * BUCKET_MS);
    let stats = retention::prune_raw_events(&mut conn, cutoff).expect("prune");
    assert_eq!(stats.deleted, 20);
    assert_eq!(event_count(&conn), 10);
    assert_eq!(rollup(&conn), before);

    // Re-indexing a pruned line must not count it twice.
    let again = upsert(&mut conn, &batch);
    assert_eq!(again.skipped_pruned, 20);
    assert_eq!(again.updated, 10);
    assert_eq!(rollup(&conn), before);

    // The watermark never moves back.
    retention::prune_raw_events(&mut conn, Timestamp::from_millis(0)).expect("prune");
    assert_eq!(
        retention::watermark(&conn, fx.account).expect("wm"),
        Some(cutoff)
    );
}

#[test]
fn accounts_added_after_pruning_index_their_history() {
    let mut conn = memory_db();
    let _fx = seed(&conn);
    retention::prune_raw_events(&mut conn, Timestamp::from_millis(10 * BUCKET_MS)).expect("prune");

    let account =
        catalog::upsert_account(&conn, &account_spec("D:/other"), Timestamp::from_millis(0))
            .expect("acct");
    let session =
        catalog::upsert_session(&conn, &SessionSpec::new(account, "x", SessionKind::Main))
            .expect("s");
    let model = catalog::upsert_model(&conn, "claude-opus-5-5", ModelFamily::Opus).expect("m");
    let fx2 = Fixture {
        account,
        sessions: [session; 2],
        models: [model; 2],
    };
    let stats = upsert(&mut conn, &[fx2.event(1, 10, 0, 0, 0)]);
    assert_eq!(stats.inserted, 1);
}

// --- Catalog, snapshots, kv ---------------------------------------------------------------

#[test]
fn session_upsert_merges_fields_and_widens_the_time_range() {
    let conn = memory_db();
    let fx = seed(&conn);
    let mut spec = SessionSpec::new(fx.account, "abc", SessionKind::Main);
    spec.title = Some("First title".into());
    spec.first_ts = Some(Timestamp::from_millis(500));
    spec.last_ts = Some(Timestamp::from_millis(600));
    let id = catalog::upsert_session(&conn, &spec).expect("insert");

    let mut later = SessionSpec::new(fx.account, "abc", SessionKind::Main);
    later.git_branch = Some("main".into());
    later.first_ts = Some(Timestamp::from_millis(100));
    later.last_ts = Some(Timestamp::from_millis(550));
    assert_eq!(catalog::upsert_session(&conn, &later).expect("merge"), id);

    let stored = catalog::session(&conn, id)
        .expect("read")
        .expect("exists")
        .spec;
    assert_eq!(stored.title.as_deref(), Some("First title"));
    assert_eq!(stored.git_branch.as_deref(), Some("main"));
    assert_eq!(stored.first_ts, Some(Timestamp::from_millis(100)));
    assert_eq!(stored.last_ts, Some(Timestamp::from_millis(600)));
}

#[test]
fn account_and_project_upserts_are_keyed_by_identity() {
    let conn = memory_db();
    let a = catalog::upsert_account(&conn, &account_spec("C:/x"), Timestamp::from_millis(1))
        .expect("a");
    let mut relabeled = account_spec("C:/x");
    relabeled.label = "Work".into();
    relabeled.plan = Some("max".into());
    assert_eq!(
        catalog::upsert_account(&conn, &relabeled, Timestamp::from_millis(2)).expect("a"),
        a
    );
    let stored = catalog::account(&conn, a).expect("read").expect("exists");
    assert_eq!(stored.spec, relabeled);
    assert_eq!(stored.created_at, Timestamp::from_millis(1));

    let spec = ProjectSpec {
        account_id: a,
        path_norm: "d:\\projects\\x".into(),
        display_name: "x".into(),
    };
    let p = catalog::upsert_project(&conn, &spec).expect("p");
    assert_eq!(catalog::upsert_project(&conn, &spec).expect("p"), p);
}

#[test]
fn unknown_model_family_is_upgraded_once_known() {
    let conn = memory_db();
    let id = catalog::upsert_model(&conn, "claude-opus-6", ModelFamily::Unknown).expect("m");
    assert_eq!(
        catalog::upsert_model(&conn, "claude-opus-6", ModelFamily::Opus).expect("m"),
        id
    );
    assert_eq!(
        catalog::upsert_model(&conn, "claude-opus-6", ModelFamily::Unknown).expect("m"),
        id
    );
    let family: String = conn
        .query_row("SELECT family FROM models WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .expect("family");
    assert_eq!(family, "opus");
}

fn observation(
    account: AccountId,
    kind: LimitKind,
    percent: f64,
    at: i64,
    source: Source,
) -> LimitObservation {
    LimitObservation {
        account,
        kind,
        scope: None,
        percent,
        resets_at: Some(Timestamp::from_millis(at + 1000)),
        severity: None,
        source,
        session_uuid: None,
        observed_at: Timestamp::from_millis(at),
    }
}

#[test]
fn snapshots_round_trip_and_estimates_are_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path().join("db")).expect("open");
    let w = store.writer();
    let a = w.upsert_account(account_spec("C:/x")).expect("a");

    w.insert_snapshot(observation(a, LimitKind::Session, 10.0, 100, Source::Api))
        .expect("s");
    w.insert_snapshot(observation(
        a,
        LimitKind::Session,
        20.0,
        200,
        Source::Statusline,
    ))
    .expect("s");
    let mut scoped = observation(a, LimitKind::WeeklyScoped, 5.0, 150, Source::Api);
    scoped.scope = Some(Scope::Model("Fable".into()));
    w.insert_snapshot(scoped.clone()).expect("s");
    w.insert_snapshot(observation(
        a,
        LimitKind::Other("monthly".into()),
        1.0,
        50,
        Source::Api,
    ))
    .expect("s");

    assert!(matches!(
        w.insert_snapshot(observation(
            a,
            LimitKind::Session,
            99.0,
            300,
            Source::Estimate
        )),
        Err(StoreError::EstimateNotPersisted)
    ));

    let latest = store.latest_snapshots(a).expect("latest");
    assert_eq!(latest.len(), 3);
    assert!(latest.contains(&observation(
        a,
        LimitKind::Session,
        20.0,
        200,
        Source::Statusline
    )));
    assert!(latest.contains(&scoped));

    let window = store
        .snapshots_between(a, Timestamp::from_millis(100), Timestamp::from_millis(200))
        .expect("between");
    assert_eq!(
        window
            .iter()
            .map(|o| o.observed_at.as_millis())
            .collect::<Vec<_>>(),
        [100, 150]
    );
}

#[test]
fn kv_set_get_delete() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path().join("db")).expect("open");
    assert_eq!(store.kv_get("k").expect("get"), None);
    store.writer().kv_set("k".into(), "1".into()).expect("set");
    store.writer().kv_set("k".into(), "2".into()).expect("set");
    assert_eq!(store.kv_get("k").expect("get").as_deref(), Some("2"));
    store.writer().kv_delete("k".into()).expect("delete");
    assert_eq!(store.kv_get("k").expect("get"), None);
}

// --- Threading ----------------------------------------------------------------------------

#[test]
fn readers_see_commits_while_the_writer_keeps_going() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path().join("db")).expect("open");
    let fx = store
        .writer()
        .call(|c| {
            let fx = seed(c);
            Ok((fx.account, fx.sessions, fx.models))
        })
        .expect("seed");
    let fx = Fixture {
        account: fx.0,
        sessions: fx.1,
        models: fx.2,
    };

    let reader = {
        let store = store.clone();
        std::thread::spawn(move || {
            let mut last = 0;
            for _ in 0..200 {
                let n: i64 = store
                    .read(|c| {
                        Ok(c.query_row("SELECT COUNT(*) FROM usage_events", [], |r| r.get(0))?)
                    })
                    .expect("read");
                assert!(n >= last, "reads must never go back in time");
                last = n;
            }
        })
    };
    for chunk in 0..50u8 {
        let batch = (0..5)
            .map(|i| fx.event(chunk * 5 + i, 10, 0, 0, 0))
            .collect();
        store.writer().upsert_events(batch).expect("write");
    }
    reader.join().expect("reader thread");
    let n: i64 = store
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM usage_events", [], |r| r.get(0))?))
        .expect("read");
    assert_eq!(n, 250);
}

#[test]
fn calls_after_shutdown_fail_cleanly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path().join("db")).expect("open");
    store.writer().shutdown();
    assert!(matches!(
        store.writer().kv_set("k".into(), "v".into()),
        Err(StoreError::WriterGone)
    ));
}

#[test]
fn release_memory_keeps_the_store_usable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path().join("db")).expect("open");
    store.kv_get("warm").expect("read");
    store.release_memory().expect("release");
    store
        .writer()
        .kv_set("k".into(), "v".into())
        .expect("write");
    assert_eq!(store.kv_get("k").expect("read").as_deref(), Some("v"));
}
