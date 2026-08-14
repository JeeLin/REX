# M81: SQL 查询保存 + SSH 初始化脚本 + 缺陷修复与覆盖率补全

## Context

M80 完成了 M78 重设计的收尾（feature 组件 token 化迁移，v0.68.0）。M81 在 M79/M80 已稳固的工作空间与分屏系统之上，补齐两个用户明确要求的体验功能，并修复 M80 阶段在用户实测中暴露的三个缺陷，同时将测试覆盖率补到 90% 门槛。

- SQL 查询保存：用户希望把常用的 SQL 查询以「命名列表」形式保存、复用（对标 Navicat 的查询文件），而非每次重新键入或从浏览器下载。
- SSH 初始化脚本：用户希望在 SSH 连接建立后自动执行一段初始化命令（如 `cd /data/logs`），进入即处于期望的工作目录/环境。
- 缺陷修复：更新检查降级（🔴）、审计日志分页不可见（🟡）、分栏不作用于聚焦 pane（🟡）。
- 覆盖率：将 Rust 与前端测试覆盖率补到 90%。

版本类型：minor（新增两个用户可见功能 + 缺陷修复，向后兼容，无破坏性变更）

## 产品边界

### 本阶段做什么
- SQL 查询命名列表：后端持久化 + 前端保存/打开/重命名/删除 UI。
- SSH 初始化脚本：后端 `SshConfig` 增加 `init_script` 字段，会话建立后逐行执行；前端在资源连接配置中增加输入框。
- 修复 M80 阶段三个缺陷（更新检查 / 审计分页 / 分栏聚焦）。
- 测试覆盖率补到 90%（Rust + 前端）。

### 本阶段不做什么
- 不引入多用户、RBAC、团队协作等概念（违反单用户自托管定位）。
- 不修改文件传输通道（数据不经过浏览器）。
- 不改变 Hub/Agent 版本兼容模型（两者版本必须一致）。
- 不做 SQL 查询的云端同步/跨设备共享（单用户本地存储即可）。
- 不改动分屏的递归渲染算法（M79 已实现），只修复「聚焦 pane 判定」这一交互缺陷。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | SQL 查询保存：后端 API + 持久化（settings 表） | ✅ |
| 2 | SQL 查询保存：前端命名列表 UI（保存/打开/重命名/删除） | ✅ |
| 3 | SSH 初始化脚本：rex-ssh 增加 `init_script` 并在会话建立后执行 | ✅ |
| 4 | SSH 初始化脚本：前端连接配置增加「初始化脚本」输入框 | ✅ |
| 5 | 缺陷修复 🔴：更新检查下载比当前更旧的版本（降级） | ✅ |
| 6 | 缺陷修复 🟡：审计日志分页对用户不可见 / 单薄 | ⬜ |
| 7 | 缺陷修复 🟡：分栏不作用于当前聚焦的 pane | ⬜ |
| 8 | 测试覆盖率补全至 90%（Rust + 前端） | ⬜ |
| 9 | 缺陷修复 🔴：saved-queries 路由 panic 导致 worker 启动崩溃 | ✅ |

## 子任务详细设计

### 1 SQL 查询保存：后端 API + 持久化

- **功能目标**：提供一个「命名 SQL 查询」的全局列表，支持创建/读取/更新/删除（CRUD），数据持久化在 Hub 本地 SQLite 的 `settings` 表（单用户，无需独立表）。
- **文件结构**：
  - `crates/rex-hub/src/sql_api.rs`（新增 `/saved-queries` 路由与 handler）
  - `crates/rex-hub/src/db.rs`（可选：复用 `get_setting`/`set_setting`；或新增 `list_saved_queries`/`upsert_saved_query`/`delete_saved_query` 辅助函数）
  - `crates/rex-hub/src/models.rs`（新增 `SavedQuery { id, name, sql, db_type, updated_at }`）
- **接口设计**（REST，挂载在 `/api/sql` 下）：
  - `GET /api/sql/saved-queries` → `Vec<SavedQuery>`：返回全部命名查询（按 `updated_at` 降序）
  - `POST /api/sql/saved-queries` → `SavedQuery`：新建/覆盖（body: `{ id?, name, sql, db_type }`；无 `id` 时服务端生成）
  - `DELETE /api/sql/saved-queries/:id` → 204：删除
  - 持久化：以 `setting key = "saved_queries"` 存储一个 JSON 数组（`Vec<SavedQuery>`），复用 `get_setting`/`set_setting`，避免新增表结构迁移。
