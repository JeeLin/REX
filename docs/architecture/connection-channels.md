# 连接通道

## 直连资源

```text
浏览器
  ↓ WebSocket / REST
Hub worker
  ↓ 协议 crate
目标资源
```

适用：

- 公网 SSH
- 云数据库
- 公网 S3/MinIO
- Hub 所在网络可直接访问的资源

## Agent 代理资源

```text
浏览器
  ↓ WebSocket / REST
Hub worker
  ↓ Agent WebSocket 隧道（/ws/agent）
Agent worker
  ↓ 协议 crate
内网目标资源
```

适用：

- 家庭内网服务器
- 公司内网数据库
- 没有公网 IP 的 NAS
- 不允许开放入站端口的设备

Agent 隧道支持的协议：`ssh`、`sql`（mysql/postgresql/sqlite 等）、`redis`、`sftp`/`s3`（文件）、`sip`、TCP 透传。

## Agent Web API 隧道（v0.85）

Agent 内嵌前端静态资源，浏览器可直接访问 Agent 的 HTTP 端口；API 请求经同一条 `/ws/agent` 隧道转发回 Hub：

```text
浏览器
  ↓ HTTP（Agent 内嵌前端同源请求）
Agent HTTP server
  ↓ WebSocket 隧道 api_request / api_response
Hub API
```

这样内网部署时前端加载后所有 API 调用不再 404。

## Agent Web 访问通道（v0.89 重设计）

> 状态：设计裁决已定（S1），实现随 v0.89.0 S2-S4 落地。根因探查与方案评审的完整记录见
> `.dev-flow/milestones/v0.89.0-agent-channel-redesign.md`（Context / 产品边界）。

### 现状问题

Agent 模式下浏览器同源的 `/ws/*`（终端、SIP）全部握手失败，根因链路（file:line 见里程碑文档 Context）：

1. 前端 WS URL 一律相对同源构造（`WorkspaceTerminal.vue:324-326`、`api/sip.ts:163-165`），
   agent 模式下打到 Agent 内嵌 http_server。
2. Agent 路由表仅 `/api/health` + `/api/{*path}` + 静态 fallback（`crates/rex-agent/src/http_server.rs:157-160`），
   没有 `/ws/*` 路由。
3. `/ws/terminal`、`/ws/sip` 落入 SPA fallback，返回 `200 text/html`
   （`crates/rex-hub/src/embedded_static.rs:62-75`），浏览器等待 101 握手超时失败——
   整条链路从未触及 `/ws/agent` 隧道。
4. 连带问题：Hub 开 TLS 时唯一 listener 被 TLS 包裹（`crates/rex-hub/src/rex-hub.rs:264`），
   API 隧道回环却用明文 `http://127.0.0.1:{port}`（`crates/rex-hub/src/agent_ws.rs:745`），
   TLS 模式下 `/api/*` 整体 502。

影响面：agent 模式下终端与 SIP 对 direct / agent 两类资源全部失败；
SQL/Redis/文件走 `/api/*` 正常，故用户感知为「只有直连 SSH 不行」。

### 目标架构：方案 C 统一流式边缘反代

Agent 退化为「静态资源 + `/api/health` 本地直答 + 纯管道」，
`/api/*` 与 `/ws/*` 全部经同一条 `/ws/agent` 隧道以字节级反代到 Hub 明文回环 listener：

```text
浏览器（同源）
  ↓ HTTP / WebSocket
Agent HTTP server（静态资源 + /api/health 直答 + /ws/*、/api/* 反代）
  ↓ /ws/agent 隧道：stream_open / stream_data / stream_close 多路复用
Hub 明文回环 listener（仅绑 127.0.0.1）
  ↓
既有 Hub 路由（/api/*、/ws/terminal、/ws/sip …）
```

约束：

- 浏览器不直连 Hub；隧道是唯一数据平面（方案 B 已否决）。
- 硬前置：Hub 新增仅绑 `127.0.0.1` 的明文回环 listener，`/api` 与 `/ws` 共用，修 TLS 502。
- 分两阶段：先方案 A 通 `/ws/*`，随后 `/api/*` 迁入 stream 通道并删除
  `api_request`/`api_response` 旧 JSON 隧道机制。
