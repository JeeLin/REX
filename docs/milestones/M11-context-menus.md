# M11: 右键上下文菜单系统

## Context

M0-M10 完成了全部核心功能开发。产品文档在多个页面定义了右键上下文菜单（§3.2 仪表盘、§3.3 环境、§3.9 Agent、§3.10 审计日志、§4 侧边栏），但前端至今未实现。M11 为这些管理页面添加右键菜单。

## 产品边界

**做什么：**
- 创建可复用的 ContextMenu 组件
- 仪表盘右键菜单（快速连接项、环境卡片、统计卡片）
- 环境列表/详情右键菜单（环境卡片、资源行）
- Agent 管理页右键菜单（Agent 卡片）
- 侧边栏右键菜单（资源项、环境节点）
- 审计日志右键菜单（日志行、操作类型标签、环境名称）

**不做什么：**
- 终端右键菜单（§3.6，全屏页面，后续阶段）
- SQL 编辑器/结果/库表结构右键菜单（§3.7，后续阶段）
- 文件管理右键菜单（§3.8，后续阶段）
- 工作空间标签右键菜单（§3.5，后续阶段）
- 工作空间连接菜单（§3.5，后续阶段）
- 跨连接文件传输（§3.8，后续阶段）

---

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 11.1 | ContextMenu 可复用组件 | 前端 | ✅ |
| 11.2 | 仪表盘右键菜单 | 前端 | ✅ |
| 11.3 | 环境/Agent/侧边栏/审计日志右键菜单 | 前端 | ✅ |

---

## 子任务 11.1：ContextMenu 可复用组件

### 功能目标

创建通用的右键上下文菜单组件，支持：
- 右键点击触发，在鼠标位置显示菜单
- 菜单项支持：文字、图标、点击回调、禁用态
- 分隔线
- 子菜单（可选）
- 点击菜单外区域自动关闭
- ESC 键关闭
- 移动端长按触发（可选，后续）
- 支持动态菜单项（根据右键目标生成不同菜单）

### 修改文件

```text
packages/rex-console-web/src/
├── components/ContextMenu.vue     新增：可复用上下文菜单组件
├── composables/useContextMenu.ts  新增：右键菜单状态管理 composable
```

### 接口设计

**ContextMenu.vue props：**

```ts
interface MenuItem {
  label: string
  icon?: string
  action?: () => void
  disabled?: boolean
  danger?: boolean        // 红色文字（删除等破坏性操作）
  separator?: boolean     // 分隔线
  children?: MenuItem[]   // 子菜单（可选）
}

// Props
{
  visible: boolean
  x: number
  y: number
  items: MenuItem[]
}
```

**useContextMenu composable：**

```ts
function useContextMenu() {
  return {
    visible: Ref<boolean>,
    x: Ref<number>,
    y: Ref<number>,
    items: Ref<MenuItem[]>,
    show: (event: MouseEvent, items: MenuItem[]) => void,
    hide: () => void,
  }
}
```

### 交互设计

参考原型 `prototype/shared.js` 中的上下文菜单 CSS 和交互：

1. 右键点击 → 在鼠标位置显示菜单（避免超出视口边界）
2. 点击菜单项 → 执行 action + 关闭菜单
3. 点击菜单外区域 → 关闭菜单
4. ESC → 关闭菜单
5. 菜单层级：主菜单 → 子菜单（右箭头展开）

### 样式

参考原型 `prototype/css/base.css` 中的 `.ctx-menu` 样式：
- 背景 `var(--bg-elevated)`
- 边框 `1px solid var(--border)`
- 圆角 `var(--radius-lg)`
- 阴影 `var(--shadow-lg)`
- 菜单项 hover 背景 `var(--bg-hover)`
- danger 菜单项文字 `var(--danger)`
- 分隔线 `var(--border)`

### 测试标准

- 右键点击显示菜单
- 菜单位置不超出视口
- 点击菜单项执行回调并关闭
- 点击外部关闭
- ESC 关闭
- 分隔线正确显示
- danger 样式正确

### 提交信息

