# M11: SQL 控制台接通

## Context

M10 完成了工作区 Tab 系统改造和组件嵌入，SqlPage 已能接收 props 并自动连接。但后端 `SqlConnectorFactory::connect()` 永远返回错误，导致 SQL 连接实际不可用。M11 修复这个致命断路。

本里程碑版本类型：minor（新功能），版本号 0.11.0 → 0.12.0。

## 产品边界

**本阶段做：**
- SqlConnectorFactory 重写（分发到真实 connector）
- sql_api.rs connect handler 适配
- 前端 SQL 连接/查询/导航树端到端验证

**本阶段不做：**
- 表设计器、DDL 抽屉、导入导出向导（后续里程碑）
- AI 助手整合

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | SqlConnectorFactory 重写（rex-hub 层分发） | ✅ |
| 2 | sql_api.rs connect handler 适配 + 前端验证 | ✅ |

## 子任务详细设计

### 1 SqlConnectorFactory 重写

**功能目标**

将 SQL 连接分发逻辑从 rex-common 移到 rex-hub，让 `SqlConnectorFactory::connect()` 能真正创建连接器。

**文件结构**

修改：
- `crates/rex-common/src/sql.rs` — 移除错误的默认 connect 实现，改为 trait-based
- `crates/rex-hub/src/sql_api.rs` — 实现真正的分发逻辑

**接口设计**

```rust
// rex-common/src/sql.rs
impl SqlConnectorFactory {
    pub async fn connect(&self, req: ConnectRequest) -> anyhow::Result<Box<dyn SqlConnector>> {
        // 由 rex-hub 注入实际实现
        match self.db_type {
            DatabaseType::MySQL => {
                let conn = rex_mysql::MySqlConnector::connect(req).await?;
                Ok(Box::new(conn))
            }
            DatabaseType::PostgreSQL => {
                let conn = rex_postgresql::PostgresConnector::connect(req).await?;
                Ok(Box::new(conn))
            }
            DatabaseType::SQLite => {
                let conn = rex_sqlite::SqliteConnector::connect(req).await?;
                Ok(Box::new(conn))
            }
        }
    }
}
```

**提交信息**

```
fix(sql): rewrite SqlConnectorFactory to dispatch to real connectors
```

### 2 sql_api.rs 适配 + 前端验证

**功能目标**

确保 connect handler 正确调用新 factory，前端导航树和查询执行端到端可用。

**文件结构**

修改：
- `crates/rex-hub/src/sql_api.rs` — 简化 connect handler
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — 验证连接流程

**测试标准**

- 创建 MySQL 资源 → 工作区打开 SQL Tab → 自动连接 → 导航树显示 databases
- 执行 SELECT 查询 → 结果网格展示
- `bun run type-check` + `cargo check` 通过

**提交信息**

```
fix(sql): wire connect handler and verify end-to-end SQL flow
```

## 设计核对点

- [ ] SqlConnectorFactory 分发到 3 种数据库
- [ ] connect handler 正确创建 session
- [ ] 前端导航树加载 databases/tables
- [ ] 查询执行返回结果
- [ ] type-check + cargo check 通过

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
