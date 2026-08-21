# M45: 工作区 Bug 修复

## Context

M44 完成后端日志补全（v0.39.0）。用户测试中发现多个工作区核心 bug：分栏后所有面板显示同一连接、无退出按钮、页面标题间距问题、Agent 空页面、审计日志目标显示 ID 而非名称、Settings 日志全量打印等。本里程碑集中修复这些 bug。

版本类型：patch（bug 修复），版本号 0.39.0 → 0.39.1。

## 产品边界

**本阶段做：**
- 工作区分栏 per-pane 标签绑定（核心 bug：所有分栏显示同一连接）
- 退出登录按钮（topbar 缺失）
- 页面标题间距修复（标题紧贴边缘）
- Agent 页面直连模式引导（EmptyState 无操作入口）
- 审计日志 target 显示资源名称（而非 resource_id）
- Settings 日志只打印变更的 key（而非全量）
- IPv6 连接支持
- localhost 空文件排查（待调查，需深入 SFTP 连接逻辑）

**本阶段不做：**
- 右键上下文菜单（M46）
- 工作区功能增强
- 新协议支持

## 子任务清单

| # | 内容 | 预计文件 | 状态 |
|---|------|----------|------|
| 1 | 工作区分栏 per-pane 标签绑定 | `WorkspacePage.vue` | ✅ f9b4df4 |
| 2 | UI 全局修复（退出按钮 + 页面间距 + Agent 引导） | `AppLayout.vue`, `global.css`, `AgentsPage.vue` | ✅ 7cdadae |
| 3 | Settings 日志优化 + Agent 直连模式说明 | `settings_api.rs`, `AgentsPage.vue` | ✅ df086b8 |
| 4 | IPv6 支持 + 资源创建表单校验 | `resource_api.rs`, `WizardModal.vue` | ✅ fd23794 |

## 子任务详细设计

### 1 工作区分栏 per-pane 标签绑定

**目标**

修复核心 bug：所有分栏面板渲染同一个 `activeTabInfo`，切换标签后终端内容不变，分栏后所有面板显示同一连接。

**根因**

`WorkspacePage.vue:532` 的 `<Pane v-for="i in splitCount">` 内所有内容都绑定到 `activeTabInfo`（当前活跃标签），没有 per-pane 的标签分配。

**修改文件**

`packages/rex-console-web/src/pages/WorkspacePage.vue`

**设计**

当前架构：
```
tabs[]       → 所有打开的标签页
activeTab    → 当前活跃标签 ID
splitCount   → 分栏数量
panes[]      → 仅存储标签 ID（未使用）
```

目标架构：
```
tabs[]         → 所有打开的标签页
activeTab      → 当前活跃标签 ID（用于标签栏高亮）
paneTabs[]     → 每个分栏绑定的标签 ID
splitCount     → 分栏数量
```

关键变更：
1. 新增 `paneTabs = ref<string[]>([''])` — 每个分栏对应的 tab ID
2. 标签栏点击 → 同时更新 `activeTab` 和当前分栏的 `paneTabs[currentPane]`
3. `<Pane>` 模板使用 `paneTabs[i]` 查找对应的 tab，而非 `activeTabInfo`
4. 新增分栏时，默认绑定到当前活跃标签（而非空）
5. 关闭标签时，如果某个 pane 绑定了该标签，重置为第一个可用标签
6. 分栏按钮增加 `currentPane` 参数，操作的是当前分栏而非全局

**交互设计**

- 标签栏点击：切换当前活跃分栏的标签内容
- 分栏内操作（split/close）：作用于当前分栏
- Ctrl+Tab：在当前分栏内切换标签（非全局）
- 每个分栏独立显示自己的终端/SQL/Redis/文件组件

**测试标准**

- [ ] 打开 2 个 SSH 标签，切换标签 → 终端内容正确切换
- [ ] 分为 2 栏，每栏绑定不同标签 → 两个终端独立运行
- [ ] 关闭一个分栏 → 另一个分栏不受影响
- [ ] 新建分栏 → 默认绑定当前活跃标签
- [ ] SQL/Redis/文件标签在分栏中正确渲染

**提交信息**: `fix(workspace): implement per-pane tab binding for split view`

### 2 UI 全局修复

**目标**

