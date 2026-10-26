# TokenRadar

> A tiny, tray-first radar for your AI coding agent usage limits.

**Status: pre-alpha, planning stage.** There is no usable build yet.

TokenRadar shows the following for Claude Code (Codex and others later):

- Your **5-hour and weekly limits** at a glance.
- How each **session and project** contributed to them.
- When you are likely to hit the next limit.

It is designed to stay out of your way:

- A Rust core and a Tauri 2 + Svelte 5 UI.
- Target: ≤ 25 MB of RAM while it sits in the tray.
- Native Windows first; macOS and Linux later.
- English and Turkish UI.

## Principles

- **Honest numbers.** Every value shows its source. Estimates are always labeled as estimates.
- **Invisible footprint.** Near-zero idle CPU, a small memory budget, and no needless disk writes.
- **Private by design.**
  - All data stays local, and there is no telemetry.
  - Conversation content is never read.
  - OAuth tokens are used read-only, only with your consent, and are never refreshed.
- **Observe, never circumvent.** TokenRadar does not try to bypass provider limits.

## Roadmap

| Version | Focus |
|---|---|
| v0.1 | Live limits in the tray, panel, notifications, statusline bridge, CLI |
| v0.2 | Analytics: calibration, per-session attribution, windows archive, patterns |
| v0.3 | Mini widget, multiple accounts, WSL |
| v0.4 | Codex provider |
| v0.5 | macOS and Linux |

Design and planning documents (in Turkish) live in [`docs/`](docs/). The implementation plan is in [`docs/plan/`](docs/plan/README.md).

## Development

### Prerequisites (Windows)

- [rustup](https://rustup.rs/) with the MSVC host toolchain. The exact Rust version is pinned in `rust-toolchain.toml` and installed automatically.
- Visual Studio Build Tools with the "Desktop development with C++" workload.
- [Node.js](https://nodejs.org/) LTS (22.12 or newer) and [pnpm](https://pnpm.io/) (version pinned in `package.json`; `corepack enable` picks it up).
- Microsoft Edge WebView2 Runtime (preinstalled on Windows 11).
- Optional: [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) for `pnpm deny` (`cargo install cargo-deny --locked`).

### Common tasks

```sh
pnpm i                         # install JS dependencies
pnpm dev                       # run the app in development mode (tray icon, no window)
pnpm check:all                 # rustfmt, clippy, cargo test, Biome, svelte-check, Vitest
pnpm fmt                       # format Rust and frontend code
pnpm deny                      # license, advisory and source checks for Rust dependencies
pnpm tauri build --no-bundle   # release binaries in target/release/
```

The first `pnpm dev` compiles the whole Tauri stack and takes a few minutes.

### Layout

| Path | What |
|---|---|
| `crates/tokenradar-core` | Domain, storage, indexer, providers, limit engine (no Tauri dependency) |
| `crates/tokenradar-i18n` | Fluent-based localization shared by the app and the CLI |
| `crates/tokenradar-cli` | The `tokenradar` console binary |
| `app/src-tauri` | The `tokenradar-app` Tauri shell: tray and windows |
| `app/src` | Svelte 5 frontend, one bundle per window |

## License

[MIT](LICENSE)
