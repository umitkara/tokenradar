-- Schema v1 (architecture §6). Times are UTC epoch milliseconds, money is micro-USD.
-- Editable until v0.1 ships; after that, schema changes go into new migrations only.

CREATE TABLE accounts (
  id          INTEGER PRIMARY KEY,
  provider    TEXT NOT NULL,                       -- 'claude' | 'codex'
  label       TEXT NOT NULL,
  config_dir  TEXT NOT NULL,                       -- normalized path (UNC for WSL)
  plan        TEXT,
  tier        TEXT,
  environment TEXT NOT NULL DEFAULT 'native',      -- native | wsl:<distro>
  created_at  INTEGER NOT NULL,
  UNIQUE (provider, config_dir)
);

CREATE TABLE projects (
  id           INTEGER PRIMARY KEY,
  account_id   INTEGER NOT NULL REFERENCES accounts (id),
  path_norm    TEXT NOT NULL,
  display_name TEXT NOT NULL,
  UNIQUE (account_id, path_norm)
);

CREATE TABLE sessions (
  id         INTEGER PRIMARY KEY,
  account_id INTEGER NOT NULL REFERENCES accounts (id),
  uuid       TEXT NOT NULL,                        -- subagent: 'agent-<id>'
  kind       TEXT NOT NULL,                        -- 'main' | 'subagent'
  parent_id  INTEGER REFERENCES sessions (id),
  root_id    INTEGER REFERENCES sessions (id),
  project_id INTEGER REFERENCES projects (id),
  title      TEXT,
  git_branch TEXT,
  entrypoint TEXT,
  cc_version TEXT,
  first_ts   INTEGER,
  last_ts    INTEGER,
  UNIQUE (account_id, uuid)
);
CREATE INDEX ix_sessions_root ON sessions (root_id);
CREATE INDEX ix_sessions_project ON sessions (project_id, last_ts);
CREATE INDEX ix_sessions_last ON sessions (account_id, last_ts);

CREATE TABLE source_files (
  id         INTEGER PRIMARY KEY,
  account_id INTEGER NOT NULL REFERENCES accounts (id),
  path       TEXT NOT NULL UNIQUE,
  kind       TEXT NOT NULL,                        -- 'main' | 'subagent'
  size       INTEGER NOT NULL,
  mtime      INTEGER NOT NULL,
  offset     INTEGER NOT NULL,
  session_id INTEGER REFERENCES sessions (id),
  deleted_at INTEGER
);

CREATE TABLE models (
  id     INTEGER PRIMARY KEY,
  name   TEXT NOT NULL UNIQUE,
  family TEXT NOT NULL
);

-- Raw deduplicated events; pruned after the retention period (ADR-020).
CREATE TABLE usage_events (
  id             INTEGER PRIMARY KEY,
  account_id     INTEGER NOT NULL REFERENCES accounts (id),
  session_id     INTEGER NOT NULL REFERENCES sessions (id),
  model_id       INTEGER NOT NULL REFERENCES models (id),
  ts             INTEGER NOT NULL,
  msg_key        BLOB NOT NULL,                    -- xxh3-128(message.id, requestId)
  input          INTEGER NOT NULL,
  output         INTEGER NOT NULL,
  cache_read     INTEGER NOT NULL,
  cache_w5m      INTEGER NOT NULL,
  cache_w1h      INTEGER NOT NULL,
  thinking       INTEGER NOT NULL,
  web_search     INTEGER NOT NULL,
  web_fetch      INTEGER NOT NULL,
  speed          INTEGER NOT NULL,                 -- 0 standard, 1 fast
  cost_micro_usd INTEGER NOT NULL,
  UNIQUE (account_id, msg_key)
);
CREATE INDEX ix_events_acct_ts ON usage_events (account_id, ts);
CREATE INDEX ix_events_session ON usage_events (session_id, ts);

