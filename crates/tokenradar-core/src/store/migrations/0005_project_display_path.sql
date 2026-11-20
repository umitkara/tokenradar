-- Step 16b: a project's path with the case it was recorded in, for the projects page and its
-- folder tree. `path_norm` stays the identity (case-folded on Windows); this is display only.
ALTER TABLE projects ADD COLUMN display_path TEXT;

-- Fill it from the transcripts, as migration 4 did: read every live file again from the start.
-- Coming from v2 or v3 this is the same full pass migration 4 starts, not a second one. Projects
-- whose files Claude Code already deleted keep `display_path` empty and show `path_norm`.
UPDATE source_files SET offset = 0, size = 0, mtime = 0 WHERE deleted_at IS NULL;
