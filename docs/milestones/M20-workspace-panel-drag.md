# M20: 工作空间面板拖拽 + 双击分屏 + 终端工具栏菜单

## Context

M19 完成了 SQL 库表结构树右键菜单和查询标签右键菜单。工作空间（§3.5）的标签拖拽排序、标签右键菜单、布局切换已实现，但面板间拖拽和双击分屏尚未实现。终端工具栏右键菜单在 M18 中标注为后续实现。

## 产品边界

**做什么：**
- 面板拖拽：将标签拖拽到不同面板位置，支持拖入空面板填充和拖入已占用面板交换
- 双击标签分屏：双击标签快速进入左右分屏模式
- 终端工具栏右键菜单：清屏、粘贴、SFTP、全屏、断开连接

**不做什么：**
- 不实现拖出分离到新窗口（浏览器限制，可选功能）
- 不实现跨连接文件传输「发送到…」（需要后端 TransferCoordinator）
- 不实现全局查询 modal（需要多数据库执行能力）
- 不实现 AI 助手面板（需要 LLM 集成）

---

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 20.1 | 面板拖拽交互 | ⬜ |
| 20.2 | 双击标签分屏 | ⬜ |
| 20.3 | 终端工具栏右键菜单 | ⬜ |

---

## 子任务详细设计

### 20.1 面板拖拽交互

**功能目标：**
支持将标签拖拽到不同的分屏面板位置，实现面板间内容交换和填充。

**修改文件：**
```text
packages/rex-console-web/src/
├── pages/Workspace.vue                修改：面板 drop zone + 拖拽逻辑
├── features/workspace/
│   ├── TabBar.vue                     修改：拖拽事件传递 panelIndex
│   └── useTabs.ts                     修改：增加面板交换逻辑
├── i18n/zh.ts, en.ts                 修改：添加面板拖拽 i18n
```

**交互设计（参考 PRODUCT.md §3.5）：**

```text
拖拽标签到面板：
1. 用户按住标签开始拖拽（已有 dragstart）
2. 目标面板显示虚线边框高亮（layout-drop-zone）
3. 松开后标签内容填充到该面板

面板间交换：
- 将标签拖到已占用的面板上，交换两个面板的内容

视觉反馈：
- 拖拽过程中，源标签降低透明度（opacity: 0.5）
- 目标面板显示橙色虚线边框 + 半透明背景提示

规则：
- 单面板模式下不支持面板拖拽
- 分屏模式下，拖入空面板自动填充，拖入已占用面板则交换内容
```

**Workspace.vue 修改：**
- 每个 `.ws-panel` 添加 `@dragover.prevent` 和 `@drop.prevent` 事件
- 拖拽悬停时添加 `layout-drop-zone` class 显示高亮
- 松开时调用 `moveTabToPanel(tabId, panelIndex)` 或交换逻辑
- 需要从 TabBar 的 dragstart 事件传递 tabId 到 Workspace

**useTabs.ts 修改：**
- 增加 `swapPanels(tabId1, tabId2)` 方法：交换两个标签的 panelIndex

**测试标准：**
- 分屏模式下拖拽标签到空面板，面板显示标签内容
- 分屏模式下拖拽标签到已占用面板，两个面板内容交换
- 拖拽过程中源标签半透明，目标面板高亮
- 单面板模式下拖拽无效果
- 拖拽完成后标签的 panelIndex 正确更新

**提交信息：**
```
feat: add panel drag-and-drop for workspace tabs
```

---

### 20.2 双击标签分屏

**功能目标：**
双击标签快速进入左右分屏模式，将当前标签分配到左侧面板。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/workspace/
│   └── TabBar.vue                     修改：双击事件处理
├── pages/Workspace.vue                修改：处理双击分屏事件
```

**交互设计（参考 PRODUCT.md §3.5）：**

```text
双击标签：
1. 如果当前是单面板模式，切换到左右分屏（layout-left-right）
2. 当前标签分配到面板 0（左侧）
3. 如果已有其他标签，自动分配到面板 1（右侧）
4. 如果是已分屏模式，双击无效果（或聚焦到该面板）
```

**TabBar.vue 修改：**
- `onTabDblclick` 调用 `emit('dblclick', tab.id)` 传递 tabId

**Workspace.vue 修改：**
- 监听 TabBar 的 `dblclick` 事件
- 切换到 `left-right` 布局
- 设置当前标签的 `panelIndex = 0`
- 查找下一个未分配的标签分配到 `panelIndex = 1`

**测试标准：**
- 单面板模式双击标签，切换到左右分屏
- 当前标签显示在左侧面板
- 已分屏模式双击标签无效果
- 布局指示器正确更新

**提交信息：**
```
feat: double-click tab to enter split view
```

---

### 20.3 终端工具栏右键菜单

**功能目标：**
为 SSH 终端工具栏添加右键上下文菜单，提供常用操作快捷入口。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/workspace/panels/
│   └── WorkspaceTerminal.vue          修改：工具栏右键菜单
├── i18n/zh.ts, en.ts                 修改：添加工具栏菜单 i18n
```

**交互设计（参考 PRODUCT.md §3.6）：**

终端工具栏右键菜单（右键工具栏区域）：
```text
复制延迟信息
打开连接详情
─── 分隔线 ───
⛶ 切换全屏
```

**WorkspaceTerminal.vue 修改：**
- 工具栏容器添加 `@contextmenu.prevent` 事件处理
- 右键时调用 `useContextMenu().show()` 展示菜单
- 菜单项 action：
  - 复制延迟信息：复制连接延迟/状态信息到剪贴板
  - 打开连接详情：跳转到资源详情页
  - 切换全屏：调用 `document.documentElement.requestFullscreen()`

**测试标准：**
- 工具栏右键弹出菜单
- 菜单项功能：复制延迟信息（clipboard）、打开连接详情（路由跳转）、切换全屏
- 菜单点击后自动关闭

**提交信息：**
```
feat: add terminal toolbar context menu
```

---

### i18n 键清单

**面板拖拽（`ws.panel.*`）：**
| 键 | 中文 | English |
|---|------|---------|
| `ws.panel.dropHere` | 放置到此面板 | Drop here |
| `ws.panel.swap` | 交换面板内容 | Swap panels |

**终端工具栏（`ws.terminal.toolbar.ctx.*`）：**
| 键 | 中文 | English |
|---|------|---------|
| `ws.terminal.toolbar.ctx.copyLatency` | 复制延迟信息 | Copy Latency Info |
| `ws.terminal.toolbar.ctx.openConnectionDetail` | 打开连接详情 | Open Connection Detail |
| `ws.terminal.toolbar.ctx.toggleFullscreen` | 切换全屏 | Toggle Fullscreen |

---

## 设计核对点

- [ ] 面板拖拽与 PRODUCT.md §3.5 一致
- [ ] 双击分屏与 PRODUCT.md §3.5 一致
- [ ] 终端工具栏右键菜单与 PRODUCT.md §3.6 一致
- [ ] i18n 覆盖所有新增文字
- [ ] 拖拽操作使用 HTML5 Drag and Drop API
- [ ] 右键菜单使用现有 useContextMenu composable

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
