# M35: 工作区 Xshell 体验补全

## Context

M34 完成了文件编辑器、连接导入导出、SSH 保活。当前工作区和 SSH 终端缺少 PRODUCT.md 3.5/3.6 描述的几项 Xshell 级交互功能：快捷键面板（F1）、终端编码子菜单、状态栏增强。本里程碑补全这些细节，同时清理死代码（未使用的 QuickConnect.vue），提升整体操作体验。

版本类型：minor（新功能），版本号 0.33.0 → 0.34.0。

## 产品边界

**本阶段做：**
- 快捷键面板（F1 触发，分组展示所有快捷键）
- 终端编码子菜单（右键菜单 → 编码 ▸ UTF-8 / GBK / ISO-8859-1）
- 状态栏增强（编码显示+切换、广播状态指示）
- 删除死代码（未使用的 QuickConnect.vue 及相关引用）

**本阶段不做：**
- 终端内查找（Ctrl+F 非模态查找栏，需要 xterm.js addon，复杂度高，留待后续）
- 内置 SFTP 抽屉（终端内嵌文件面板，需要 SSH 通道复用，留待后续）
- 终端录制/回放

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 快捷键面板（F1） | ✅ |
| 2 | 终端编码子菜单（右键菜单 → 编码 ▸） | ✅ |
| 3 | 状态栏增强（编码 + 广播指示） | ✅ |
| 4 | 删除死代码 QuickConnect相关代码 | ✅ |

## 子任务详细设计

### 1 快捷键面板（F1）

**功能目标**

按 F1 弹出快捷键帮助面板，分组展示所有工作区/终端/SQL/文件管理快捷键。

**文件结构**

新建：
- `packages/rex-console-web/src/features/workspace/ShortcutsPanel.vue` — 快捷键面板组件

修改：
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — F1 按键绑定 + 面板集成

**交互设计**

- F1 触发 → 居中弹窗（70vw × 70vh），标题「Keyboard Shortcuts」
- 分组展示：Workspace / SSH Terminal / SQL Console / File Manager
- 每组：快捷键（kbd 样式） + 功能描述
- 点击遮罩 / Esc / 再按 F1 关闭
- 快捷键列表参考 PRODUCT.md §5

**快捷键数据**

```typescript
const shortcuts = [
  { group: 'Workspace', items: [
    { key: 'Ctrl+K', desc: 'Global search / Command palette' },
    { key: 'Ctrl+N', desc: 'New connection (Quick Connect)' },
    { key: 'Ctrl+T', desc: 'New tab (same protocol)' },
    { key: 'Ctrl+W', desc: 'Close current tab' },
    { key: 'Ctrl+Tab', desc: 'Switch next tab' },
    { key: 'Ctrl+Shift+Tab', desc: 'Switch previous tab' },
    { key: 'Alt+1~9', desc: 'Jump to tab N' },
    { key: 'Alt+1~5', desc: 'Layout: single / split-v / split-h / grid / main+side' },
    { key: 'F11', desc: 'Toggle fullscreen' },
    { key: 'F1', desc: 'This panel' },
  ]},
  { group: 'SSH Terminal', items: [
    { key: 'Ctrl+Shift+C', desc: 'Copy' },
    { key: 'Ctrl+Shift+V', desc: 'Paste (bracketed)' },
    { key: 'Ctrl+F', desc: 'Find in terminal' },
    { key: 'Ctrl+L', desc: 'Clear screen' },
  ]},
  { group: 'SQL Console', items: [
    { key: 'Ctrl+Enter', desc: 'Execute (Run)' },
    { key: 'Ctrl+Shift+F', desc: 'Format SQL' },
    { key: 'Ctrl+S', desc: 'Save' },
    { key: 'Ctrl+F', desc: 'Find' },
    { key: 'Ctrl+Shift+R', desc: 'Find & Replace' },
    { key: 'Ctrl+Shift+Q', desc: 'Global query' },
    { key: 'Ctrl+Shift+A', desc: 'AI assistant' },
  ]},
  { group: 'File Manager', items: [
    { key: 'F2', desc: 'Rename' },
    { key: 'F4', desc: 'Edit (temporary download)' },
    { key: 'F5', desc: 'Download (active → opposite)' },
    { key: 'F6', desc: 'Upload (active → opposite)' },
    { key: 'F7', desc: 'New folder' },
    { key: 'F8 / Delete', desc: 'Delete' },
    { key: 'Ctrl+R', desc: 'Refresh' },
    { key: 'Tab', desc: 'Switch active panel' },
  ]},
]
```

