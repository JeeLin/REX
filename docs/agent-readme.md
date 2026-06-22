# REX Agent

轻量级反向代理进程，部署在内网服务器上，通过 TLS 加密隧道连接 [REX Hub](https://github.com/rexhub/rex-hub)。

内网服务器**无需开放任何入站端口** — Agent 从内网主动出站连接 Hub。

---

## 快速开始

### Docker

```bash
docker run -d \
  --name rex-agent \
  --restart unless-stopped \
  -e REX_SERVER=https://your-hub.com \
  -e REX_TOKEN=<注册令牌> \
  -v agent-data:/app/data \
  -v /var/run/docker.sock:/var/run/docker.sock \
  ghcr.io/rexhub/rex-agent:latest
```

### Docker Compose

```yaml
services:
  agent:
    image: ghcr.io/rexhub/rex-agent:latest
    container_name: rex-agent
    restart: unless-stopped
    volumes:
      - agent-data:/app/data
      - /var/run/docker.sock:/var/run/docker.sock
    environment:
      - REX_SERVER=https://your-hub.com
      - REX_TOKEN=<注册令牌>

volumes:
  agent-data:
    driver: local
```

```bash
docker compose -f docker-compose.agent.yaml up -d
```

### 二进制

```bash
# 下载
curl -fsSL https://get.rexhub.dev/agent/linux-amd64 -o rex-agent
chmod +x rex-agent

# 启动
./rex-agent \
  --server https://your-hub.com \
  --token <注册令牌> \
  --name "内网 Agent"
```

### 配置文件

```yaml
# agent.yaml
server: https://your-hub.com
token: <注册令牌>
name: "内网 Agent"
data-dir: "./agent-data"
auto-update: true
```

```bash
./rex-agent --config agent.yaml
```

---

## 环境变量

| 变量 | 必需 | 说明 |
|------|------|------|
| `REX_SERVER` | 是 | Hub 服务器地址（`https://...`） |
| `REX_TOKEN` | 是 | 环境注册令牌（从 Hub 环境详情页获取） |
| `DATA_DIR` | 否 | 数据目录（默认 `/app/data`） |

---

## 支持平台

| 平台 | 架构 | 二进制 |
|------|------|--------|
| Linux | amd64 | `rex-agent-linux-amd64` |
| Linux | arm64 | `rex-agent-linux-arm64` |
| Linux | armv7l | `rex-agent-linux-armv7l` |
| macOS | arm64 | `rex-agent-mac-arm64` |
| macOS | amd64 | `rex-agent-mac-amd64` |
| Windows | amd64 | `rex-agent-windows-amd64.exe` |

---

## 工作原理

```text
REX Agent (内网)
    │ 主动出站连接 (WebSocket TLS)
    ▼
REX Hub (公网)
    │
    ▼
浏览器控制台
```

Agent 从内网主动连接 Hub，建立 WebSocket 加密隧道。Hub 通过隧道代理所有协议请求（SSH、MySQL、SFTP 等）到内网资源。

---

## 许可证

自托管 · 开源
