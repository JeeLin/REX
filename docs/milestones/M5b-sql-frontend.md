# M5b: SQL 控制台前端

## Context

M5a 完成了 SqlConnector trait、MySQL/PostgreSQL connector 和 SQL REST API。M5b 实现 SQL 控制台的前端页面。参考原型 `prototype/sql.html`。

## 产品边界

**做什么：**
- SQL API 客户端模块（`src/api/sql.ts`）
- SQL 控制台页面（路由 `/sql/:resourceId`）
- 数据库选择器（顶部栏）
- 左侧数据库结构树（库 → 表 → 列）
- SQL 编辑器（textarea，支持 Ctrl+Enter 执行）
- 执行按钮 + 工具栏
- 查询结果表格（列名 + 数据行 + 状态栏）
- 结果区显示行数和执行耗时
- 移动端底部导航（库表、执行、保存）

**不做什么：**
- 多标签页（后续阶段）
- 查询文件保存/加载（后续阶段）
- 全局查询（跨库，后续阶段）
- AI 助手（后续阶段）
- SQL 格式化（后续阶段）
- 结果导出 CSV/JSON（后续阶段）
- 右键菜单（后续阶段）
- 表结构搜索过滤（后续阶段）

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 5b.1 | SQL API 客户端模块 | 前端 | ⬜ |
| 5b.2 | SQL 控制台页面骨架 + 顶部栏 + 数据库选择器 | 前端 | ⬜ |
| 5b.3 | 左侧数据库结构树 | 前端 | ⬜ |
| 5b.4 | SQL 编辑器 + 执行 + 结果表格 | 前端 | ⬜ |
| 5b.5 | 路由注册 + 全屏布局集成 | 前端 | ⬜ |

---

## 子任务 5b.1：SQL API 客户端模块

### 功能目标

创建 SQL API 客户端，封装 M5a 后端的 4 个端点。

### 文件结构

```text
packages/rex-console-web/src/
└── api/
    └── sql.ts              新增：SQL API 客户端
```

### 接口设计

```typescript
// 类型定义
interface SqlColumn {
  name: string
  data_type: string
}

interface SqlResult {
  columns: SqlColumn[]
  rows: any[][]
  affected_rows: number
  elapsed_ms: number
}

interface DatabaseInfo {
  name: string
}

interface TableInfo {
  name: string
  row_count: number | null
}

interface ColumnInfo {
  name: string
  data_type: string
  is_nullable: boolean
  is_primary_key: boolean
}

// API 函数
function executeSql(resourceId: string, sql: string): Promise<SqlResult>
function listDatabases(resourceId: string): Promise<DatabaseInfo[]>
function listTables(resourceId: string, database: string): Promise<TableInfo[]>
function listColumns(resourceId: string, database: string, table: string): Promise<ColumnInfo[]>
```

### 测试标准

- 类型定义正确
- API 函数签名正确
- 返回值正确解析 `r.data.data`

### 提交信息

```
feat: add SQL API client module
```

---

## 子任务 5b.2：SQL 控制台页面骨架 + 顶部栏 + 数据库选择器

### 功能目标

创建 SQL 控制台页面骨架，包含顶部栏（面包屑 + 数据库选择器 + 刷新按钮）和全屏布局。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/
│   └── SqlConsole.vue      新增：SQL 控制台页面
└── features/
    └── sql/
        └── SqlTopbar.vue   新增：顶部栏组件
```

### 前端交互

参考原型 `prototype/sql.html` 顶部栏：

```text
┌─────────────────────────────────────────────────────┐
│ ← 返回  │  环境 › 资源名 › MySQL  │  ↻  │ 数据库: [▾ production_db] │
└─────────────────────────────────────────────────────┘
```

- 面包屑显示：环境名 › 资源名 › 协议类型
- 数据库下拉选择器：从 API 加载数据库列表
- 刷新按钮：重新加载数据库列表
- 返回按钮：返回上一页

### 状态管理

```typescript
// 页面状态
const resourceId = ref<string>('')        // 从路由参数获取
const resource = ref<Resource | null>(null) // 资源信息
const databases = ref<DatabaseInfo[]>([])   // 数据库列表
const selectedDatabase = ref<string>('')    // 当前选中数据库
const loading = ref(false)
const error = ref('')
```

### 测试标准

- 页面可访问，顶部栏显示正确
- 数据库下拉列表从 API 加载
- 选择数据库后触发加载表列表

### 提交信息

```
feat: add SQL console page skeleton with topbar
```

---

## 子任务 5b.3：左侧数据库结构树

### 功能目标

显示左侧数据库结构树：数据库 → 表列表 → 点击展开列信息。

### 文件结构

```text
packages/rex-console-web/src/features/sql/
├── SqlTopbar.vue           已有
├── SqlSidebar.vue          新增：左侧结构树
└── SqlTreeItem.vue         新增：树节点组件
```

### 前端交互

参考原型左侧树：

```text
┌──────────────────────────┐
│ [▾] ⊞ production_db      │
│   [▾] 表 (8)             │
│     [⊞] users (12,847)   │
│       PK  id         INT │
│           username   VARCHAR(64) │
│           email      VARCHAR(255) │
│     [⊞] orders          │
│     [⊞] products        │
│   [▸] 视图 (3)           │
└──────────────────────────┘
```

- 点击表名：展开显示列信息（名称、类型、PK 标识）
- 点击表名：同时在编辑器插入 `SELECT * FROM table_name LIMIT 100;`
- 折叠/展开切换

### 组件设计

```vue
<!-- SqlSidebar.vue -->
<div class="sql-sidebar">
  <div class="sql-sidebar-header">
    <span>库表结构</span>
  </div>
  <div class="sql-tree">
    <SqlTreeNode
      v-for="table in tables"
      :key="table.name"
      :table="table"
      :database="selectedDatabase"
      @select="handleTableSelect"
    />
  </div>
