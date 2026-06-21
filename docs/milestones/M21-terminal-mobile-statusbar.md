# M21: Terminal 移动端浮动工具栏 + 状态栏增强 + 工具栏 i18n

## Context

M20 完成了工作空间面板拖拽、双击分屏、终端工具栏右键菜单。SSH 终端的核心功能（xterm.js、WebSocket 连接、SFTP 面板、右键菜单）已完整，但状态栏信息不足，移动端缺少虚拟键盘操作支持，工具栏按钮文案未国际化。

## 产品边界

**做什么：**
- 状态栏增强：显示终端尺寸（cols×rows）、连接方式（直连/Agent 代理）、操作提示
- 移动端浮动工具栏：方向键、Tab、Enter、^C、^L、历史记录、粘贴、字体缩放
- 工具栏按钮 i18n：将硬编码中文（清屏、粘贴、断开）替换为 i18n 键
- 移动端检测：根据视口宽度自动显示/隐藏浮动工具栏

**不做什么：**
- 不实现终端分屏（已通过工作空间面板实现）
- 不实现终端录制/回放
- 不实现终端搜索（xterm.js addon，可后续添加）
- 不实现 SSH 隧道/端口转发

---

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 21.1 | 状态栏增强 | ✅ |
| 21.2 | 移动端浮动工具栏 | ✅ |
| 21.3 | 工具栏按钮 i18n | ✅ |

---

## 子任务详细设计

### 21.1 状态栏增强

**功能目标：**
丰富终端底部状态栏信息，显示终端尺寸、连接方式和操作提示。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/workspace/panels/
│   └── WorkspaceTerminal.vue          修改：状态栏模板和样式
├── i18n/zh.ts, en.ts                 修改：添加状态栏 i18n 键
```

**交互设计（参考 PRODUCT.md §3.6）：**

```text
状态栏布局（底部橙色条）：

左侧：SSH · UTF-8 · 180×42
右侧：直连 · Ctrl+Shift+C 复制

- 终端尺寸随 xterm resize 事件实时更新
- 连接方式需要从资源信息中获取（props 或 API）
- 操作提示固定显示
```

**WorkspaceTerminal.vue 修改：**
- 新增 `termSize` ref（cols, rows），监听 `terminal.onResize` 更新
- 状态栏左侧显示：协议 · 编码 · 终端尺寸
- 状态栏右侧显示：连接方式 + 操作提示
- 连接方式通过新 prop `connectionMode` 传入（'direct' | 'agent'）
- 从 Workspace.vue 传递 connectionMode（根据资源的 agent_id 判断）

**Workspace.vue 修改：**
- 传递 `connectionMode` prop 给 WorkspaceTerminal

**i18n 键：**
| 键 | 中文 | English |
|---|------|---------|
| `ws.terminal.statusbar.protocol` | SSH | SSH |
| `ws.terminal.statusbar.encoding` | UTF-8 | UTF-8 |
| `ws.terminal.statusbar.size` | {cols}×{rows} | {cols}×{rows} |
| `ws.terminal.statusbar.direct` | 直连 | Direct |
| `ws.terminal.statusbar.agent` | 通过 Agent 代理 | Via Agent |
| `ws.terminal.statusbar.hint` | Ctrl+Shift+C 复制 | Ctrl+Shift+C to copy |

**测试标准：**
- 状态栏左侧显示协议、编码、终端尺寸
- 终端 resize 后尺寸数字实时更新
- 状态栏右侧显示连接方式（直连/Agent）
- 中英文切换后状态栏文案正确

**提交信息：**
```
feat: enhance terminal status bar with size and connection mode
```

---

### 21.2 移动端浮动工具栏

**功能目标：**
为移动端用户提供虚拟键盘操作支持，解决手机无法输入方向键、Tab、Ctrl+C 等终端常用按键的问题。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/workspace/panels/
│   └── WorkspaceTerminal.vue          修改：添加浮动工具栏模板、样式、逻辑
├── i18n/zh.ts, en.ts                 修改：添加浮动工具栏 i18n 键
```

**交互设计（参考 PRODUCT.md §3.6）：**

