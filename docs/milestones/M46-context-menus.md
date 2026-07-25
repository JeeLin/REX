# M46: 右键上下文菜单补全

## Context

M45 完成工作区 bug 修复（v0.39.1）。PRODUCT.md 规定了各区域的右键菜单，但实现不完整。本里程碑补齐所有缺失的上下文菜单项，统一交互范式。

版本类型：patch（交互补全），版本号 0.39.1 → 0.39.2。

## 产品边界

**本阶段做：**
- Tab 标签右键菜单：移动到面板、新建连接、断开连接
- 终端右键菜单：复制地址、打开 SFTP
- SQL 导航树右键菜单：复制表名、刷新、新查询
- Redis 键树右键菜单：文件夹操作（按前缀加载、批量删除）
- 审计日志右键菜单：查看详情、复制、按类型/环境筛选、刷新、导出、清除筛选
- 环境/资源列表右键菜单：编辑、删除、新建资源
- 侧栏资源右键菜单：打开、属性、删除
- 统一 ContextMenu 组件样式

**本阶段不做：**
- SQL create table/view/procedure/function（需后端 DDL API）
- SQL dump SQL / truncate（需后端 API）
- Redis 文件夹内存分析（需后端 MEMORY USAGE API）
- Redis 按前缀扫描删除（需后端 SCAN+DEL 批量 API）

## 子任务清单

| # | 内容 | 预计文件 | 依赖 |
|---|------|----------|------|
| 1 | Tab 标签右键菜单补全 | `WorkspacePage.vue` | — |
| 2 | 终端右键菜单补全 | `TerminalContextMenu.vue`, `TerminalView.vue` | — |
| 3 | SQL 导航树右键菜单补全 | `SqlNavTree.vue` | — |
| 4 | Redis 文件夹右键菜单 | `RedisPage.vue` | — |
| 5 | 审计日志右键菜单 | `AuditLogPage.vue` | — |
| 6 | 环境/资源右键菜单 | `EnvironmentsPage.vue`, `EnvironmentDetailPage.vue` | — |
| 7 | 侧栏资源右键菜单 | `AppLayout.vue` (ResourcePanel) | — |

## 子任务详细设计

### 1 Tab 标签右键菜单补全

**目标**

PRODUCT.md 规定：关闭 / 关闭其他 / 关闭右侧 / 关闭左侧 / 关闭全部 / 复制标签 / 移动到面板 ▸ / 新建连接 / 全部断开。

当前已有：重命名、复制、广播切换、关闭系列、属性、颜色。

**缺失项**
- 移动到面板（submenu，列出所有 pane）
- 新建连接（打开 Quick Connect）
- 断开连接（关闭 tab + 断开 WebSocket）

**修改文件**

`packages/rex-console-web/src/pages/WorkspacePage.vue`

**设计**

```html
<div class="tab-ctx-item" @click="openQuickConnect">
  <span class="tab-ctx-icon">➕</span>
  <span>{{ t('workspace.newConnection') }}</span>
</div>
<div v-if="splitCount > 1" class="tab-ctx-item" @mouseenter="showMovePane = true" @mouseleave="showMovePane = false">
  <span class="tab-ctx-icon">↗</span>
  <span>{{ t('workspace.moveToPane') }}</span>
  <span class="tab-ctx-arrow">▸</span>
  <div v-if="showMovePane" class="tab-ctx-submenu">
    <div v-for="p in splitCount" :key="p" class="tab-ctx-item" @click="moveTabToPane(tabContextMenu.tabId, p - 1)">
      {{ t('workspace.pane') }} {{ p }}{{ p - 1 === currentPane ? ' ✓' : '' }}
    </div>
  </div>
</div>
<div class="tab-ctx-item tab-ctx-item--danger" @click="disconnectTab(tabContextMenu.tabId)">
  <span class="tab-ctx-icon">🔌</span>
  <span>{{ t('workspace.disconnect') }}</span>
</div>
```

**实现逻辑**

