# M75: Bug Fix & UX Polish

## Context
M74 完成 bug 修复轮次。本里程碑继续修复已知 bug 和 UX 问题。SSH 终端存在较多问题，需以 `bak` 分支中的版本为基准重写。

版本类型：patch
版本号：0.65.2

## 产品边界
本阶段做什么：修复已知 bug，优化 UX，SSH 终端以 bak 分支代码为基准重写
本阶段不做什么：不新增功能

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 1 | SSH 终端重写（以 bak 分支为基准） | 前端 | ✅ |
| 2 | SSH 连接 IPv6 地址支持 | 后端 | ⬜ |
| 3 | isTriggerKey TypeError 崩溃修复 | 前端 | ✅ |
| 4 | 删除资源后侧栏实时更新 | 前端 | ✅ |
| 5 | 更新进度切页面后丢失修复 | 前端 | ⬜ |
| 6 | Agent 下载链接 HTTPS 协议处理 | 后端 | ⬜ |
| 7 | 右键菜单图标大小统一 | 前端 | ⬜ |
| 8 | 设置页面右侧改为滑动式 | 前端 | ⬜ |

## 子任务详细设计

### 1 SSH 终端重写（以 bak 分支为基准）

- **功能目标**：以 `origin/bak` 分支中的 SSH 终端代码为基准，重写当前终端功能，解决字间距过大、多开异常等问题
- **文件结构**（修改）：
  - `packages/rex-console-web/src/features/terminal/useTerminal.ts`
  - `packages/rex-console-web/src/features/terminal/TerminalView.vue`
  - `packages/rex-console-web/src/features/terminal/terminal-themes.ts`
  - `packages/rex-console-web/src/features/terminal/TerminalContextMenu.vue`
  - `packages/rex-console-web/src/features/terminal/TerminalSearch.vue`
  - `packages/rex-console-web/src/features/terminal/MobileTerminalBar.vue`
- **参考代码**：`git diff main..origin/bak -- packages/rex-console-web/src/features/terminal/`
- **实现步骤**：
  1. 对比当前 main 与 origin/bak 分支的终端代码差异
  2. 以 bak 分支代码为基准，保留当前版本中已修复的功能（如 xterm-helpers 遮挡、字体持久化等）
  3. 修复字间距过大、多开异常等问题
  4. 确保主题、右键菜单、查找栏、移动端工具栏功能完整
- **测试标准**：终端可正常打开、多开会话正常、字间距正常、主题切换正常、右键菜单正常
- **提交信息**：`fix: rewrite terminal based on bak branch, fix multi-session and spacing issues`

### 2 SSH 连接 IPv6 地址支持

- **功能目标**：修复 SSH 连接 IPv6 地址失败的问题
- **文件结构**（修改）：
  - `crates/rex-ssh/src/lib.rs`（连接逻辑）
  - `crates/rex-hub/src/terminal_ws.rs`（WebSocket 处理）
- **接口设计**：SSH 连接时对 IPv6 地址加方括号处理（如 `[::1]`），确保端口解析正确
- **后端流程**：解析 host 字段，检测 IPv6 格式，规范化后传入 SSH 连接函数
- **测试标准**：IPv4 和 IPv6 地址均可正常建立 SSH 连接
- **提交信息**：`fix: handle IPv6 addresses in SSH connection`

### 3 isTriggerKey TypeError 崩溃修复

- **功能目标**：修复 `e.key` 为 undefined 时 `toLowerCase()` 抛出 TypeError 的崩溃
- **文件结构**（修改）：
  - `packages/rex-console-web/src/composables/useKeyboardShortcuts.ts`
- **实现**：在 `isTriggerKey` 函数入口增加 `e.key` 的 null/undefined 检查
- **测试标准**：特殊按键（如多媒体键、未识别键）不再触发崩溃
- **提交信息**：`fix: guard against undefined e.key in isTriggerKey`

### 4 删除资源后侧栏实时更新

- **功能目标**：删除资源后左侧连接树侧栏立即同步更新，无需刷新页面
- **文件结构**（修改）：
  - `packages/rex-console-web/src/stores/` 或相关资源 store
  - `packages/rex-console-web/src/components/Sidebar.vue` 或连接树组件
- **实现**：资源删除 API 成功后，触发侧栏资源列表重新加载或从本地状态移除该资源
- **测试标准**：删除资源后侧栏立即反映变化
- **提交信息**：`fix: sync sidebar after resource deletion`

