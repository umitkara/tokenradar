# Contributing to TokenRadar

Thanks for your interest. Bug reports, ideas and pull requests are welcome.

- **Bugs:** open an issue with the bug template. Attach a diagnostics bundle (`tokenradar doctor`) if you can; it holds no tokens, conversation content or readable paths, but check it before sharing.
- **Security issues:** do not open a public issue; see [SECURITY.md](SECURITY.md).
- **Larger changes:** open an issue first so we can agree on the approach.

## Ground rules

These come from the architecture decisions in [`docs/decisions.md`](docs/decisions.md) and are not negotiable without a new decision record:

- **No conversation content.** Parsers deserialize only usage and metadata fields. Prompts, responses, tool output and system instructions are never parsed, stored or shown.
- **Credentials are read-only.** The `accessToken` is read only with the user's consent and only at request time; it is never stored or logged. The `refreshToken` is never read and tokens are never refreshed.
- **No real transcripts in git.** Test fixtures are synthetic or anonymized (see [`testdata/README.md`](testdata/README.md)).
- **Network allowlist.** Only `api.anthropic.com` (with consent) and GitHub Releases (updates). No telemetry or analytics.
- **Honest numbers.** Every value carries its source; estimates are marked `≈` and never shown as measurements.
- **No limit circumvention.** TokenRadar observes; it never tries to bypass provider limits.
- **Performance budgets are requirements** ([`docs/performance-budget.md`](docs/performance-budget.md)).

## Development setup (Windows)

- [rustup](https://rustup.rs/) with the MSVC host toolchain. The exact Rust version is pinned in `rust-toolchain.toml` and installed automatically.
- Visual Studio Build Tools with the "Desktop development with C++" workload.
- [Node.js](https://nodejs.org/) 22.12 or newer and [pnpm](https://pnpm.io/) (version pinned in `package.json`; `corepack enable` picks it up).
- Microsoft Edge WebView2 Runtime (preinstalled on Windows 11).
- Optional: [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) for `pnpm deny` (`cargo install cargo-deny --locked`).

```sh
pnpm i                         # install JS dependencies
pnpm dev                       # run the app in development mode (tray icon, no window)
pnpm check:all                 # rustfmt, clippy, cargo test, Biome, svelte-check, Vitest
pnpm fmt                       # format Rust and frontend code
pnpm deny                      # license, advisory and source checks for Rust dependencies
pnpm tauri build               # installer in target/release/bundle/nsis/
.\scripts\package-portable.ps1 # portable zip from the build above
```

The first `pnpm dev` compiles the whole Tauri stack and takes a few minutes.

Running the app or the CLI from `target\` uses your real TokenRadar data and Claude Code settings. To experiment safely, copy the binaries to a folder with an empty `portable` file next to them: everything then stays in that folder's `data` directory.

### Layout

| Path | What |
|---|---|
| `crates/tokenradar-core` | Domain, storage, indexer, providers, limit engine (no Tauri dependency) |
| `crates/tokenradar-i18n` | Fluent-based localization shared by the app and the CLI |
| `crates/tokenradar-cli` | The `tokenradar` console binary |
| `app/src-tauri` | The `tokenradar-app` Tauri shell: tray, windows, updater, installer hooks |
| `app/src` | Svelte 5 frontend, one bundle per window |
| `locales/{en,tr}` | Fluent files for the UI and the CLI |
| `docs/` | Vision, architecture, decisions and the step plan (Turkish); [`docs/en/`](docs/en/) has an English summary |

### Conventions

- **Commits:** [Conventional Commits](https://www.conventionalcommits.org/) in English (`feat:`, `fix:`, `docs:`, `perf:`, `refactor:`, `test:`, `chore:`).
- **Rust:** stable, edition 2024; `cargo fmt` and `cargo clippy -- -D warnings` must be clean.
- **Frontend:** `pnpm -C app check`, `pnpm -C app test` and `pnpm -C app lint` must be clean. No computation in the UI: view models come from the backend through the generated `tauri-specta` bindings (`UPDATE_BINDINGS=1 cargo test -p tokenradar-app bindings_are_fresh` regenerates them).
- **UI text:** only through the Fluent files in `locales/`; every key exists in both English and Turkish.
- **Languages:** code, comments, commits, README and this file are in English. The design documents in `docs/` are in Turkish.
- **No continuous animations.**

### The step plan

The implementation follows [`docs/plan/`](docs/plan/README.md), one step per branch (`step/NN-short-name`). Each step file lists what to read first, its scope, its tasks and how to verify it, and ends with a handoff note. When reality differs from the plan, the change is recorded as a new decision in `docs/decisions.md`.

## Releasing (maintainers)

1. Update `CHANGELOG.md` and bump the version in `Cargo.toml` (workspace) and `app/package.json`.
2. Tag `vX.Y.Z` on `main` and push the tag. `release.yml` checks the tag against the version, builds the installer and the portable zip, verifies the updater signature against the public key in `tauri.conf.json`, and creates a **draft** release with `latest.json` and `SHA256SUMS.txt`.
3. From the workflow's `release-assets` artifact, copy `manifests/scoop/tokenradar.json` to `bucket/tokenradar.json` and commit it; open a pull request to [winget-pkgs](https://github.com/microsoft/winget-pkgs) with the files under `manifests/winget/`.
4. Publish the draft. The updater endpoint (`releases/latest/download/latest.json`) only sees published releases.

The updater signing key (`TAURI_SIGNING_PRIVATE_KEY` and its password) lives in the repository secrets. GitHub cannot show a secret again, so the maintainer keeps the key file and its password in a password manager as well. Losing them means no installed copy can be updated automatically again.