- `moveTabToPane(tabId, paneIndex)`：将 `paneTabs[paneIndex] = tabId`，`currentPane = paneIndex`
- `disconnectTab(tabId)`：发送断开消息（如 SSH disconnect），然后 `closeTab(tabId)`
- `openQuickConnect`：设置 QuickConnect 可见性

**i18n 新增 key**

```json
{
  "workspace.newConnection": "新建连接",
  "workspace.moveToPane": "移动到面板",
  "workspace.disconnect": "断开连接",
  "workspace.pane": "面板"
}
```

**提交信息**: `feat(workspace): add move-to-pane, new connection, disconnect to tab context menu`

### 2 终端右键菜单补全

**目标**

PRODUCT.md 规定：复制 / 粘贴 / 全选 / 清屏 / 查找 / 编码▸ / 重连 / 复制地址 / 打开 SFTP / 断开。

当前已有：Copy / Paste / Select All / Clear / Find / Encoding▸ / Reconnect / Disconnect。

**缺失项**
- 复制地址（copy resource address to clipboard）
- 打开 SFTP（toggle SFTP drawer in same terminal tab）

**修改文件**

- `packages/rex-console-web/src/features/terminal/TerminalContextMenu.vue` — 添加菜单项
- `packages/rex-console-web/src/features/terminal/TerminalView.vue` — 新增 emit 事件

**设计**

TerminalContextMenu 新增两个菜单项：
```html
<div class="tcm-item" @click="$emit('copyAddress')">
  <span class="tcm-icon">📍</span>
  <span>Copy Address</span>
</div>
<div class="tcm-item" @click="$emit('openSftp')">
  <span class="tcm-icon">📁</span>
  <span>Open SFTP</span>
</div>
```

TerminalView 新增 emit：
- `copyAddress`：`navigator.clipboard.writeText(resourceAddress)`
- `openSftp`：toggle SFTP drawer（复用现有 SFTP 抽屉逻辑）

**i18n 新增 key**

```json
{
  "terminal.copyAddress": "复制地址",
  "terminal.openSftp": "打开 SFTP"
}
```

**提交信息**: `feat(terminal): add copy address and open SFTP to context menu`

### 3 SQL 导航树右键菜单补全

**目标**

PRODUCT.md 规定：建表/视图/过程/函数/事件、Dump SQL、Execute SQL、Truncate、属性等（10-20 项按对象类型）。

后端暂无 DDL API，本阶段只做前端可实现的部分。

**当前已有**：View DDL（只读）、Properties（只读）

**本阶段新增**（纯前端操作）
- 复制表名（clipboard）
- 复制 DDL（clipboard）
- 新建查询（在对象区打开新查询 Tab，预填 `SELECT * FROM table_name`）
- 刷新导航树

**修改文件**

`packages/rex-console-web/src/features/sql/SqlNavTree.vue`

**设计**

```html
<div class="ctx-item" @click="copyTableName">📋 {{ t('sql.copyTableName') }}</div>
<div class="ctx-item" @click="copyDDL">📋 {{ t('sql.copyDDL') }}</div>
<div class="ctx-item" @click="openNewQuery">🔍 {{ t('sql.newQuery') }}</div>
<div class="ctx-item" @click="$emit('refresh')">🔄 {{ t('sql.refresh') }}</div>
<div class="ctx-separator" />
<div class="ctx-item" @click="$emit('viewDDL', { db: ctxMenu.dbName, table: ctxMenu.tableName })">📄 {{ t('sql.viewDDL') }}</div>
<div class="ctx-item" @click="$emit('viewProps', { db: ctxMenu.dbName, table: ctxMenu.tableName })">⚙ {{ t('sql.properties') }}</div>
```

**i18n 新增 key**

```json
{
  "sql.copyTableName": "复制表名",
  "sql.copyDDL": "复制 DDL",
  "sql.newQuery": "新建查询",
  "sql.refresh": "刷新",
  "sql.viewDDL": "查看 DDL",
  "sql.properties": "属性"
}
```

**提交信息**: `feat(sql): enhance nav tree context menu with copy, new query, refresh`

### 4 Redis 文件夹右键菜单

**目标**

PRODUCT.md 规定：文件夹右键（多选/内存分析/按前缀加载/批量删除）。

