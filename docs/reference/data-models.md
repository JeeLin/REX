# 数据模型

## 存储选择

Hub 使用 SQLite 作为本地数据库：

```text
{data-dir}/hub.db
```

原因：

- 单文件，适合自托管。
- 部署简单。
- 足够支撑个人使用场景。
- 后续可替换为 PostgreSQL，不影响业务模型。

## 核心表

```sql
CREATE TABLE environments (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  connection_mode TEXT NOT NULL,
  agent_token_hash TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE agents (
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
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE resources (
  id TEXT PRIMARY KEY,
  environment_id TEXT NOT NULL,
  name TEXT NOT NULL,
  protocol TEXT NOT NULL,
  connection_mode TEXT NOT NULL,
  agent_id TEXT,
  config_json TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE audit_log (
  id TEXT PRIMARY KEY,
  time TEXT NOT NULL,
  user TEXT NOT NULL,
  environment_id TEXT,
  resource_id TEXT,
  agent_id TEXT,
  type TEXT NOT NULL,
  result TEXT NOT NULL,
  summary TEXT NOT NULL,
  detail_json TEXT
);
```

## 凭据加密

资源密码、SSH 私钥密码、Agent token 等敏感字段不应明文存库。

加密流程：

```text
REX_SECRET_KEY
  ↓
派生加密密钥
  ↓
AES-256-GCM 加密资源配置中的敏感字段
  ↓
config_json 中只保存密文、nonce、算法标识
```

敏感字段包括：

- SSH 密码
- SSH 私钥密码
- MySQL 密码
- PostgreSQL 密码
- Redis 密码
- S3 secret key
- Agent token hash 的原始 token

## 资源配置结构

```json
{
  "host": "192.0.2.1",
  "port": 22,
  "username": "pi",
  "auth": {
    "type": "password",
    "passwordEncrypted": "..."
  },
  "terminal": {
    "encoding": "utf-8",
    "keepAliveSeconds": 60
  }
}
```
