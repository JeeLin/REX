# REX — 产品重定位 (v2 草案)

> 本档为重定位**草案**（SELF-17）。定位确定后迁入 `README.md` + `PRODUCT.md` §1。

## 0. 锚点：“公网触及不到的东西，要能到达”

REX 存在的根本理由不是“多个资源用多个工具”，而是：**公网无法直接访问的资源，总要能访问** —— 这正是
`Agent` 隧道 + 环境(Environment) / 资源(Resource) 模型的出发点 (`docs/architecture/connection-channels.md`、
`docs/reference/data-models.md`)。

在 Multica 里，你也遇到同一个问题：agent/runtime 是你无法直连的地方，你通过它领取并执行任务。
因此借鉴的落脚点不是“把 Agent 作为一种协议资源类型”，而是**把 Node(节点) 作为不可达→可达的桥**。

## 1. 一句话定位

REX = **个人自托管的统一管理平台**：在同一个界面管你的远程资源与你的本地 Agent，
并经部署在各网络的节点(用 **ACP** 与跑在你机器上的编码 agent 对话)，把任务派下、结果留痕。
> 即:除了"手动操作远程资源",还能"在 REX 里新建/打开一个 agent 会话对话" —— 见 §4。

- **单用户 · 自托管 · 数据自主 · 深色优先** — 沿用原有四个硬约束不变。
- **不引入**：多用户 / RBAC / 团队协作 / 人工审批 —— 见 `AGENTS.md` 硬性约束。

## 2. 统一模型：Target → via Node → Task → Result

```text
                ┌-----------┐
你 (Human)  ────│ Task(工作项) │─── 同步/指派
                └-----------┘
                    │ 作用对象
        ┌-----------┴------------┐
        │  Target = Resource     │  要干什么   (SSH/DB/Redis/文件/S3/SIP …)
        │  Node   = Agent        │  从哪儿到   (公网直连 / Agent 隧道)
        └-----------┬------------┘
                    │ 执行
        via Node ───┴──> Result  (落 durable 记录)
```

- **Target (Resource)** — 资源本身不变（`resources` 表），只改称呼语义：它是"要作用的对象"。
- **Node (Agent)** — `agents` 表不变，语义升格为"可达 + 可执行的节点"。分层于 Target:
  Node 负责"怎么到", Target 负责"要对什么"。这就是"不用把 Agent 当作普通资源类型"的道理。
- **Task** — 新的一等工作项：一次"针对 Target, 通过 Node, 做一件事, 留下一条结果"。有状态生命周期。
  - `Console Session` = 你手动驱动、同步的 Task（即今天的终端 / SQL / Redis / 文件格子）。
  - `Agent Task` = 你指派、Node 自主执行、异步的 Task，回写 Result。

> 同构：一个协议资源是 **Terminal**(你去它那看)；一个 Agent/Node 是 **Runtime**(你派活给它,它跑)。
> 同一个统一模型管两者 —— 这才是"统一管理平台"。

## 3. Multica 借鉴（思维方式，非复制）

| Multica 概念 | REX 对应 | 如何落地 |
|---|---|---|
| Issue = 一等工作项，带状态生命周期 | **Task** | `tasks` 表:目标/指派/状态/结果；状态即事实 |
| Agent 绑 runtime 领活，非点开 | **Node** | Agent 升格为可派发节点的 Runtime |
| daemon 认领队列并执行 | **Hub supervisor** | 任务队列 + 调度器，挂在 supervisor/worker |
| Result 为持久记录 | **Task Log** | 复用 `audit_log` (已有 environment/resource/agent 维度) |
| 能力与身份分开 (skills) | Node 能力绑定 | Node 的可用能力/工具按任务或节点绑定 |
| 条件触发重跑 (wakeups) | 任务等待条件 | 任务可挂条件( Agent 在线 / 资源就绪 …) |

