# M77: Bug Fix

## Context
M76 完成 Bug Fix。本里程碑继续修复已知 bug，涵盖 SSH 终端、设置 API、S3 文件管理、Agent 连接、工作区交互等多个模块。

版本类型：patch
版本号：0.65.4

## 产品边界
本阶段做什么：修复已知 bug
本阶段不做什么：不新增功能、不重设计页面（Agent 页面重设计规划在 M78）

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 1 | SSH 终端 bug 修复（字体、中文、复制粘贴、SFTP 按钮、sz 下载） | 前端 | ⬜ |
| 2 | 设置 API 类型修复（terminal_font_size） | 后端 | ⬜ |
| 3 | SSH 连接 IPv6 支持 | 后端 | ⬜ |
| 4 | 工作区分屏关闭按钮指向错误 | 前端 | ⬜ |
| 5 | S3 文件管理目录内容不显示 | 前端+后端 | ⬜ |
| 6 | Windows Agent 二进制下载失败 | 后端 | ⬜ |
| 7 | 环境页导航右键跳转显示错误环境 | 前端 | ⬜ |
| 8 | 资源删除操作添加确认弹窗 | 前端 | ⬜ |
| 9 | Agent SSH 测试连接走 Agent 隧道 | 后端 | ⬜ |

## 子任务详细设计

### 1 SSH 终端 bug 修复

- **功能目标**：修复 SSH 终端在不同设备下字体不一致、不支持中文、复制粘贴行为异常、缺少 SFTP 按钮、不支持 sz 下载
- **文件结构**：
  - `packages/rex-console-web/src/features/terminal/` — 终端相关组件
  - `packages/rex-console-web/src/stores/settings.ts` — 设置 store
- **修复项**：
  1. **字体不一致**：终端字体设置未正确应用到 xterm.js 实例，检查 `TerminalSettings` 是否正确传入 `fontFamily` 和 `fontSize`
  2. **中文支持**：xterm.js 需要加载 `@xterm/addon-unicode11` 或 `@xterm/addon-ligatures` 支持中文字符宽度计算，确保 `charset` 和 `locale` 正确
  3. **复制粘贴**：Ctrl+C 在终端中应为复制（选中文本时），Ctrl+V 为粘贴；右键菜单的复制/粘贴应调用 `navigator.clipboard`；工具栏复制/粘贴按钮应生效
  4. **SFTP 按钮**：SSH 终端工具栏应有 SFTP 按钮，点击打开 SFTP 抽屉（复用 SSH 通道）
  5. **sz 下载**：终端需支持 sz/rz 协议（通过 xterm.js 的 binary message 检测 sz 开头的 base64 数据，触发文件下载）
- **测试标准**：字体在不同设备一致、中文显示正常、Ctrl+C/V 正确复制粘贴、SFTP 按钮可见可用
- **提交信息**：`fix: SSH terminal font consistency, Chinese support, copy/paste, SFTP button, sz download`

### 2 设置 API 类型修复

- **功能目标**：修复 `/api/settings` 接口中 `terminal_font_size` 字段类型不匹配
- **文件结构**：`crates/rex-hub/src/api/settings.rs`（或对应路由文件）
- **问题**：前端发送 `"terminal_font_size": 30`（整数），后端期望 string
- **修复方案**：将 `terminal_font_size` 字段改为接受整数类型 `i32` 或 `u32`，而非 string
- **测试标准**：`/api/settings` 接口能正确接收和返回整数类型的 font_size
- **提交信息**：`fix: settings API terminal_font_size type integer instead of string`

### 3 SSH 连接 IPv6 支持

- **功能目标**：修复 SSH 连接 IPv6 地址失败
- **文件结构**：`crates/rex-ssh/src/` — SSH 连接逻辑
- **问题**：IPv6 地址格式（如 `[::1]`）在连接时可能未正确处理方括号
- **修复方案**：检查 SSH 连接地址解析逻辑，确保 IPv6 地址的方括号被正确处理（连接时去除方括号，DNS 解析时保留）
- **测试标准**：IPv6 地址的 SSH 连接能成功建立
- **提交信息**：`fix: SSH connection IPv6 address handling`

