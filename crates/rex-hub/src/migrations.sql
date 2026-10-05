-- REX Hub database schema

CREATE TABLE IF NOT EXISTS environments (
  id                 TEXT PRIMARY KEY,
  name               TEXT NOT NULL UNIQUE,
  description        TEXT DEFAULT '',
  connection_mode    TEXT NOT NULL DEFAULT 'direct',
  registration_token TEXT DEFAULT '',
  created_at         TEXT NOT NULL,
  updated_at         TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS resources (
  id              TEXT PRIMARY KEY,
  environment_id  TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
  name            TEXT NOT NULL,
  protocol        TEXT NOT NULL,
  host            TEXT NOT NULL,
  port            INTEGER,
  username        TEXT DEFAULT '',
  config_json     TEXT NOT NULL DEFAULT '{}',
  subtype         TEXT,   -- v0.70.7: 资源子类（通用可空列）。SQL 资源存探测出的方言（mysql/postgresql/sqlite）；其他资源为 NULL
  color           TEXT,
  sort_order      INTEGER DEFAULT 0,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agents (
  id              TEXT PRIMARY KEY,
  environment_id  TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
  name            TEXT NOT NULL,
  token_hash      TEXT NOT NULL DEFAULT '',
  version         TEXT DEFAULT '',
  os              TEXT DEFAULT '',
  arch            TEXT DEFAULT '',
  hostname        TEXT DEFAULT '',
  ip              TEXT DEFAULT '',
  status          TEXT NOT NULL DEFAULT 'offline',
  last_seen_at    TEXT,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_log (
  id              TEXT PRIMARY KEY,
  time            TEXT NOT NULL,
  action          TEXT NOT NULL,
  target          TEXT,
  environment_id  TEXT,
  resource_id     TEXT,
  agent_id        TEXT,
  result          TEXT NOT NULL DEFAULT 'success',
  detail          TEXT DEFAULT '',
  ip              TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Performance indexes
CREATE INDEX IF NOT EXISTS idx_resources_environment_id ON resources(environment_id);
CREATE INDEX IF NOT EXISTS idx_resources_protocol ON resources(protocol);
CREATE INDEX IF NOT EXISTS idx_agents_environment_id ON agents(environment_id);
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
CREATE INDEX IF NOT EXISTS idx_audit_log_time ON audit_log(time);
CREATE INDEX IF NOT EXISTS idx_audit_log_action ON audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_log_environment_id ON audit_log(environment_id);

-- SIP 通话记录 (CDR)
CREATE TABLE IF NOT EXISTS cdr (
  id            TEXT PRIMARY KEY,
  resource_id   TEXT NOT NULL,
  peer          TEXT NOT NULL DEFAULT '',
  call_id       TEXT NOT NULL DEFAULT '',
  start_time    TEXT NOT NULL,
  end_time      TEXT,
  duration_sec  INTEGER DEFAULT 0,
  direction     TEXT NOT NULL DEFAULT 'out',   -- out / in
  state         TEXT NOT NULL DEFAULT 'ended', -- missed / answered / ended
  recording_url TEXT DEFAULT '',
  pcap_url      TEXT DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_cdr_resource_id ON cdr(resource_id);
CREATE INDEX IF NOT EXISTS idx_cdr_start_time ON cdr(start_time);

-- v0.91.0：传输任务持久化（TransferTask 模型 + 进度查询 API 的事实源）。
-- v0.92.0：新增 kind（transfer|sync）与 sync_options（SyncOptions JSON）。
CREATE TABLE IF NOT EXISTS transfer_task (
  id              TEXT PRIMARY KEY,
  source_resource_id TEXT NOT NULL,
  target_resource_id TEXT NOT NULL,
  source_path     TEXT NOT NULL,
  target_path     TEXT NOT NULL,
  conflict_policy TEXT NOT NULL DEFAULT 'overwrite',
  kind            TEXT NOT NULL DEFAULT 'transfer',
  sync_options    TEXT NOT NULL DEFAULT '',
  status          TEXT NOT NULL DEFAULT 'pending',
  total_bytes     INTEGER NOT NULL DEFAULT 0,
  transferred_bytes INTEGER NOT NULL DEFAULT 0,
  speed_bytes_per_sec INTEGER NOT NULL DEFAULT 0,
  eta_seconds     INTEGER,
  error           TEXT DEFAULT '',
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_transfer_task_status ON transfer_task(status);
CREATE INDEX IF NOT EXISTS idx_transfer_task_created_at ON transfer_task(created_at);