```text
移动端浮动工具栏：固定在终端区域底部，仅移动端显示（max-width: 768px）

第一行 — 方向键区：
┌────┬────┬────┬────┐
│  ↑ │  ← │  ↓ │  → │
├────┼────┼────┼────┤
│ Tab│Enter│ ^C │ ^L │
└────┴────┴────┴────┘

第二行 — 功能按钮：
┌──────┬──────┬─────┬─────┬──────┐
│ 📜历史 │ 📋粘贴 │ A-  │ A+  │ ⚙更多 │
└──────┴──────┴─────┴─────┴──────┘

按钮功能：
- ↑↓←→：发送 ANSI 转义序列（\x1b[A 等）
- Tab：发送 \t
- Enter：发送 \r
- ^C：发送 \x03
- ^L：发送 \x0c
- 📜历史：打开 bash 历史记录面板（从终端获取最近命令）
- 📋粘贴：打开文本输入弹窗，用户输入后发送到终端
- A-/A+：调整终端字体大小（fontSize ± 1，范围 10-20）
- ⚙更多：展开更多选项（清屏、重连等）
```

**浮动工具栏结构：**

```html
<div class="ws-term-mobile-bar">
  <!-- 第一行：方向键 -->
  <div class="mobile-row">
    <button @click="sendKey('\x1b[A')">↑</button>
    <button @click="sendKey('\x1b[D')">←</button>
    <button @click="sendKey('\x1b[B')">↓</button>
    <button @click="sendKey('\x1b[C')">→</button>
    <button @click="sendKey('\t')">Tab</button>
    <button @click="sendKey('\r')">⏎</button>
    <button @click="sendKey('\x03')">^C</button>
    <button @click="sendKey('\x0c')">^L</button>
  </div>
  <!-- 第二行：功能按钮 -->
  <div class="mobile-row">
    <button @click="showHistoryPanel = true">📜 {{ t('ws.terminal.mobile.history') }}</button>
    <button @click="showPasteDialog = true">📋 {{ t('ws.terminal.mobile.paste') }}</button>
    <button @click="adjustFontSize(-1)">A-</button>
    <button @click="adjustFontSize(1)">A+</button>
    <button @click="showMobileMore = true">⚙</button>
  </div>
</div>
```

**新增功能逻辑：**

`sendKey(seq: string)` — 向 WebSocket 发送按键序列
```ts
function sendKey(seq: string) {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({
      type: 'terminal.input',
      payload: { data: btoa(seq) },
    }))
  }
}
```

`adjustFontSize(delta: number)` — 调整终端字体大小
```ts
function adjustFontSize(delta: number) {
  const newSize = Math.max(10, Math.min(20, terminalFontSize.value + delta))
  terminalFontSize.value = newSize
  terminal?.options.fontSize = newSize
  fitAddon?.fit()
}
```

`showPasteDialog` — 粘贴文本弹窗（移动端无法直接访问剪贴板）
```html
<div v-if="showPasteDialog" class="ws-term-modal-overlay" @click.self="showPasteDialog = false">
  <div class="ws-term-modal">
    <div class="ws-term-modal-title">粘贴文本</div>
    <textarea ref="pasteInput" v-model="pasteText" rows="4" placeholder="在此输入要粘贴的文本..."></textarea>
    <div class="ws-term-modal-actions">
      <button class="btn" @click="showPasteDialog = false">取消</button>
      <button class="btn btn-primary" @click="doPasteText">粘贴</button>
    </div>
  </div>
</div>
```

`showMobileMore` — 更多功能弹窗（清屏、重连、断开等）

**移动端检测：**
```ts
const isMobile = ref(false)
onMounted(() => {
  const mq = window.matchMedia('(max-width: 768px)')
  isMobile.value = mq.matches
  mq.addEventListener('change', (e) => { isMobile.value = e.matches })
})
```

CSS 媒体查询：
```css
.ws-term-mobile-bar { display: none; }
@media (max-width: 768px) {
  .ws-term-mobile-bar { display: flex; flex-direction: column; gap: 4px; padding: 6px; }
}
```