后端暂无 MEMORY USAGE API，本阶段只做前端可实现的部分。

**当前已有**：键右键菜单（copy/delete/new tab/export），无文件夹右键菜单。

**本阶段新增**
- 文件夹右键：按前缀加载（展开命名空间）、复制前缀、删除前缀下所有键（需 SCAN+DEL，暂用前端逐个 DEL）
- 空白区域右键：刷新、粘贴（导入键）

**修改文件**

`packages/rex-console-web/src/features/redis/RedisPage.vue`

**设计**

在键树虚拟滚动区域添加文件夹行的 `@contextmenu` 事件：

```html
<div class="ns-folder" @contextmenu.prevent="onFolderCtx($event, folder.prefix)">
```

```typescript
function onFolderCtx(e: MouseEvent, prefix: string) {
  e.preventDefault()
  folderCtx.value = { show: true, x: e.clientX, y: e.clientY, prefix }
}
```

菜单项：
```html
<div class="ctx-item" @click="loadPrefix(folderCtx.prefix)">📂 {{ t('redis.loadPrefix') }}</div>
<div class="ctx-item" @click="copyPrefix(folderCtx.prefix)">📋 {{ t('redis.copyPrefix') }}</div>
<div class="ctx-item tab-ctx-item--danger" @click="deletePrefix(folderCtx.prefix)">🗑 {{ t('redis.deletePrefix') }}</div>
```

**注意**：`deletePrefix` 实现为 SCAN 匹配 `prefix*` 逐个 DEL，显示进度 toast。大批量操作需用户确认。

**i18n 新增 key**

```json
{
  "redis.loadPrefix": "按前缀加载",
  "redis.copyPrefix": "复制前缀",
  "redis.deletePrefix": "删除前缀下所有键"
}
```

**提交信息**: `feat(redis): add folder context menu with prefix load, copy, and delete`

### 5 审计日志右键菜单

**目标**

PRODUCT.md 规定：右键（查看详情/复制/按类型或环境筛选/刷新/导出/清除筛选）。

当前：无右键菜单。

**修改文件**

`packages/rex-console-web/src/pages/AuditLogPage.vue`

**设计**

在表格行添加 `@contextmenu` 事件：

```html
<tr v-for="entry in entries" :key="entry.id" @contextmenu.prevent="onRowCtx($event, entry)">
```

菜单项：
```html
<div class="ctx-item" @click="expandRow(ctxMenu.entry)">📋 {{ t('auditLog.viewDetail') }}</div>
<div class="ctx-item" @click="copyEntry(ctxMenu.entry)">📋 {{ t('auditLog.copy') }}</div>
<div class="ctx-separator" />
<div class="ctx-item" @click="filterByType(ctxMenu.entry.action)">🏷 {{ t('auditLog.filterByType') }}</div>
<div class="ctx-item" @click="filterByEnv(ctxMenu.entry.environment_id)">🌍 {{ t('auditLog.filterByEnv') }}</div>
<div class="ctx-separator" />
<div class="ctx-item" @click="refreshLog">🔄 {{ t('auditLog.refresh') }}</div>
<div class="ctx-item" @click="exportCSV">📥 {{ t('auditLog.export') }}</div>
<div class="ctx-item" @click="clearFilters">🧹 {{ t('auditLog.clearFilters') }}</div>
```

**实现逻辑**

- `expandRow`：复用现有行展开逻辑
- `copyEntry`：`navigator.clipboard.writeText(JSON.stringify(entry, null, 2))`
- `filterByType`：设置 `filters.action = entry.action`，触发查询
- `filterByEnv`：设置 `filters.environment_id = entry.environment_id`，触发查询
- `refreshLog`：调用现有 `loadEntries()`
- `exportCSV`：复用现有 CSV 导出逻辑
- `clearFilters`：重置所有 filter 为 undefined，触发查询

**i18n 新增 key**

```json
{
  "auditLog.viewDetail": "查看详情",
  "auditLog.copy": "复制记录",
  "auditLog.filterByType": "按类型筛选",
  "auditLog.filterByEnv": "按环境筛选",
  "auditLog.refresh": "刷新",
  "auditLog.export": "导出 CSV",
  "auditLog.clearFilters": "清除筛选"
}
```

