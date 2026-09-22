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
