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

核心表还包括 SIP 相关表（见 `migrations.sql`）：

```sql
CREATE TABLE cdr (
  id            TEXT PRIMARY KEY,
  resource_id   TEXT NOT NULL,
  peer          TEXT NOT NULL DEFAULT '',
  call_id       TEXT NOT NULL DEFAULT '',
  start_time    TEXT NOT NULL,
  end_time      TEXT,
  duration_sec  INTEGER DEFAULT 0,
  direction     TEXT NOT NULL DEFAULT 'out',   -- out / in
  status        TEXT NOT NULL,                 -- answered / missed / failed
  recording     TEXT                                -- 录音文件路径（可选）
);
```

```sql
CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
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

## 资源 config_json 形状

`resources.config_json` 按协议存放各自特有参数（敏感字段先经 `CredentialCrypto` AES-256-GCM 加密再存库）。

### SIP（按名称管理 + 多账户，0.70.4）

名称只做展示分组，取 `Resource.name`，不进 `config_json`；`config_json` 为 `SipProfile` 形状，每个账户自带完整 server profile + 凭据：

```json
{
  "accounts": [
    {
      "id": "a1",
      "server": "pbx.example.com",
      "port": 5060,
      "transport": "udp",
      "username": "alice",
      "password": "secret",
      "displayName": "Alice"
    },
    {
      "id": "a2",
      "server": "pbx2.example.com",
      "port": 5061,
      "transport": "tls",
      "username": "bob",
      "password": "secret2",
      "displayName": "Bob"
    }
  ],
  "activeAccount": "a1"
}
```

- 名称（= `Resource.name`）= 展示分组标签，不绑定服务器，不进 `config_json`。
- 每个账户自带完整 server profile（`server`/`port`/`transport`，默认 `udp`/5060）与凭据（`username`/`password`/`displayName`）。
- `activeAccount` = 当前生效账户 id；解析层（`load_sip_conn`）取该账户构造生效的 `SipConfig`，Hub/Agent 仍按单 `SipConfig` 注册/拨号（FFI/隧道帧不变）。
- 本版本仅支持此形状，不做数据迁移、不做旧形状兼容。
