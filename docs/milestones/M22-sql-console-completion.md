# M22: SQL 控制台完善（全局查询 + AI 助手 + 导入向导 + 内联编辑）

## Context

M0–M21 完成了从项目骨架到 SQL 控制台高级功能的全部开发。SQL 控制台已有导航树、查询编辑器、结果网格、表设计器、DDL 预览、导出向导。本里程碑对标 Navicat 的完整 SQL 管理能力，补齐全局查询、AI 助手、导入向导和结果网格内联编辑。

本里程碑版本类型：minor（新功能），版本号 0.22.0 → 0.23.0。

## 产品边界

**本阶段做：**
- 全局查询（Ctrl+Shift+Q）：跨库查询模态，多选库，提示「仅支持相同方言」
- AI 助手（Ctrl+Shift+A）：右侧抽屉，上下文（库/表/查询），快捷操作（分析慢查询/优化/生成 SQL/表关系），风险提示，复制优化 SQL
- 导入向导：拖文件到表触发导入向导，支持 CSV/JSON/SQL 解析和执行
- 结果网格内联编辑：单元格编辑（日期选择器、JSON 编辑器等），变更追踪，Apply/Discard
- 表单视图：一次一条记录，与网格视图切换

**本阶段不做：**
- 剪贴板栈（Ctrl+Shift+V 循环最近 10 项）— 优先级低，后续考虑
- 缩放（Ctrl+=/-/0）— 优先级低
- 格式化 / 大小写转换 / 注释 — 查询编辑器已有基本功能，增强留后续
- 结果网格列过滤 — 优先级低
- 导入配置 profile 复用 — 复杂度高，后续里程碑

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 全局查询模态（跨库选择 + SQL 生成） | ⬜ |
| 2 | AI 助手抽屉（上下文 + 快捷操作） | ⬜ |
| 3 | 导入向导（CSV/JSON/SQL 解析 + 执行） | ⬜ |
| 4 | 结果网格内联编辑 + Apply/Discard | ⬜ |
| 5 | 表单视图切换 | ⬜ |

## 子任务详细设计

### 1 全局查询模态（跨库选择 + SQL 生成）

**功能目标**

支持跨多个数据库执行查询，用户可选择多个库，系统生成对应的 SQL 语句。

**文件结构**

新建：
- `packages/rex-console-web/src/features/sql/GlobalQueryModal.vue` — 全局查询模态组件

修改：
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — 集成全局查询，添加 Ctrl+Shift+Q 快捷键

**接口设计**

```typescript
interface GlobalQueryModalProps {
  visible: boolean
  sessionId: string
  databases: string[]
}

interface GlobalQueryResult {
  db: string
  result: QueryResult | null
  error: string | null
}
```

**交互设计**

```
┌─ Global Query ──────────────────────────────────────┐
│ Select databases:                                   │
│   ☐ mydb                                           │
│   ☑ production                                     │
│   ☑ staging                                        │
│                                                     │
│ ⚠️ Only supports same dialect databases            │
│                                                     │
│ Query:                                              │
│ ┌─────────────────────────────────────────────────┐ │
│ │ SELECT * FROM users WHERE status = 'active'    │ │
│ └─────────────────────────────────────────────────┘ │
│                                                     │
│                      [Cancel] [Execute]             │
└─────────────────────────────────────────────────────┘
```

**后端流程**

1. 获取当前连接的所有数据库列表
2. 前端选择多个库，生成 SQL 语句（简单场景：同一 SQL 在多个库执行）
3. 对每个选中的库执行查询，返回结果列表
4. 结果网格显示所有库的查询结果（带库名标签）

**测试标准**

- Ctrl+Shift+Q 打开全局查询模态
- 选择多个库 → 执行 → 显示各库结果
- 选择不同方言库 → 显示警告提示
- type-check + build 通过

**提交信息**: `feat(sql): add global query modal for cross-database queries`

