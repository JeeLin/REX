# 配置与目录约定

## Hub 配置

```yaml
# hub.yaml
listen: ":3000"
data_dir: "./data"
secret_key: "${REX_SECRET_KEY}"
tls:
  cert: ""
  key: ""
update:
  enabled: true
  check_interval: 86400
  github_repo: "owner/rex-hub"
```

## Agent 配置

```yaml
# agent.yaml
server: "https://hub.example.com"
token: "rex_env_xxx"
name: "内网 Agent"
data_dir: "./data"
auto_update: true
```

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

```text
rex-hub/
├── Cargo.toml
├── README.md
├── crates/
│   ├── rex-common/        通用类型、错误定义、配置解析
│   ├── rex-ssh/           SSH/SFTP 协议实现
│   ├── rex-mysql/         MySQL 协议实现
│   ├── rex-postgresql/    PostgreSQL 协议实现
│   ├── rex-redis/         Redis 协议实现
│   ├── rex-sqlite/        SQLite 协议实现
│   ├── rex-s3/            S3/MinIO 协议实现
│   ├── rex-transfer/      文件传输引擎
│   ├── rex-tunnel/        WebSocket 隧道
│   ├── rex-supervisor/    进程 supervisor
│   ├── rex-hub/           Hub 二进制入口
│   └── rex-agent/         Agent 二进制入口
├── packages/
│   └── rex-console-web/   Vue 3 前端
└── docs/
```

### Workspace 依赖

根 `Cargo.toml` 定义 workspace 和共享依赖版本：

```toml
[workspace]
members = [
  "crates/rex-common",
  "crates/rex-ssh",
  "crates/rex-mysql",
  "crates/rex-postgresql",
  "crates/rex-redis",
  "crates/rex-sqlite",
  "crates/rex-s3",
  "crates/rex-transfer",
  "crates/rex-tunnel",
  "crates/rex-supervisor",
  "crates/rex-hub",
  "crates/rex-agent",
]
resolver = "2"

[workspace.package]
edition = "2021"
license = "MIT"

[workspace.dependencies]
anyhow = "1"
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
uuid = { version = "1", features = ["v4", "serde"] }
```

### 协议 crate 边界

每个协议 crate 只负责协议实现，不依赖 Hub 或 Agent 业务层。

协议 crate 输出统一能力：

```rust
pub trait ResourceConnector: Send + Sync {
    fn protocol(&self) -> ResourceProtocol;
    fn connect(&self, config: ResourceConfig) -> impl Future<Output = Result<Connection>>;
}
```

Hub 和 Agent 都通过同一套协议 crate 建立连接，区别只在于连接入口：

- Hub 直连资源：Hub worker 直接连接目标。
- Agent 代理资源：Hub 通过 Agent WebSocket 隧道请求 Agent worker 连接目标。