**i18n 键：**
| 键 | 中文 | English |
|---|------|---------|
| `ws.terminal.mobile.history` | 历史 | History |
| `ws.terminal.mobile.paste` | 粘贴 | Paste |
| `ws.terminal.mobile.more` | 更多 | More |
| `ws.terminal.mobile.pasteTitle` | 粘贴文本 | Paste Text |
| `ws.terminal.mobile.pastePlaceholder` | 在此输入要粘贴的文本... | Type or paste text here... |
| `ws.terminal.mobile.clear` | 清屏 | Clear |
| `ws.terminal.mobile.reconnect` | 重连 | Reconnect |
| `ws.terminal.mobile.disconnect` | 断开连接 | Disconnect |

**测试标准：**
- 桌面端（>768px）不显示浮动工具栏
- 移动端（≤768px）显示浮动工具栏
- 方向键发送正确的 ANSI 转义序列
- Tab、Enter、^C、^L 发送正确字符
- 粘贴弹窗可输入文本并发送到终端
- 字体缩放按钮调整终端字号（10-20px 范围）
- 更多菜单包含清屏、重连、断开

**提交信息：**
```
feat: add mobile floating toolbar for terminal
```

---

### 21.3 工具栏按钮 i18n

**功能目标：**
将终端工具栏中硬编码的中文按钮文案替换为 i18n 键，支持中英文切换。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/workspace/panels/
│   └── WorkspaceTerminal.vue          修改：工具栏按钮文案
├── i18n/zh.ts, en.ts                 修改：添加工具栏按钮 i18n 键
```

**当前硬编码文案 → i18n 键：**

| 当前文案 | i18n 键 | 中文 | English |
|----------|---------|------|---------|
| 清屏 | `ws.terminal.toolbar.clear` | 清屏 | Clear |
| 粘贴 | `ws.terminal.toolbar.paste` | 粘贴 | Paste |
| SFTP | `ws.terminal.toolbar.sftp` | SFTP | SFTP |
| 断开 | `ws.terminal.toolbar.disconnect` | 断开 | Disconnect |
| 连接已断开 | `ws.terminal.reconnect.title` | 连接已断开 | Disconnected |
| 重新连接 | `ws.terminal.reconnect.btn` | 重新连接 | Reconnect |
| 断开连接？ | `ws.terminal.disconnect.title` | 断开连接？ | Disconnect? |
| 断开后... | `ws.terminal.disconnect.desc` | 断开后当前会话将终止，未保存的工作可能会丢失。 | The current session will be terminated. Unsaved work may be lost. |
| 取消 | `common.cancel` | 取消 | Cancel |
| 确认 | `common.confirm` | 确认 | Confirm |

**WorkspaceTerminal.vue 修改：**
- 工具栏按钮：`清屏` → `t('ws.terminal.toolbar.clear')`
- 工具栏按钮：`粘贴` → `t('ws.terminal.toolbar.paste')`
- 工具栏按钮：`断开` → `t('ws.terminal.toolbar.disconnect')`
- 重连提示：`连接已断开` → `t('ws.terminal.reconnect.title')`
- 重连按钮：`重新连接` → `t('ws.terminal.reconnect.btn')`
- 断开弹窗标题/描述/按钮

**测试标准：**
- 工具栏所有按钮文案通过 i18n 渲染
- 切换到英文后按钮文案变为英文
- 断开确认弹窗文案正确国际化
- 重连提示文案正确国际化

**提交信息：**
```
feat: add i18n for terminal toolbar buttons
```

---

## 设计核对点

- [x] 状态栏与 PRODUCT.md §3.6 一致
- [x] 移动端浮动工具栏与 PRODUCT.md §3.6 一致
- [x] 工具栏按钮 i18n 覆盖所有可见文案
- [x] 移动端检测使用 matchMedia，不依赖 userAgent
- [x] 字体缩放范围 10-20px，不超出合理边界
- [x] 浮动工具栏仅移动端显示，不影响桌面端布局

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交

## 设计核对点

- [x] 状态栏与 PRODUCT.md §3.6 一致
- [x] 移动端浮动工具栏与 PRODUCT.md §3.6 一致
- [x] 工具栏按钮 i18n 覆盖所有可见文案
- [x] 移动端检测使用 matchMedia，不依赖 userAgent
- [x] 字体缩放范围 10-20px，不超出合理边界
- [x] 浮动工具栏仅移动端显示，不影响桌面端布局

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
