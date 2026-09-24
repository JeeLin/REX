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

> v0.89 起被 Alt 1 边缘反代取代：`api_request`/`api_response` 隧道机制删除，改由 Agent 标准反代直打 `REX_HUB_URL`（见下节）。

Agent 内嵌前端静态资源，浏览器可直接访问 Agent 的 HTTP 端口；API 请求经同一条 `/ws/agent` 隧道转发回 Hub：

```text
浏览器
  ↓ HTTP（Agent 内嵌前端同源请求）
Agent HTTP server
  ↓ WebSocket 隧道 api_request / api_response
Hub API
```

这样内网部署时前端加载后所有 API 调用不再 404（v0.85 方案；v0.89 起由下节 Alt 1 反代取代）。

## Agent Web 访问通道（v0.89 重设计）

> 状态：设计裁决已定（S1 + S1′ 改判），实现随 v0.89.0 S-fix/S4 落地。根因探查与方案评审的完整记录见
> `.dev-flow/milestones/v0.89.0-agent-channel-redesign.md`（Context / 产品边界）。
> **2026-09-24 用户确认改判**：由原「方案 C：隧道流式帧 + Hub 明文回环自代理」改判为
> 「**Alt 1：Agent 标准反向代理直打 `REX_HUB_URL`**」，方案 C 否决理由见文末「方案 C 否决记录」。

### 现状问题

Agent 模式下浏览器同源的 `/ws/*`（终端、SIP）全部握手失败，根因链路（file:line 见里程碑文档 Context）：

1. 前端 WS URL 一律相对同源构造（`WorkspaceTerminal.vue:345`（S5 提交后行号）、`api/sip.ts:163-165`），
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

### 目标架构：Alt 1 Agent 标准反向代理（2026-09-24 改判定稿）

Agent 退化为「静态资源 + `/api/health` 本地直答 + 标准反向代理」，链路拆为两段、方向互不重叠：

**腿 1（浏览器 → Hub，本里程碑核心）**：浏览器同源打 Agent http_server，Agent 出站反代直打 `REX_HUB_URL`，
不经 `/ws/agent` 隧道：

```text
浏览器（同源）
  ↓ HTTP / WebSocket
Agent HTTP server（静态资源 + /api/health 直答 + 反向代理）
  ├─ /api/*（health 除外）→ reqwest 流式反代 → REX_HUB_URL
  └─ /ws/{*path} → tokio-tungstenite 消息级互转 → REX_HUB_URL
                     ↓
              既有 Hub 路由（/api/*、/ws/terminal、/ws/sip …）
```

- `/api/*`：reqwest **流式**转发，body 直通；client **只设 connect timeout、禁设 total timeout**
  （否则杀 SSE / 大文件 / 长连接）。
- `/ws/{*path}`：tokio-tungstenite 消息级互转（Text/Binary/Ping/Pong/Close 透传）；
  分片/压缩两端各自终结，101 握手由两端库处理，不手写。
- 头策略：剥 hop-by-hop（`Connection`/`Keep-Alive`/`Transfer-Encoding`/`TE`/`Trailer`；
  `Upgrade`/`Sec-WebSocket-*` 仅升级路径保留）；`Host` = Hub 主机；
  **`Origin`/`Referer` 改写为 Hub origin**（绕 CSRF Origin=Host 校验，`middleware.rs:103-179`，
  否则 LAN IP 下 POST/PUT/DELETE 全 403）；补 `X-Forwarded-For`/`Proto`/`Host`，防头注入（值校验）。
- TLS 信任复用 `resolve_hub_tls_settings`（insecure > REX_CA_CERT > 系统根），不新写信任逻辑。
- URL scheme 解析统一走 `hub_origin()` helper（https→wss / http→ws），反代与隧道 dialer 共用，
  消除散落手写拼接（修 `REX_HUB_URL=https` 仍连 ws 的 🟡 bug，Alt 1 硬前置）。
- 路由优先级：`/ws/agent` 属隧道不进反代；静态 fallback 保持但不吞 `/api` `/ws`。

**腿 2（Hub → 内网资源，不变）**：既有 `/ws/agent` 隧道只承担控制面 + 文件传输 Binary 帧；
Hub 够不着的内网资源由 Agent 拨号侧连接；direct 资源 Hub 直连。

```text
Hub ←→ /ws/agent 隧道 ←→ Agent 拨号侧 → 内网目标资源
```

删除：`api_request`/`api_response` 旧机制、Hub 回环自调（`agent_ws.rs:732-800`）——
TLS 模式 502 随之自然消失，不需要任何回环 listener。

硬约束核对（全部通过）：浏览器不直连 Hub（只见同源）；文件传输数据不经过浏览器（仍走隧道 Binary 帧）；
Agent 不开入站（反代是出站）；Hub/Agent 版本一致。

### 通道清单

