# M25: SQL 查询文件管理 + 结果导出

## Context

M24 完成了开发文档重构。M0-M23 已实现所有核心产品功能。SQL 控制台有编辑器、结果表格、库表结构树和右键菜单，但缺少两个产品文档中定义的能力：

1. **查询文件保存/加载** — 后端无查询文件 CRUD API，前端侧边栏只有"库表结构"模式，缺少"查询文件"模式
2. **结果集导出** — 结果区只有单个"结果"标签，无 CSV/JSON 导出按钮

## 产品边界

**做什么：**
- 后端查询文件 CRUD API（保存、列表、读取、删除、重命名）
- 前端侧边栏"查询文件"模式
- 前端保存/打开查询文件
- SQL 结果 CSV 和 JSON 导出

**不做什么：**
- 不实现全局查询（跨数据库）
- 不实现 AI 助手
- 不实现执行计划
- 不实现查询文件同步/协作

---

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 25.1 | 后端查询文件 CRUD API | ✅ |
| 25.2 | 前端侧边栏查询文件模式 + 保存/打开 | ⬜ |
| 25.3 | SQL 结果 CSV/JSON 导出 | ⬜ |

---

## 子任务详细设计

### 25.1 后端查询文件 CRUD API

**功能目标：**
提供查询文件（.sql）的保存、列表、读取、删除、重命名 API。文件存储在 `{data-dir}/queries/` 目录下。

**创建/修改文件：**
```text
crates/rex-hub/src/
├── sql.rs                修改：添加查询文件 handler
├── routes.rs             修改：注册查询文件路由
```

**接口设计：**
```text
GET    /api/queries                              # 列出所有查询文件
POST   /api/queries                              # 保存查询文件
GET    /api/queries/:id                          # 读取查询文件
PUT    /api/queries/:id                          # 更新查询文件
DELETE /api/queries/:id                          # 删除查询文件
PUT    /api/queries/:id/rename                   # 重命名查询文件
```

保存请求：
```json
{
  "name": "用户统计",
  "sql": "SELECT COUNT(*) FROM users",
  "database": "mydb"
}
```

查询文件存储路径：
```text
{data-dir}/queries/
├── {id}.json              文件元数据（name, database, created_at, updated_at）
└── {id}.sql               SQL 内容
```

**测试标准：**
- 保存查询文件成功
- 列表查询返回已保存的文件
- 读取查询文件返回内容
- 更新查询文件成功
- 删除查询文件成功
- 重命名查询文件成功

**提交信息：**
```
feat: add query file CRUD API
```

---

### 25.2 前端侧边栏查询文件模式 + 保存/打开

**功能目标：**
SQL 侧边栏支持"库表结构"和"查询文件"两种模式切换。工具栏"保存"按钮可将当前查询保存为文件。查询文件列表可点击打开。

**修改文件：**
```text
packages/rex-console-web/src/
├── api/sql.ts                      修改：添加查询文件 API 方法
├── features/sql/
│   ├── SqlSidebar.vue              修改：添加模式切换和查询文件列表
│   ├── SqlTopbar.vue               修改：保存按钮激活逻辑
│   ├── SqlTabs.vue                 修改：保存/打开查询文件
│   └── useSqlTabActions.ts         修改：保存/打开逻辑
├── i18n/zh.ts, en.ts              修改：添加查询文件相关 i18n 键
```

**交互设计：**
- 侧边栏顶部添加模式切换标签：`库表结构` | `查询文件`
- "查询文件"模式显示文件列表（文件名 + 日期）
- 工具栏"💾 保存"按钮：首次保存弹出命名对话框，已保存的直接更新
- 工具栏"📂 打开"按钮：弹出文件选择列表
- 保存后标签标题更新为文件名
- 查询文件右键菜单：重命名、删除

**测试标准：**
- 模式切换正常
- 保存查询文件后列表更新
- 打开查询文件加载到编辑器
- 重命名/删除操作正常

**提交信息：**
```
feat: add SQL query file save/load with sidebar mode
```

---

### 25.3 SQL 结果 CSV/JSON 导出

**功能目标：**
SQL 结果区添加 CSV 和 JSON 导出按钮，支持将完整结果集导出为文件下载。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/sql/
│   ├── SqlResults.vue              修改：添加导出按钮
│   └── result-export.ts            新增：CSV/JSON 导出逻辑
├── i18n/zh.ts, en.ts              修改：添加导出相关 i18n 键
```

**交互设计：**
- 结果区底部 footer 右侧添加导出按钮组：`📋 复制` | `⬇ CSV` | `⬇ JSON`
- 点击 CSV：生成 CSV 文件并触发浏览器下载
- 点击 JSON：生成 JSON 文件并触发浏览器下载
- 导出范围：当前结果集全部行

**导出逻辑：**
```ts
// result-export.ts
export function exportCsv(columns: SqlColumn[], rows: unknown[][]): void {
  const header = columns.map(c => c.name).join(',')
  const body = rows.map(row =>
    row.map(cell => cell === null ? '' : `"${String(cell).replace(/"/g, '""')}"`).join(',')
  ).join('\n')
  downloadFile(`${header}\n${body}`, 'query-result.csv', 'text/csv')
}

export function exportJson(columns: SqlColumn[], rows: unknown[][]): void {
  const data = rows.map(row => {
    const obj: Record<string, unknown> = {}
    columns.forEach((c, i) => { obj[c.name] = row[i] })
    return obj
  })
  downloadFile(JSON.stringify(data, null, 2), 'query-result.json', 'application/json')
}
```

**测试标准：**
- CSV 导出包含正确的表头和数据
- JSON 导出格式正确
- 空结果导出不报错
- 特殊字符（逗号、引号、换行）正确转义

**提交信息：**
```
feat: add SQL result CSV and JSON export
```

---

## 设计核对点

- [ ] 查询文件存储路径与 PRODUCT.md §3.7 一致
- [ ] 侧边栏模式切换与 PRODUCT.md §3.7 一致
- [ ] 结果导出与 PRODUCT.md §3.7 一致
- [ ] 不引入全局查询或 AI 助手

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
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
