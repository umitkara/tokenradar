# CLAUDE.md

Guidance for Claude Code sessions working in this repository.

## What this is

**TokenRadar** is a tray-first desktop app that tracks usage limits of AI coding agents: Claude Code first, then Codex and others. It shows the 5-hour and weekly limits, sessions, projects, and how each session affected each window. It is open source (MIT), local-only, and has no telemetry.

**Status:** Planning complete, no code yet. Implementation follows `docs/plan/`, one step per session.

## Start every session here

1. Read `docs/plan/README.md`: the session protocol and the status table.
2. Open the step file for the current step and read everything in its **"Önce oku"** list.
3. Read the **Devir notu** (handoff note) of the previous step.
4. Follow the 7-step protocol in `docs/plan/README.md`. In short:
   - Branch `step/NN-short-name`.
   - Stop and ask when reality differs from the plan, then record an ADR.
   - Run every command under "Doğrulama".
   - Fill in the handoff note and update the status table.
   - Fast-forward merge to `main`.
5. Stay inside the step's scope. Anything listed under "Kapsam dışı" goes into the handoff note, not into code.

## Language policy (ADR-015)

| What | Language |
|---|---|
| Conversation with the user | **Turkish** |
| Code, identifiers, code comments, commit messages, README, CONTRIBUTING | English |
| `docs/` (vision, architecture, decisions, plan) | Turkish |
| UI strings | Fluent files under `locales/{en,tr}/`. Never hard-code UI text. |

Turkish text must use correct orthography (ç, ğ, ı, İ, ö, ş, ü).

## Map of the docs

| File | Read it for |
|---|---|
| `docs/vision.md` | Problem, principles, roadmap v0.1–v0.5, success criteria |
| `docs/data-sources.md` | Every data source (S1–S6), observed formats, pitfalls |
| `docs/architecture.md` | Process model, repo layout, SQLite schema, limit/calibration/attribution math, IPC, i18n, security |
| `docs/performance-budget.md` | Hard memory, CPU, latency, and size budgets, plus how to measure them |
| `docs/design-language.md` | Color tokens, typography, components, tray icon, EN/TR glossary |
| `docs/decisions.md` | ADRs. Append new ones; never rewrite accepted ones, supersede them instead. |
| `docs/plan/` | Step-by-step implementation plan |
| `testdata/README.md` | Fixture rules |

## Non-negotiables

These come from the ADRs. Breaking one requires a new ADR and the user's approval.

- **Credentials are read-only (ADR-006).**
  - Read `accessToken` only with explicit user consent, only at request time.
  - Never persist or log it; zeroize it after use.
  - **Never read `refreshToken` and never refresh.**
  - On 401, wait until the mtime of `.credentials.json` changes.
  - Never read Codex `auth.json`.
- **No conversation content.**
  - Parsers deserialize only usage and metadata fields. Prompts, responses, tool output, and system instructions are never parsed, stored, or shown.
  - Pre-filter lines with `memchr` and use selective serde structs.
- **No real transcripts in git.** Fixtures are synthetic or anonymized (see `testdata/README.md`).
- **User config is sacred.**
  - Change `~/.claude/settings.json` (statusline install) only through show-diff → user confirms → backup → atomic write.
- **Network allowlist (ADR-013).** Only `api.anthropic.com` (with consent) and GitHub Releases (updates). No telemetry, no analytics.
- **Honest numbers.**
  - Every value carries its source: statusline, API, estimate, or Codex log.
  - Estimates are marked with `≈`, low-confidence values are dimmed.
  - Never present an estimate as a measurement.
- **No limit circumvention.** The app observes; it never tries to bypass or game provider limits.
- **Performance budgets are requirements**, not goals. A step that touches a budgeted path measures and records results under `docs/perf/`.
- **Ask the user before** creating the GitHub repo, pushing, tagging, publishing a release, or spending money (for example on code signing or an Apple Developer ID).

## Architecture in one paragraph

- **Core:** `tokenradar-core` is a Rust crate with no Tauri dependency. It holds the domain, the SQLite store (WAL, single writer), the incremental JSONL indexer, providers, the limit engine, calibration, attribution, and the IPC protocol.
- **GUI:** `tokenradar-app` is a Tauri 2 shell with a Svelte 5 multi-page frontend. Windows are created on demand and destroyed after a warm TTL.
- **CLI:** `tokenradar` is a separate small console binary. It is used for the Claude Code statusline bridge and for scripting. The binary names differ because Windows filesystems are case-insensitive (ADR-017).
- **Limit sources, in fusion priority:** statusline `rate_limits` > OAuth usage API > estimate.
- **i18n:** Fluent files in `locales/` are shared by the Rust and TypeScript sides.

## Conventions

- **Commits:** Conventional Commits (`feat:`, `fix:`, `docs:`, `perf:`, `refactor:`, `test:`, `chore:`), in English.
- **Rust:** stable, edition 2024. `cargo fmt` and `cargo clippy -- -D warnings` must be clean.
- **Frontend:** pnpm. `pnpm -C app check` and `pnpm -C app test` must be clean.
- Do not do computation in the UI. View models come from the backend through `tauri-specta` typed bindings.
- No continuous animations (ADR-019).
