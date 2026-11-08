-- Step 12n (ADR-054): compactions of main sessions, for the statusline's `compact` item.
-- Keyed by the record's uuid, so a record copied into another file by `--resume` counts once.
-- `session_uuid` is the record's own `sessionId`, so the owner does not depend on which file
-- was read first.
CREATE TABLE compact_events (
  uuid         TEXT PRIMARY KEY,
  session_uuid TEXT NOT NULL,
  at           INTEGER NOT NULL,
  trigger    TEXT NOT NULL,                        -- 'auto' | 'manual' | 'other'
  pre_tokens INTEGER
) WITHOUT ROWID;
CREATE INDEX ix_compact_session ON compact_events (session_uuid);
-- The statusline asks by session uuid alone, without the account.
CREATE INDEX ix_sessions_uuid ON sessions (uuid);

-- Earlier passes skipped these records. Make every live file look changed and read it again
-- from the start; deduplication keeps the events idempotent and the retention watermark keeps
-- pruned ones out.
UPDATE source_files SET offset = 0, size = 0, mtime = 0 WHERE deleted_at IS NULL;