**刻意不借** (守住单用户内核): 多用户 / RBAC / 团队与人工审批。只借"思维方式":
工作项 + 执行者 + 结果留痕 + 状态即事实。

> 注: Multica 是团队协作平台, REX 是个人自托管。我们借的是**如何组织"活"**, 而不是借多租户模型。

## 4. Agent 侧落地:ACP 会话 (Agent Client Protocol)

> 本节取代早期"确定性 playbook"的默认假设。ACP 是**标准协议**(类比 LSP),
> REX 只做 **ACP 客户端 + 会话 broker**, 不把 LLM 烧进 REX —— 这才是"借鉴 Multica"
> (其 agent runtime 皆走 ACP)的真正落点。

### 4.1 为什么是 ACP
- **标准**:Zed 发起,`agentclientprotocol` 组织维护,Apache-2.0,稳定 **v1**;客户端=编辑器,服务端=agent。
- **Rust 一等公民**:官方 crate `agent-client-protocol`(Zed 自己用的就是它)+ `…-http` 传输版。
- **生态现成**:opencode `opencode acp`、Gemini CLI `gemini --acp`、Goose `goose acp`、
  Qwen `qwen --acp` 皆原生;Claude Code / Codex 经官方 shim(`claude-agent-acp` / `codex-acp`)。
- **会话原生**:`session/new` / `session/load` / `session/prompt` / `session/cancel`
  正好对上"新建或打开会话对话"。

### 4.2 关键决策:不走"交互式 SSH 终端",走 **Node 隧道**
- **否决**:骑在 SSH 终端 PTY 上。终端是裸字节流(回显/行编辑/多路复用不可控),
  ACP 要的是**换行分隔的 JSON-RPC**,塞进共享交互 shell 不可靠。
- **主路(推荐)**:agent CLI 跑在 **Node(你项目所在的机器)** —— 因为 agent 需要你的代码与工具。
  Hub 经既有 `/ws/agent` 隧道新增一条 **ACP channel**(channel_id 多路复用)下发指令;
  Node 本地 spawn agent 子进程,把其 stdio 的 ACP 帧桥接进隧道。**全程不需要 SSH**,复用 REX 现有隧道/鉴权/版本锁。
- **兜底**:无 Node 时,才用已有 SSH 通道 spawn agent 并管道传输 ACP(更脆弱,仅备选)。

### 4.3 通道与会话
- **浏览器 ↔ Hub**:新开 `/ws/acp` WebSocket —— Hub 多会话中继 `session/prompt`、
  下行 `session/update` 流(agent_message_chunk / thought / tool_call / tool_call_update / plan / usage)、
  并把 `session/request_permission` 抬到 UI。
- **Hub ↔ Node**:ACP channel over `/ws/agent`,帧 = 换行分隔 JSON-RPC。
- **会话持久化**:新 `sessions` 表(node_id / agent_id / acp_sessionId / cwd / created / lastActive)
  → 支撑"新建会话"(`session/new`)与"打开会话"(`session/load` 重放历史)。

### 4.4 权限模型(单用户信任面)
agent 发 `session/request_permission {toolCall, options[allow_once/always, reject_once/always]}` →
REX UI 出确认条;或按会话设"只读自动放行 / 写与执行需确认"。这是单用户下的唯一信任闸门。

### 4.5 与"opencode 允许网页"的对照
opencode `serve`/`web` = 服务端持会话,浏览器经 **REST + SSE**(`GET /event`)接入,自带文件树/diff/权限/分享 ——
是**第一方单体契约**,非 ACP。REX 同构:Node=serve 端,Hub=会话 broker+鉴权,浏览器=UI;
差别在 REX **用自己的隧道到达内网 agent**,且与资源管理同在一个界面。
接 opencode 两条路:①`opencode acp` 走统一 ACP;②其 REST/SSE 走更丰富 UI(文件树/diff)—— 二选一,建议先①。

