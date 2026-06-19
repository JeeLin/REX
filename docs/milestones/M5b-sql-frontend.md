c# M5b: SQL 控制台前端

## Context

M5a 完成了 SQL 后端（rex-mysql、rex-postgresql、SqlConnector trait、REST API）。M5b 实现 SQL 控制台的前端页面。参考 `docs/PRODUCT.md` §3.7 SQL 控制台和原型 `prototype/sql.html`。

## 产品边界

**做什么：**
- SQL 控制台页面（路由 `/sql/:resourceId`）
- 数据库选择器（顶部栏）
- SQL 编辑器（textarea，Ctrl+Enter 执行）
- 结果表格（列名、数据行、行数、执行耗时）
- 左侧数据库结构树（库 → 表 → 列）
- 查询标签页（多标签）
- 工具栏（执行、格式化、清空）

**不做什么：**
- AI 助手（后续阶段）
- 全局查询（跨库，后续阶段）
- 查询文件保存/加载（后续阶段）
- 导出 CSV/JSON（后续阶段）
- 结果排序/分页（后续阶段）
- 右键菜单（后续阶段）

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 5b.1 | SQL API 客户端（sql.ts） | 前端 | ✅ |
| 5b.2 | SQL 控制台页面骨架（SqlConsole.vue） | 前端 | ✅ |
| 5b.3 | 数据库结构树组件 | 前端 | ✅ |
| 5b.4 | SQL 编辑器组件 | 前端 | ✅ |
| 5b.5 | 查询结果表格组件 | 前端 | ✅ |
| 5b.6 | 查询标签页管理 | 前端 | ✅ |
| 5b.7 | 路由注册 + 全屏布局 | 前端 | ✅ |

---

## 子任务 5b.1：SQL API 客户端

### 功能目标

创建前端 API 客户端，封装 M5a 后端的 SQL REST API。

### 文件结构

```text
packages/rex-console-web/src/
└── api/
    └── sql.ts              新增：SQL API 客户端
```

### 接口设计

```typescript
export function executeSql(resourceId: string, sql: string): Promise<SqlResult>
export function listDatabases(resourceId: string): Promise<DatabaseInfo[]>
export function listTables(resourceId: string, database: string): Promise<TableInfo[]>
export function listColumns(resourceId: string, database: string, table: string): Promise<ColumnInfo[]>
export function getResourceInfo(resourceId: string): Promise<{ name: string; protocol: string }>
```

### 测试标准

- 类型定义与后端一致
- API 函数签名正确
- 错误处理（复用 client 拦截器）

### 提交信息

```
feat: add SQL API client
```

---

## 子任务 5b.2：SQL 控制台页面骨架

### 功能目标

创建 SQL 控制台的全屏布局页面，包含顶部栏（面包屑 + 数据库选择器）、工具栏、主区域（编辑器 + 结果）。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/
│   └── SqlConsole.vue      新增：SQL 控制台页面
└── features/
    └── sql/
        └── SqlTopbar.vue   新增：顶部栏（面包屑 + DB 选择器）
```

### 前端交互

参考原型 `prototype/sql.html` 顶部栏：
- 左侧：面包屑（资源名 › 协议）
- 右侧：数据库下拉选择器 + 刷新按钮
- 从路由参数 `:resourceId` 获取资源信息

### 提交信息

```
feat: add SQL console page skeleton
```

---

## 子任务 5b.3：数据库结构树组件

### 功能目标

左侧可折叠面板，展示当前数据库的表列表，点击表名展开列信息。

### 文件结构

```text
packages/rex-console-web/src/features/sql/
└── SqlSidebar.vue          新增：结构树面板
```

### 前端交互

参考原型左侧树：
- 数据库名（已选中的）
- 表列表（可展开/折叠）
- 展开表后显示列名、类型、PK 标记
- 点击表名 → 在编辑器插入 `SELECT * FROM {table} LIMIT 100;`

### 提交信息

```
feat: add SQL database structure sidebar
```

---

## 子任务 5b.4：SQL 编辑器组件

### 功能目标

SQL 编辑器，支持多行输入，Ctrl+Enter 执行选中文本或全部。

### 文件结构

```text
packages/rex-console-web/src/features/sql/
└── SqlEditor.vue           新增：SQL 编辑器
```

### 前端交互

参考原型编辑器：
- textarea，等宽字体
- Ctrl+Enter 执行
- Tab 键插入两个空格
- 自动增长高度

### 提交信息

```
feat: add SQL editor component
```

---

## 子任务 5b.5：查询结果表格组件

### 功能目标

显示 SQL 查询结果，包含列名表头、数据行、底部状态栏（行数 + 执行耗时）。

### 文件结构

```text
packages/rex-console-web/src/features/sql/
└── SqlResults.vue          新增：结果表格
```

### 前端交互

参考原型结果区域：
- 表头：列名，sticky
- 数据行：等宽字体，NULL 斜体灰色，数字蓝色
- 底部：行数 + 执行时间
- 空结果时显示提示信息
- 加载中显示 spinner

### 提交信息

```
feat: add SQL results table component
```

---

## 子任务 5b.6：查询标签页管理

### 功能目标

多标签页支持，每个标签页独立 SQL 内容和结果。

### 文件结构

```text
packages/rex-console-web/src/features/sql/
└── SqlTabs.vue             新增：标签页组件
```

### 前端交互

参考原型标签页：
- 默认一个「查询 1」标签
- 点击 + 新建标签
- 标签可关闭（至少保留一个）
- 切换标签保留各自的 SQL 和结果

### 提交信息

```
feat: add SQL query tabs
```

---

## 子任务 5b.7：路由注册 + 全屏布局

### 功能目标

注册 SQL 控制台路由，使用全屏布局（无侧边栏导航）。

### 路由设计

```typescript
{
  path: '/sql/:resourceId',
  name: 'sql',
  component: () => import('@/pages/SqlConsole.vue'),
  meta: { layout: 'fullscreen' },
}
```

### 提交信息

```
feat: register SQL console route with fullscreen layout
```

## 设计核对点

- [x] 单用户，无 RBAC
- [x] 文件数据不经过浏览器（SQL 文本发送，后端执行）
- [x] 全屏布局（无侧边栏导航）
- [x] 参考原型 `prototype/sql.html` 交互
- [x] 组件按功能域组织在 `features/sql/`
- [x] API 客户端复用 `client.ts` 拦截器
- [x] 不引入 AI 助手、全局查询等超出范围的功能

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [ ] 步骤8：提交