-- Permanent five-minute rollup; never pruned.
CREATE TABLE usage_5m (
  account_id     INTEGER NOT NULL REFERENCES accounts (id),
  session_id     INTEGER NOT NULL REFERENCES sessions (id),
  model_id       INTEGER NOT NULL REFERENCES models (id),
  bucket_ts      INTEGER NOT NULL,
  input          INTEGER NOT NULL DEFAULT 0,
  output         INTEGER NOT NULL DEFAULT 0,
  cache_read     INTEGER NOT NULL DEFAULT 0,
  cache_w5m      INTEGER NOT NULL DEFAULT 0,
  cache_w1h      INTEGER NOT NULL DEFAULT 0,
  cost_micro_usd INTEGER NOT NULL DEFAULT 0,
  events         INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (account_id, session_id, model_id, bucket_ts)
) WITHOUT ROWID;
CREATE INDEX ix_5m_acct_bucket ON usage_5m (account_id, bucket_ts);

CREATE TABLE limit_snapshots (
  id           INTEGER PRIMARY KEY,
  account_id   INTEGER NOT NULL REFERENCES accounts (id),
  limit_kind   TEXT NOT NULL,                      -- session | weekly_all | weekly_scoped | spend | …
  scope        TEXT,                               -- e.g. 'model:Fable'
  percent      REAL NOT NULL,
  resets_at    INTEGER,
  severity     TEXT,
  source       TEXT NOT NULL,                      -- 'statusline' | 'api' | 'codex_log'
  session_uuid TEXT,
  observed_at  INTEGER NOT NULL
);
CREATE INDEX ix_snap ON limit_snapshots (account_id, limit_kind, scope, observed_at);

CREATE TABLE limit_windows (
  id           INTEGER PRIMARY KEY,
  account_id   INTEGER NOT NULL REFERENCES accounts (id),
  limit_kind   TEXT NOT NULL,
  scope        TEXT,
  starts_at    INTEGER NOT NULL,
  resets_at    INTEGER NOT NULL,
  peak_percent REAL,
  hit_limit    INTEGER NOT NULL DEFAULT 0,
  estimated    INTEGER NOT NULL DEFAULT 0,         -- derived from logs, no snapshot
  UNIQUE (account_id, limit_kind, scope, resets_at)
);

CREATE TABLE surface_breakdown (
  account_id        INTEGER NOT NULL REFERENCES accounts (id),
  as_of             INTEGER NOT NULL,
  window_started_at INTEGER,
  key               TEXT NOT NULL,
  percent           REAL,
  PRIMARY KEY (account_id, as_of, key)
);

-- Repository APIs for the next two tables arrive in Steps 13–14.
CREATE TABLE calibrations (
  id          INTEGER PRIMARY KEY,
  account_id  INTEGER NOT NULL REFERENCES accounts (id),
  limit_kind  TEXT NOT NULL,
  scope       TEXT,
  k           REAL NOT NULL,
  alpha_json  TEXT,
  confidence  REAL NOT NULL,
  n_obs       INTEGER NOT NULL,
  version     INTEGER NOT NULL,
  computed_at INTEGER NOT NULL
);
CREATE INDEX ix_calib ON calibrations (account_id, limit_kind, scope, computed_at);

CREATE TABLE attributions (
  window_id           INTEGER NOT NULL REFERENCES limit_windows (id),
  root_session_id     INTEGER REFERENCES sessions (id),  -- NULL = "Other"
  est_percent         REAL NOT NULL,
  cost_micro_usd      INTEGER NOT NULL,
  calibration_version INTEGER NOT NULL,
  PRIMARY KEY (window_id, root_session_id)
);

CREATE TABLE notification_log (
  id         INTEGER PRIMARY KEY,
  rule       TEXT NOT NULL,
  dedupe_key TEXT NOT NULL UNIQUE,
  fired_at   INTEGER NOT NULL
);

CREATE TABLE kv (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
) WITHOUT ROWID;
