# 0.5.0: M4 SQL 控制台

## Context
M0 骨架 → M1 设计系统 → M2 工作空间外壳 → M3 SSH 终端。M4 在 M2 工作空间内接入数据库查询控制台，是第二个有后端协议接入的功能模块。

前序：M3 SSH 终端（后端 WebSocket 桥接模式已验证，M4 复用相同架构思路）。
后续：M5 Redis 控制台、M6 文件管理。

版本类型：minor

## 产品边界
- **做**：后端 MySQL/PostgreSQL/SQLite 协议接入、REST/WebSocket 查询端点、前端导航树、查询编辑器（多 Tab/语法高亮/补全/折叠/查找替换/剪贴板栈/执行模式）、结果网格（JSON 表格视图）、SQL 执行与错误处理
- **不做**：结果网格内联编辑+Apply/Discard（M4b）、表设计器（M4b）、DDL 抽屉/导入导出向导/AI 助手（M4b）、全局查询（M4b）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 后端 SQL 协议接入（MySQL/PostgreSQL/SQLite 连接器 + REST/WebSocket 端点） | ⬜ |
| 2 | 前端导航树（连接组→库→表/视图/函数/过程/事件 + 搜索 + 右键） | ⬜ |
| 3 | 查询编辑器（Tab 多开、语法高亮、`.` 补全、折叠、查找替换、剪贴板栈） | ⬜ |
| 4 | 结果网格（JSON 表格视图 + 排序/过滤 + 执行状态栏） | ⬜ |
| 5 | SQL 执行与错误处理（执行模式、错误高亮、结果持久化） | ⬜ |
| 6 | 测试与收尾 | ⬜ |

## 子任务详细设计

### 1 后端 SQL 协议接入

- **功能目标**：rex-hub 提供统一 SQL 连接器抽象（`SqlConnector` trait），支持 MySQL/PostgreSQL/SQLite，通过 REST API 和 WebSocket 暴露查询能力
- **文件结构**：
  - `crates/rex-mysql/Cargo.toml`（修改：添加 sqlx 依赖）
  - `crates/rex-mysql/src/lib.rs`（实现：MySqlConnector）
  - `crates/rex-postgresql/Cargo.toml`（修改：添加 sqlx 依赖）
  - `crates/rex-postgresql/src/lib.rs`（实现：PostgresConnector）
  - `crates/rex-sqlite/Cargo.toml`（修改：添加 rusqlite 依赖）
  - `crates/rex-sqlite/src/lib.rs`（实现：SqliteConnector）
  - `crates/rex-common/src/sql.rs`（新增：SqlConnector trait + 共享类型）
  - `crates/rex-hub/src/sql_api.rs`（新增：REST + WebSocket 路由）
  - `crates/rex-hub/src/bin/rex-hub.rs`（修改：注册 `/api/sql/*` 路由）
- **接口设计**：
  ```rust
  // crates/rex-common/src/sql.rs
  #[async_trait]
  pub trait SqlConnector: Send + Sync {
      async fn execute(&mut self, sql: &str) -> Result<QueryResult>;
      async fn databases(&mut self) -> Result<Vec<String>>;
      async fn tables(&mut self, db: &str) -> Result<Vec<TableInfo>>;
      async fn columns(&mut self, db: &str, table: &str) -> Result<Vec<ColumnInfo>>;
      async fn close(&mut self) -> Result<()>;
  }

  #[derive(Debug, Serialize, Deserialize)]
  pub struct QueryResult {
      pub columns: Vec<ColumnInfo>,
      pub rows: Vec<Vec<serde_json::Value>>,
      pub affected_rows: u64,
      pub elapsed_ms: u64,
  }

  #[derive(Debug, Serialize, Deserialize)]
  pub struct ColumnInfo {
      pub name: String,
      pub data_type: String,
      pub nullable: bool,
      pub is_primary_key: bool,
  }

  #[derive(Debug, Serialize, Deserialize)]
  pub struct TableInfo {
      pub name: String,
      pub table_type: String, // "BASE TABLE" | "VIEW"
  }
  ```
  REST API：
  ```http
  POST /api/sql/connect       # 建立连接，返回 sessionId
  POST /api/sql/disconnect     # 断开连接
  POST /api/sql/query          # 执行 SQL（body: { sessionId, sql, database? }）
  GET  /api/sql/databases      # 获取库列表
  GET  /api/sql/tables?db=xxx  # 获取表列表
  GET  /api/sql/columns?db=xxx&table=yyy  # 获取字段列表
  ```
  WebSocket `/ws/sql`（可选，用于流式大结果集）：
  ```json
  { "type": "sql.query", "payload": { "sessionId": "...", "sql": "...", "database": "..." } }
  { "type": "sql.result", "payload": { "columns": [...], "rows": [...], "elapsed_ms": 12 } }
  { "type": "sql.error", "payload": { "message": "...", "code": "...", "position": 42 } }
  ```
- **后端流程**：
  1. 前端 POST `/api/sql/connect` → 后端创建 SqlConnector 实例，存入连接池（HashMap<sessionId, Box<dyn SqlConnector>>）
  2. 前端 POST `/api/sql/query` → 后端从连接池取出 connector → `execute(sql)` → 返回 QueryResult
  3. 连接超时或前端断开 → 自动关闭 connector
- **测试标准**：`cargo build`、`cargo clippy` 通过；手动测试：可通过 REST API 连接 SQLite 并执行查询
- **提交**：`feat(sql): add SQL connector trait and MySQL/PostgreSQL/SQLite backends`

### 2 前端导航树

