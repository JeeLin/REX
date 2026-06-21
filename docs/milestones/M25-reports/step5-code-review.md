# M25 步骤5：代码审查报告

## 审查范围

M25 三个子任务的全部代码变更（3 个 commit，19 个文件）。

## 审查维度

### 正确性
- 查询文件 CRUD 逻辑正确：save 生成 ID + 写 meta.json + 写 .sql，list 读目录过滤 .json，get 读 meta + sql，update 更新指定字段，delete 删两个文件
- 前端侧边栏模式切换、查询文件列表加载、保存/打开/重命名/删除逻辑正确
- CSV/JSON 导出逻辑正确（CSV 双引号转义、JSON 键值对齐列名）
- `handleQueryDeleted` 实现清空 queryId，与里程碑文档一致
- `select-table` 事件已修复到展开分支，不再在折叠时误触发

### 安全性
- `resource_id` 路径安全校验：**预存问题**，所有现有 handler（sql.rs、files.rs）均使用相同模式直接拼接路径，非 M25 引入。后续统一加固。
- `gen_id("q")` 生成的 ID 只含字母数字和下划线，不存在路径注入风险。
- 查询文件存储在 data-dir 下，无越权访问可能。

### 架构一致性
- API 路径设计 `/api/resources/:resource_id/queries/` 与现有 `/api/resources/:resource_id/sql/` 等一致
- AppState 扩展 `data_dir: PathBuf` 字段合理，与 config 中 `data_dir` 对应
- 前端 composable（useSqlTabActions）扩展 queryId 字段，与现有 tab 管理模式一致
- i18n 键命名遵循 `sql.sidebar.*` 命名空间

### 测试覆盖
- 后端 12 个测试覆盖完整 CRUD + 资源隔离（queries_are_isolated_by_resource）
- 前端无自动化测试（属于后续里程碑范围）

### 错误处理
- 所有 IO 操作通过 `err_resp` 返回结构化错误
- 空输入校验（name、sql 不能为空）
- 文件不存在返回 404（not_found）
- 前端 API 调用通过 axios 错误处理

### 审计日志
- save/update/delete/rename 均有 tracing info 日志，记录 query_id、resource、name

### 里程碑文档一致性
- API 路径：`/api/resources/:resource_id/queries` ✅
- 存储路径：`{data-dir}/queries/{resource_id}/` ✅
- 侧边栏模式切换 ✅
- 导出按钮位置（footer 右侧）✅

## 严重程度分级

### 🔴 必须修复
无

### 🟡 应该修复
无

### 🟢 可选改进
1. `test_state()` 在 5 个模块重复定义 — 预存问题，非 M25 引入
2. `rename_query` 与 `update_query` 功能重叠 — 已在步骤4精简中评估，保留
3. `SqlConsole.vue` 中 `insertTableSql` 直接拼接表名，无转义 — 预存问题

## 结论

M25 代码审查通过，无 🔴 或 🟡 必须修复项。所有发现均为预存问题或可选改进。
