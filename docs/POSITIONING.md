# REX — 产品重定位 (v3 草案 · 精简)

> 本档为重定位**草案**（SELF-17）。定位确定后迁入 `README.md` + `PRODUCT.md` §1。
> v3 相比 v2 的变化：**砍掉任务模型与远程 agent 通道**，只保留"连接 agent + 对话"这一件事。

## 0. 锚点（不变）

REX 存在的根本理由不是"多个资源用多个工具"，而是：**公网无法直接访问的东西，总要能访问**
—— 这正是 `Agent` 隧道 + 环境(Environment) / 资源(Resource) 模型的出发点
(`docs/architecture/connection-channels.md`、`docs/reference/data-models.md`)。

## 1. 一句话定位

REX = **个人自托管的统一管理平台**：一个界面里，既管你公网够不到的资源，
也**连上你所有的 agent 并直接对话**。

- **单用户 · 自托管 · 数据自主 · 深色优先** — 原有四个硬约束不变（`AGENTS.md`）。
- **不引入**：多用户 / RBAC / 团队协作 / 人工审批。

## 2. Agent 不等于"一种资源类型"

资源(SSH/DB/Redis/文件)的交互是**连上去操作**；agent 的交互是**说话**——
它有会话、会回话、会请求授权。所以 agent 在 REX 里不是"多一个资源格子"，
而是**多一个能对话的对象**。这是"统一"的含义：资源管，agent 聊，同一个界面。

> 一个命名问题待定：现有 `agents` 表指的是**隧道节点**(rex-agent)，与这里的 **AI agent** 撞名。
> 建议 UI 上把隧道叫「节点」，把 agent 留给对话对象。

## 3. 连接方式：ACP (Agent Client Protocol)

用标准协议对接现成的 agent 生态，不自己造轮子、不把 LLM 烧进 REX。

- **标准**：Zed 发起，`agentclientprotocol` 组织维护，Apache-2.0，稳定 **v1**。
- **会话原生**：`session/new`(新建) / `session/load`(打开) / `session/prompt`(对话) / `session/cancel`
  正好对上"新建或打开会话对话"。
- **生态现成**：opencode `opencode acp`、Gemini CLI `gemini --acp`、Goose `goose acp`、
  Qwen `qwen --acp` 皆原生；Claude Code / Codex 经官方 shim。
- **Rust 一等公民**：官方 crate `agent-client-protocol`(Zed 自用，Apache-2.0)。

### 3.1 已核实的接入事实（省得再踩）

- **精确 pin**：`agent-client-protocol = { version = "=3.3.0", features = ["process"] }`。
  crate 大版本 ≠ 协议版本（线上 v1 靠运行时 `ProtocolVersion::V1` 选）；SDK 发版极快
  （83 个版本，3.0→3.3 只隔 4 天），必须 pin。3.x 取消默认特性，不写 `process` 装不上。
- **没有 Client trait 要实现**：`Client.builder()` 注册闭包 ——
  `.on_receive_notification::<SessionNotification>(…)` 消费 `session/update`，
  `.on_receive_request::<RequestPermissionRequest>(…)` 接权限，再 `.connect_with(transport, …)`。
- **本机零手写组帧**：`AcpAgent::from_str("opencode acp")` 自动拉进程 + 换行分隔 JSON-RPC 组帧。
- `session/cancel` 是 **notification**（fire-and-forget，无响应）。
- **不冲突**：本仓 `tokio features = ["full"]`（已含 process）；crate 的 `process` 走 smol 系
  spawn，官方示例本身就跑在 `#[tokio::main]` 下。MSRV 1.88 / edition 2024，stable 可编。
- **参考实现**：官方 `yolo_one_shot_client.rs`（~120 行，流程与我们要做的一致）。
  注意官方 test crate 是 `publish = false`，CI 的 mock 需照抄其结构 vendor。

## 4. M1：连上 agent + 对话（最小可用）

1. **agent 清单** —— 一张表或配置项：`id / name / command / cwd`。
   手填几个即可：`opencode acp`、`gemini --acp`、`goose acp`。
2. **连接** —— Hub 用官方 crate 连上并拉起进程，自动完成 ACP 握手与组帧。
3. **会话** —— `session/new` 新建、`session/load` 打开；只存最小元信息
   （哪个 agent / 哪个 cwd / acp sessionId），全量 transcript 靠 `session/load` 重放，不另建 message 表。