### 4.6 定位合流
一个 ACP 会话 = **Target(机器/项目) × via Node × Task(一段持续对话) × Result(记录+diff+产物)** ——
正是 §2 统一模型在 Agent 侧的落地;会话即 Task 的具体实例。

## 5. M1 可落地骨架 (与两个分叉无关的公共底座)

> 这节把 §4 落到可开工的粒度。以下骨架对"M1 先接 Node 还是先接 Hub 本地"、"首个 agent 选谁"**都不敏感** ——
> 先把这层做出来, 任一方向都能立刻接上。

### 5.1 数据: 会话表
```sql
CREATE TABLE acp_sessions (
  id TEXT PRIMARY KEY,            -- REX 侧 uuid
  acp_session_id TEXT NOT NULL,   -- ACP 分配的 opaque sessionId
  node_id TEXT,                   -- 所在 Node(=agents.id); Hub 本地运行时 NULL
  environment_id TEXT,            -- 归属环境
  agent_key TEXT NOT NULL,        -- opencode | goose | gemini | ...
  cwd TEXT NOT NULL,
  title TEXT,
  status TEXT NOT NULL,           -- active | idle | closed | error
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_active_at TEXT
);
```
> 转录(历史消息)可先不落库, 靠 `session/load` 重放; 需要"跨设备留痕"再补 message 表。
> 落点: `crates/rex-hub/src/migrations.sql`, 沿用 `CREATE TABLE IF NOT EXISTS` 约定
> (现有 environments/resources/agents/audit_log/settings 都在这)。

### 5.2 通道一: 浏览器 ↔ Hub (`/ws/acp`)
Hub 多会话中继, 消息直接映射 ACP 语义:
- client→server: `session.new` / `session.open` / `prompt{blocks[]}` / `cancel` / `permission.reply{optionId}`
- server→client: `session.ready` / `update{sessionUpdate}` / `permission.request{toolCall,options[]}` / `stop{stopReason}` / `error`
> 落点: `crates/rex-hub/src/rex-hub.rs:461-472` 现有 `.route("/ws/terminal"…) / /ws/files / /ws/sip / /ws/tunnel`
> 并列处加一行 + 一个 `acp_ws` handler 模块, 照 `terminal_ws` 模式, 纯加法。

### 5.3 通道二: Hub ↔ Node (复用 `/ws/agent` 隧道)
- 新增 `AgentSessionMsg` 变体(如 `acp_frame`): 隧道本来就按 `channel_id: String` 多路复用, 变体用 `#[serde(tag = "type")]` 区分(现有 `session_open/session_request/session_opened/session_error/session_response/file_chunk`, 见 `crates/rex-common/src/agent_proto.rs`)—— 纯加法, 不动现有变体。
- 建链: Hub 下 `{"type":"acp_spawn","channel_id":N,"agent_key":..,"cwd":..}` → Node spawn agent, 回 `acp_spawned`;
  之后该 channel 上的帧 = **换行分隔 ACP JSON-RPC**(对隧道不透明, Node 负责 pump stdin↔stdout, 不解析 ACP)。
  Hub 侧直接用 crate 自带 `Channel`/`Lines` 传输适配器"说" ACP, 不手写组帧。
- 不采纳 ACP 的 HTTP/WS 草案传输做隧道(v1 稳定=stdio; HTTP/WS 互操作尚早) —— 直接桥 stdio 更稳。

### 5.4 Rust 选型 (已对代码与 crate 双向核实)

- 依赖 `agent-client-protocol`(Zed 自用, Apache-2.0), **精确 pin `="3.3.0"` + `features = ["process"]`**:
  crate 大版本(3.x)≠协议版本, 线上 v1 靠运行时 `ProtocolVersion::V1` 选择;
  SDK 发版极快(83 个版本, 3.0→3.3 只隔 4 天), 必须 pin, 线协议 v1 则稳定。
  `process` 用 smol 系 spawn, 与本仓 tokio(`features = ["full"]`, 已含 process)无冲突,
  官方 client 示例本身就跑在 `#[tokio::main]` 下。MSRV 1.88 / edition 2024, 当前 stable 可编。
