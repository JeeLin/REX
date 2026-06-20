# M14: 工作空间面板集成

## Context

M13 完成了工作空间外壳：标签栏、连接菜单、分屏布局、快捷键和侧边栏集成。但面板内容仍是占位符（仅显示协议图标 + 资源名 + 状态文字）。

M14 将 Terminal、SQL、Files 的实际功能嵌入工作空间面板，使工作空间成为真正的操作界面。独立路由页面（/terminal、/sql、/files）保留为向后兼容，但优先使用工作空间。

## 产品边界

**做什么：**
- SSH 面板组件：在工作空间面板内渲染 xterm.js 终端（工具栏 + 终端 + 状态栏）
- SQL 面板组件：在工作空间面板内渲染 SQL 控制台（标签 + 编辑器 + 结果网格）
- Files 面板组件：在工作空间面板内渲染文件管理器（面包屑 + 文件列表 + 工具栏）
- 未知协议面板的 fallback 状态展示
- 连接状态管理（active tab 变化时更新面板状态）
- 面板内断开连接处理

**不做什么：**
- 重构现有 Terminal.vue / SqlConsole.vue / Files.vue（保持独立页面可用）
- WebSocket 终端通道的实际连接（后端 API 层面，不在本里程碑范围）
- 面板间拖拽标签交换（M13 后续增强）
- SSH 内置 SFTP split view（后续增强）

---

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 14.1 | SSH 面板组件 | 前端 | ✅ |
| 14.2 | SQL 面板组件 | 前端 | ✅ |
| 14.3 | Files 面板组件 | 前端 | ✅ |
| 14.4 | 面板路由渲染与连接状态 | 前端 | ✅ |

---

## 子任务 14.1：SSH 面板组件

### 功能目标

创建 `WorkspaceTerminal.vue`，在工作空间面板内嵌入 SSH 终端。复用现有 xterm.js 逻辑，但作为可嵌入组件而非独立页面。

### 修改文件

```text
packages/rex-console-web/src/
├── features/workspace/panels/
│   └── WorkspaceTerminal.vue    新增：SSH 面板组件
├── pages/Workspace.vue          修改：面板区域引入 WorkspaceTerminal
```

### 交互设计

参考原型 `prototype/terminal.html` 和现有 `Terminal.vue`：

**SSH 面板结构：**

```text
WorkspaceTerminal
├── 工具栏（toolbar）
│   ├── 左侧：● 连接状态 + 资源名称
│   └── 右侧：清屏 · 粘贴 · 断开
├── 终端容器（xterm.js 挂载点）
└── 状态栏（statusbar）
    ├── SSH · UTF-8
    └── 已连接 / 连接中 / 未连接
```

**与独立 Terminal.vue 的区别：**
- 去掉顶部「← 返回」按钮（工作空间通过标签栏导航）
- 工具栏紧凑模式（减小 padding）
- 断开连接时在面板内显示重连提示，而非跳转离开

### 接口设计

```ts
// WorkspaceTerminal.vue props
interface Props {
  resourceId: string
  resourceName: string
}

// 组件事件
interface Emits {
  (e: 'disconnect'): void    // 断开连接
  (e: 'error', msg: string): void  // 连接错误
}
```

### 实现要点

1. 从 `Terminal.vue` 提取核心逻辑（xterm 初始化、FitAddon、WebSocket 连接）
2. 移除路由依赖（不使用 `useRoute`/`useRouter`）
3. 工具栏简化为紧凑模式
4. 断开连接时显示面板内重连 UI，不跳转路由
5. 组件挂载时自动初始化终端，卸载时清理

### 测试标准

- 在工作空间面板中正确渲染 xterm.js 终端
- 工具栏按钮功能正常（清屏、粘贴、断开）
- 断开后显示重连提示
- 组件卸载时正确清理终端实例
- 在分屏布局下终端正确 resize

### 提交信息

```
feat: add workspace SSH terminal panel component
```

---

## 子任务 14.2：SQL 面板组件

### 功能目标

创建 `WorkspaceSql.vue`，在工作空间面板内嵌入 SQL 控制台。复用现有 SqlEditor、SqlResults、SqlSidebar、SqlTabs 组件。

### 修改文件

```text
packages/rex-console-web/src/
├── features/workspace/panels/
│   └── WorkspaceSql.vue         新增：SQL 面板组件
├── pages/Workspace.vue          修改：面板区域引入 WorkspaceSql
```

### 交互设计

参考现有 `SqlConsole.vue`：

