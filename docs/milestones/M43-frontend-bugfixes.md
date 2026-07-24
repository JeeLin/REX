# M43: 前端交互修复 + WebSocket 鉴权

## Context
M42 完成了 axum 0.8 升级。在实际使用中发现多个前端交互问题和一个安全缺陷：WebSocket 连接未携带鉴权 token、侧栏资源无法点击打开工作区、新增资源不实时刷新、工作区 conn-tree 与侧栏重复、全屏布局未撑满、侧栏收起后资源混乱、Agent 部署指南入口丢失。
版本类型：patch（bug 修复 + 安全修复），版本号 0.38.1 → 0.38.2。

## 产品边界

**本阶段做：**
- 修复 WebSocket（terminal/tunnel）缺少鉴权 token 的安全缺陷
- 侧栏资源项添加点击事件，在工作区打开对应 Tab
- 新增资源后侧栏实时刷新（无需手动刷新页面）
- 移除工作区页面冗余的 ConnectionTree 组件
- 修复页面右侧空白、未覆盖全屏的布局问题
- 移除侧栏收起功能（收起后资源连接信息混乱）
- 恢复 Agent 部署指南入口

**本阶段不做：**
- 新功能开发
- 数据库 schema 变更
- API 端点变更
- 后端协议实现

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | WebSocket 鉴权修复（前端传 token + 后端校验） | ✅ |
| 2 | 侧栏资源点击打开工作区 | ⬜ |
| 3 | 新增资源实时更新侧栏 | ⬜ |
| 4 | 移除工作区冗余 ConnectionTree | ⬜ |
| 5 | 全屏布局修复 | ⬜ |
| 6 | 移除侧栏收起功能 | ⬜ |
| 7 | 恢复 Agent 部署指南入口 | ⬜ |

## 子任务详细设计

### 1 WebSocket 鉴权修复

- **功能目标**
  前端 WebSocket 连接（terminal、tunnel）必须携带 JWT token，后端校验失败时拒绝连接。

- **文件结构**
  修改：
  - `packages/rex-console-web/src/features/terminal/useTerminal.ts` — WebSocket URL 添加 `&token=`
  - `packages/rex-console-web/src/features/tunnel/useTunnel.ts`（如存在） — 同上
  - `crates/rex-hub/src/terminal_ws.rs` — TerminalQuery 添加 token 字段，ws_handler 校验
  - `crates/rex-hub/src/tunnel_ws.rs` — TunnelQuery 添加 token 字段，ws_handler 校验

- **接口设计**
  前端 WebSocket URL 格式：
  ```
  ws://host/ws/terminal?resourceId=xxx&token=rex_session_xxx
  ws://host/ws/tunnel?agent_id=xxx&resource_id=xxx&token=rex_session_xxx
  ```

  后端 TerminalQuery / TunnelQuery 新增：
  ```rust
  #[serde(rename = "token")]
  pub token: Option<String>,
  ```

- **交互设计**
  无 UI 变更。后端校验失败时通过 WebSocket 发送 error 消息后关闭连接。

- **后端流程**
  1. `ws_handler` 从 Query 提取 token
  2. 调用 `state.auth.verify_token(&token)` 验证
  3. 验证失败 → 发送 `{"type":"terminal.error","payload":{"message":"unauthorized"}}` → 关闭连接
  4. 验证通过 → 继续原有逻辑

  注：`AuthUser` 中间件已注册在 `/ws/terminal` 和 `/ws/tunnel` 路由上，支持 `?token=` query param。但 WebSocket upgrade 后中间件不再执行，因此 handler 内需要二次校验（或确认中间件在 upgrade 前已校验）。经检查，`AuthUser` 作为 axum extractor 在 upgrade 前的 HTTP 请求阶段执行，已能拦截无 token 请求。因此**前端只需在 URL 中添加 token**，后端 handler 无需额外校验。

- **测试标准**
  - 无 token 的 WebSocket 连接返回 401 并被拒绝
  - 带有效 token 的连接正常建立
  - 带过期/无效 token 的连接返回 401
  - 前端 `bun run type-check` 通过

- **提交信息**
  `fix(security): add JWT token to WebSocket connections for terminal and tunnel`

### 2 侧栏资源点击打开工作区