- **没有 Client trait 要实现**: `Client.builder()` 注册闭包 —
  `.on_receive_notification::<SessionNotification>(…)` 消费 `session/update`(≥14 变体要做分发),
  `.on_receive_request::<RequestPermissionRequest>(…)` 接 `session/request_permission`,
  再 `.connect_with(transport, …)` 驱动 `initialize → session/new|load → session/prompt`。
- 本机场景零手写帧: `AcpAgent::from_str("opencode acp")` 自动 spawn + 换行分隔 JSON-RPC 组帧。
- `session/cancel` 是 **notification**(fire-and-forget, 无响应); `/ws/acp` 的 `cancel` 由 Hub 翻译成它。
- 参考实现: 官方 `yolo_one_shot_client.rs`(~120 行, 正好是我们的流程:
  spawn→initialize(V1)→session/new→session/prompt + update 打印 + 权限自动放行)。
  注意 `agent-client-protocol-test` 是 `publish = false`, CI 的 mock 要照它的样子 vendor, 不能直接依赖。

### 5.5 M1 开工顺序(可并行/可测试)
1. crate 接入 + Hub 侧 ACP client 封装(照官方 yolo 示例); 用 vendor 的 echo mock agent 过 stdio 集成测试
   (官方 test crate `publish = false`, 不能直接依赖)。
2. `acp_sessions` 表 + 迁移。
3. `/ws/acp` 服务端: session.new/open/prompt/cancel/permission 中继。
4. Node 侧 ACP channel spawn + stdio 桥(或 M1 先在 Hub 本机 spawn 走捷径)。
5. 前端最小会话列表 + 聊天视图(文本 + tool_call + 权限确认)。

### 5.6 仍待你拍板(只影响第 4/5 步的"从哪起")
- M1 先落 **Node 上的 agent**(推荐, 差异化) 还是 **Hub 本机 agent**(最快)?
- 首个接 **opencode**(`opencode acp`) 还是 **goose/gemini**?

## 6. 具体改动计划 (由你确认后执行)

### 文档
- `README.md`:标语 +「核心承诺」加"可达不可达 + 调度任务"。
- `docs/PRODUCT.md` §1:「核心问题 / 两个目标」→"不可达 → 可达 / 手动 → 派发"。
- `docs/PRODUCT.md` §2:加 `2.5 Node(节点)` 与 `2.6 Task(任务)`。
- `docs/PRODUCT.md` §3:加 `3.x 任务管理(Task)`。
- 把本档并入 `docs/POSITIONING.md`（稳定后删或并入 §1）。

### 数据
- 新增 `tasks` 表 (target=resource, node=agent, status, result_id→audit_log, config_json)。
- `agents` 表加 `capabilities`/`node_kind`(tunnel|executor) 列 (向后兼容默认 tunnel)。
- `audit_log` 加 `task_id` 维度 (已有 3 个维度,接起来很顺)。

### 代码
- Hub: 任务队列 + 调度器 (挂 supervisor/worker); Node 认领; 状态推给前端 (WebSocket)。
- Node: 加 executor 子系统 (跑 playbook → 回写 Result); 能力绑定。
- 前端: Task 列表/详情/Result 视图; Node 面板从"隧道管理"→"节点与任务"。

### 门禁
Rust: `cargo fmt --check` / `clippy --workspace --all-targets` / `test`; 前端 `bun run type-check/lint/build` —— 沿用 `AGENTS.md` 质量门禁,任务粒度小不改依赖。

## 7. 边界判定 (什么时候改改就好)

- 把 Agent 当作一种资源类型行不行？— 不行:它不是"Terminal"而是"Runtime"。
- 引入多用户/RBAC 行不行？— 不行:违反硬约束。
- (a)→(b) LLM 自主怎么切？— 换 §4 默认,改 executor 实现,定位不变。
