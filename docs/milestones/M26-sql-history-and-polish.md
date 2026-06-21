# M26: SQL 历史记录 + 编辑器精化

## Context

M25 完成了 SQL 查询文件管理（保存/加载/重命名/删除）和结果导出（CSV/JSON）。SQL 控制台核心功能基本完整，但有几处 PRODUCT.md §3.7 中定义但未实现的功能：

1. **SQL 历史记录** — 编辑器右键菜单已有"历史记录"入口，但 `@show-history` 事件绑定到空函数 `() => {}`。用户无法查看和重用之前执行过的 SQL。
2. **编辑器快捷键** — 工具栏提示 `Ctrl+S 保存`，但实际未绑定。
3. **结果区复制按钮** — footer 只有 CSV/JSON 导出，缺少📋复制按钮。

## 产品边界

**做什么：**
- SQL 执行历史记录（后端存储 + 前端列表展示 + 点击重用）
- Ctrl+S 保存快捷键
- 结果区📋复制按钮

**不做什么：**
- 不实现执行计划（需要数据库引擎支持，后续里程碑）
- 不实现消息标签（当前执行错误已在 Toast 中展示）
- 不实现全局查询（跨数据库）
- 不实现 AI 助手

---

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 26.1 | 后端 SQL 历史记录 API | ✅ |
| 26.2 | 前端 SQL 历史记录面板 | ✅ |
| 26.3 | 编辑器快捷键 + 结果复制按钮 | ✅ |

---

## 子任务详细设计

### 26.1 后端 SQL 历史记录 API

**功能目标：**
存储用户执行过的 SQL 语句，支持列出历史和清理。每个资源独立存储。

**文件结构：**
```text
crates/rex-hub/src/
├── sql.rs                修改：添加历史记录 handler
├── routes.rs             修改：注册历史记录路由
```

**数据模型：**
```text
{data-dir}/sql-history/
├── {resource_id}.json    该资源的执行历史（JSON 数组）
```

每条记录：
```json
{
  "id": "uuid",
  "sql": "SELECT * FROM users LIMIT 100",
  "database": "mydb",
  "executed_at": "2026-06-21T10:30:00Z",
  "elapsed_ms": 12,
  "row_count": 42
}
```

**接口设计：**
```text
GET    /api/resources/:resource_id/sql/history          # 列出历史（最近 100 条）
POST   /api/resources/:resource_id/sql/history          # 记录一条执行（内部调用）
DELETE /api/resources/:resource_id/sql/history          # 清空历史
```

列出历史响应：
```json
{
  "data": [
    {
      "id": "abc-123",
      "sql": "SELECT * FROM users",
      "database": "mydb",
      "executed_at": "2026-06-21T10:30:00Z",
      "elapsed_ms": 12,
      "row_count": 42
    }
  ]
}
```

**后端流程：**
1. `execute_sql` handler 执行成功后，自动写入历史记录（异步，不阻塞响应）
2. 历史记录最多保留 100 条，超过时删除最早的
3. 文件存储路径使用 `queries_dir()` 同样的路径安全校验

**路径安全：**
复用 `queries_dir()` 的 `resource_id` 校验逻辑，新建 `sql_history_dir()` 辅助函数：
```rust
fn sql_history_dir(state: &AppState, resource_id: &str) -> Result<PathBuf, (StatusCode, Json<ErrorResponse>)> {
    if resource_id.contains('/') || resource_id.contains('\\') || resource_id.contains("..") {
        return Err(bad_request("resource_id 包含非法字符"));
    }
    Ok(state.data_dir.join("sql-history").join(resource_id))
}
```

**测试标准：**
- 执行 SQL 后自动记录历史
- 列出历史返回正确数据
- 清空历史成功
- resource_id 路径安全校验
- 超过 100 条时自动清理

**提交信息：**
```
feat: add SQL execution history API
```

---

### 26.2 前端 SQL 历史记录面板

**功能目标：**
点击编辑器右键菜单"历史记录"或工具栏按钮，打开历史记录面板。用户可点击历史项将其加载到编辑器。