- **功能目标**
  点击侧栏 ResourcePanel 中的资源项，自动在工作区打开对应的 Tab 并导航到工作区页面。

- **文件结构**
  修改：
  - `packages/rex-console-web/src/features/resource-panel/ResourcePanel.vue` — 添加点击事件 emit
  - `packages/rex-console-web/src/pages/WorkspacePage.vue` — 监听资源打开事件
  - `packages/rex-console-web/src/stores/workspace.ts`（新建） — 工作区状态管理（打开的 Tab 列表、活跃 Tab）

- **接口设计**
  ResourcePanel emit 事件：
  ```ts
  const emit = defineEmits<{
    openResource: [resource: { id: string; name: string; protocol: string; host: string; port?: number; username?: string; environmentId: string }]
  }>()
  ```

  新建 `stores/workspace.ts`：
  ```ts
  export const useWorkspaceStore = defineStore('workspace', () => {
    const tabs = ref<Tab[]>([])
    const activeTab = ref('')
    function openResource(resource: ResourceInfo) { /* 去重 + 打开 Tab */ }
    return { tabs, activeTab, openResource }
  })
  ```

- **交互设计**
  点击资源项 → 导航到 `/workspace` → workspace store 打开 Tab（去重：相同 resourceId 不重复打开）。

- **后端流程**
  无后端变更。

- **测试标准**
  - 点击侧栏 SSH 资源 → 跳转工作区，SSH Tab 打开并连接
  - 点击已打开的资源 → 切换到已有 Tab（不重复打开）
  - 点击不同协议资源 → 打开对应协议 Tab
  - 前端 `bun run type-check` 通过

- **提交信息**
  `feat(workspace): click resource in sidebar to open workspace tab`

### 3 新增资源实时更新侧栏

- **功能目标**
  通过 WizardModal 新增资源后，侧栏 ResourcePanel 立即显示新资源，无需手动刷新页面。

- **文件结构**
  修改：
  - `packages/rex-console-web/src/stores/environments.ts` — createResource 后更新本地资源列表
  - `packages/rex-console-web/src/features/resource-panel/ResourcePanel.vue` — 从 store 读取资源而非独立 fetch

- **接口设计**
  environments store 新增：
  ```ts
  const envResources = ref<Map<string, Resource[]>>(new Map())

  async function fetchResources(envId: string): Promise<Resource[]> {
    const resources = await resourcesApi.listByEnv(envId)
    envResources.value.set(envId, resources)
    return resources
  }

  async function createResource(envId: string, data: NewResource): Promise<Resource> {
    const resource = await resourcesApi.create(envId, data)
    // 更新 store 中的资源列表
    const list = envResources.value.get(envId) || []
    list.push(resource)
    envResources.value.set(envId, [...list])
    // 更新环境 resource_count
    const env = environments.value.find(e => e.id === envId)
    if (env) env.resource_count++
    return resource
  }
  ```

- **交互设计**
  WizardModal 创建成功 → store 更新 → ResourcePanel 通过 watch 自动刷新列表。

- **后端流程**
  无后端变更。

- **测试标准**
  - 新增资源后侧栏立即显示
  - 删除资源后侧栏立即移除
  - 多环境切换时资源列表正确
  - 前端 `bun run type-check` 通过

- **提交信息**
  `fix(ui): refresh resource list in sidebar after create/delete`

### 4 移除工作区冗余 ConnectionTree

- **功能目标**
  移除工作区页面中独立的 ConnectionTree 组件，统一使用侧栏 ResourcePanel 作为资源入口。

- **文件结构**
  修改：
  - `packages/rex-console-web/src/pages/WorkspacePage.vue` — 移除 ConnectionTree 引用、ws-tree 区域、treeCollapsed 状态、相关样式
  删除（或保留但不再引用）：
  - `packages/rex-console-web/src/features/workspace/ConnectionTree.vue` — 如无其他引用则删除

- **接口设计**
  移除 WorkspacePage 中的：
  - `import ConnectionTree from ...`
  - `<ConnectionTree @open-resource="openResourceFromTree" />` 及其外层 `<div class="ws-tree">`
  - `treeCollapsed` ref
  - `openResourceFromTree` 函数（功能迁移到 workspace store 的 `openResource`）
  - `.ws-tree` / `.ws-tree-toggle` 相关 CSS

