# REX Hub 部署指南

## Docker 部署（推荐）

### Hub

```bash
docker run -d \
  --name rex-hub \
  -p 3000:3000 \
  -v rex-data:/app/data \
  -e REX_PORT=3000 \
  -e REX_SECRET_KEY=your-secret-key \
  ghcr.io/JeeLin/rex-hub:latest
```

### Agent

```bash
docker run -d \
  --name rex-agent \
  -e REX_HUB_URL=https://your-hub.example.com \
  -e REX_AGENT_TOKEN=YOUR_REGISTRATION_TOKEN \
  -e REX_AGENT_NAME=my-agent \
  ghcr.io/JeeLin/rex-agent:latest
```

说明：`REX_HUB_URL` 填 Hub 基地址即可（`http(s)://` 或 `ws(s)://` 均可，Agent 会自动归一化并拼上 `/ws/agent?token=…`）。

## Docker Compose

```yaml
services:
  rex-hub:
    image: ghcr.io/JeeLin/rex-hub:latest
    ports:
      - "3000:3000"
    volumes:
      - rex-data:/app/data
    environment:
      - REX_PORT=3000
      - REX_SECRET_KEY=your-secret-key
    restart: unless-stopped

  rex-agent:
    image: ghcr.io/JeeLin/rex-agent:latest
    environment:
      - REX_HUB_URL=https://rex-hub
      - REX_AGENT_TOKEN=YOUR_REGISTRATION_TOKEN
      - REX_AGENT_NAME=local-agent
    restart: unless-stopped

volumes:
  rex-data:
```

## 二进制部署

### Hub

```bash
# 下载
curl -LO https://github.com/JeeLin/REX/releases/latest/download/rex-hub-linux-amd64
chmod +x rex-hub-linux-amd64

# 运行
./rex-hub-linux-amd64
```

### Agent

```bash
# 下载
curl -LO https://github.com/JeeLin/REX/releases/latest/download/rex-agent-linux-amd64
chmod +x rex-agent-linux-amd64

# 配置（数据目录下的 agent.yaml，字段 hub_url / token）
cat > ~/.rex/agent.yaml << EOF
hub_url: "https://your-hub.example.com"
token: "YOUR_REGISTRATION_TOKEN"
EOF

# 运行（或用命令行参数 --hub-url / --token，优先级：CLI > env > 配置文件）
./rex-agent-linux-amd64
```

## 配置

### 环境变量

#### Hub

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `REX_PORT` | 监听端口 | `3000` |
| `REX_DATA_DIR` | 数据目录（SQLite、TLS 证书等） | `~/.rex` |
| `REX_SECRET_KEY` | 数据加密密钥（派生用于敏感字段加密） | — |
| `REX_STATIC_DIR` | 前端静态文件目录 | 内嵌 |
| `REX_WORKER` | Worker 进程标识（supervisor 自动设置） | — |
| `REX_TLS_CERT` | TLS 证书路径（PEM） | — |
| `REX_TLS_KEY` | TLS 私钥路径（PEM） | — |
| `REX_TLS_SELF_SIGNED` | 启用自签名证书 | — |
| `REX_ACME_DOMAIN` | ACME 自动证书域名 | — |
| `REX_ACME_EMAIL` | ACME 注册邮箱 | — |
| `REX_ACME_STAGING` | 使用 Let's Encrypt 测试环境 | — |
| `REX_AGENT_BINARIES_DIR` | Agent 二进制预置目录（供 `/api/agents/download`） | `{data-dir}/agent-binaries` |
| `REX_UPDATE_GITHUB_OWNER` | 更新源 GitHub Owner | `JeeLin` |
| `REX_UPDATE_GITHUB_REPO` | 更新源 GitHub Repo | `REX` |
| `REX_UPDATE_PENDING` | 更新验证阶段标识（supervisor 自动设置） | — |

#### Agent

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `REX_HUB_URL` | Hub 基地址（自动归一化为 `/ws/agent`） | **必填** |
| `REX_AGENT_TOKEN` | 认证令牌 | **必填** |
| `REX_AGENT_NAME` | Agent 名称 | `agent` |
| `REX_AGENT_HTTP_PORT` | Agent 内嵌 HTTP 端口 | `3000` |
| `REX_HEARTBEAT_INTERVAL` | 心跳间隔（秒） | `30` |
| `REX_TLS_INSECURE` | 跳过 TLS 验证（仅内网测试） | — |
| `REX_AUTO_UPDATE` | 启用自动更新 | `true` |
| `REX_DATA_DIR` | 数据目录 | `~/.rex` |
| `REX_WORKER` | Worker 进程标识 | — |

## TLS / HTTPS

Hub 支持自动 HTTPS（ACME/Let's Encrypt）：

```bash
# 设置 ACME 域名环境变量即可自动启用
REX_ACME_DOMAIN=hub.example.com REX_ACME_EMAIL=admin@example.com ./rex-hub
```

## 反向代理

如果使用 Nginx / Caddy 反向代理：

```nginx
# Nginx 配置
location / {
    proxy_pass http://127.0.0.1:3000;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
}
```

## 备份与恢复

### 备份

数据存储在 `REX_DATA_DIR`（默认 `~/.rex`），核心文件是 `rex.db`（SQLite）。

**手动备份：**
```bash
# 停止 Hub 服务后复制数据目录
cp -r ~/.rex ~/rex-backup-$(date +%Y%m%d)
```

### 恢复

```bash
# 停止 Hub 服务 → 替换数据目录 → 重启 Hub 服务
cp -r ~/rex-backup-$(date +%Y%m%d) ~/.rex
```

## 故障排查

| 问题 | 解决方案 |
|------|----------|
| 无法访问 | 检查端口是否被防火墙阻止 |
| Agent 无法连接 | 确认 Hub 地址和注册令牌正确 |
| 数据库错误 | 检查数据目录权限 |
| WebSocket 断开 | 检查反向代理的 WebSocket 配置 |