- **后端流程**：handler 调用 db 辅助函数读取现有 JSON 数组 → 增/删/改 → 写回 `set_setting`。重命名即「按 id 找到后改 name 字段」。
- **测试标准**：`db.rs` 增加单元测试覆盖 list/upsert/delete（使用内存 SQLite 或现有测试 fixture）；`sql_api.rs` 增加 handler 测试（若已有集成测试框架则加 e2e）。
- **提交信息**：`feat: add backend CRUD API for named SQL queries (M81 #1)`

### 2 SQL 查询保存：前端命名列表 UI

- **功能目标**：在 SQL 控制台提供「保存查询 / 打开已存查询 / 重命名 / 删除」入口，对标 Navicat 查询文件。
- **文件结构**：
  - `packages/rex-console-web/src/api/sql.ts`（新增 `savedQueries` 的 list/create/remove 调用）
  - `packages/rex-console-web/src/features/sql/SqlPage.vue`（顶栏增加「保存」「查询列表」按钮；列表用 `Select`/弹层展示）
  - 新增 `packages/rex-console-web/src/features/sql/SavedQueryList.vue`（保存/重命名/打开/删除弹层，使用 `components/ui/*`）
  - `packages/rex-console-web/src/stores/`（如需要，`useSqlSavedQueries` store 缓存列表）
- **交互设计**：
  - 编辑器现有「保存」按钮（`SqlPage.vue:299 onSave` 当前为浏览器下载 `.sql`）改为「保存为命名查询」：弹出输入名称的弹层 → 调 `POST /saved-queries`。
  - 顶栏增加「查询列表」下拉：列出已存查询，点击「打开」把 `sql` 载入当前编辑器、「重命名」弹出改名、「删除」二次确认后调 `DELETE`。
  - 写入设计 token（`--bg-elevated`/`--border`/`--text-*`），不引入硬编码 hex。
- **前端流程**：打开列表 → `GET /saved-queries` 载入 store → 操作触发对应 API → 乐观更新 store。
- **测试标准**：存在的前端测试（如有）覆盖 store 的 list/open/rename/delete 逻辑；至少补充一个组件挂载冒烟测试。
- **提交信息**：`feat: add named SQL query save/open/rename/delete UI (M81 #2)`

### 3 SSH 初始化脚本：rex-ssh 增加 init_script

- **功能目标**：SSH 会话建立、认证完成后，自动逐行发送 `init_script` 中的命令（如 `cd /data/logs`），让用户进入即处于期望上下文。
- **文件结构**：
  - `crates/rex-ssh/src/lib.rs`（`SshConfig` 增加 `pub init_script: Option<String>`；`SshSession::connect` 建立后执行）
  - `crates/rex-hub/src/terminal_ws.rs`（`SshConfig { ... }` 构造处补充 `init_script`，从 `config_json` 解析）
  - `crates/rex-hub/src/db.rs`（`ResourceConnInfo` 透传 `init_script`）
- **接口设计**（数据模型）：`SshConfig.init_script: Option<String>`；多行以 `\n` 分隔，逐行发送。
- **后端流程**：`SshSession::connect` 在 shell 通道建立、收到首屏提示后，对 `init_script` 按 `\n` split，逐行 `channel.write()` + `flush()`；空行/None 跳过。错误（如命令失败）仅记录日志，不阻断连接（初始化脚本失败不应导致整个终端不可用）。
- **测试标准**：`rex-ssh` 增加单元测试（mock 或独立 sshd 不可用时，至少测试「按行拆分并发送」的纯逻辑：空/单行/多行/None 分支）。
- **提交信息**：`feat: execute init_script after SSH session established (M81 #3)`

### 4 SSH 初始化脚本：前端连接配置输入框

- **功能目标**：在 SSH 资源连接配置中增加「初始化脚本」多行输入框，保存到 `config_json.initScript`。
- **文件结构**：
  - `packages/rex-console-web/src/features/...`（资源创建/编辑向导中 SSH 配置段，增加 `<textarea>`/`Input`）
  - `packages/rex-console-web/src/...`（连接配置 model/类型补充 `initScript?: string`）
