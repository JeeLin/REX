# M17: TLS/HTTPS 支持

## Context

M0–M16 完成了从项目骨架到 Agent 自动更新的全部功能。当前 Hub 仅支持 HTTP，生产环境需要 TLS 加密传输。

产品文档要求：控制台 ↔ Hub 使用 HTTPS，Agent ↔ Hub 使用 WSS（WebSocket Secure），传输层 TLS 1.3。

本里程碑版本类型：minor（新功能），版本号 0.17.0 → 0.18.0。

## 产品边界

**本阶段做：**
- Hub TLS 支持（HTTPS + WSS）
- 三种证书模式：自签名（开发）、ACME Let's Encrypt（生产）、手动证书（用户自备）
- Agent WSS 连接支持
- 前端 HTTPS 适配
- Docker TLS 端口配置

**本阶段不做：**
- 证书自动续期（ACME 自动续期在 M18 或后续）
- mTLS（双向 TLS）
- 证书管理 UI（后续）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Hub TLS 配置与监听 | ⬜ |
| 2 | Agent WSS 连接支持 | ⬜ |
| 3 | Docker TLS 配置 | ⬜ |

## 子任务详细设计

### 1 Hub TLS 配置与监听

**功能目标**

Hub 支持三种 TLS 模式，通过环境变量配置：

| 模式 | 环境变量 | 说明 |
|------|----------|------|
| 无 TLS | （默认） | HTTP on :3000 |
| 自签名 | `REX_TLS_SELF_SIGNED=true` | 自动生成自签名证书，HTTPS on :3000 |
| 手动证书 | `REX_TLS_CERT` + `REX_TLS_KEY` | 用户提供 PEM 证书和私钥 |
| ACME | `REX_ACME_DOMAIN` + `REX_ACME_EMAIL` | Let's Encrypt 自动签发 |

**文件结构**

新建：
- `crates/rex-hub/src/tls.rs` — TLS 配置、证书加载、ACME 处理

修改：
- `crates/rex-hub/src/rex-hub.rs` — 根据配置选择 TLS 或非 TLS 监听

**接口设计**

环境变量：
```
REX_PORT=3000                    # 监听端口（TLS 和非 TLS 共用）
REX_TLS_SELF_SIGNED=true         # 自签名模式
REX_TLS_CERT=/path/to/cert.pem   # 手动证书模式
REX_TLS_KEY=/path/to/key.pem     # 手动证书模式
REX_ACME_DOMAIN=hub.example.com  # ACME 模式
REX_ACME_EMAIL=admin@example.com # ACME 模式
REX_ACME_STAGING=false           # ACME 测试环境
```

**后端流程**

1. 读取环境变量判断 TLS 模式
2. 自签名：使用 `rcgen` 生成临时证书 + `tokio-rustls` 加载
3. 手动证书：读取 PEM 文件 + `rustls-pemfile` 解析
4. ACME：使用 `rustls-acme` 自动签发 + 缓存证书
5. 配置 `rustls::ServerConfig` + `tokio_rustls::TlsAcceptor`
6. `TcpListener::bind` → `tls_acceptor.accept(stream)` → `axum::serve`

**测试标准**

- 无 TLS：Hub 正常启动，HTTP 可访问
- 自签名：Hub 启动后 HTTPS 可访问（浏览器会警告证书不可信）
- 手动证书：提供有效证书后 HTTPS 正常
- ACME：提供域名后自动签发证书（测试环境）
- cargo clippy + cargo test 通过

**提交信息**

```
feat(hub): add TLS support with self-signed, manual, and ACME modes
```

### 2 Agent WSS 连接支持

**功能目标**

Agent 已有 `https → wss` 的 scheme 转换逻辑，需要确保：
- Agent 连接 Hub 时自动使用 WSS（如果 Hub URL 是 https）
- 自签名证书场景下 Agent 能跳过证书验证（`--insecure` 模式）
- 连接失败时给出明确错误提示

**文件结构**

修改：
- `crates/rex-agent/src/agent_ws.rs` — WebSocket 连接支持 TLS
- `crates/rex-agent/src/rex-agent.rs` — 添加 `REX_TLS_INSECURE` 配置

**接口设计**

新增环境变量：
```
REX_TLS_INSECURE=true    # 跳过证书验证（仅用于自签名开发环境）
```

**后端流程**

1. `build_ws_url` 已支持 `https → wss` 转换
2. `connect_async` 需要配置 TLS connector
3. `REX_TLS_INSECURE=true` 时使用 `DangerousClientConfig` 跳过验证
4. 连接失败时日志输出具体原因（证书错误 vs 网络错误）

**测试标准**

- Agent 连接 HTTP Hub → 正常工作（向后兼容）
- Agent 连接 HTTPS Hub（自签名 + insecure）→ 正常工作
- Agent 连接 HTTPS Hub（无 insecure）→ 证书验证失败，日志明确
- cargo clippy + cargo test 通过

**提交信息**

```
feat(agent): add WSS support with optional TLS insecure mode
```

### 3 Docker TLS 配置

**功能目标**

Docker 镜像和 compose 配置支持 TLS，用户可通过环境变量启用。

**文件结构**

修改：
- `docker-compose.hub.yaml` — 添加 TLS 相关环境变量注释
- `Dockerfile.hub` — 无需改动（已有 EXPOSE 443）

**配置示例**

```yaml
# docker-compose.hub.yaml
services:
  hub:
    ports:
      - "3000:3000"   # HTTP（无 TLS 时）或 HTTPS（有 TLS 时）
      - "80:80"       # ACME HTTP-01 验证
      - "443:443"     # 可选：额外 HTTPS 入口
    environment:
      # 自签名模式（开发）
      - REX_TLS_SELF_SIGNED=true
      # 或手动证书模式（生产）
      # - REX_TLS_CERT=/app/data/certs/hub.crt
      # - REX_TLS_KEY=/app/data/certs/hub.key
      # 或 ACME 模式（生产，需要域名指向此服务器）
      # - REX_ACME_DOMAIN=hub.example.com
      # - REX_ACME_EMAIL=admin@example.com
    volumes:
      # 手动证书模式需要挂载证书目录
      # - ./certs:/app/data/certs:ro
```

**测试标准**

- Docker 启动后默认 HTTP 可访问
- 设置 `REX_TLS_SELF_SIGNED=true` 后 HTTPS 可访问
- 端口映射正确（3000/80/443）
- docker compose up 正常启动

**提交信息**

```
feat(docker): add TLS environment variable configuration
```

## 设计核对点

- [ ] Hub 无 TLS 时正常启动（向后兼容）
- [ ] Hub 自签名模式 HTTPS 可访问
- [ ] Hub 手动证书模式 HTTPS 可访问
- [ ] Hub ACME 模式自动签发证书
- [ ] Agent 连接 HTTPS Hub 正常工作
- [ ] Agent insecure 模式跳过证书验证
- [ ] Docker 配置支持 TLS 环境变量
- [ ] cargo test 通过
- [ ] type-check + build 通过

## Flow Status

- [x] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
