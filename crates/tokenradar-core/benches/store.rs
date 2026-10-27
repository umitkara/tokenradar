//! Batch upsert throughput of the usage-event store (Step 03 target: ≥ 150k events/s).
//!
//! Each iteration writes 100k synthetic events into a fresh on-disk database (WAL) through the
//! writer thread, in batches the size the indexer will send. Database creation is excluded.
//!
//! Batch size dominates new-key throughput: random `msg_key` hashes spread over the unique
//! index, and every commit rewrites each page it touched. Measured 2026-10-27: 1k → ~100k/s,
//! 2k → ~127k/s, 5k → ~218k/s, 10k → ~255k/s (`docs/perf/2026-10-27-store.md`).

// Indices and synthetic counts stay far below the cast limits.
#![allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use tokenradar_core::domain::{
    AccountSpec, MicroUsd, ModelFamily, ModelId, MsgKey, Provider, SessionId, SessionKind,
    SessionSpec, Timestamp, TokenCounts, Usage, UsageEvent,
};
use tokenradar_core::store::Store;

const EVENTS: usize = 100_000;
const BATCH: usize = 5_000;
const SESSIONS: usize = 200;

struct Db {
    _dir: tempfile::TempDir,
    store: Store,
    events: Vec<UsageEvent>,
}

/// A fresh database plus `EVENTS` events spread over sessions, two models and ~30 days.
fn fresh_db() -> Db {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path().join("bench.db")).expect("open");
    let w = store.writer();
    let account = w
        .upsert_account(AccountSpec {
            provider: Provider::Claude,
            label: "bench".into(),
            config_dir: "C:/bench/.claude".into(),
            plan: None,
            tier: None,
            environment: "native".into(),
        })
        .expect("account");
    let sessions: Vec<SessionId> = (0..SESSIONS)
        .map(|i| {
            w.upsert_session(SessionSpec::new(
                account,
                format!("session-{i}"),
                SessionKind::Main,
            ))
            .expect("session")
        })
        .collect();
    let models: Vec<ModelId> = [
        ("claude-opus-5-5", ModelFamily::Opus),
        ("claude-sonnet-5-5", ModelFamily::Sonnet),
    ]
    .into_iter()
    .map(|(name, family)| w.upsert_model(name.into(), family).expect("model"))
    .collect();

    let start = 1_790_000_000_000_i64;
    let events = (0..EVENTS)
        .map(|i| {
            let n = i as u64;
            UsageEvent {
                account_id: account,
                // Sessions run in sequence, like real usage: consecutive events share one.
                session_id: sessions[i * SESSIONS / EVENTS],
                model_id: models[i % 7 / 6],
                ts: Timestamp::from_millis(start + i as i64 * 26_000),
                msg_key: MsgKey::from_ids(&format!("msg_{i:08}"), &format!("req_{i:08}")),
                usage: Usage {
                    tokens: TokenCounts {
                        input: n % 13,
                        output: 200 + n % 4000,
                        cache_read: 20_000 + n % 50_000,
                        cache_w5m: 0,
                        cache_w1h: n % 9000,
                        thinking: n % 500,
                    },
                    ..Usage::default()
                },
                cost: MicroUsd(100_000 + (n % 50_000) as i64),
            }
        })
        .collect();
    Db {
        _dir: dir,
        store,
        events,
    }
}

fn write_all(db: &Db, events: &[UsageEvent]) {
    for batch in events.chunks(BATCH) {
        black_box(
            db.store
                .writer()
                .upsert_events(batch.to_vec())
                .expect("upsert"),
        );
    }
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("store");
    group.throughput(Throughput::Elements(EVENTS as u64));
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    // First index: every key is new.
    group.bench_function("upsert_100k_new", |b| {
        b.iter_batched(
            fresh_db,
            |db| write_all(&db, &db.events),
            BatchSize::PerIteration,
        );
    });

    // Re-index after an offset reset: every key exists, half grow (update), half shrink (kept).
    group.bench_function("upsert_100k_existing", |b| {
        b.iter_batched(
            || {
                let db = fresh_db();
                write_all(&db, &db.events);
                let again: Vec<UsageEvent> = db
                    .events
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let mut e = e.clone();
                        e.usage.tokens.output = if i % 2 == 0 {
                            e.usage.tokens.output + 1
                        } else {
                            1
                        };
                        e
                    })
                    .collect();
                (db, again)
            },
            |(db, again)| write_all(&db, &again),
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