### 2 AI 助手抽屉（上下文 + 快捷操作）

**功能目标**

提供 AI 辅助功能，帮助用户分析查询、优化 SQL、理解表结构。

**文件结构**

新建：
- `packages/rex-console-web/src/features/sql/AiAssistantDrawer.vue` — AI 助手抽屉组件

修改：
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — 集成 AI 助手，添加 Ctrl+Shift+A 快捷键

**接口设计**

```typescript
interface AiAssistantDrawerProps {
  visible: boolean
  sessionId: string
  db: string
  table?: string
  query?: string
}

interface AiAction {
  id: string
  label: string
  icon: string
  description: string
}
```

**交互设计**

```
┌─ AI Assistant ─────────────────────────────────────┐
│ Context: mydb.users                                │
│                                                     │
│ Quick Actions:                                      │
│   🔍 Analyze Slow Query                            │
│   ⚡ Optimize SQL                                  │
│   📝 Generate SQL                                  │
│   🔗 Table Relationships                           │
│                                                     │
│ ┌─────────────────────────────────────────────────┐ │
│ │ [AI Response Area]                              │ │
│ │                                                 │ │
│ │ Based on the table structure...                 │ │
│ │                                                 │ │
│ │ ⚠️ Risk: This query may scan full table        │ │
│ │                                                 │ │
│ │ [Copy Optimized SQL]                            │ │
│ └─────────────────────────────────────────────────┘ │
│                                                     │
│ Input: _________________________________________  │
│                                                [→] │
└─────────────────────────────────────────────────────┘
```

**后端流程**

1. 接收用户请求（动作类型 + 上下文）
2. 调用 AI API（预留接口，可配置）
3. 返回优化建议或生成的 SQL
4. 前端展示结果，支持复制

**测试标准**

- Ctrl+Shift+A 打开 AI 助手抽屉
- 选择快捷操作 → 显示 AI 响应
- 复制优化 SQL → 粘贴到查询编辑器
- type-check + build 通过

**提交信息**: `feat(sql): add AI assistant drawer with context-aware actions`

### 3 导入向导（CSV/JSON/SQL 解析 + 执行）

**功能目标**

支持从文件导入数据到数据库表，解析 CSV/JSON/SQL 文件并生成 INSERT 语句。

**文件结构**

新建：
- `packages/rex-console-web/src/features/sql/ImportWizard.vue` — 导入向导组件

修改：
- `packages/rex-console-web/src/features/sql/SqlNavTree.vue` — 右键菜单添加「Import Data」
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — 集成导入向导

**接口设计**

```typescript
interface ImportWizardProps {
  visible: boolean
  sessionId: string
  db: string
  table: string
}

interface ImportConfig {
  format: 'csv' | 'json' | 'sql'
  file: File
  delimiter?: string
  hasHeader?: boolean
  encoding?: string
}
```

**交互设计**

```
┌─ Import Data to users ──────────────────────────────┐
│ Step 1: Select File                                 │
│   📁 Choose File or drag & drop here                │
│   Format: CSV (auto-detected)                       │
│                                                     │
│ Step 2: Preview                                     │
│   ┌─────┬─────────┬───────────────┐                │
│   │ id  │ name    │ email         │                │
│   ├─────┼─────────┼───────────────┤                │
│   │ 1   │ Alice   │ a@test.com    │                │
│   │ 2   │ Bob     │ b@test.com    │                │
│   └─────┴─────────┴───────────────┘                │
│   100 rows ready to import                          │
│                                                     │
│ Step 3: Options                                     │
│   ☑ Skip duplicates (ON DUPLICATE KEY IGNORE)      │
│   ☐ Truncate table before import                   │
│                                                     │
│                      [Cancel] [Import]              │
└─────────────────────────────────────────────────────┘
```

**后端流程**

1. 接收文件内容和配置
2. 解析文件（CSV/JSON/SQL）
3. 生成 INSERT 语句批次
4. 执行导入，返回进度和结果
5. 支持事务回滚（可选）

