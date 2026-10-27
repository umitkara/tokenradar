# testdata

Fixtures for parser and engine tests. **Never commit real user transcripts.** Everything here is either
synthetic or sanitized (content, thinking, signatures and tool payloads removed; paths hashed) via
`cargo run -p tokenradar-core --example sanitize_jsonl`.

| Path | Origin |
|---|---|
| `api/claude-usage-2026-10-26-limits-array.json` | Real `GET /api/oauth/usage` response captured 2026-10-26 (contains no identifiers or secrets). Shows both the legacy flat fields and the newer `limits[]` array, plus `seven_day_breakdown`. |
| `claude/projects/C--Work-Alpha/0a1f…a001.jsonl` | Synthetic main session. Partial and duplicate streaming lines (`iterations: []`), all three title records, a record without the `cache_creation` breakdown, an advisor iteration, an unknown record type, a malformed line, an `api-request` line (no usage) and a last line without a trailing newline. |
| `claude/projects/C--Work-Alpha/0a1f…a001/subagents/agent-a1b2c3.jsonl` | Synthetic subagent of Alpha. A fork copy of a main-session message (cross-session tie) and a record without `requestId`. |
| `claude/projects/C--Work-Alpha/0a1f…a001/tool-results/toolu_1.jsonl` | Contains usage-looking lines; must never be indexed. |
| `claude/projects/C--Work-Beta/0b2e…b001.jsonl` | Synthetic session with no `cwd` (unresolved project), a `<synthetic>` model and an `agent-name` title. |
| `statusline/sample.json` | Synthetic Claude Code statusline payload (S1): escaped Windows paths, both `rate_limits` windows with past `resets_at` values, plus fields TokenRadar ignores (`workspace`, `prompt_cache`, `effort`). |
| `claude/projects/C--Work-Gamma/0c3d…c001/subagents/agent-ffee01.jsonl` | Synthetic subagent whose main session file does not exist (placeholder parent); `cwd` with forward slashes and a trailing separator. |

## Sanitizer

`cargo run -p tokenradar-core --example sanitize_jsonl -- <input.jsonl> [--out <path>] [--max-lines N]` keeps an allowlist of
fields only: record type, ids, timestamps, model, `usage` and session metadata. `cwd` becomes
`C:\work\proj-<hash8>`, branch names other than `main`/`master`/`HEAD` become `branch-<hash8>` and titles become
`title-<hash8>`. Without `--out` it writes `testdata/claude/sanitized/<hash8>.jsonl`. Review the output before
committing it: the allowlist protects content, but ids and timestamps are still yours.