- **交互设计**：在 SSH 配置区（与 `keepalive`/`encoding` 同级）增加「初始化脚本」多行输入，placeholder 提示如 `cd /data/logs\necho ready`；保存时序列化进 `config_json.initScript`；读取时回填。
- **前端流程**：向导保存 → 写入 `config_json` → 后端 `terminal_ws.rs` 解析 `initScript` → 构造 `SshConfig.init_script`。
- **测试标准**：组件测试验证 `initScript` 正确读写于 `config_json`；如已有向导测试则补充。
- **提交信息**：`feat: add init_script field to SSH resource config UI (M81 #4)`

### 5 缺陷修复 🔴：更新检查降级

- **功能目标**：更新检查不再把正在运行的版本降级到更旧的「Latest」Release。
- **文件结构**：`crates/rex-hub/src/update_checker.rs`
- **接口设计**：无新接口。
- **后端流程**：
  - 当前：`check_for_update` 用 `/releases/latest` 且仅 `if latest == current { None }`，两者不等即下载（「Latest」被标成更旧版本时降级）。
  - 修复：仍用 `/releases/latest`，但对 `tag_name` 与 `current_version` 做**语义化版本比较**，仅当 `latest` 严格大于 `current` 时返回 `Some`（相等或更旧均不更新，防降级）。比较用仓库内自写的轻量 `compare_version`（按 `.` 分段逐段比较，无第三方依赖；`Cargo.toml` 不新增 crate，保持 `workspace = true` 规则）。
- **测试标准**：`update_checker.rs` 增加单元测试 `compare_version`：`0.68.0` vs `0.65.4` → 不更新；`0.65.4` vs `0.68.0` → 更新；`1.2.0` vs `1.10.0` → `1.10.0` 更靠前（验证逐段比较而非字符串比较）。
- **提交信息**：`fix: update checker only updates to strictly newer semver, not GitHub Latest (M81 #5)`

### 6 缺陷修复 🟡：审计日志分页

- **功能目标**：审计日志分页控件始终可见（数据少时显示「共 N 条，无需翻页」），补充每页条数选择、跳页、明确总数；后端排序稳定。
- **文件结构**：
  - `packages/rex-console-web/src/pages/AuditLogPage.vue`（`v-if="totalPages > 1"` → 始终渲染分页区；补充 page-size `<Select>`、页码输入、总数文案）
  - `packages/rex-console-web/src/api/audit.ts`（`query` 支持传入 `limit` 来自 page-size 选择）
  - `crates/rex-hub/src/db.rs`（`query_audit_log` 的 `ORDER BY time DESC` 改为 `ORDER BY time DESC, id DESC` 稳定二级排序）
- **接口设计**：无新接口；复用 `GET /audit-log?limit&offset` 与 `GET /audit-log/stats`。
- **后端流程**：`ORDER BY time DESC, id DESC` 保证翻页稳定（id 为 UUID 排序稳定）。
- **前端流程**：分页区常显；`pageSize` 可选 20/50/100；`currentPage` 变化时重取；显示「共 {totalCount} 条」。
- **测试标准**：`db.rs` 补充 `query_audit_log` 分页+排序单测（插多行、断言 LIMIT/OFFSET/稳定排序）；前端如有测试补充分页组件逻辑。
- **提交信息**：`fix: always show audit pagination with page-size/total and stable ordering (M81 #6)`

### 7 缺陷修复 🟡：分栏聚焦 pane