**SQL 面板结构：**

```text
WorkspaceSql
├── 顶部栏（topbar）
│   ├── 左侧：数据库选择下拉
│   └── 右侧：刷新 Schema
├── 查询标签栏（SqlTabs）
├── 工具栏（toolbar）
│   ├── ▶ 执行 · 清空
│   └── Ctrl+Enter 提示
├── 主区域
│   ├── Schema 侧边栏（SqlSidebar）
│   └── 编辑器 + 结果网格
│       ├── SqlEditor
│       └── SqlResults
└── 状态栏
```

**与独立 SqlConsole.vue 的区别：**
- 去掉顶部「← 返回」按钮
- 顶部栏紧凑模式
- 面板宽度较窄时隐藏 Schema 侧边栏

### 接口设计

```ts
// WorkspaceSql.vue props
interface Props {
  resourceId: string
  resourceName: string
  protocol: string   // 'mysql' | 'postgresql' | 'sqlite'
}

// 组件事件
interface Emits {
  (e: 'disconnect'): void
  (e: 'error', msg: string): void
}
```

### 实现要点

1. 复用现有 `SqlEditor`、`SqlResults`、`SqlSidebar`、`SqlTabs` 组件
2. 从 `SqlConsole.vue` 提取查询执行、数据库列表、Schema 加载逻辑
3. 移除路由依赖
4. 面板宽度 < 400px 时自动隐藏 Schema 侧边栏
5. 执行 SQL 结果写入审计日志（复用现有 API 调用）

### 测试标准

- 在工作空间面板中正确渲染 SQL 控制台
- 数据库选择、Schema 浏览、SQL 执行功能正常
- 结果网格正确显示
- 面板宽度变化时侧边栏自动折叠
- 查询标签增删切换正常

### 提交信息

```
feat: add workspace SQL console panel component
```

---

## 子任务 14.3：Files 面板组件

### 功能目标

创建 `WorkspaceFiles.vue`，在工作空间面板内嵌入文件管理器。复用现有 FileList、FileBreadcrumb、TransferQueuePanel 组件。

### 修改文件

```text
packages/rex-console-web/src/
├── features/workspace/panels/
│   └── WorkspaceFiles.vue       新增：Files 面板组件
├── pages/Workspace.vue          修改：面板区域引入 WorkspaceFiles
```

### 交互设计

参考现有 `Files.vue`：

**Files 面板结构：**

```text
WorkspaceFiles
├── 顶部栏（topbar）
│   ├── 面包屑导航（FileBreadcrumb）
│   └── 刷新按钮
├── 工具栏（toolbar）
│   ├── 新建文件夹 · 新建文件 · 删除
│   └── 文件计数
├── 主区域
│   ├── 文件列表（FileList）
│   └── 传输队列面板（TransferQueuePanel）
└── 状态栏
```

**与独立 Files.vue 的区别：**
- 去掉顶部「← 返回」按钮
- 顶部栏紧凑模式
- 传输队列面板默认收起（面板空间有限）

### 接口设计

```ts
// WorkspaceFiles.vue props
interface Props {
  resourceId: string
  resourceName: string
}

// 组件事件
interface Emits {
  (e: 'disconnect'): void
  (e: 'error', msg: string): void
}
```

### 实现要点

1. 复用现有 `FileList`、`FileBreadcrumb`、`TransferQueuePanel` 组件
2. 从 `Files.vue` 提取文件列表加载、导航、CRUD 逻辑
3. 移除路由依赖
4. 传输队列面板默认收起，点击展开
5. 文件操作结果写入审计日志（复用现有 API 调用）

### 测试标准

- 在工作空间面板中正确渲染文件管理器
- 文件列表浏览、新建、删除功能正常
- 面包屑导航正确
- 传输队列可展开/收起
- 文件操作触发审计日志

### 提交信息

```
feat: add workspace file manager panel component
```

---

## 子任务 14.4：面板路由渲染与连接状态

### 功能目标

更新 Workspace.vue，根据标签的协议类型渲染对应的面板组件（SSH/SQL/Files），替换当前占位符。管理面板的连接状态和生命周期。

### 修改文件

```text
packages/rex-console-web/src/
├── pages/Workspace.vue           修改：面板区域根据协议渲染组件
├── features/workspace/useTabs.ts 修改：添加面板连接状态管理
├── i18n/zh.ts, en.ts             修改：添加面板相关 i18n
```

### 交互设计

**面板渲染逻辑：**

