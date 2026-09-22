# Docker 构建与部署

## PID 1 信号转发

```text
docker stop → SIGTERM → PID 1 (supervisor)
  ↓
supervisor 转发 SIGTERM → worker 子进程
  ↓
worker 优雅关闭（关闭 WebSocket 连接、释放资源）
  ↓
worker 退出
  ↓
supervisor 退出
  ↓
容器停止
```

## 超时处理

```text
docker stop -t 30 → SIGTERM → 等待 30 秒 → SIGKILL
```

supervisor 收到 SIGTERM 后，应在 30 秒内完成 worker 关闭和自身退出。

---

## Hub Dockerfile（`Dockerfile.hub`）

```dockerfile
FROM ubuntu:24.04

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Hub 二进制（已通过 include_dir! 嵌入前端 dist）
COPY dist/rex-hub /app/rex-hub
RUN chmod +x /app/rex-hub

# Agent 二进制（供 /api/agents/download 使用）
COPY dist/agents/ /app/agent-binaries/

RUN mkdir -p /app/data /app/data/certs /app/data/acme /app/data/self-signed

ENV REX_DATA_DIR=/app/data
ENV REX_AGENT_BINARIES_DIR=/app/agent-binaries
EXPOSE 3000
EXPOSE 80
EXPOSE 443

# supervisor 作为 PID 1，spawn worker 子进程
CMD ["/app/rex-hub"]
```

## Agent Dockerfile（`Dockerfile.agent`）

```dockerfile
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false agent

WORKDIR /app
RUN mkdir -p /app/data && chown agent:agent /app/data

# buildx 按 TARGETARCH 复制对应架构二进制
ARG TARGETARCH
COPY dist/rex-${TARGETARCH} /app/rex-agent
RUN chmod +x /app/rex-agent

USER agent
ENV REX_DATA_DIR=/app/data

CMD ["/app/rex-agent"]
```

## Docker 内更新限制

Docker 内可以更新二进制文件，但不能更新"当前镜像"。

因此 Docker 部署下的自动更新语义是：

- 容器内二进制可以替换、重启 worker。
- 镜像本身不会自动变成新版本。
- 用户后续仍需要手动 `docker pull` / 重启镜像，或者由外部部署系统完成镜像更新。
- REX 内部只保证当前容器内二进制和 worker 的更新/回滚。

这一点需要在前端更新提示中区分：

- Hub Docker：提示"检测到新版本，点击后下载二进制并重启容器内进程；镜像仍需手动更新"。
- Hub 二进制：提示"检测到新版本，点击后替换二进制并重启"。

## Docker 停止流程

```text
docker stop
  ↓
SIGTERM 发给 PID 1
  ↓
supervisor 设置 stopping
  ↓
supervisor 发送 SIGTERM 给 worker
  ↓
worker 关闭连接、刷新状态
  ↓
worker 退出
  ↓
supervisor 退出
```

实现要求：

- supervisor 必须处理 `SIGTERM`。
- supervisor 不能忽略 Docker stop。
- worker 必须在 30 秒内退出。
- 如果 worker 不退出，supervisor 可以发送 `SIGKILL`，然后自身退出。

---

## Docker Compose 部署

### Hub

创建 `.env` 文件：

```bash
RUST_LOG=info
GITHUB_REPO_OWNER=<ghcr.io 仓库 owner，镜像为 ghcr.io/<owner>/rex-hub:latest>
```

镜像与 compose 文件：`ghcr.io/${GITHUB_REPO_OWNER}/rex-hub:latest`，compose 文件为 `docker-compose.hub.yaml`（映射端口 3000/80，healthcheck 探测 `http://127.0.0.1:3000/`）。

启动：

```bash
docker compose -f docker-compose.hub.yaml up -d
```

停止：

```bash
docker compose -f docker-compose.hub.yaml down
```

查看日志：

```bash
docker compose -f docker-compose.hub.yaml logs -f hub
```

数据持久化在 Docker 命名卷 `hub-data` 中。

### Agent

创建 `.env` 文件：

```bash
REX_HUB_URL=https://your-hub.com
REX_AGENT_TOKEN=<环境注册令牌>
GITHUB_REPO_OWNER=<ghcr.io 仓库 owner，镜像为 ghcr.io/<owner>/rex-agent:latest>
```

可选：`REX_TLS_INSECURE=true`（Hub 使用自签名证书时）。Agent 无入站端口，healthcheck 用 `pgrep` 探测进程存活；卷中挂载了 `/var/run/docker.sock`。

启动：

```bash
docker compose -f docker-compose.agent.yaml up -d
```

停止：

```bash
docker compose -f docker-compose.agent.yaml down
```

查看日志：

```bash
docker compose -f docker-compose.agent.yaml logs -f agent
```

数据持久化在 Docker 命名卷 `agent-data` 中（包含 `agent.json` 身份文件）。
