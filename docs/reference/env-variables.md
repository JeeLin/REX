# REX Hub — 环境变量参考

所有环境变量均以 `REX_` 前缀开头。Hub 和 Agent 各自使用不同的变量子集。

---

## Hub 变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `REX_PORT` | `3000` | Hub HTTP 监听端口 |
| `REX_DATA_DIR` | `./data` | 数据存储目录（SQLite、TLS 证书、Agent 二进制等） |
| `REX_STATIC_DIR` | 内嵌资源 | 前端静态资源目录（开发时可指向 `dist/`） |
| `REX_WORKER` | — | 存在时启动 worker 子进程（supervisor 自动设置，不要手动设置） |

### TLS

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `REX_TLS_CERT` | — | TLS 证书文件路径（PEM 格式） |
| `REX_TLS_KEY` | — | TLS 私钥文件路径（PEM 格式） |
| `REX_TLS_SELF_SIGNED` | — | 存在时自动生成自签名证书 |
| `REX_ACME_DOMAIN` | — | ACME 自动证书域名 |
| `REX_ACME_EMAIL` | — | ACME 注册邮箱 |
| `REX_ACME_STAGING` | — | 存在时使用 Let's Encrypt 测试环境 |

### 更新

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `REX_AUTO_UPDATE` | — | 存在时启用自动更新 |
| `REX_UPDATE_GITHUB_OWNER` | `JeeLin` | GitHub 更新源 Owner |
| `REX_UPDATE_GITHUB_REPO` | `REX` | GitHub 更新源 Repo |
| `REX_UPDATE_PENDING` | — | 存在时标记有待应用的更新 |

---

## Agent 变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `REX_HUB_URL` | — | **必填**。Hub 服务地址（如 `https://hub.example.com`） |
| `REX_AGENT_TOKEN` | — | **必填**。Agent 认证令牌（Hub 环境详情页获取） |
| `REX_AGENT_NAME` | `agent` | Agent 名称（显示在 Hub 管理界面） |
| `REX_HEARTBEAT_INTERVAL` | `30` | 心跳间隔（秒），Agent 每隔此时间向 Hub 发送心跳 |
| `REX_TLS_INSECURE` | — | 存在时跳过 TLS 证书验证（仅限内网测试环境） |
| `REX_AUTO_UPDATE` | — | 存在时启用自动更新 |
| `REX_WORKER` | — | 存在时启动 worker 子进程（supervisor 自动设置，不要手动设置） |

---

## 快速开始

### Hub（Docker）

```bash
docker run -d \
  -p 3000:3000 \
  -v rex-data:/app/data \
  -e REX_TLS_SELF_SIGNED=1 \
  ghcr.io/jielin/rex-hub:latest
```

### Hub（自签名 + 自定义端口）

```bash
REX_PORT=8443 \
REX_TLS_SELF_SIGNED=1 \
./rex-hub
```

### Agent

```bash
REX_HUB_URL=https://hub.example.com \
REX_AGENT_TOKEN=your-token-here \
./rex-agent
```

### Agent（Docker）

```bash
docker run -d \
  -e REX_HUB_URL=https://hub.example.com \
  -e REX_AGENT_TOKEN=your-token-here \
  ghcr.io/jielin/rex-agent:latest
```