```text
getPanelTab(panelIndex) 返回 tab
  ↓
tab.proto == 'ssh'
  → 渲染 <WorkspaceTerminal>
tab.proto == 'mysql' || 'postgresql' || 'sqlite'
  → 渲染 <WorkspaceSql>
tab.proto == 'sftp'
  → 渲染 <WorkspaceFiles>
tab.proto == 其他
  → 渲染 fallback 占位符（协议图标 + "暂不支持 {protocol}"）
```

**连接状态管理：**

```ts
// useTabs.ts 扩展
interface Tab {
  // ... 现有字段
  panelIndex: number
  status: 'online' | 'offline' | 'connecting'
  component: 'terminal' | 'sql' | 'files' | 'unsupported'
}
```

**面板生命周期：**
- 标签激活时：面板组件挂载/恢复
- 标签切换时：上一个面板保持状态（不销毁），下一个面板显示
- 标签关闭时：面板组件卸载，清理 WebSocket 连接
- 分屏模式：多个面板同时活跃，各自独立

**断开连接处理：**
- 面板内断开 → 更新 tab.status 为 'offline'
- 面板内错误 → 显示面板内错误提示，不跳转
- 用户可点击重连按钮重新建立连接

### 接口设计

```ts
// Workspace.vue 面板渲染
<template v-for="i in panelCount" :key="i">
  <div class="ws-panel" :class="{ active: isPanelActive(i - 1) }">
    <template v-if="getPanelTab(i - 1)">
      <WorkspaceTerminal
        v-if="getPanelTab(i - 1)!.component === 'terminal'"
        :resource-id="getPanelTab(i - 1)!.resourceId"
        :resource-name="getPanelTab(i - 1)!.name"
        @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
      />
      <WorkspaceSql
        v-else-if="getPanelTab(i - 1)!.component === 'sql'"
        :resource-id="getPanelTab(i - 1)!.resourceId"
        :resource-name="getPanelTab(i - 1)!.name"
        :protocol="getPanelTab(i - 1)!.proto"
        @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
      />
      <WorkspaceFiles
        v-else-if="getPanelTab(i - 1)!.component === 'files'"
        :resource-id="getPanelTab(i - 1)!.resourceId"
        :resource-name="getPanelTab(i - 1)!.name"
        @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
      />
      <div v-else class="panel-unsupported">
        <span>{{ getProtocolIcon(getPanelTab(i - 1)!.proto).icon }}</span>
        {{ t('ws.panel.unsupported', { protocol: getPanelTab(i - 1)!.proto.toUpperCase() }) }}
      </div>
    </template>
    <div v-else class="panel-empty">
      <span class="panel-empty-text">面板 {{ i }}</span>
    </div>
  </div>
</template>
```

### useTabs.ts 扩展

```ts
// 协议 → 组件类型映射
const PROTOCOL_COMPONENT: Record<string, 'terminal' | 'sql' | 'files' | 'unsupported'> = {
  ssh: 'terminal',
  mysql: 'sql',
  postgresql: 'sql',
  sqlite: 'sql',
  sftp: 'files',
}

// addTab 时自动设置 component
function addTab(name: string, proto: Protocol, resourceId: string): string {
  const id = `tab-${++tabCounter.value}`
  const component = PROTOCOL_COMPONENT[proto] || 'unsupported'
  tabs.value.push({ id, name, proto, resourceId, panelIndex: findFreePanel(), status: 'connecting', component })
  activeTabId.value = id
  return id
}
```

### 测试标准

- SSH 标签激活时面板渲染 Terminal 组件
- MySQL/PostgreSQL/SQLite 标签激活时面板渲染 SQL 组件
- SFTP 标签激活时面板渲染 Files 组件
- 未知协议标签显示 unsupported 提示
- 分屏模式下多个面板同时渲染各自组件
- 标签关闭时组件正确卸载
- 面板断开连接时 tab.status 更新
- 切换标签时面板状态保持（不销毁重建）

### 提交信息

```
feat: integrate protocol panels into workspace with connection state
```

---

## 设计核对点

- [ ] SSH 面板组件与独立 Terminal.vue 功能一致
- [ ] SQL 面板组件与独立 SqlConsole.vue 功能一致
- [ ] Files 面板组件与独立 Files.vue 功能一致
- [ ] 面板渲染逻辑与协议类型匹配
- [ ] 分屏模式下面板独立运作
- [ ] 连接状态正确反映在标签和面板中
- [ ] i18n 覆盖所有新增文字
- [ ] 独立路由页面仍可正常使用

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
