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

## License

[MIT](LICENSE)
