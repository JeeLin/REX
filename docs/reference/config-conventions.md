# 配置与目录约定

## Hub 配置

配置优先级：**命令行参数 > 环境变量 > 配置文件**。Hub 配置文件：`~/.rex/config.yaml`（即 `<data_dir>/config.yaml`），Agent 配置文件：`~/.rex/agent.yaml`。

```yaml
# config.yaml（Hub）
port: 3000
data_dir: "./data"
```

对应环境变量：`REX_PORT`、`REX_DATA_DIR`。TLS 由环境变量配置（`REX_TLS_CERT`/`REX_TLS_KEY` 或 `REX_ACME_DOMAIN` 等，见 `tls.rs::TlsConfig::from_env`），不进配置文件。

## Agent 配置

```yaml
# agent.yaml
hub_url: "https://hub.example.com"
token: "rex_env_xxx"
data_dir: "./data"
```

对应环境变量：`REX_HUB_URL`、`REX_AGENT_TOKEN`、`REX_DATA_DIR`；Agent 名称用 `REX_AGENT_NAME`。CLI：`rex-agent run --hub-url ... --token ... --data-dir ...`。

## Hub 数据目录

```text
{data-dir}/
├── hub.db              SQLite 数据库
├── certs/              TLS 证书
├── queries/            保存的 SQL 查询
├── settings.json       系统设置
├── agent-binaries/     Agent 二进制文件
└── update/
    ├── staging/        待替换的新版本
    └── rollback/       旧版本备份
```

## Agent 数据目录

```text
{data-dir}/
├── agent.json          Agent 身份（ID、token、名称）
├── update-state.json   更新状态
└── logs/               运行日志
```

---

## 后端工程结构

### 仓库结构

仓库根即 workspace 根（不是 `rex-hub/` 子目录）。crate 位于 `crates/*`，前端位于 `packages/rex-console-web`，文档位于 `docs/`：

```text
REX/
├── Cargo.toml                 # workspace 根 + [workspace.dependencies]
├── crates/
│   ├── rex-common/        通用类型、错误定义、配置解析、supervisor 模块、sip_media、更新
│   ├── rex-ssh/           SSH/SFTP 协议实现
│   ├── rex-mysql/         MySQL 协议实现
│   ├── rex-postgresql/    PostgreSQL 协议实现
│   ├── rex-redis/         Redis 协议实现
│   ├── rex-sqlite/        SQLite 协议实现
│   ├── rex-mariadb/       MariaDB 协议实现（复用 MySQL 驱动）
│   ├── rex-clickhouse/    ClickHouse 协议实现（HTTP API）
│   ├── rex-mssql/         SQL Server 协议实现（tiberius）
│   ├── rex-oracle/        Oracle 协议实现（oracle-rs）
│   ├── rex-s3/            S3/MinIO 协议实现
│   ├── rex-sip/           SIP 电话（baresip FFI：UA/音频桥/视频桥/抓包/CDR/录音）
│   ├── rex-transfer/      文件传输引擎（FileConnector 抽象）
│   ├── rex-hub/           Hub 二进制入口（整合所有 crate + 前端托管 + WebSocket 隧道模块）
│   └── rex-agent/         Agent 二进制入口（整合所有 crate + WebSocket 隧道）
├── packages/
│   └── rex-console-web/   Vue 3 前端
└── docs/
```

> 注：`supervisor`（进程模型）是 `rex-common` 的模块，`tunnel`（WebSocket 隧道）是 `rex-hub`/`rex-agent` 内的模块——它们不是独立 crate。workspace 用 `members = ["crates/*"]` 自动包含全部 crate。

### 协议 crate 边界

每个协议 crate 只负责协议实现，不依赖 Hub 或 Agent 业务层。各协议以统一 trait 向上层输出能力（`rex-common` 内定义）：

- `rex-common::sql::SqlConnector`（`sql_api.rs` 用 `SqlConnectorFactory` 按 `DatabaseType` 分派 `MySqlConnector`/`PostgresConnector`/`SqliteConnector`/`MariaDBConnector`/`ClickHouseConnector`/`SqlServerConnector`/`OracleConnector`）
- `rex-common::redis::RedisConnector`
- `rex-common::file_transfer::FileConnector`（`SftpConnector` / `S3Connector` / 本地实现）
- `rex-sip` 通过 `rex-common::sip_media` 的 PCM/视频帧编解码与隧道帧封装对接 Hub/Agent
- MongoDB 无独立协议 crate：`rex-hub/src/mongodb_api.rs` 直接使用 `mongodb` crate（rustls-tls）

Hub 和 Agent 都通过同一套协议 crate 建立连接，区别只在于连接入口：

- Hub 直连资源：Hub worker 直接连接目标。
- Agent 代理资源：Hub 通过 Agent WebSocket 隧道请求 Agent worker 连接目标。
