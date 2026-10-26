# testdata

Fixtures for parser and engine tests. **Never commit real user transcripts.** Everything here is either
synthetic or sanitized (content, thinking, signatures and tool payloads removed; paths hashed) via
`cargo run -p tokenradar-core --example sanitize_jsonl`.

| Path | Origin |
|---|---|
| `api/claude-usage-2026-10-26-limits-array.json` | Real `GET /api/oauth/usage` response captured 2026-10-26 (contains no identifiers or secrets). Shows both the legacy flat fields and the newer `limits[]` array, plus `seven_day_breakdown`. |