</div>
```

### 状态管理

```typescript
// 在 SqlConsole.vue 中
const tables = ref<TableInfo[]>([])
const columns = ref<Map<string, ColumnInfo[]>>(new Map())  // table_name -> columns
const expandedTables = ref<Set<string>>(new Set())

// 加载表列表
async function loadTables() { ... }
// 加载列信息
async function loadColumns(tableName: string) { ... }
```

### 测试标准

- 侧边栏显示表列表
- 点击展开表显示列信息
- 点击表名在编辑器插入 SQL

### 提交信息

```
feat: add SQL database structure sidebar
```

---

## 子任务 5b.4：SQL 编辑器 + 执行 + 结果表格

### 功能目标

SQL 编辑器（textarea）+ 执行按钮 + 结果表格显示。

### 文件结构

```text
packages/rex-console-web/src/features/sql/
├── SqlTopbar.vue           已有
├── SqlSidebar.vue          已有
├── SqlEditor.vue           新增：SQL 编辑器
├── SqlResults.vue          新增：结果表格
└── SqlToolbar.vue          新增：工具栏
```

### 前端交互

参考原型：

```text
┌─────────────────────────────────────────────┐
│ [▶ 执行] │ 格式化 │ 清空 │  Ctrl+Enter 执行  │  ← 工具栏
├─────────────────────────────────────────────┤
│ SELECT * FROM users LIMIT 100;              │  ← 编辑器
│                                             │
├─────────────────────────────────────────────┤
│ 结果 │ 消息                     │ 📋 复制  │  ← 结果区
├─────────────────────────────────────────────┤
│ # │ id  │ username │ email │ created_at    │
│ 1 │ 1024│ zhang    │ z@... │ 2024-01-15    │
│ 2 │ 512 │ li_na    │ l@... │ 2024-02-03    │
├─────────────────────────────────────────────┤
│ 10 行 · 执行时间: 0.023s                    │  ← 状态栏
└─────────────────────────────────────────────┘
```

- 编辑器：textarea，支持 Ctrl+Enter 快捷键执行
- 执行按钮：调用 `executeSql` API
- 结果表格：列名作为表头，数据行显示
- 状态栏：显示行数和执行耗时

### 组件设计

```vue
<!-- SqlEditor.vue -->
<div class="sql-editor-wrap">
  <textarea
    v-model="sqlText"
    class="sql-editor"
    spellcheck="false"
    @keydown.ctrl.enter="handleExecute"
    @keydown.meta.enter="handleExecute"
    placeholder="输入 SQL 查询..."
  />
</div>

<!-- SqlResults.vue -->
<div class="sql-results">
  <div class="results-header">
    <span>结果</span>
  </div>
  <div class="results-table-wrap">
    <table v-if="result" class="results-table">
      <thead>
        <tr>
          <th>#</th>
          <th v-for="col in result.columns" :key="col.name">{{ col.name }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(row, i) in result.rows" :key="i">
          <td class="text-muted">{{ i + 1 }}</td>
          <td v-for="(cell, j) in row" :key="j" :class="getCellClass(cell)">
            {{ formatCell(cell) }}
          </td>
        </tr>
      </tbody>
    </table>
  </div>
  <div class="results-footer">
    <span v-if="result">{{ result.rows.length }} 行 · 执行时间: {{ (result.elapsed_ms / 1000).toFixed(3) }}s</span>
    <span v-else-if="error" style="color: var(--danger)">{{ error }}</span>
  </div>
</div>
```

### 状态管理

```typescript
// 在 SqlConsole.vue 中
const sqlText = ref('SELECT 1;')
const result = ref<SqlResult | null>(null)
const executing = ref(false)

async function handleExecute() {
  if (!sqlText.value.trim() || executing.value) return
  executing.value = true
  try {
    result.value = await executeSql(resourceId.value, sqlText.value)
  } catch (e: any) {
    error.value = e.response?.data?.error?.message || '执行失败'
  } finally {
    executing.value = false
  }
}
```

### 测试标准

- 编辑器可输入 SQL
- Ctrl+Enter 触发执行
- 结果表格正确显示列名和数据
- 状态栏显示行数和耗时

### 提交信息

```
feat: add SQL editor and results table
```

---

## 子任务 5b.5：路由注册 + 全屏布局集成

### 功能目标

注册路由，确保 SQL 控制台使用全屏布局。

### 文件结构

```text
packages/rex-console-web/src/
├── router/
│   └── index.ts            修改：添加 /sql/:resourceId 路由
└── pages/
    └── SqlConsole.vue      已有
```

### 路由设计

```typescript
{
  path: '/sql/:resourceId',
  name: 'sql',
  component: () => import('@/pages/SqlConsole.vue'),
  meta: { fullscreen: true },
}
```

### 前端交互

- 从资源列表或环境详情页点击 MySQL/PostgreSQL 资源跳转到 `/sql/:resourceId`
- 全屏布局，无侧边栏导航
- 返回按钮回到来源页面

### 测试标准

- 路由可访问
- 全屏布局正确
- 返回按钮工作正常

### 提交信息

```
feat: add SQL console route and fullscreen layout
```
