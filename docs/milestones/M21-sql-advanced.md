# M21: SQL 控制台高级功能（表设计器 + DDL 预览 + 导出向导）

## Context

M0–M20 完成了基础设施、工作空间和核心功能。SQL 控制台已有导航树、查询编辑器、结果网格基本视图。本里程碑对标 Navicat 的高级 Schema 管理和数据操作能力，补齐表设计器、DDL 预览和数据导出。

本里程碑版本类型：minor（新功能），版本号 0.21.0 → 0.22.0。

## 产品边界

**本阶段做：**
- 表设计器：对象区多 Tab（字段/索引/外键/DDL 预览），可视化编辑表结构
- DDL 预览抽屉：选中表/视图 → 底部抽屉显示 CREATE TABLE DDL
- 结果网格增强：列头排序、导出（CSV/JSON/SQL）、行数/执行时间状态栏
- 导出向导：查询结果导出为 CSV/JSON/SQL 文件

**本阶段不做：**
- 结果网格内联编辑 + Apply/Discard（需后端变更，后续里程碑）
- 全局查询（Ctrl+Shift+Q）
- AI 助手（Ctrl+Shift+A）
- 导入向导（拖文件导入）
- 索引/外键可视化编辑（DDL 预览为只读，后续里程碑增加编辑）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 表设计器（对象区多 Tab + DDL 预览） | ✅ |
| 2 | DDL 抽屉 + 结果网格增强 + 导出向导 | ✅ |

## 子任务详细设计

### 1 表设计器（对象区多 Tab + DDL 预览）

**功能目标**

在 SQL 控制台对象区打开表设计器 Tab，可视化编辑表结构并实时预览 DDL。

**文件结构**

新建：
- `packages/rex-console-web/src/features/sql/TableDesigner.vue` — 表设计器主组件
- `packages/rex-console-web/src/features/sql/ColumnEditor.vue` — 字段编辑器子组件

修改：
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — 导航树双击表/字段 → 打开表设计器 Tab
- `packages/rex-console-web/src/features/sql/SqlNavTree.vue` — 右键菜单增加「Design Table」

**交互设计**

```
┌─ SQL Console ────────────────────────────────────────┐
│ [导航树] │ [查询1] [users 表设计]                      │
│          │                                            │
│ 📂 db    │ ┌─ Table Designer: users ──────────────┐  │
│  📋 users│ │ [Columns] [Indexes] [FK] [DDL]       │  │
│  📋 posts│ ├──────────────────────────────────────┤  │
│          │ │ #  Name     Type        PK  NN  Def  │  │
│          │ │ 1  id       INT         ✓   ✓        │  │
│          │ │ 2  name     VARCHAR(50)     ✓         │  │
│          │ │ 3  email    VARCHAR(100)    ✓         │  │
│          │ │ 4  created  DATETIME            NOW() │  │
│          │ │                                      │  │
│          │ │ [Add Column]           [Apply] [DDL] │  │
│          │ └──────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**数据模型**

```typescript
interface DesignerColumn {
  name: string
  type: string
  length?: number
  unsigned?: boolean
  defaultValue?: string
  autoIncrement?: boolean
  charset?: string
  comment?: string
  primaryKey: boolean
  notNull: boolean
}

interface DesignerIndex {
  name: string
  columns: string[]
  unique: boolean
  type: 'BTREE' | 'HASH' | 'FULLTEXT'
}

