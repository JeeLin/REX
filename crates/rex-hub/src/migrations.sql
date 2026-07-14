CREATE TABLE IF NOT EXISTS environments (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    connection_mode TEXT NOT NULL,
    agent_token_hash TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    environment_id TEXT NOT NULL,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    version TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    os TEXT NOT NULL,
    arch TEXT NOT NULL,
    hostname TEXT,
    os_version TEXT,
    status TEXT NOT NULL,
    last_seen_at TEXT,
    config_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (environment_id) REFERENCES environments(id)
);

CREATE TABLE IF NOT EXISTS resources (
    id TEXT PRIMARY KEY,
    environment_id TEXT NOT NULL,
    name TEXT NOT NULL,
    protocol TEXT NOT NULL,
    agent_id TEXT,
    config_json TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (environment_id) REFERENCES environments(id)
);

CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    time TEXT NOT NULL,
    user TEXT NOT NULL,
    ip TEXT,
    environment_id TEXT,
    resource_id TEXT,
    agent_id TEXT,
    type TEXT NOT NULL,
    result TEXT NOT NULL,
    summary TEXT NOT NULL,
    detail_json TEXT
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    resource_id TEXT NOT NULL,
    metric_type TEXT NOT NULL,      -- 'latency' | 'throughput' | 'error' | 'connection'
    value REAL NOT NULL,            -- 指标值（延迟ms、吞吐量bytes/s、错误计数、连接事件
    tags TEXT,                      -- JSON 扩展标签 '{"db":"mysql","query":"SELECT"}'
    recorded_at TEXT NOT NULL,      -- ISO 8601 时间戳
    FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_metrics_resource_type ON metrics(resource_id, metric_type);
CREATE INDEX IF NOT EXISTS idx_metrics_recorded_at ON metrics(recorded_at);

CREATE TABLE IF NOT EXISTS ai_config (
    id TEXT PRIMARY KEY DEFAULT 'default',
    provider TEXT NOT NULL DEFAULT 'openai',
    api_key_encrypted TEXT NOT NULL DEFAULT '',
    model TEXT NOT NULL DEFAULT 'gpt-4o',
    base_url TEXT NOT NULL DEFAULT 'https://api.openai.com/v1',
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL DEFAULT '#58A6FF',
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS resource_tags (
    resource_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    PRIMARY KEY (resource_id, tag_id),
    FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS notebooks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL DEFAULT 'Untitled',
    description TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS notebook_blocks (
    id TEXT PRIMARY KEY,
    notebook_id TEXT NOT NULL,
    block_type TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    resource_id TEXT,
    protocol TEXT,
    order_index INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (notebook_id) REFERENCES notebooks(id) ON DELETE CASCADE,
    FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS notebook_executions (
    id TEXT PRIMARY KEY,
    block_id TEXT NOT NULL,
    status TEXT NOT NULL,
    output TEXT NOT NULL DEFAULT '',
    duration_ms INTEGER,
    executed_at TEXT NOT NULL,
    FOREIGN KEY (block_id) REFERENCES notebook_blocks(id) ON DELETE CASCADE
);

