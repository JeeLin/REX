# 0.4.0: M3 SSH 终端

## Context
M0 骨架 → M1 设计系统 → M2 工作空间外壳。M3 在 M2 工作空间内接入真实 SSH 终端，是第一个有后端协议接入的功能模块。

前序：M2 工作空间外壳（Tab/分屏/连接树外壳完成，终端区为占位文本）。
后续：M4 数据库控制台、M5 Redis 控制台、M6 文件管理。

版本类型：minor

## 产品边界
- **做**：后端 SSH 连接 + WebSocket 桥接、前端 xterm.js 终端核心、查找栏、右键菜单、主题预设、移动端工具栏
- **不做**：SFTP 抽屉（M6）、终端录制/回放、SSH 隧道/端口转发

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 后端 SSH crate + WebSocket 终端桥接 | ✅ |
| 2 | 前端 xterm.js 终端核心集成 | ✅ |
| 3 | 终端内查找栏（Ctrl+F） | ✅ |
| 4 | 终端右键菜单 | ✅ |
| 5 | 终端主题预设 | ✅ |
| 6 | 移动端浮动工具栏 | ✅ |
| 7 | 测试与收尾 | ✅ |

## 子任务详细设计

### 1 后端 SSH crate + WebSocket 终端桥接

- **功能目标**：rex-hub 通过 WebSocket 接收前端终端指令，经 rex-ssh crate 连接目标 SSH 服务器，双向转发数据
- **文件结构**：
  - `crates/rex-ssh/Cargo.toml`（修改：添加 russh 依赖）
  - `crates/rex-ssh/src/lib.rs`（新增：SshClient 连接/断开/发送/接收）
  - `crates/rex-hub/src/bin/rex-hub.rs`（修改：添加 WebSocket 路由 `/ws/terminal`）
  - `crates/rex-hub/Cargo.toml`（修改：添加 axum ws + tokio-tungstenite 依赖）
- **接口设计**：
  WebSocket 消息协议（JSON）：
  ```ts
  // 前端 → 后端
  { type: "terminal.connect", payload: { host: string, port: number, username: string, password?: string, privateKey?: string } }
  { type: "terminal.data",   payload: { data: string } }  // base64 编码的终端输入
  { type: "terminal.resize",  payload: { cols: number, rows: number } }
  { type: "terminal.disconnect" }

  // 后端 → 前端
  { type: "terminal.connected",  payload: { sessionId: string } }
  { type: "terminal.data",       payload: { data: string } }  // base64 编码的终端输出
  { type: "terminal.disconnected", payload: { reason: string } }
  { type: "terminal.error",      payload: { message: string } }
  ```
- **后端流程**：
  1. 前端发起 WebSocket 连接 `/ws/terminal`
  2. 收到 `terminal.connect` 后，用 russh 建立 SSH 连接
  3. 连接成功发送 `terminal.connected`，失败发送 `terminal.error`
  4. 双向转发：WS `terminal.data` ↔ SSH session stdout/stdin
  5. `terminal.resize` 调用 SSH session 的 set_pty_size
  6. SSH 断开时发送 `terminal.disconnected`
- **测试标准**：`cargo build`、`cargo clippy` 通过；手动测试：前端可通过 WS 建立 SSH 连接并交互
- **提交**：`feat(ssh): add SSH WebSocket terminal bridge`

### 2 前端 xterm.js 终端核心集成

- **功能目标**：在工作空间 Pane 内渲染真实 xterm.js 终端，通过 WebSocket 与后端通信
- **文件结构**：
  - `src/features/terminal/TerminalView.vue`（新增：xterm.js 封装组件）
  - `src/features/terminal/useTerminal.ts`（新增：终端连接/数据/resize 逻辑 composable）
  - `src/pages/WorkspacePage.vue`（修改：Tab 内容从占位文本切换为 TerminalView）
- **接口设计**：
  ```ts
  // useTerminal.ts
  function useTerminal(options: {
    host: string; port: number; username: string;
    password?: string; privateKey?: string;
  }): {
    terminal: ShallowRef<Terminal>;
    connect: () => void;
    disconnect: () => void;
    status: Ref<'disconnected' | 'connecting' | 'connected'>;
    sendResize: (cols: number, rows: number) => void;
  }

  // TerminalView.vue props
  defineProps<{
    tabId: string;
    host?: string;
    port?: number;
    username?: string;
    protocol?: string;
  }>()
  ```