### 4 工作区分屏关闭按钮指向错误

- **功能目标**：修复左右/上下分屏时，点击左侧/上侧关闭按钮实际关闭的是右侧/下侧
- **文件结构**：`packages/rex-console-web/src/features/workspace/` — 分屏 Pane 组件
- **问题**：分屏 Pane 的关闭按钮事件绑定了错误的 Pane ID
- **修复方案**：检查 Pane 组件的 `close` 事件，确保关闭按钮绑定到当前 Pane 而非对面 Pane
- **测试标准**：左右分屏关闭左 pane 只关左，关闭右 pane 只关右；上下同理
- **提交信息**：`fix: split pane close button targets correct pane`

### 5 S3 文件管理目录内容不显示

- **功能目标**：修复 S3 资源打开后第一层可见但进入子目录后内容为空
- **文件结构**：
  - `packages/rex-console-web/src/features/files/` — S3 文件浏览组件
  - `crates/rex-s3/src/` — S3 后端逻辑
- **问题**：S3 ListObjectsV2 请求的 Prefix 参数可能未正确拼接（缺少尾部 `/`）
- **修复方案**：检查 S3 目录浏览的 Prefix 参数，确保进入子目录时正确添加 `/` 分隔符
- **测试标准**：S3 资源能正常浏览多级目录内容
- **提交信息**：`fix: S3 directory listing prefix handling for subdirectories`

### 6 Windows Agent 二进制下载失败

- **功能目标**：修复 Windows 平台 Agent 二进制下载失败
- **文件结构**：`crates/rex-hub/src/` — Agent 二进制下载相关代码
- **问题**：可能是下载 URL 构建错误、文件名后缀缺失 `.exe`、或 Content-Type 不正确
- **修复方案**：检查 Agent 二进制下载 API，确保 Windows 平台的下载 URL 和文件名正确包含 `.exe` 后缀
- **测试标准**：Windows 平台能正常下载 Agent 二进制文件
- **提交信息**：`fix: Windows agent binary download URL and filename`

### 7 环境页导航右键跳转显示错误环境

- **功能目标**：修复在侧栏右键环境 B 的数据跳转到环境页时，显示的是环境 A
- **文件结构**：`packages/rex-console-web/src/` — 环境页路由和侧栏右键菜单
- **问题**：右键菜单跳转时未正确传递环境 ID，或环境页读取了默认/缓存的环境 ID
- **修复方案**：检查侧栏右键菜单的路由跳转参数，确保传递正确的环境 ID；检查环境页组件的 `route.params.id` 读取逻辑
- **测试标准**：右键不同环境跳转后显示对应环境内容
- **提交信息**：`fix: environment page navigation from sidebar context menu`

### 8 资源删除操作添加确认弹窗

- **功能目标**：资源删除操作添加二次确认弹窗
- **文件结构**：`packages/rex-console-web/src/features/` — 资源删除相关组件
- **修复方案**：在资源删除按钮/右键菜单删除项的点击事件中，弹出确认对话框（Modal），用户确认后才执行删除
- **测试标准**：删除资源前弹出确认框，取消不删除，确认才删除
- **提交信息**：`fix: add confirmation dialog for resource deletion`

### 9 Agent SSH 测试连接走 Agent 隧道

- **功能目标**：修复通过 Agent 连接的 SSH 资源，测试连接时应走 Agent 隧道而非直连
- **文件结构**：`crates/rex-hub/src/api/` — 资源测试连接 API
- **问题**：测试连接 API 可能未检查资源的连接方式（Agent 代理 vs 直连），统一走了直连
- **修复方案**：测试连接 API 检查资源的 `connection_type`，如果为 Agent 代理则通过 Agent WebSocket 隧道发起测试连接
- **测试标准**：Agent 代理的 SSH 资源测试连接成功通过 Agent 隧道
- **提交信息**：`fix: test connection for agent-proxied resources through agent tunnel`

## 设计核对点

- 所有修复不改变产品语义
- SSH 终端行为对标 Xshell（复制粘贴、SFTP 抽屉）
- 文件管理对标 Xftp（S3 目录浏览）
- 不引入多用户/RBAC 概念

## Flow Status

- [x] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
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