**提交信息**: `feat(audit): add row context menu with view, copy, filter, export`

### 6 环境/资源右键菜单

**目标**

环境卡片和资源表格行添加右键菜单，提供常用操作入口。

**修改文件**

- `packages/rex-console-web/src/pages/EnvironmentsPage.vue` — 环境卡片右键
- `packages/rex-console-web/src/pages/EnvironmentDetailPage.vue` — 资源表格行右键

**环境卡片右键菜单**

```html
<div class="ctx-item" @click="editEnv(ctxMenu.env)">✏ {{ t('environments.edit') }}</div>
<div class="ctx-item" @click="newResource(ctxMenu.env)">➕ {{ t('environments.newResource') }}</div>
<div class="ctx-item tab-ctx-item--danger" @click="deleteEnv(ctxMenu.env)">🗑 {{ t('environments.delete') }}</div>
```

**资源表格行右键菜单**

```html
<div class="ctx-item" @click="openResource(ctxMenu.resource)">🚀 {{ t('resources.open') }}</div>
<div class="ctx-item" @click="editResource(ctxMenu.resource)">✏ {{ t('resources.edit') }}</div>
<div class="ctx-item tab-ctx-item--danger" @click="deleteResource(ctxMenu.resource)">🗑 {{ t('resources.delete') }}</div>
```

**实现逻辑**

- `editEnv`：导航到 `/environments/:id`
- `newResource`：打开资源创建向导，预选环境
- `deleteEnv`：复用现有删除逻辑（确认对话框）
- `openResource`：调用 `openResourceFromTree(resource)` 在工作区打开
- `editResource`：打开资源属性对话框
- `deleteResource`：复用现有删除逻辑

**i18n 新增 key**

```json
{
  "environments.newResource": "新建资源",
  "resources.open": "在工作区打开",
  "resources.edit": "编辑",
  "resources.delete": "删除"
}
```

**提交信息**: `feat(env): add context menus to environment cards and resource rows`

### 7 侧栏资源右键菜单

**目标**

侧栏连接树中的资源项添加右键菜单。

**修改文件**

`packages/rex-console-web/src/layouts/AppLayout.vue`（ResourcePanel 区域）

**设计**

```html
<div class="resource-item" @contextmenu.prevent="onResourceCtx($event, resource)">
```

菜单项：
```html
<div class="ctx-item" @click="openResource(ctxMenu.resource)">🚀 {{ t('sidebar.open') }}</div>
<div class="ctx-item" @click="editResource(ctxMenu.resource)">✏ {{ t('sidebar.properties') }}</div>
<div class="ctx-item tab-ctx-item--danger" @click="deleteResource(ctxMenu.resource)">🗑 {{ t('sidebar.delete') }}</div>
```

**i18n 新增 key**

```json
{
  "sidebar.open": "在工作区打开",
  "sidebar.properties": "属性",
  "sidebar.delete": "删除"
}
```

**提交信息**: `feat(sidebar): add resource context menu with open, properties, delete`

## 设计核对点

- [ ] Tab 右键：移动到面板子菜单正常工作
- [ ] Tab 右键：新建连接打开 Quick Connect
- [ ] Tab 右键：断开连接关闭 tab 并断开 WebSocket
- [ ] 终端右键：复制地址到剪贴板
- [ ] 终端右键：打开 SFTP 抽屉
- [ ] SQL 右键：复制表名、复制 DDL、新建查询
- [ ] Redis 文件夹右键：按前缀加载、复制前缀、删除前缀
- [ ] 审计日志右键：查看详情、复制、筛选、导出、清除
- [ ] 环境卡片右键：编辑、新建资源、删除
- [ ] 资源行右键：打开、编辑、删除
- [ ] 侧栏资源右键：打开、属性、删除
- [ ] 所有右键菜单点击外部关闭
- [ ] 所有右键菜单靠近视口边界时自动翻转

## Flow Status

- [ ] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