- **交互设计**：
  - 双击连接树资源或 Quick Connect 连接 → 新建 Tab → 自动建立 WebSocket + SSH 连接
  - 终端自适应 Pane 大小（FitAddon），resize 时发送 `terminal.resize`
  - 断开后 Tab 显示 "Disconnected" + "Reconnect" 按钮
  - 连接中显示 spinner + "Connecting to host..."
- **测试标准**：`bun run type-check && bun run lint && bun run build` 通过
- **提交**：`feat(web): integrate xterm.js terminal with WebSocket bridge`

### 3 终端内查找栏（Ctrl+F）

- **功能目标**：非模态查找栏，匹配高亮 + 上/下导航 + 选项
- **文件结构**：
  - `src/features/terminal/TerminalSearch.vue`（新增）
- **接口设计**：
  ```ts
  // TerminalSearch.vue
  defineProps<{ visible: boolean; terminal: Terminal }>()
  defineEmits<{ close: [] }>()
  ```
- **交互设计**：
  - Ctrl+F 打开/关闭（非模态，终端仍可操作）
  - 顶部查找条：输入框 + 上/下 + 区分大小写 + 整词 + 正则 开关
  - 匹配高亮（xterm.js SearchAddon）
  - ESC 关闭
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add terminal search bar (Ctrl+F)`

### 4 终端右键菜单

- **功能目标**：终端区右键弹出操作菜单
- **文件结构**：
  - `src/features/terminal/TerminalContextMenu.vue`（新增）
- **菜单项**：
  - 复制（选中文本 → 剪贴板）
  - 粘贴（剪贴板 → bracketed paste）
  - 全选
  - 清屏（发送 `Ctrl+L`）
  - 查找（打开 Ctrl+F 查找栏）
  - 编码 ▸ 子菜单（UTF-8 / GBK / ISO-8859-1）
  - 重连
  - 断开
- **交互设计**：
  - 右键 → Teleport 弹出菜单
  - 编码子菜单 hover 展开
  - 点击外部/ESC 关闭
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add terminal context menu with encoding submenu`

### 5 终端主题预设

- **功能目标**：可切换的终端 ANSI 调色板预设
- **文件结构**：
  - `src/features/terminal/terminal-themes.ts`（新增：预设主题定义）
  - `src/features/terminal/TerminalView.vue`（修改：应用选中主题）
- **预设主题**：
  | 名称 | 背景 | 前景 | 风格 |
  |------|------|------|------|
  | Default | #0D1117 | #E6EDF3 | REX 深色（当前） |
  | Ubuntu | #300A24 | #FFFFFF | 经典紫色 |
  | Solarized Dark | #002B36 | #839496 | 经典护眼 |
- **交互设计**：
  - 主题通过 xterm.js 的 `theme` 选项应用
  - 默认使用 "Default"（与 REX 深色主题一致）
  - 设置页可切换（M7 设置模块预留接口，本里程碑通过代码切换）
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add terminal theme presets (Default/Ubuntu/Solarized)`

### 6 移动端浮动工具栏

- **功能目标**：移动端终端下方显示浮动工具栏，提供常用快捷操作
- **文件结构**：
  - `src/features/terminal/MobileTerminalBar.vue`（新增）
- **工具栏内容**：
  - 方向键区（↑↓←→ + Enter）：bash 历史/补全操作
  - 快捷键：Tab、Ctrl+C、Ctrl+L
  - 功能按钮：粘贴、字体放大/缩小
- **交互设计**：
  - 仅在 `max-width: 768px` 时显示
  - 固定在终端底部
  - 半透明背景，不遮挡终端内容
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add mobile floating toolbar for terminal`

### 7 测试与收尾

- **功能目标**：验证全部功能，修复问题
- **测试标准**：type-check + lint + build + cargo build + cargo clippy 全通过
- **提交**：`fix(web): terminal polish and fixes`

## 设计核对点
- [ ] 后端 WebSocket 端点可建立 SSH 连接并双向转发数据
- [ ] 前端 xterm.js 在 Pane 内正确渲染，自适应大小
- [ ] Ctrl+F 查找栏可打开、输入、高亮匹配、上下导航
- [ ] 右键菜单项完整（复制/粘贴/清屏/编码/重连/断开）
- [ ] 编码子菜单可切换 UTF-8/GBK/ISO-8859-1
- [ ] 3 个主题预设可切换
- [ ] 移动端工具栏在窄屏下显示
- [ ] 断开后可重连

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

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