- **交互设计**
  工作区左侧不再有独立的连接树面板。用户通过侧栏 ResourcePanel 点击资源打开 Tab。

- **后端流程**
  无后端变更。

- **测试标准**
  - 工作区页面无 ConnectionTree 显示
  - 侧栏点击资源仍能正常打开 Tab
  - 工作区空间更宽敞
  - 前端 `bun run type-check` 通过

- **提交信息**
  `refactor(workspace): remove redundant ConnectionTree, use sidebar ResourcePanel`

### 5 全屏布局修复

- **功能目标**
  修复页面右侧空白问题，确保内容区域撑满可用空间。

- **文件结构**
  修改：
  - `packages/rex-console-web/src/layouts/AppLayout.vue` — 修复 `.main` 和 `.content` 的 CSS
  - 可能涉及 `packages/rex-console-web/src/pages/WorkspacePage.vue` — 工作区页面自身布局

- **接口设计**
  CSS 修复（具体值需在开发时确认）：
  ```css
  .main {
    flex: 1;
    min-width: 0;        /* 防止 flex 子元素溢出 */
    overflow: hidden;
  }
  .content {
    width: 100%;
    height: 100%;
  }
  ```

- **交互设计**
  页面内容自然撑满浏览器窗口，右侧无空白。

- **后端流程**
  无后端变更。

- **测试标准**
  - 页面内容撑满浏览器窗口
  - 工作区全屏模式正常
  - 不同分辨率下布局正确
  - 前端 `bun run type-check` 通过

- **提交信息**
  `fix(layout): ensure content fills full viewport width`

### 6 移除侧栏收起功能

- **功能目标**
  移除侧栏收起/展开功能，侧栏始终以展开状态显示，避免收起后资源连接信息混乱。

- **文件结构**
  修改：
  - `packages/rex-console-web/src/layouts/AppLayout.vue` — 移除 collapsed 状态、收起按钮、collapsed 相关 CSS

- **接口设计**
  移除：
  - `const collapsed = ref(...)` 及 localStorage 持久化
  - `watch(collapsed, ...)` 
  - 侧栏收起按钮（`.nav-toggle`）
  - `.app-layout--collapsed` 相关 CSS
  - nav-item 的 collapsed 模式样式

- **交互设计**
  侧栏始终展开显示导航图标 + 文字 + 资源面板。不再有收起按钮。

- **后端流程**
  无后端变更。

- **测试标准**
  - 侧栏始终展开
  - 无收起按钮
  - 侧栏内容完整显示
  - 前端 `bun run type-check` 通过

- **提交信息**
  `refactor(ui): remove sidebar collapse, keep sidebar always expanded`

### 7 恢复 Agent 部署指南入口

- **功能目标**
  恢复 M41 添加的 Agent 部署指南弹窗入口，确保用户能访问 Agent 部署说明。

- **文件结构**
  修改：
  - `packages/rex-console-web/src/pages/AgentPage.vue`（或对应 Agent 管理页面） — 检查部署指南按钮/弹窗是否被意外移除或隐藏

- **接口设计**
  检查 Agent 页面中 DeployGuideModal 的引用和触发按钮是否存在。如果存在但被隐藏，恢复显示。如果组件被移除，从 git 历史恢复。

- **交互设计**
  Agent 管理页面中能看到「部署指南」按钮，点击弹出部署方式选择弹窗。

- **后端流程**
  无后端变更。

- **测试标准**
  - Agent 页面显示部署指南入口
  - 点击后弹窗正常显示 4 种部署方式
  - 前端 `bun run type-check` 通过

- **提交信息**
  `fix(ui): restore Agent deployment guide modal entry`

## 设计核对点

1. **安全性**：所有 WebSocket 连接必须携带有效 JWT token
2. **交互一致性**：侧栏是唯一的资源入口，工作区不再有重复的资源树
3. **实时性**：资源 CRUD 操作后侧栏立即反映变更
4. **布局完整性**：内容区域撑满视口，无空白区域
5. **信息可见性**：Agent 部署指南入口始终可达
6. **代码简洁性**：移除未使用的组件和状态（ConnectionTree、collapsed 状态）

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