- **功能目标**：SQL 控制台左侧导航树，展示连接组→库→表/视图/函数/过程/事件层级，支持搜索和右键菜单
- **文件结构**：
  - `src/features/sql/SqlPage.vue`（新增：两栏布局主页面）
  - `src/features/sql/SqlNavTree.vue`（新增：导航树组件）
  - `src/features/sql/useSqlNav.ts`（新增：导航数据 composable）
  - `src/api/sql.ts`（新增：SQL API 调用封装）
- **接口设计**：
  ```ts
  // SqlNavTree.vue
  defineProps<{
    databases: DatabaseNode[]
    loading: boolean
  }>()
  defineEmits<{
    selectTable: [db: string, table: string]
    selectView: [db: string, view: string]
    selectFunction: [db: string, fn: string]
    refresh: []
  }>()

  // 数据模型
  interface DatabaseNode {
    name: string
    tables: TableNode[]
    views: ViewNode[]
    functions: FunctionNode[]
    procedures: ProcedureNode[]
  }
  ```
- **交互设计**：
  - 左栏可拖拽调宽（min 200px, max 400px）
  - 层级：连接组 → 库名 → 表/视图/函数/过程/事件（按类型分组，显示数量徽章）
  - 表可展开：字段/索引/外键
  - 双击表 → 打开数据网格 Tab
  - 双击字段 → 打开表设计器（M4b 预留）
  - 顶部搜索框实时过滤，自动展开匹配分支
  - 右键：建表/刷新/复制名称（10-20 项按对象类型，M4 先做基础项）
- **测试标准**：`bun run type-check && bun run lint && bun run build` 通过
- **提交**：`feat(web): add SQL navigation tree with search and context menu`

### 3 查询编辑器

- **功能目标**：基于 CodeMirror 6 的 SQL 查询编辑器，支持多 Tab、语法高亮、`.` 补全、折叠、查找替换、剪贴板栈
- **文件结构**：
  - `src/features/sql/SqlEditor.vue`（新增：CodeMirror 封装组件）
  - `src/features/sql/useSqlEditor.ts`（新增：编辑器状态 composable）
- **接口设计**：
  ```ts
  // SqlEditor.vue
  defineProps<{
    modelValue: string
    database?: string
    readOnly?: boolean
  }>()
  defineEmits<{
    'update:modelValue': [value: string]
    execute: [sql: string]
    save: [sql: string]
  }>()
  ```
- **交互设计**：
  - Tab 多开：每个 Tab 独立编辑器实例，顶部 Tab 栏切换
  - 新 Tab 标题 "查询 N"，保存后显示文件名
  - 语法高亮：按 SQL 方言高亮保留字、字符串、数字、注释
  - `.` 补全：输入 `.` 触发表/字段属性补全
  - 代码折叠：括号匹配折叠
  - 查找替换栏：Ctrl+F / Ctrl+Shift+R（复用前端已有的搜索组件模式）
  - 剪贴板栈：Ctrl+Shift+V 循环最近 10 项
  - 快捷键：Ctrl+Enter 执行、Ctrl+S 保存、Ctrl+F 查找
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add SQL query editor with CodeMirror 6`

### 4 结果网格

- **功能目标**：查询结果以表格形式展示，支持排序、过滤、分页、状态栏
- **文件结构**：
  - `src/features/sql/SqlResultGrid.vue`（新增：结果网格组件）
- **接口设计**：
  ```ts
  // SqlResultGrid.vue
  defineProps<{
    result: QueryResult | null
    loading: boolean
    error: string | null
  }>()
  ```
- **交互设计**：
  - 类电子表格：列头可点击排序（升序/降序/无），列头下方过滤输入框
  - 数据类型格式化：NULL 显示灰色斜体、日期格式化、JSON 缩进、长文本截断
  - 底部状态栏：返回行数 / 执行时间 / 当前库
  - 空结果：显示 "No results" 提示
  - 加载中：骨架屏 + spinner
  - 错误：红色错误信息 + 错误位置高亮（如有 position）
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add SQL result grid with sort and filter`

### 5 SQL 执行与错误处理

- **功能目标**：完善 SQL 执行流程，支持执行模式、错误高亮、结果持久化到 Tab
- **文件结构**：
  - `src/features/sql/SqlPage.vue`（修改：整合执行流程）
  - `src/features/sql/useSqlQuery.ts`（新增：查询执行 composable）
- **执行模式**：
  - **Run**（全部）：执行编辑器全部内容
  - **Run Current**（光标所在语句）：自动分割 SQL，执行光标所在语句
  - **Run Selected**（选中）：仅执行选中文本
- **错误处理**：
  - 后端返回错误 position → 编辑器高亮对应位置（红色波浪线）
  - 错误信息显示在结果区（红色背景卡片 + 错误码 + 位置）
- **结果持久化**：
  - 每个 Tab 维护独立的查询结果
  - 切换 Tab 后结果保留
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add SQL execution modes and error highlighting`

### 6 测试与收尾

- **功能目标**：验证全部功能，修复问题
- **测试标准**：type-check + lint + build + cargo build + cargo clippy 全通过
- **提交**：`fix(web): SQL console polish and fixes`

## 设计核对点
- [ ] 后端 SqlConnector trait 统一三库接入，REST API 可执行查询
- [ ] 前端导航树正确展示库→表/视图/函数层级
- [ ] 导航树搜索可实时过滤并展开匹配分支
- [ ] 查询编辑器多 Tab 独立，语法高亮正确
- [ ] `.` 补全可触发字段建议
- [ ] 查找替换栏 Ctrl+F / Ctrl+Shift+R 正常工作
- [ ] 结果网格正确展示查询结果，排序/过滤可用
- [ ] 执行模式（Run/Run Current/Run Selected）正确分割并执行
- [ ] 错误信息带位置高亮
- [ ] 各 Tab 结果独立、切换后保留

## Flow Status

- [x] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
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
