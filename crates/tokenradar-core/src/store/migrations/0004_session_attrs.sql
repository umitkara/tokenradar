-- Step 16a: the distinct working directories, effort levels and fast mode of a session, for the
-- session detail. Metadata only: `cwd`, `effort` and `usage.speed` of assistant records, never
-- content. Kept apart from `usage_events` so retention does not drop them. One row per
-- distinct value; re-reading a file only widens `first_ts`/`last_ts`, so it stays idempotent.
CREATE TABLE session_attrs (
  session_id INTEGER NOT NULL REFERENCES sessions (id),
  kind       TEXT NOT NULL,                        -- 'cwd' | 'effort' | 'speed' (only 'fast')
  value      TEXT NOT NULL,
  first_ts   INTEGER NOT NULL,
  last_ts    INTEGER NOT NULL,
  PRIMARY KEY (session_id, kind, value)
) WITHOUT ROWID;

-- "Transcript deleted": the files of one session.
CREATE INDEX ix_source_session ON source_files (session_id);

-- Earlier passes did not keep these fields. Read every live file again from the start, as
-- migration 2 did: deduplication keeps events idempotent, the retention watermark keeps pruned
-- ones out. Files Claude Code already deleted keep their sessions without these rows.
UPDATE source_files SET offset = 0, size = 0, mtime = 0 WHERE deleted_at IS NULL;
