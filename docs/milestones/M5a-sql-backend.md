# M5a: SQL 控制台后端

## Context

M4a+M4b 完成了文件传输和文件管理。M5a 实现 SQL 控制台的后端：MySQL 和 PostgreSQL 协议 crate、SQL 执行引擎、数据库结构查询、REST API。参考 `docs/DEVELOPMENT.md` §15 SQL 控制台实现和 `docs/PRODUCT.md` §3.7 SQL 控制台。

## 产品边界

**做什么：**
- rex-mysql crate：MySQL 连接和查询执行
- rex-postgresql crate：PostgreSQL 连接和查询执行
- SQL 执行引擎（统一接口）
- 数据库结构查询（库列表、表列表、列信息）
- REST API（/api/sql/execute、/api/sql/databases、/api/sql/tables、/api/sql/columns）

**不做什么：**
- 前端 SQL 控制台页面（M5b）
- AI 助手
- 全局查询（跨库）
- 查询文件保存/加载
- 执行计划分析

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 5a.1 | SQL 连接器 trait + MySQL connector | 后端 | ✅ |
| 5a.2 | PostgreSQL connector | 后端 | ✅ |
| 5a.3 | SQL REST API（execute/databases/tables/columns） | 后端 | ✅ |

---

## 子任务 5a.1：SQL 连接器 trait + MySQL connector

### 功能目标

定义 SqlConnector trait，实现 MySQL 连接器。参考 rex-transfer 的 FileConnector 模式。

### 文件结构

```text
crates/
├── rex-common/src/lib.rs       修改：添加 SqlConnector trait
├── rex-mysql/
│   ├── Cargo.toml              新增
│   └── src/
│       ├── lib.rs              新增：MySQL connector
│       └── connector.rs        新增：SqlConnector impl
```

### 接口设计

```rust
/// SQL 查询结果列
pub struct SqlColumn {
    pub name: String,
    pub data_type: String,
}

/// SQL 查询结果
pub struct SqlResult {
    pub columns: Vec<SqlColumn>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub affected_rows: u64,
    pub elapsed_ms: u64,
}

/// 数据库信息
pub struct DatabaseInfo {
    pub name: String,
}

/// 表信息
pub struct TableInfo {
    pub name: String,
    pub row_count: Option<u64>,
}

/// 列信息
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
}

/// SQL 连接器 trait（动态分发）
#[async_trait]
pub trait SqlConnector: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn execute(&self, sql: &str) -> Result<SqlResult>;
    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>>;
    async fn list_tables(&self, database: &str) -> Result<Vec<TableInfo>>;
    async fn list_columns(&self, database: &str, table: &str) -> Result<Vec<ColumnInfo>>;
    async fn close(&self) -> Result<()>;
}
```

### 测试标准

- SqlConnector trait 定义正确
- MySQL connector 结构体可创建
- 单元测试 trait 定义

### 提交信息

```
feat: add SqlConnector trait and MySQL connector
```

---

## 子任务 5a.2：PostgreSQL connector

### 功能目标

实现 PostgreSQL 连接器，复用 SqlConnector trait。

### 文件结构

```text
crates/
├── rex-postgresql/
│   ├── Cargo.toml              新增
│   └── src/
│       ├── lib.rs              新增
│       └── connector.rs        新增：SqlConnector impl
```

### 测试标准

- PostgreSQL connector 结构体可创建
- 单元测试 trait 实现

### 提交信息

```
feat: add PostgreSQL connector
```

---

## 子任务 5a.3：SQL REST API

### 功能目标

提供 SQL 操作的 REST API，通过资源关联的 connector 执行查询。

### 文件结构

```text
crates/rex-hub/src/
├── sql.rs              新增：SQL API handlers
└── routes.rs           修改：注册路由
```

### 接口设计

```
POST /api/resources/:resource_id/sql/execute    — 执行 SQL
GET  /api/resources/:resource_id/sql/databases  — 列出数据库
GET  /api/resources/:resource_id/sql/tables?database=x  — 列出表
GET  /api/resources/:resource_id/sql/columns?database=x&table=y  — 列出列
```

### 测试标准

- 执行 SQL 返回正确结构
- 列出数据库/表/列返回正确结构
- 不存在的资源返回 404

### 提交信息

```
feat: add SQL REST API
```
