# M25 步骤7：设计再确认报告

## 确认范围

M25 三个子任务的已实现代码 vs 里程碑文档设计。

## 逐项核对

### 25.1 后端查询文件 CRUD API

| 设计项 | 文档 | 实际实现 | 结果 |
|--------|------|----------|------|
| GET /api/queries | 列出所有查询文件 | `list_queries` handler | ✅ |
| POST /api/queries | 保存查询文件 | `save_query` handler | ✅ |
| GET /api/queries/:id | 读取查询文件 | `get_query` handler | ✅ |
| PUT /api/queries/:id | 更新查询文件 | `update_query` handler | ✅ |
| DELETE /api/queries/:id | 删除查询文件 | `delete_query` handler | ✅ |
| PUT /api/queries/:id/rename | 重命名查询文件 | `rename_query` handler | ✅ |
| 存储路径 `{data-dir}/queries/` | `{data-dir}/queries/{resource_id}/` | 资源隔离改进 | ✅ |

### 25.2 前端侧边栏查询文件模式 + 保存/打开

| 设计项 | 文档 | 实际实现 | 结果 |
|--------|------|----------|------|
| 模式切换标签 | 库表结构 / 查询文件 | `sql-sidebar-tabs` | ✅ |
| 查询文件列表 | 文件名 + 日期 | `tree-query-item` | ✅ |
| 保存按钮 | 首次保存弹出命名对话框 | `handleTabSave` + `prompt` | ✅ |
| 已保存直接更新 | 更新 SQL 内容 | `updateQuery` 调用 | ✅ |
| 侧边栏刷新 | 保存后列表更新 | `loadQueries()` | ✅ |
| 右键菜单 | 重命名、删除 | `handleQueryContextMenu` | ✅ |

### 25.3 SQL 结果 CSV/JSON 导出

| 设计项 | 文档 | 实际实现 | 结果 |
|--------|------|----------|------|
| CSV 导出按钮 | ⬇ CSV | `SqlResults.vue` 按钮 | ✅ |
| JSON 导出按钮 | ⬇ JSON | `SqlResults.vue` 按钮 | ✅ |
| CSV 转义 | 双引号转义 | `csvEscape` 函数 | ✅ |
| 文件下载 | 浏览器下载 | `downloadFile` 函数 | ✅ |

## 产品边界检查

- 不实现全局查询（跨数据库） — ✅ 未引入
- 不实现 AI 助手 — ✅ 未引入
- 不实现执行计划 — ✅ 未引入
- 不实现查询文件同步/协作 — ✅ 未引入
- 单用户、自托管 — ✅ 未引入多用户概念

## 设计核对点

- 查询文件存储路径与 PRODUCT.md §3.7 一致 — ✅
- 侧边栏模式切换与 PRODUCT.md §3.7 一致 — ✅
- 结果导出与 PRODUCT.md §3.7 一致 — ✅
- 不引入全局查询或 AI 助手 — ✅

## 结论

✅ 通过。所有子任务实现与里程碑文档一致，产品边界未被污染。