| 路径 | 用途 | 状态（v0.89） |
|------|------|---------------|
| `/api/{*path}` | REST API（agent 模式经 Agent 标准反代直打 `REX_HUB_URL`；`/api/health` Agent 本地直答） | 现有 `api_request` 机制，v0.89 内改反代并删除旧机制 |
| `/ws/terminal` | SSH 终端 WebSocket | 现有 Hub 路由；agent 模式修复为经 Agent 反代 |
| `/ws/sip` | SIP 信令/媒体 WebSocket | 现有 Hub 路由；agent 模式修复为经 Agent 反代 |
| `/ws/tunnel` | 浏览器直连隧道 WebSocket（`tunnel_ws.rs`，Hub 于 `crates/rex-hub/src/rex-hub.rs:390` 注册） | **死路由，保留不删**：当前无前端调用方，v0.90 后评估删除（裁决见下） |
| `/ws/agent` | Hub ↔ Agent 双向隧道（腿 2：控制面 + 文件传输 Binary 帧） | 现有；不再扩展 stream 帧多路复用，仅保留本职 |

### Hairpin 拓扑标注

agent 前端访问「本 agent 环境」资源为双跳 hairpin，**不做本地短路**：

```text
浏览器 → Agent 反代（腿 1）→ Hub → Hub 直连目标（direct）
                              └→ /ws/agent 隧道 → Agent 拨号侧 → 内网目标（agent，腿 2）
```

- `connection_mode` 是环境部署事实，判定留在 Hub；不在访问入口层面翻转，
  不把鉴权 / 审计 / 资源判定拆出 Hub。
- direct 环境在 agent 模式下语义为「可见可连、连接路径经 Hub（经 Agent 反代）」，前端零分支。
- 隧道断线解耦：`/ws/agent` 掉线时浏览器同源 `/api` 仍可用（腿 1 不依赖隧道）；
  Agent 进程退出则两腿全断。

### 范围裁决（S1/S1′，2026-09-24）

| 裁决项 | 结论 | 理由 |
|--------|------|------|
| `/ws/tunnel` 死路由 | **保留不删**，本里程碑不动 | Hub 已注册但前端零引用；删除属范围外改动，避免 v0.89 膨胀。标注「当前无前端调用方，v0.90 后评估删除」 |
| health 响应是否暴露 `hub_url` | **不暴露** | 方案 B（前端 agent 模式 WS 直连 Hub origin）已否决：破坏「浏览器够不着 Hub 也能用」的存在意义，且制造 API/WS 双传输平面。Agent health 保持本地直答，不泄漏回程地址 |
| stream 通道公平调度硬约束 | **已作废** | 原为方案 C 有界队列 HOL 缺陷打的补丁；Alt 1 下腿 1 反代分连接、与隧道物理隔离，结构性无 HOL，约束整类消失。作废理由见「方案 C 否决记录」 |

### 方案 C 否决记录（2026-09-24 用户确认推翻 ora-9 定稿）

方案 C（隧道流式帧 + Hub 明文回环自代理）经复核判负，改采 Alt 1。入档防后人再发明：

1. **方向错配**：隧道本职是 Hub→内网方向；浏览器 API/WS 是 Agent→Hub 同向同址流量，
   绕「自定义帧 → 隧道复用 → Hub 解帧 → 手写 HTTP head → 回环 TCP」三跳纯属自找复杂度。
2. **结构性 HOL**：Hub 共享读循环 `await` 打在每流有界队列上，一慢全堵（文件/心跳全卡）；
   原「公平调度硬约束」正是给该缺陷打的补丁。
3. **S1 评审遗漏 CSRF**：CSRF 强制 Origin=Host（`middleware.rs:103-179`），
   方案 C 全头透传 + Host 回环改写会使 POST/PUT/DELETE 全 403。
4. **自制件成本**：`stream_data` 用 JSON 数组编码 `Vec<u8>` 字节膨胀 2-4×；
   `build_loopback_request_head` 手写 HTTP/1.1 头为自制件。
5. **原始动机复核无一关键**：ora-9 的单数据平面美学、Agent 纯管道、防 HOL 复核均不成立——
   Alt 1 反而更薄（删自定义机制换库代码），HOL 分连接后整类消失。
6. **fix-32 残留全弃**：S2+S3 实现共 1031 行已存档全弃（patch 存于 `.dev-flow`，不入库）。

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

后续数据通过 `channel_id` 复用同一条 WebSocket。其余消息类型：`auth`/`auth_ok`/`auth_fail`（认证）、`heartbeat`/`heartbeat_ack`（心跳）、`close`/`closed`（关闭通道）、`resize`（终端尺寸）、`update`/`update_progress`（自更新指令）。`api_request`/`api_response`（v0.85 Web API 隧道）随 v0.89 Alt 1 改判删除，Web API/WS 改走 Agent 边缘反代（见上）。