interface DesignerForeignKey {
  name: string
  columns: string[]
  refTable: string
  refColumns: string[]
  onDelete: 'CASCADE' | 'SET NULL' | 'RESTRICT' | 'NO ACTION'
  onUpdate: 'CASCADE' | 'SET NULL' | 'RESTRICT' | 'NO ACTION'
}
```

**功能点**

- **Columns Tab**：表格编辑字段（名/类型/PK/NN/Default/Auto Increment），可拖拽重排，Add/Delete 按钮
- **Indexes Tab**：列表显示索引，Add/Index Name/Columns/Unique/Type
- **FK Tab**：列表显示外键，Add/Name/Columns/Ref Table/Ref Columns/On Delete/On Update
- **DDL Tab**：只读 Monaco 编辑器，实时生成 `CREATE TABLE` DDL 预览
- 底部工具栏：Apply（执行 ALTER TABLE）/ Discard（丢弃修改）/ DDL（切换到 DDL Tab）

**后端流程**

- 前端获取表结构：调用现有 `GET /api/sql/columns?db=&table=` 接口
- 前端获取索引：新增 `GET /api/sql/indexes?db=&table=` 接口
- 前端获取外键：新增 `GET /api/sql/foreign_keys?db=&table=` 接口
- Apply：生成 `ALTER TABLE ... ADD COLUMN / MODIFY / DROP COLUMN` 语句，调用现有 `POST /api/sql/execute` 接口

**测试标准**

- 导航树双击表 → 打开表设计器 Tab → 显示字段列表
- 切换到 DDL Tab → 显示 CREATE TABLE 语句
- Add Column → 填写字段 → Apply → 刷新导航树验证新字段
- type-check + build 通过

**提交信息**

```
feat(sql): add table designer with column editor and DDL preview
```

### 2 DDL 抽屉 + 结果网格增强 + 导出向导

**功能目标**

增强 SQL 控制台的数据查看和导出能力。

**文件结构**

新建：
- `packages/rex-console-web/src/features/sql/DdlDrawer.vue` — DDL 预览抽屉
- `packages/rex-console-web/src/features/sql/ExportWizard.vue` — 导出向导组件

修改：
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — 集成 DDL 抽屉和导出按钮
- `packages/rex-console-web/src/features/sql/SqlResultGrid.vue` — 增加列头排序、导出按钮、状态栏

**交互设计**

**DDL 抽屉（底部可折叠）：**
```
┌─ DDL Preview: users ──────────────────────────────┐
│ CREATE TABLE `users` (                             │
│   `id` INT NOT NULL AUTO_INCREMENT,                │
│   `name` VARCHAR(50) NOT NULL,                     │
│   `email` VARCHAR(100) NOT NULL,                   │
│   PRIMARY KEY (`id`)                               │
│ ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;           │
│                                        [Copy] [Max] │
└────────────────────────────────────────────────────┘
```

**结果网格增强：**
- 列头点击排序（升序/降序/取消）
- 底部状态栏：`125 rows · 0.023s · mydb`
- 工具栏增加「Export」按钮

**导出向导（Modal）：**
```
┌─ Export Results ─────────────────────┐
│ Format: (●) CSV  ( ) JSON  ( ) SQL  │
│ File: [users_export]                 │
│ Options:                             │
│   ☐ Include headers (CSV)           │
│   ☐ Pretty print (JSON)             │
│   ☐ Include table name (SQL)        │
│                      [Cancel] [Export]│
└──────────────────────────────────────┘
```

**功能点**

- **DDL 抽屉**：导航树右键表 → 「View DDL」→ 底部抽屉显示 DDL，可复制、可最大化
- **结果网格排序**：点击列头排序，再次点击切换升序/降序，第三次取消排序
- **结果网格状态栏**：行数 + 执行时间 + 当前数据库
- **导出向导**：选择格式（CSV/JSON/SQL）→ 配置选项 → 下载文件
- 导出使用前端生成（已有查询结果在内存中），不需要后端新接口

**后端流程**

- DDL 抽屉：新增 `GET /api/sql/ddl?db=&table=` 接口，返回 DDL 字符串
- 导出：前端根据查询结果直接生成文件下载，无需后端

**测试标准**

- 导航树右键表 → View DDL → 底部抽屉显示 CREATE TABLE
- 点击结果网格列头 → 数据排序
- 点击 Export → 选择 CSV → 下载文件内容正确
- 点击 Export → 选择 JSON → 下载文件内容正确
- type-check + build 通过

**提交信息**

```
feat(sql): add DDL drawer, result grid sorting, and export wizard
```

## 设计核对点

- [ ] 导航树双击表 → 打开表设计器 Tab
- [ ] 表设计器 Columns Tab 可编辑字段
- [ ] 表设计器 DDL Tab 实时预览 CREATE TABLE
- [ ] 导航树右键 → View DDL → 底部抽屉
- [ ] 结果网格列头排序
- [ ] 结果网格状态栏（行数/时间/库名）
- [ ] 导出向导（CSV/JSON/SQL）
- [ ] type-check + build 通过

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
