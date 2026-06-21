# M20 步骤7：设计再确认报告

## 确认维度

对照 M20 里程碑文档和 PRODUCT.md §3.5/§3.6，逐项验证实现。

### 子任务 20.1：面板拖拽交互 ✅

| 检查项 | 结果 |
|--------|------|
| 拖拽标签到空面板填充 | ✅ `onPanelDrop` 调用 `moveTabToPanel` |
| 拖拽标签到已占用面板交换 | ✅ `onPanelDrop` 调用 `swapPanels` |
| 源标签半透明 | ✅ CSS `.ws-tab.dragging { opacity: 0.5 }` |
| 目标面板橙色虚线高亮 | ✅ CSS `.layout-drop-zone` |
| 单面板模式不支持拖拽 | ✅ `onPanelDragOver` 和 `onPanelDrop` 检查 `currentLayout === 'single'` |
| panelIndex 正确更新 | ✅ `swapPanels` 和 `moveTabToPanel` 直接修改 `tab.panelIndex` |

### 子任务 20.2：双击标签分屏 ✅

| 检查项 | 结果 |
|--------|------|
| 单面板双击切换左右分屏 | ✅ `handleTabDblclick` 设置 `currentLayout = 'left-right'` |
| 当前标签分配到面板 0 | ✅ `moveTabToPanel(tabId, 0)` |
| 其他标签分配到面板 1 | ✅ 查找 candidate 或 fallback |
| 已分屏模式双击无效果 | ✅ `if (currentLayout !== 'single') return` |

### 子任务 20.3：终端工具栏右键菜单 ✅

| 检查项 | 结果 |
|--------|------|
| 复制延迟信息 | ✅ clipboard 写入 |
| 打开连接详情 | ✅ 跳转 environments 页（路由待完善） |
| 切换全屏 | ✅ requestFullscreen / exitFullscreen |
| i18n 覆盖 | ✅ 3 个菜单项均有 zh/en 键 |

### 架构一致性

| 检查项 | 结果 |
|--------|------|
| 单用户模型 | ✅ 无多用户/RBAC 概念 |
| 文件不经过浏览器 | ✅ 拖拽仅操作 tab 元数据，不涉及文件传输 |
| 右键菜单使用 useContextMenu | ✅ 工具栏菜单复用现有 composable |
| HTML5 Drag and Drop API | ✅ 使用原生 dragstart/dragover/drop 事件 |
| i18n 覆盖新增文字 | ✅ ws.panel.*, ws.terminal.toolbar.ctx.* |

---

## 结论

✅ 实现与 M20 里程碑文档一致，产品语义正确。
