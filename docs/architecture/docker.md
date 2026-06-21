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

## Hub Dockerfile

```dockerfile
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY rex-hub /usr/local/bin/rex-hub
WORKDIR /app

VOLUME ["/app/data"]
EXPOSE 3000

ENTRYPOINT ["rex-hub"]
```

## Agent Dockerfile

```dockerfile
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY rex-agent /usr/local/bin/rex-agent
WORKDIR /app

VOLUME ["/app/data"]

ENTRYPOINT ["rex-agent"]
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