### 5 更新进度切页面后丢失修复

- **功能目标**：设置页面执行 Hub 更新时，切换到其他页面仍能看到更新进度
- **文件结构**（修改）：
  - `packages/rex-console-web/src/features/settings/` 更新相关组件
  - `packages/rex-console-web/src/stores/` 更新状态 store
- **实现**：将更新进度状态提升到全局 store（如 Pinia），不在页面组件内局部管理；或在全局显示进度条/Toast
- **测试标准**：执行更新时切换页面，进度状态不丢失，返回设置页能看到当前进度
- **提交信息**：`fix: persist update progress state across page navigation`

### 6 Agent 下载链接 HTTPS 协议处理

- **功能目标**：Agent 部署指南中的下载链接根据 Hub 当前协议自适应（HTTP/HTTPS），避免浏览器不安全警告
- **文件结构**（修改）：
  - `crates/rex-hub/src/agent_routes.rs` 或前端 Agent 部署指南组件
- **实现**：下载链接使用当前页面的 `location.protocol` 构造，或后端 API 返回完整 URL 时根据 TLS 配置选择协议
- **测试标准**：HTTPS 部署时下载链接为 HTTPS，HTTP 部署时不弹出安全警告（或提示用户）
- **提交信息**：`fix: use matching protocol for agent download URL`

### 7 右键菜单图标大小统一

- **功能目标**：统一左侧资源树右键菜单中所有图标的尺寸
- **文件结构**（修改）：
  - 右键菜单组件（ContextMenu 相关）
- **实现**：为所有菜单项图标设置统一的 width/height（如 16px）
- **测试标准**：右键菜单中所有图标大小一致
- **提交信息**：`fix: unify context menu icon sizes`

### 8 设置页面右侧改为滑动式

- **功能目标**：设置页面保留左侧导航，右侧内容区改为滑动切换而非翻页
- **文件结构**（修改）：
  - `packages/rex-console-web/src/pages/SettingsPage.vue` 或相关设置组件
- **实现**：左侧导航保留，右侧使用 CSS transition 或滑动容器实现平滑切换，替换当前的条件渲染翻页
- **测试标准**：点击左侧导航项时右侧内容平滑滑动切换，无跳变
- **提交信息**：`fix: settings page use slide transition instead of page swap`

## 设计核对点

- [ ] SSH 终端以 bak 分支为基准重写，功能完整性不低于当前版本
- [ ] IPv6 地址连接正常
- [ ] 所有已知 bug 均已修复
- [ ] 未引入新功能
- [ ] 前端 type-check + lint + build 通过

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
| [x] | 🔴 | SSH 终端问题多，需按 bak 版本重写 | 用户反馈 | 已以 bak 分支模式重写为 WorkspaceTerminal.vue，自包含组件 + 自动重连 + 延迟测量 + 命令历史 |
| ⬜ | 🔴 | SSH 连接 IPv6 地址失败 | 用户反馈 | SSH 测试时使用 IPv6 地址会导致连接失败 |
| ⬜ | 🟡 | 更新进度切页面后丢失 | 用户反馈 | 设置页面执行更新时切到其他页面就看不到更新进度了 |
| ⬜ | 🟡 | Agent 下载链接 HTTP 不安全警告 | 用户反馈 | Agent 部署指南下载链接使用 HTTP，浏览器提示不安全连接，需强制 HTTPS 或检测协议 |
| [x] | 🟡 | SSH 终端不支持多开 | 用户反馈 | 新终端组件已支持多开（由 WorkspacePage 的 tab 系统管理） |
| [x] | 🟡 | 删除资源后侧栏不更新 | 用户反馈 | 修复 EnvironmentDetailPage 使用 store.deleteResource 而非直接调用 API |
| [x] | 🔴 | isTriggerKey TypeError 崩溃（复发） | 用户反馈 | 确认为浏览器扩展 content_main.js 触发，非应用代码问题，应用已有 e.key 检查 |
| ⬜ | 🟢 | 左侧资源右键菜单图标大小不一 | 用户反馈 | 侧栏资源右键菜单中各图标尺寸不统一 |
| ⬜ | 🟢 | 设置页面右侧改为滑动式 | 用户反馈 | 设置页面左侧导航保留，右侧选项改为滑动切换而非一页一页翻页 |