**测试标准**

- F1 打开面板，Esc 关闭
- 快捷键列表完整（4 组 30+ 项）
- 深色主题样式一致
- type-check + build 通过

**提交信息**: `feat(workspace): add keyboard shortcuts panel (F1)`

### 2 终端编码子菜单

**功能目标**

SSH 终端右键菜单新增「编码 ▸」子菜单，支持 UTF-8 / GBK / ISO-8859-1 切换。切换编码后终端重新解码输出。

**文件结构**

修改：
- `packages/rex-console-web/src/features/terminal/TerminalView.vue` — 右键菜单编码子菜单 + 编码切换逻辑
- `crates/rex-ssh/src/lib.rs` — 终端输出编码转换（后端不改，前端 xterm.js 层处理）

**交互设计**

- 右键菜单新增「编码 ▸」子菜单项
- 子菜单选项：UTF-8（默认 ✓）/ GBK / ISO-8859-1
- 点击切换 → 终端重新解码后续输出
- 当前编码显示在状态栏（子任务3）

**实现方案**

xterm.js 本身使用 UTF-8。GBK/ISO-8859-1 切换在前端处理：
1. 后端始终以 UTF-8 发送（SSH 默认 UTF-8）
2. 编码切换主要影响**粘贴内容的编码**（GBK 服务器可能需要 GBK 编码的输入）
3. 使用 `TextEncoder` / `TextDecoder` 进行编码转换
4. 编码设置保存到 localStorage，按资源 ID 隔离

**测试标准**

- 右键菜单显示「编码 ▸」子菜单
- 切换编码后状态栏更新
- 编码设置持久化
- type-check + build 通过

**提交信息**: `feat(terminal): add encoding submenu to context menu`

### 3 状态栏增强

**功能目标**

底部状态栏显示更多终端信息：当前编码、广播状态指示、连接延迟。

**文件结构**

修改：
- `packages/rex-console-web/src/features/workspace/StatusBar.vue` — 增加编码和广播指示

**交互设计**

状态栏布局（从左到右）：
```
[SSH] host:22  ● Connected  80×24  UTF-8  [BROADCAST]  12ms
```

- 编码显示：`UTF-8` / `GBK` / `ISO-8859-1`，点击弹出编码选择菜单
- 广播指示：当 Tab 广播模式开启时显示 `[BROADCAST]` 橙色标签
- 连接延迟：显示最后测量的 ping 延迟（ms）

**实现方案**

- 编码从 TerminalView 的编码状态读取
- 广播状态从 Tab 管理的 broadcast flag 读取
- 延迟从 WebSocket ping/pong 测量（如已有）或显示 `-`

**测试标准**

- 状态栏正确显示编码
- 广播开启时显示 [BROADCAST] 标签
- 编码点击可切换
- type-check + build 通过

**提交信息**: `feat(workspace): enhance status bar with encoding and broadcast indicators`

### 4 删除死代码 QuickConnect.vue

**功能目标**

`QuickConnect.vue` 从未被任何页面导入或渲染，是死代码。删除该文件及其所有引用，保持代码库整洁。

**文件结构**

删除：
- `packages/rex-console-web/src/features/workspace/QuickConnect.vue` — 未使用的组件

修改：
- `packages/rex-console-web/src/features/workspace/ShortcutPanel.vue` — 移除 Ctrl+N 对 Quick Connect 的引用（已完成）
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — 移除占位文字中对 Quick Connect 的引用（已完成）

**测试标准**

- `QuickConnect.vue` 文件已删除
- 代码中无 `QuickConnect` / `quick-connect` 引用
- type-check + build 通过

**提交信息**: `chore(web): remove unused QuickConnect component and references`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ PRODUCT.md 3.5 要求 F1 快捷键面板
- ✅ PRODUCT.md 3.6 要求编码子菜单
- ✅ 不引入多用户/RBAC 概念
- ✅ 不做终端内查找（留待后续）
- ✅ 不做内置 SFTP 抽屉（留待后续）

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
