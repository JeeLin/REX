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