```
feat: add reusable context menu component
```

---

## 子任务 11.2：仪表盘右键菜单

### 功能目标

为仪表盘页面添加右键菜单，匹配产品文档 §3.2。

### 修改文件

```text
packages/rex-console-web/src/
├── pages/Dashboard.vue             修改：添加右键菜单
├── i18n/zh.ts, en.ts               修改：添加右键菜单 i18n keys
```

### 交互设计

参考原型 `prototype/dashboard.html`：

**快速连接项右键菜单：**
- 连接
- 在新标签中连接
- 复制地址
- 添加到收藏
- 从最近使用中移除

**环境卡片右键菜单：**
- 打开环境详情
- 在此环境中新建资源
- 查看 Agent
- ─── 分隔线 ───
- 编辑环境
- 删除环境（红色）

**统计卡片右键菜单：**
- 刷新统计

### 测试标准

- 右键快速连接项显示对应菜单
- 右键环境卡片显示对应菜单
- 右键统计卡片显示对应菜单
- 各菜单项功能正确（导航、复制、删除确认等）

### 提交信息

```
feat: add context menus to dashboard page
```

---

## 子任务 11.3：环境/Agent/侧边栏/审计日志右键菜单

### 功能目标

为环境列表、Agent 管理、侧边栏、审计日志页面添加右键菜单。

### 修改文件

```text
packages/rex-console-web/src/
├── pages/Environments.vue          修改：环境卡片右键菜单
├── pages/EnvironmentDetail.vue     修改：资源行右键菜单
├── pages/Agents.vue                修改：Agent 卡片右键菜单
├── layouts/AppLayout.vue           修改：侧边栏右键菜单
├── pages/AuditLog.vue              修改：日志行、操作类型标签右键菜单
├── i18n/zh.ts, en.ts               修改：添加右键菜单 i18n keys
```

### 交互设计

参考原型：

**环境列表 — 环境卡片右键菜单（§3.3）：**
- 打开详情
- 在工作区打开
- ─── 分隔线 ───
- 新建资源
- 添加 Agent
- ─── 分隔线 ───
- 编辑环境
- 删除环境（红色）

**环境详情 — 资源行右键菜单（§3.3）：**
- 连接
- 在新标签中连接
- 编辑资源
- 删除资源（红色）
- 复制地址

**Agent 卡片右键菜单（§3.9）：**
- 查看日志
- 配置
- ─── 分隔线 ───
- 复制 Agent ID
- 复制注册令牌
- ─── 分隔线 ───
- 重启 Agent（红色）
- 重置令牌（红色）

**侧边栏 — 资源项右键菜单（§4）：**
- 连接
- 在新标签中连接
- 编辑资源
- 删除资源（红色）
- 复制地址

**侧边栏 — 环境节点右键菜单（§4）：**
- 打开环境详情
- 折叠/展开
- 在工作区打开所有资源
- ─── 分隔线 ───
- 新建资源
- 编辑环境
- 删除环境（红色）

**审计日志 — 日志行右键菜单（§3.10）：**
- 查看详情（展开行）
- 复制摘要
- 复制操作类型
- 复制时间戳

**审计日志 — 操作类型标签右键菜单：**
- 按此操作类型筛选
- 查看同类操作

**审计日志 — 环境名称右键菜单：**
- 按此环境筛选

**审计日志 — 空白区域右键菜单：**
- 刷新日志
- 导出全部为 CSV
- 清除筛选条件

### 测试标准

- 各页面右键菜单正确显示
- 菜单项功能正确（导航、复制、筛选、删除确认等）
- 菜单不超出视口
- 移动端不受影响

### 提交信息

```
feat: add context menus to environments, agents, sidebar, and audit log
```

---

## 设计核对点

- [ ] ContextMenu 组件支持所有菜单项类型（文字、分隔线、danger、禁用）
- [ ] 菜单位置不超出视口
- [ ] 各页面右键菜单与产品文档一致
- [ ] i18n 覆盖所有菜单项文字
- [ ] 移动端不受右键菜单影响

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交
