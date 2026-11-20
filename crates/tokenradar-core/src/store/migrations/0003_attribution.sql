-- Step 14a (ADR-064): attribution of limit windows to session trees.
-- `attributions` from v1 was never written; it is rebuilt so a share can carry cost without a
-- percentage (no usable calibration) and rows go with their window.
DROP TABLE attributions;
CREATE TABLE attributions (
  window_id       INTEGER NOT NULL REFERENCES limit_windows (id) ON DELETE CASCADE,
  root_session_id INTEGER REFERENCES sessions (id),  -- NULL = "Other"
  est_percent     REAL,                              -- NULL = cost only, no percentage
  cost_micro_usd  INTEGER NOT NULL
);
CREATE UNIQUE INDEX ux_attributions ON attributions (window_id, IFNULL(root_session_id, 0));
CREATE INDEX ix_attributions_root ON attributions (root_session_id);

-- One row per attributed window: how and when it was computed.
CREATE TABLE attribution_runs (
  window_id           INTEGER PRIMARY KEY REFERENCES limit_windows (id) ON DELETE CASCADE,
  calibration_version INTEGER NOT NULL,              -- 0 = no calibration used
  method              TEXT NOT NULL,                 -- calibrated | proportional | estimated | cost_only
  confidence          REAL,                          -- calibration confidence for this window
  total_percent       REAL,                          -- = Σ shares + Other
  other_percent       REAL,                          -- NULL when "Other" is not computed
  breakdown_other     REAL,                          -- seven_day_breakdown non-Claude-Code share
  suspect             INTEGER NOT NULL DEFAULT 0,    -- Other well below breakdown_other
  other_excess        INTEGER NOT NULL DEFAULT 0,    -- Other well above breakdown_other
  closed              INTEGER NOT NULL DEFAULT 0,    -- computed after resets_at: final
  computed_at         INTEGER NOT NULL
);