**测试标准**

- 拖拽 CSV 文件到表 → 打开导入向导
- 预览数据正确显示
- 执行导入 → 数据写入数据库
- type-check + build 通过

**提交信息**: `feat(sql): add import wizard for CSV/JSON/SQL files`

### 4 结果网格内联编辑 + Apply/Discard

**功能目标**

支持在结果网格中直接编辑单元格，追踪变更，提交或回滚修改。

**文件结构**

修改：
- `packages/rex-console-web/src/features/sql/SqlResultGrid.vue` — 添加内联编辑功能
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — 集成 Apply/Discard 按钮

**接口设计**

```typescript
interface EditCell {
  rowIndex: number
  colIndex: number
  oldValue: any
  newValue: any
}

interface ApplyChangesRequest {
  sessionId: string
  db: string
  table: string
  changes: EditCell[]
}
```

**交互设计**

```
┌─ Result Grid ───────────────────────────────────────┐
│ [Run] [Apply] [Discard] [Export]                    │
├─────┬─────────┬───────────────┬────────────────────┤
│ id  │ name    │ email         │ status             │
├─────┼─────────┼───────────────┼────────────────────┤
│ 1   │ Alice   │ a@test.com    │ ● active           │
│ 2   │ Bob     │ b@test.com    │ ● active           │
│ 3   │ Charlie │ c@test.com    │ ○ inactive         │
└─────┴─────────┴───────────────┴────────────────────┘
  3 rows · 0.023s · mydb
```

**编辑流程**

1. 双击单元格 → 进入编辑模式
2. 修改值 → 单元格高亮（黄色背景）
3. 点击 Apply → 生成 UPDATE 语句 → 执行
4. 点击 Discard → 恢复原始值

**后端流程**

1. 接收变更列表
2. 生成 UPDATE 语句（基于主键）
3. 批量执行更新
4. 返回执行结果

**测试标准**

- 双击单元格 → 编辑模式
- 修改值 → Apply → 数据更新
- 修改值 → Discard → 恢复原值
- type-check + build 通过

**提交信息**: `feat(sql): add inline editing with apply/discard to result grid`

### 5 表单视图切换

**功能目标**

支持在网格视图和表单视图之间切换，表单视图一次显示一条记录。

**文件结构**

新建：
- `packages/rex-console-web/src/features/sql/SqlFormView.vue` — 表单视图组件

修改：
- `packages/rex-console-web/src/features/sql/SqlResultGrid.vue` — 添加视图切换按钮
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — 集成表单视图

**接口设计**

```typescript
interface SqlFormViewProps {
  columns: ColumnInfo[]
  row: any[]
  rowIndex: number
  totalRows: number
}
```

**交互设计**

```
┌─ Form View (1 of 100) ─────────────────────────────┐
│ ◀ 1 / 100 ▶                                        │
├────────────────────────────────────────────────────┤
│ id:       [1         ]                              │
│ name:     [Alice     ]                              │
│ email:    [a@test.com]                              │
│ status:   [● active  ▼]                            │
├────────────────────────────────────────────────────┤
│ [Previous] [Next] [Save] [Discard]                 │
└────────────────────────────────────────────────────┘
```

**切换流程**

1. 结果网格工具栏添加视图切换按钮（网格/表单）
2. 点击表单视图 → 显示当前选中行的表单
3. 表单中可编辑字段 → 保存/丢弃
4. 导航按钮切换到上一条/下一条记录

**测试标准**

- 点击表单视图按钮 → 显示表单
- 导航按钮 → 切换记录
- 编辑字段 → 保存 → 数据更新
- type-check + build 通过

**提交信息**: `feat(sql): add form view toggle for single record editing`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ 架构一致（前端组件 + 后端 API）
- ✅ 不引入多用户/RBAC 概念
- ✅ 不跳阶段实现
- ✅ 实现细节不污染产品文档

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

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
