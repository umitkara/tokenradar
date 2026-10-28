# Vendored crates

Third-party crates patched in through `[patch.crates-io]` in the root `Cargo.toml`. Each one is
the published source with a minimal, marked change (search for `TokenRadar patch`). They are
excluded from the workspace, so `cargo fmt` and `cargo clippy` leave them as published.

| Crate | Version | Change | Why | Remove when |
|---|---|---|---|---|
| `notify` | 8.2.1 | `src/windows.rs`: the server waits with `INFINITE` instead of a 100 ms timeout; the wakeup semaphore is shared through `Arc<WakeupSem>`; `configure` wakes the server | The idle watcher thread woke 10 times a second (ADR-038, ADR-039) | [notify-rs/notify#1056](https://github.com/notify-rs/notify/pull/1056) is released |
