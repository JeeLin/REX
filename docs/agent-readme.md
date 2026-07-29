# REX Agent

REX Agent 是部署在目标网络中的轻量级反向代理进程，为主进程 REX Hub 提供内网穿透能力。

## 功能

- 主动出站连接 Hub，建立 WebSocket 加密隧道
- 内网服务器无需开放入站端口
- 支持 SSH/SFTP、MySQL、PostgreSQL、Redis、SQLite、S3/MinIO 协议代理
- 自动更新机制

## 快速开始

### 1. 下载

从 [GitHub Releases](https://github.com/JeeLin/REX/releases) 下载对应平台的二进制文件。

### 2. 配置

创建配置文件 `agent.toml`：

```toml
[agent]
name = "my-agent"
environment = "home-network"
hub_url = "wss://your-hub.example.com/ws/agent"
token = "YOUR_REGISTRATION_TOKEN"
```

### 3. 运行

```bash
./rex-agent --config agent.toml
```

## Docker 部署

```bash
docker run -d \
  --name rex-agent \
  -e REX_HUB_URL=wss://your-hub.example.com/ws/agent \
  -e REX_AGENT_TOKEN=YOUR_REGISTRATION_TOKEN \
  -e REX_AGENT_NAME=my-agent \
  ghcr.io/jrelin/rex-agent:latest
```

## 配置项

| 配置项 | 环境变量 | 默认值 | 说明 |
|--------|----------|--------|------|
| `hub_url` | `REX_HUB_URL` | — | Hub WebSocket 地址（必填） |
| `token` | `REX_AGENT_TOKEN` | — | 注册令牌（必填） |
| `name` | `REX_AGENT_NAME` | `agent` | Agent 名称 |
| `log_level` | `REX_LOG_LEVEL` | `info` | 日志级别 |

## 架构

```
内网服务器 → Agent（出站连接）→ WebSocket 隧道 → Hub（公网）
```

Agent 主动连接 Hub，无需在内网防火墙开放端口。

## 版本要求

Hub 和 Agent 版本必须一致，不存在跨版本兼容。

## 更新

Agent 支持自动更新。Hub 检测到新版本后通过 WebSocket 推送更新指令，Agent 自动下载、校验、替换二进制并重启。

## 许可证

MIT License
