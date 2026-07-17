# REX Agent 部署指南

REX Agent 是部署在内网服务器上的轻量级反向代理进程。它通过 WebSocket 连接到 Hub，代理内网资源的访问，无需开放入站端口。

## 前置条件

1. **Hub 已部署并运行** — 需要 Hub 的 URL（如 `http://hub.example.com:3000`）
2. **获取注册令牌** — 在 Hub 管理页面创建环境后，获取 Agent 注册令牌（token）
3. **记录 Agent ID** — 创建环境时会分配 Agent ID

## 配置

Agent 通过环境变量配置：

| 变量 | 必填 | 说明 |
|------|------|------|
| `REX_HUB_URL` | ✅ | Hub 的 URL（如 `http://192.168.1.100:3000`） |
| `REX_AGENT_TOKEN` | ✅ | Agent 注册令牌 |
| `REX_AGENT_ID` | ✅ | Agent ID（创建环境时分配） |

## 方式一：二进制部署

### 下载

```bash
# 根据系统架构下载对应二进制
# Linux amd64
curl -L https://github.com/your-org/rex/releases/latest/download/rex-agent-linux-amd64 -o rex-agent
chmod +x rex-agent

# Linux arm64
curl -L https://github.com/your-org/rex/releases/latest/download/rex-agent-linux-arm64 -o rex-agent
chmod +x rex-agent
```

### 运行

```bash
export REX_HUB_URL="http://hub.example.com:3000"
export REX_AGENT_TOKEN="your-agent-token-here"
export REX_AGENT_ID="your-agent-id-here"

./rex-agent
```

### Systemd 服务

创建 `/etc/systemd/system/rex-agent.service`：

```ini
[Unit]
Description=REX Agent
After=network.target

[Service]
Type=simple
Environment=REX_HUB_URL=http://hub.example.com:3000
Environment=REX_AGENT_TOKEN=your-agent-token-here
Environment=REX_AGENT_ID=your-agent-id-here
ExecStart=/opt/rex-agent/rex-agent
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

启用并启动：

```bash
sudo systemctl daemon-reload
sudo systemctl enable rex-agent
sudo systemctl start rex-agent

# 查看状态
sudo systemctl status rex-agent

# 查看日志
sudo journalctl -u rex-agent -f
```

## 方式二：Docker 部署

```bash
docker run -d \
  --name rex-agent \
  --restart always \
  -e REX_HUB_URL="http://hub.example.com:3000" \
  -e REX_AGENT_TOKEN="your-agent-token-here" \
  -e REX_AGENT_ID="your-agent-id-here" \
  your-registry/rex-agent:latest
```

## 方式三：Docker Compose 部署

创建 `docker-compose.yml`：

```yaml
version: '3.8'
services:
  rex-agent:
    image: your-registry/rex-agent:latest
    container_name: rex-agent
    restart: always
    environment:
      - REX_HUB_URL=http://hub.example.com:3000
      - REX_AGENT_TOKEN=your-agent-token-here
      - REX_AGENT_ID=your-agent-id-here
```

启动：

```bash
docker compose up -d
```

## 验证连接

部署后，在 Hub 管理页面的 Agent 列表中确认 Agent 状态为 🟢 在线。

也可以查看 Agent 日志确认：

```bash
# 二进制部署
journalctl -u rex-agent -f

# Docker 部署
docker logs -f rex-agent
```

正常日志输出：

```
INFO REX Agent version=0.16.0 status=starting
INFO agent configured hub_url=http://hub.example.com:3000 agent_id=xxx
INFO connecting hub_url=http://hub.example.com:3000
INFO connecting url=ws://hub.example.com:3000/ws/agent?token=xxx
INFO authenticated agent_id=xxx
```

## 故障排查

### Agent 无法连接 Hub

| 现象 | 可能原因 | 解决方案 |
|------|----------|----------|
| `connection refused` | Hub 未运行或端口不对 | 确认 Hub 正在运行，检查 `REX_HUB_URL` |
| `auth failed: invalid token` | Token 错误 | 在 Hub 页面重新获取 Token |
| `auth failed: agent not found` | Agent ID 错误 | 确认 `REX_AGENT_ID` 与 Hub 中的一致 |
| 超时 | 网络不通 | 检查防火墙规则，确保能访问 Hub 的端口 |

### Agent 连接后频繁断开

- 检查网络稳定性
- 检查 Hub 是否重启
- Agent 会自动重连（5 秒间隔）

## 安全建议

1. **Token 保管** — Token 等同于 Agent 的认证凭证，不要泄露
2. **网络限制** — 建议限制 Agent 只能访问 Hub 的端口
3. **HTTPS** — 生产环境建议 Hub 使用 HTTPS（`wss://`），Agent 自动跟随
4. **日志** — 定期检查 Agent 日志，排查异常连接