4. **聊天界面** —— 新开 `/ws/acp`（落在 `crates/rex-hub/src/rex-hub.rs:461-472`
   与 `/ws/terminal`、`/ws/files`、`/ws/sip` 并列处，照 `terminal_ws` 模式，纯加法）：
   发 prompt、收 `session/update` 流式渲染、展示工具调用、处理权限请求。
5. **权限** —— 先跟随官方示例自动放行（单用户自用够用）；想收紧就把 handler 换成弹确认条。

**M1 明确不做**（避免又做大）：远程 agent 通道、任务派发/队列、结果留痕维度、多 agent 编排。

### 4.1 布局：复用工作空间，agent 作为新的一类标签

> **决策（2026-10-11，用户提出并采纳）：agent 会话直接开在现有工作空间里，不新建 agent 页面。**
> 原提案是"独立三栏页"，已废弃 —— 理由见下。

```text
┌──────────────────────────────────────────────────────────────────┐
│ (资源连接树里点 agent)  →  工作空间标签条：                        │
│   ▏projA-chat ▏projB-chat ▏server1$ ▏mysql-main ▏＋              │
├────────────┬─────────────────────────────────────────────────────┤
│  侧边栏     │   对话区（流式输出 / 工具调用折叠一行）             │
│  连接树     ├─────────────────────────────────────────────────────┤
│  环境→资源  │   输入…                                    [发送 ⏎] │
│  + agent    │                                                     │
└────────────┴─────────────────────────────────────────────────────┘
```

**为什么复用工作空间（而不是单开一页）：**

1. **这是项目既定的架构约束，不是偏好。** `PRODUCT.md` §4 已写明：
   "各资源控制台不再作为独立全屏页面 —— 它们统一在工作空间的格子内打开"。
   agent 会话若单开一页，等于第一个违反这条约束的东西。
2. **基建几乎全部现成。** 工作空间已有：标签管理（`useTabs`，`TabProtocol` 加一个
   `'acp'` 分支即可）、分栏树（`PaneNode`/`PaneLeaf`/`paneContext`）、
   焦点与快捷键归属（`ownsKeystroke`）、全屏、状态色（`Tab.color` / `Tab.status`）。
   在 `PaneLeaf.vue` 加一个 `acp` 分支渲染对话组件，与 Redis / Files / SIP 现有分支并列。
3. **能力白送。** 一个 agent 在跑时，你照样能同时开着 ssh、mysql 分屏 —— 这正是工作空间的价值。
   单开页则要重新造分屏、全屏、快捷键，且丢掉"边跑 agent 边看服务器"。
4. **统一入口自然成立。** 连接树里 agent 和资源并列（环境→资源→agent），
   点谁开谁 —— "统一管理"由此达成，不需要额外页面。

**改动量（相对单开页）：** `useTabs` 加 `'acp'` 协议分支、`PaneLeaf.vue` 加一个渲染分支、
连接树加 agent 节点、新增 `/ws/acp` 与对话组件。不动分栏/焦点/全屏逻辑。

**添加 agent**：连接树底部的「＋」→ 小弹窗填 **名称 / 启动命令 / 工作目录**
（`opencode acp`、`gemini --acp`），不单独开设置页。

**状态一眼可辨**（沿用现有令牌与 `Tab.status`，不新增颜色）：
| 状态 | 表现 |
|---|---|
| 在线 / 空闲 | `--success` `#3FB950` 实心点 |
| 运行中 | `--accent` `#E8912D` 呼吸/脉冲点 |
| 离线 / 报错 | `--danger` `#F85149` 或 `--st-off` `#484F58` |

**权限请求**：单用户默认自动放行，**界面上不打断**；只有切到"逐次确认"模式才
在对话区底部弹一条确认条（工具名 + 允许/拒绝），不用弹窗盖住整页。

**导航**：侧边栏「Agent 管理」（隧道 agent）更名为「**节点**」；AI agent 出现在
连接树里而非新增导航项 —— 保持侧边栏不膨胀。

**刻意不做**：独立 agent 页面、agent 卡片网格、跨 agent 统计图表、agent 市场或安装器。

## 5. 以后再说（不阻塞 M1）

- **远程 agent**：agent 跑在内网机器上时，复用现有 `/ws/agent` 隧道加一条 ACP channel
  （隧道本就按 `channel_id` 多路复用，加一个 `AgentSessionMsg` 变体即可，
  见 `crates/rex-common/src/agent_proto.rs`）。
- **已跑着的 agent 平台**：用 `agent-client-protocol-http`（reqwest SSE / tungstenite WS）接进来。
- **任务化**：若日后确需"派活 + 留痕"，再引入 Task 模型（v2 草案的 §2/§3 留档备查）。