**修改文件：**
```text
packages/rex-console-web/src/
├── api/sql.ts                      修改：添加历史记录 API 方法
├── features/sql/
│   ├── SqlHistoryPanel.vue         新增：历史记录面板组件
│   └── SqlTopbar.vue               修改：添加历史记录按钮
├── pages/SqlConsole.vue            修改：集成历史记录面板
├── i18n/zh.ts, en.ts              修改：添加历史记录相关 i18n 键
```

**交互设计：**
- 编辑器右键菜单"历史记录"→ 打开底部抽屉面板
- 工具栏添加"📜 历史"按钮
- 面板内容：
  - 顶部：搜索框 + 清空按钮
  - 列表：每项显示 SQL 片段（截断 80 字符）+ 数据库名 + 执行时间 + 耗时 + 行数
  - 点击历史项：将其 SQL 加载到当前标签的编辑器
  - 空态：「暂无执行历史」
- 面板可折叠/展开，不遮挡编辑器和结果区

**组件设计：**
```vue
<!-- SqlHistoryPanel.vue -->
<template>
  <div class="history-panel" v-if="visible">
    <div class="history-header">
      <span>{{ t('sql.history.title') }}</span>
      <div class="history-actions">
        <input v-model="search" class="history-search" :placeholder="t('sql.history.search')" />
        <button class="btn btn-ghost btn-xs" @click="handleClear">{{ t('sql.history.clear') }}</button>
        <button class="btn btn-ghost btn-xs" @click="emit('close')">✕</button>
      </div>
    </div>
    <div class="history-list">
      <div v-for="item in filtered" :key="item.id" class="history-item" @click="emit('select', item)">
        <code class="history-sql">{{ truncate(item.sql, 80) }}</code>
        <div class="history-meta">
          <span>{{ item.database }}</span>
          <span>{{ item.elapsed_ms }}ms</span>
          <span>{{ item.row_count }} rows</span>
        </div>
      </div>
      <div v-if="filtered.length === 0" class="history-empty">
        {{ t('sql.history.empty') }}
      </div>
    </div>
  </div>
</template>
```

**测试标准：**
- 右键菜单点击历史记录打开面板
- 工具栏按钮打开面板
- 搜索过滤正常
- 点击历史项加载到编辑器
- 清空历史正常

**提交信息：**
```
feat: add SQL history panel with search
```

---

### 26.3 编辑器快捷键 + 结果复制按钮

**功能目标：**
1. `Ctrl+S` 保存快捷键
2. 结果区 footer 添加📋复制按钮（复制全部结果为 TSV）

**修改文件：**
```text
packages/rex-console-web/src/
├── features/sql/
│   ├── SqlEditor.vue               修改：添加 Ctrl+S 快捷键
│   └── SqlResults.vue              修改：添加复制按钮
├── pages/SqlConsole.vue            修改：处理 Ctrl+S 事件
├── i18n/zh.ts, en.ts              修改：添加相关 i18n 键
```

**交互设计：**

Ctrl+S：
- 在 SqlEditor.vue 的 `handleKeydown` 中添加 Ctrl+S 监听
- 触发 `save` 事件（已存在）
- SqlConsole.vue 中已绑定 `@save="handleToolbarSave"`

📋 复制按钮：
- 在 SqlResults.vue footer 的 `results-footer-actions` 区域添加
- 位于 CSV/JSON 按钮左侧
- 点击后复制所有结果行为 TSV 格式（Tab 分隔），可直接粘贴到 Excel
- 复制成功后 Toast 提示

**测试标准：**
- Ctrl+S 触发保存
- 复制按钮复制完整结果为 TSV
- 空结果时复制按钮不显示

**提交信息：**
```
feat: add Ctrl+S save shortcut and result copy button
```

---

## 设计核对点

- [ ] SQL 历史记录与 PRODUCT.md §3.7 一致
- [ ] 历史记录存储路径安全（无路径逃逸）
- [ ] Ctrl+S 快捷键与工具栏提示一致
- [ ] 不引入执行计划或 AI 助手
- [ ] 不引入多用户概念

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
