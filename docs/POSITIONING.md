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
并经部署在各网络的节点，把任务派下、结果留痕。

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

## 4. 默认执行假设 (a) — 未确定即为草案

由于思路仍在明晗,本草案 **默认假设**:

- **(a) 确定性执行**: Node 自主"干活"先按固定 playbook/脚本; 结果确定可复现, **零 LLM 依赖**,
  贴合 REX 单二进制自守理念。`tasks.config_json` 存指令/脚本体; executor 在 Node 本地运行。
- **暂不引入**:外部 AI CLI (codex/claude 等)的对接 — 独立后续, 会牵"单二进制自包含"理念。

> (a) 决定工程里程碴,不改变本 §1 定位。换 (b) LLM 自主只改 §5 的执行器实现。

## 5. 具体改动计划 (由你确认后执行)

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

## 6. 边界判定 (什么时候改改就好)

- 把 Agent 当作一种资源类型行不行？— 不行:它不是"Terminal"而是"Runtime"。
- 引入多用户/RBAC 行不行？— 不行:违反硬约束。
- (a)→(b) LLM 自主怎么切？— 换 §4 默认,改 executor 实现,定位不变。