- **功能目标**：让「当前聚焦/最近交互的 pane」可靠地写回 `activePaneId`，使状态栏分栏按钮与 `Ctrl+\` 快捷键作用于正确的 pane。
- **文件结构**：
  - `packages/rex-console-web/src/features/workspace/PaneLeaf.vue`（`.ws-pane` 增加 `@focusin`/`@pointerdown` 监听更新 `activePaneId`，替代仅 `@click`）
  - `packages/rex-console-web/src/pages/WorkspacePage.vue`（`splitHorizontal/Vetical` 在不带参时优先取「最近聚焦 leaf」而非陈旧的 `activePaneId`；可由 `usePaneLayout` 暴露 `lastFocusedPaneId`）
- **接口设计**：无新接口。
- **后端流程**：无。
- **前端流程**：任意 pane 内点击/聚焦（含 xterm 终端内部，因 `focusin` 会从终端内部冒泡到 `.ws-pane`）→ 更新 `lastFocusedPaneId` → 分栏时以它为 target。
- **测试标准**：`usePaneLayout` 或 PaneLeaf 增加测试，验证聚焦更新逻辑与 `splitPane` 以聚焦 leaf 为目标（已有 `usePaneLayout` 测试文件可扩展）。
- **提交信息**：`fix: track focused pane so split acts on the active pane, not default leaf (M81 #7)`

### 8 测试覆盖率补全至 90%

- **功能目标**：将 `cargo llvm-cov --workspace` 与前端覆盖率补到 90%。
- **文件结构**：
  - 各 crate `src/*.rs` 的 `#[cfg(test)] mod tests`（优先补齐 rex-common / rex-hub / rex-ssh 的纯逻辑单测）
  - `crates/rex-hub/tests/`（集成测试，沿用已有 `api_integration.rs` 模式）
  - `packages/rex-console-web/src/**/__tests__/`（前端单测，沿用 `usePaneLayout.test.ts` 模式）
- **接口设计**：无。
- **后端流程**：运行 `mise run test-coverage`（即 `cargo llvm-cov --workspace --summary-only`），定位低覆盖模块，补单测；不改变既有行为。
- **前端流程**：运行 `bun run test`（或项目既有 coverage 命令），补关键 composable/store/工具函数测试。
- **测试标准**：`cargo llvm-cov --workspace` 总覆盖率 ≥ 90%；前端覆盖率 ≥ 90%；CI 命令全绿。该子任务贯穿 #1–#7（每个功能都自带测试），最后统一补缺口。
- **提交信息**：`test: raise Rust + frontend coverage to 90% (M81 #8)`

### 9 缺陷修复 🔴：saved-queries 路由 panic 导致 worker 启动崩溃

- **功能目标**：修复 `sql_api.rs:63` 用 axum 0.8 不兼容的 `:id` 冒号捕获语法注册 DELETE 路由，导致 worker 启动即 panic（exit_code=101），整个 Hub 起不来。
- **文件结构**：`crates/rex-hub/src/sql_api.rs`
- **接口设计**：无接口变更（`DELETE /api/sql/saved-queries/{id}` 语义不变，仅路由字面量语法修正）。
- **后端流程**：将路由字面量 `/saved-queries/:id` 改为 axum 0.8 要求的 `/saved-queries/{id}`；handler `delete_saved_query` 的 `Path(id)` 提取无需改动。
- **测试标准**：编译通过 + `cargo build -p rex-hub` 不 panic；如已有路由冒烟测试可补充对 DELETE 端点的覆盖。
- **提交信息**：`fix: use axum 0.8 path syntax for saved-queries DELETE route (M81 #9)`

## 设计核对点

- 单用户、自托管定位不被破坏（SQL 保存用本地 settings，无云端共享/RBAC）。
- 文件传输通道不变（init_script 仅影响终端会话，不经过浏览器）。
- Hub/Agent 版本一致模型不变（更新检查修复后仍要求严格匹配）。
- 分屏递归渲染算法（M79）不被改动，只补交互层聚焦判定。
- 前端改动统一使用设计 token，不引入硬编码 hex。
- 新增功能均带测试，覆盖率达到 90% 门槛。

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
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

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
| [x] | 🔴 | 更新检查会下载比当前更旧的版本（降级） | 缺陷池（M80） | 当前 0.68.0，GitHub「Latest」为 0.65.4 时仍提示 UPDATE_AVAILABLE 并下载降级。`update_checker.rs` 仅用 `releases/latest` 且只判 `==`，未做语义化版本比较。M81 #5 修复。 |
| ⬜ | 🟡 | 审计日志分页对用户不可见 / 单薄 | 缺陷池（M80） | 分页条被 `v-if="totalPages > 1"` 隐藏，≤50 条时不可见；缺每页条数/跳页/总数。M81 #6 修复。 |
| ⬜ | 🟡 | 分栏不作用于当前聚焦的 pane | 缺陷池（M80） | `activePaneId` 仅 `.ws-pane` `@click` 更新，xterm focus 后失效，状态栏/快捷键分栏永远作用于默认叶子。M81 #7 修复。 |
| [x] | 🔴 | saved-queries 路由 panic 导致 worker 启动崩溃 | 用户反馈 | `sql_api.rs:63` 用 axum 0.8 不兼容的 `:id` 冒号捕获语法注册 DELETE 路由，worker 启动即 panic（exit_code=101）。已改为 `{id}`。M81 #9 修复。 |
