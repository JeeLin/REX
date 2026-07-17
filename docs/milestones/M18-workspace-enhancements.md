# M18: 工作区核心增强

## Context

M0–M17 完成了基础设施和核心功能。工作区（WorkspacePage.vue）已实现 splitpanes 分屏、Tab 系统（关闭/设色/重命名）、连接树、协议路由。但产品文档描述的完整 Xshell 体验还缺少状态栏和部分 Tab 操作。

本里程碑版本类型：minor（新功能），版本号 0.18.0 → 0.19.0。

## 产品边界

**本阶段做：**
- 状态栏（协议+主机+端口、连接状态、终端尺寸 cols×rows）
- Tab 拖拽排序
- 关闭左侧 / 关闭全部
- 复制标签（在同一 Pane 内打开相同资源）

**本阶段不做：**
- Tab 拖拽到另一 Pane（复杂度高，后续）
- 广播模式（发送到全部 Tab）
- Pane 可分离到新窗口
- 移动到面板子菜单

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 状态栏 | ⬜ |
| 2 | Tab 拖拽排序 + 关闭操作补全 | ⬜ |

## 子任务详细设计

### 1 状态栏

**功能目标**

工作区底部显示当前连接的关键信息，紧凑一行，类似 Xshell 状态栏。

**显示内容**

| 区域 | 内容 | 示例 |
|------|------|------|
| 左侧 | 协议 + 主机 + 端口 | `SSH 192.168.1.100:22` |
| 中间 | 连接状态 | `🟢 Connected` / `🟠 Connecting` / `⚫ Disconnected` |
| 右侧 | 终端尺寸（仅 SSH） | `80×24` |
| 最右 | 时间 | `14:32:05` |

**文件结构**

修改：
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — 添加状态栏 HTML + CSS

**交互设计**

- 状态栏固定在工作区底部，高度 28px
- 信息跟随当前 activeTab 实时更新
- 终端尺寸通过 `TerminalView` 的 resize 事件更新
- 连接状态通过 `onTabStatusChange` 更新

**测试标准**

- 打开 SSH Tab → 状态栏显示 SSH + 主机 + 端口 + Connected + 终端尺寸
- 打开 MySQL Tab → 状态栏显示 MySQL + 主机 + 端口 + Connected（无终端尺寸）
- 切换 Tab → 状态栏内容切换
- 无 Tab 时 → 状态栏显示 "No connection"

**提交信息**

```
feat(workspace): add status bar with connection info and terminal size
```

### 2 Tab 拖拽排序 + 关闭操作补全

**功能目标**

- Tab 支持拖拽排序（在 Tab 栏内拖动调整顺序）
- 右键菜单补全：关闭左侧、关闭全部、复制标签

**文件结构**

修改：
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — 添加拖拽逻辑和新菜单项

**交互设计**

Tab 拖拽：
- 鼠标按住 Tab → 拖动 → 其他 Tab 自动让位
- 松开 → Tab 固定到新位置
- 使用 HTML5 drag-and-drop API（不引入新依赖）

右键菜单新增：
- `Close Left` — 关闭当前 Tab 左侧所有 Tab
- `Close All` — 关闭所有 Tab
- `Duplicate` — 在当前 Pane 复制打开相同资源

**测试标准**

- 拖拽 Tab → 顺序改变 → 刷新后顺序保持（如果后续添加持久化）
- 关闭左侧 → 当前 Tab 左侧的 Tab 全部关闭
- 关闭全部 → 所有 Tab 关闭
- 复制标签 → 新 Tab 打开相同资源
- type-check + build 通过

**提交信息**

```
feat(workspace): add tab drag reorder and close left/all/duplicate
```

## 设计核对点

- [ ] 状态栏显示协议+主机+端口
- [ ] 状态栏显示连接状态
- [ ] 状态栏显示终端尺寸（SSH）
- [ ] Tab 可拖拽排序
- [ ] 关闭左侧 / 关闭全部 / 复制标签可用
- [ ] type-check + build 通过

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