- `stream_open` 后 Hub 侧为原始字节泵，不解析 upgrade 头（hop-by-hop 头保真）；
  发起回环请求时 Host 改写为回环地址。

### 通道清单

| 路径 | 用途 | 状态（v0.89） |
|------|------|---------------|
| `/api/{*path}` | REST API（agent 模式经隧道反代；`/api/health` Agent 本地直答） | 现有 `api_request` 机制，v0.89 内迁入 stream 通道 |
| `/ws/terminal` | SSH 终端 WebSocket | 现有 Hub 路由；agent 模式修复为经 stream 通道反代 |
| `/ws/sip` | SIP 信令/媒体 WebSocket | 现有 Hub 路由；agent 模式修复为经 stream 通道反代 |
| `/ws/tunnel` | 浏览器直连隧道 WebSocket（`tunnel_ws.rs`，Hub 于 `crates/rex-hub/src/rex-hub.rs:390` 注册） | **死路由，保留不删**：当前无前端调用方，v0.90 后评估删除（裁决见下） |
| `/ws/agent` | Hub ↔ Agent 双向隧道（唯一数据平面） | 现有；v0.89 扩展 stream 帧多路复用 |

### Hairpin 拓扑标注

agent 前端访问「本 agent 环境」资源为双跳 hairpin，**不做本地短路**：

```text
浏览器 → Agent → /ws/agent 隧道 → Hub → Hub 直连目标（或再经隧道回 Agent）
```

- `connection_mode` 是环境部署事实，判定留在 Hub；不在访问入口层面翻转，
  不把鉴权 / 审计 / 资源判定拆出 Hub。
- direct 环境在 agent 模式下语义为「可见可连、连接路径经 Hub」，前端零分支。

### 范围裁决（S1，2026-09-24）

| 裁决项 | 结论 | 理由 |
|--------|------|------|
| `/ws/tunnel` 死路由 | **保留不删**，本里程碑不动 | Hub 已注册但前端零引用；删除属范围外改动，避免 v0.89 膨胀。标注「当前无前端调用方，v0.90 后评估删除」 |
| health 响应是否暴露 `hub_url` | **不暴露** | 方案 B（前端 agent 模式 WS 直连 Hub origin）已否决：破坏「浏览器够不着 Hub 也能用」的存在意义，且制造 API/WS 双传输平面。Agent health 保持本地直答，不泄漏回程地址 |
| stream 通道调度约束 | **公平调度为硬约束** | `stream_data` 走有界 mpsc 背压，与文件传输 `Binary` 帧公平调度，防「开大文件时终端卡顿」（HOL 阻塞）；实现随 S2 落地并测试 |

## SIP 通道

```text
浏览器（/ws/sip）
  ↓
Hub
  ├─ 直连：Hub 内起 UA₁（rex-sip/baresip）
  └─ Agent 代理：经 /ws/agent 隧道转发控制帧 + 媒体帧，Agent 内起 UA₂
```

- 控制帧为 JSON（`sip.dial` / `sip.answer` / `sip.hangup` / `sip.hold` 等），媒体为 Binary（S16LE PCM）
- Agent 链式时 Hub 只做 JSON 控制/事件中继，真实 SIP 终端在 Agent 侧

## 通道协议

Hub 与 Agent 之间通过同一条 WebSocket（`/ws/agent`）复用传输：控制消息为 JSON 文本帧，数据通道按 `channel_id` 多路复用，媒体/二进制数据为 Binary 帧。

建立连接（Hub → Agent）：

```json
{
  "type": "connect",
  "payload": {
    "request_id": "req_abc",
    "resource_id": "res_ssh_1",
    "protocol": "ssh",
    "config": {}
  }
}
```

Agent 响应（Agent → Hub）：

```json
{
  "type": "connected",
  "payload": {
    "request_id": "req_abc",
    "channel_id": "1"
  }
}
```

后续数据通过 `channel_id` 复用同一条 WebSocket。其余消息类型：`auth`/`auth_ok`/`auth_fail`（认证）、`heartbeat`/`heartbeat_ack`（心跳）、`close`/`closed`（关闭通道）、`resize`（终端尺寸）、`update`/`update_progress`（自更新指令）、`api_request`/`api_response`（Web API 隧道）。