修复退出按钮缺失、页面标题间距问题、Agent 空页面引导。

**修改文件**

- `packages/rex-console-web/src/layouts/AppLayout.vue` — 添加退出按钮
- `packages/rex-console-web/src/styles/global.css` — 页面内容 padding
- `packages/rex-console-web/src/pages/AgentsPage.vue` — EmptyState 添加操作按钮

**2a 退出登录按钮**

在 AppLayout.vue topbar 的 `topbar-actions` 区域添加 logout 按钮：

```html
<button class="logout-btn" @click="sessionLogout" title="退出登录">⏻</button>
```

**2b 页面标题间距**

在 `global.css` 中为 `.content` 添加 padding：

```css
.content {
  padding: var(--space-4) var(--space-6);
}
```

确保所有页面标题（`.page-title`）不紧贴边缘。

**2c Agent 空页面引导**

修改 AgentsPage.vue 的 EmptyState，在无 agent 时显示：
- 说明文本："Agent 用于远程设备的内网穿透。直连模式无需部署 Agent。"
- "快速开始"按钮 → 打开部署指南 Modal

**提交信息**: `fix(ui): add logout button, page padding, and agent empty state guidance`

### 3 审计日志 target 名称 + Settings 日志优化

**目标**

- 审计日志页面的 target 列显示资源名称而非 resource_id
- Settings 更新日志只打印实际变更的 key

**修改文件**

- `packages/rex-console-web/src/pages/AuditLogPage.vue` — 审计日志 target 名称解析
- `packages/rex-console-web/src/api/audit.ts` — 可能需要扩展 API
- `crates/rex-hub/src/settings_api.rs` — 对比当前值只记录变更

**3a 审计日志 target 名称**

当前 audit log 的 target 字段存储的是 `resource_id`（如 `file_ddf2391c`）。前端需要：
1. 在加载审计日志时，同时加载所有资源列表（从 environments store）
2. 用 `resource_id` → `resource_name` 映射显示名称
3. 如果找不到对应资源，显示原始 ID（fallback）

**3b Settings 日志优化**

当前问题：前端每次 PUT /api/settings 时发送全量 JSON，后端 `body.X.is_some()` 全部为 true。

修复方案：后端在写入前从 DB 读取当前值，只记录实际变更的 key：

```rust
let current_theme = db.get_setting("theme").unwrap_or_default();
if body.theme.is_some() && body.theme.as_deref() != current_theme.as_deref() {
    changed_keys.push("theme");
}
```

**提交信息**: `fix(audit): resolve resource names in audit log target, diff settings changes`

### 4 IPv6 支持 + localhost 空文件排查

**目标**

- 支持 IPv6 地址格式的连接（如 `[::1]:22`、`2001:db8::1`）
- 排查连接后本地出现 localhost 开头空文件的原因

**修改文件**

- `crates/rex-hub/src/resource_api.rs` — host 验证支持 IPv6
- `crates/rex-ssh/src/` — SSH 连接支持 IPv6
- 前端资源创建向导 — host 输入验证支持 IPv6

**4a IPv6 支持**

当前 host 验证可能只接受 IPv4 格式。需要：
1. 后端 host 字段验证支持 IPv6 地址（带方括号或纯十六进制）
2. SSH 连接层正确解析 IPv6 地址
3. 前端 host 输入框的验证规则

**4b localhost 空文件排查**

用户报告连接后本地出现 `localhost` 开头的空文件。可能原因：
1. SSH 端口转发（LocalForward）在本地创建 socket 文件
2. SFTP 连接器在连接时创建临时文件
3. 前端 SFTP drawer 在连接时触发了意外的文件创建

需要在代码中搜索 `localhost` 相关的文件创建逻辑。

**提交信息**: `fix(connection): support IPv6 addresses and investigate localhost empty files`

## 设计核对点

- ✅ 分栏后每个面板独立显示不同标签的内容
- ✅ 退出按钮在 topbar 右侧可见
- ✅ 页面标题不紧贴左侧边缘
- ✅ Agent 空页面有操作引导
- ✅ 审计日志 target 显示资源名称
- ✅ Settings 日志只打印变更 key
- ✅ IPv6 地址格式支持
- ✅ localhost 空文件问题定位

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交

## 打回记录

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
