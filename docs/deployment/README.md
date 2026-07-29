# REX Hub 部署指南

## Docker 部署（推荐）

### Hub

```bash
docker run -d \
  --name rex-hub \
  -p 3000:3000 \
  -v rex-data:/data \
  -e REX_PORT=3000 \
  ghcr.io/jlin/rex-hub:latest
```

### Agent

```bash
docker run -d \
  --name rex-agent \
  -e REX_HUB_URL=wss://your-hub.example.com/ws/agent \
  -e REX_AGENT_TOKEN=YOUR_REGISTRATION_TOKEN \
  -e REX_AGENT_NAME=my-agent \
  ghcr.io/jlin/rex-agent:latest
```

## Docker Compose

```yaml
services:
  rex-hub:
    image: ghcr.io/jlin/rex-hub:latest
    ports:
      - "3000:3000"
    volumes:
      - rex-data:/data
    environment:
      - REX_PORT=3000
    restart: unless-stopped

  rex-agent:
    image: ghcr.io/jlin/rex-agent:latest
    environment:
      - REX_HUB_URL=wss://rex-hub/ws/agent
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
curl -LO https://github.com/jlin/rex/releases/latest/download/rex-hub-linux-amd64
chmod +x rex-hub-linux-amd64

# 运行
./rex-hub-linux-amd64
```

### Agent

```bash
# 下载
curl -LO https://github.com/jlin/rex/releases/latest/download/rex-agent-linux-amd64
chmod +x rex-agent-linux-amd64

# 配置
cat > agent.toml << EOF
[agent]
hub_url = "wss://your-hub.example.com/ws/agent"
token = "YOUR_REGISTRATION_TOKEN"
name = "my-agent"
auto_update = true
EOF

# 运行
./rex-agent-linux-amd64 --config agent.toml
```

## 配置

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `REX_PORT` | 监听端口 | `3000` |
| `REX_DATA_DIR` | 数据目录 | `~/.rex` |
| `REX_STATIC_DIR` | 前端静态文件目录 | 内嵌 |
| `REX_LOG_LEVEL` | 日志级别 | `info` |

### Agent 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `REX_HUB_URL` | Hub WebSocket 地址 | 必填 |
| `REX_AGENT_TOKEN` | 注册令牌 | 必填 |
| `REX_AGENT_NAME` | Agent 名称 | `agent` |
| `REX_AUTO_UPDATE` | 自动更新 | `true` |

## TLS / HTTPS

Hub 支持自动 HTTPS（ACME/Let's Encrypt）：

```bash
# 设置域名环境变量即可自动启用
REX_DOMAIN=hub.example.com ./rex-hub
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

## 备份

数据存储在 `REX_DATA_DIR`（默认 `~/.rex`），备份 `rex.db` 文件即可：

```bash
cp ~/.rex/rex.db ~/rex-backup-$(date +%Y%m%d).db
```

## 故障排查

| 问题 | 解决方案 |
|------|----------|
| 无法访问 | 检查端口是否被防火墙阻止 |
| Agent 无法连接 | 确认 Hub 地址和注册令牌正确 |
| 数据库错误 | 检查数据目录权限 |
| WebSocket 断开 | 检查反向代理的 WebSocket 配置 |
