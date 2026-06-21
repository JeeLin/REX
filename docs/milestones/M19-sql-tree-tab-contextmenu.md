# M19: SQL 库表结构树右键菜单 + 查询标签右键菜单

## Context

M18 完成了终端内置 SFTP、SQL 编辑器/结果右键菜单、终端右键菜单。SQL 控制台（§3.7）的库表结构树（SqlSidebar）已有基本的表→列浏览功能，但缺少右键上下文菜单。查询标签栏（SqlTabs）也缺少右键菜单。

PRODUCT.md §3.7 定义了 5 类节点的右键菜单和查询标签右键菜单，在 M18 设计审查中被标注为后续里程碑。

## 产品边界

**做什么：**
- 库表结构树右键菜单：表节点、列节点、数据库节点、空白区域
- 查询标签右键菜单：关闭、保存、重命名、复制 SQL、执行 SQL
- SqlSidebar 增加「查询文件」标签页切换

**不做什么（标注与 PRODUCT.md §3.7 的差异）：**
- 不实现视图节点右键菜单 — 后端未支持视图列表 API
- 不实现表节点「导出表数据」— 需要后端流式导出 API
- 不实现表节点「刷新」— 当前无实时刷新需求
- 不实现列节点「查看列约束」— 需要后端约束查询 API
- 不实现数据库节点「新建表」— 需要 DDL 执行 API
- 不实现查询标签「关闭已保存的」— 需要保存状态追踪
- 不实现查询标签「另存为…」— 需要查询文件保存流程
- 不实现全局查询 modal（需要多数据库执行能力）
- 不实现 AI 助手面板（需要 LLM 集成）
- 不实现结果导出 CSV/JSON（需要后端流式导出 API）
- 不实现结果分页（需要后端分页 API）

---

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 19.1 | 库表结构树右键菜单 | ✅ |
| 19.2 | 查询标签右键菜单 | ✅ |

---

## 子任务详细设计

### 19.1 库表结构树右键菜单

**功能目标：**
为 SQL 侧边栏的库表结构树添加右键上下文菜单，支持表节点、列节点、数据库节点和空白区域的快捷操作。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/sql/
│   └── SqlSidebar.vue            修改：添加右键菜单
├── i18n/zh.ts, en.ts            修改：添加菜单 i18n
```

**交互设计（参考 PRODUCT.md §3.7）：**

表节点右键菜单：
```text
查看表结构
查看行数统计
─── 分隔线 ───
复制表名
SELECT * 查询
─── 分隔线 ───
刷新
```

列节点右键菜单：
```text
复制列名
复制列类型
```

数据库节点右键菜单（右键 sidebar header 区域）：
```text
刷新
复制数据库名
```

空白区域右键菜单（右键 tree 空白处）：
```text
全部展开
全部折叠
─── 分隔线 ───
刷新结构
```

**SqlSidebar.vue 修改：**
- 表节点 `tree-group-header` 添加 `@contextmenu.prevent`
- 列节点 `tree-col-item` 添加 `@contextmenu.prevent`
- sidebar header 添加 `@contextmenu.prevent`
- tree 容器添加 `@contextmenu.prevent`（空白区域）
- 各节点右键时调用 `useContextMenu().show()` 展示对应菜单
- 菜单项 action：
  - 查看表结构：展开该表节点以查看列信息（若已展开则折叠）
  - 复制表名/列名/列类型/数据库名：`navigator.clipboard.writeText()`
  - SELECT * 查询：emit `selectTable` 事件，父组件插入 SQL
  - 全部展开/折叠：操作 `expanded` Set
  - 刷新：emit `refresh` 事件

**测试标准：**
- 表节点右键弹出菜单
- 列节点右键弹出菜单
- 数据库节点右键弹出菜单
- 空白区域右键弹出菜单
- 复制操作写入剪贴板
- SELECT * 查询插入到编辑器
- 全部展开/折叠正确工作

**提交信息：**
```
feat: add context menus for SQL schema tree nodes
```

---

### 19.2 查询标签右键菜单

**功能目标：**
为 SQL 查询标签栏添加右键上下文菜单，支持标签关闭、保存、重命名等操作。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/sql/
│   ├── SqlTabs.vue               修改：添加右键菜单
│   └── useSqlTabActions.ts       修改：增加标签操作方法
├── features/workspace/panels/
│   └── WorkspaceSql.vue          修改：处理新事件
├── pages/
│   └── SqlConsole.vue            修改：处理新事件
├── i18n/zh.ts, en.ts            修改：添加菜单 i18n
```

**交互设计（参考 PRODUCT.md §3.7）：**

查询标签右键菜单：
```text
关闭
关闭其他
─── 分隔线 ───
保存
重命名
─── 分隔线 ───
复制 SQL
执行 SQL
```

**SqlTabs.vue 修改：**
- 标签元素添加 `@contextmenu.prevent` 事件处理
- 右键时调用 `useContextMenu().show()` 展示菜单
- 菜单项 action：
  - 关闭：emit `close` 事件
  - 关闭其他：emit `closeOthers` 事件
  - 保存：emit `save` 事件（带 tabId）
  - 重命名：emit `rename` 事件（带 tabId）
  - 复制 SQL：emit `copySql` 事件（带 tabId）
  - 执行 SQL：emit `executeSql` 事件（带 tabId）

**useSqlTabActions.ts 修改：**
- 增加 `closeOthers(id)` 方法
- 增加 `renameTab(id, newTitle)` 方法
- 增加 `getTabSql(id)` 方法

**测试标准：**
- 标签右键弹出菜单
- 关闭其他只保留当前标签
- 重命名修改标签标题
- 复制 SQL 写入剪贴板
- 执行 SQL 触发查询

**提交信息：**
```
feat: add context menu for SQL query tabs
```

---

### i18n 键清单

**库表结构树右键菜单（`sql.tree.ctx.*`）：**
| 键 | 中文 | English |
|---|------|---------|
| `sql.tree.ctx.viewStructure` | 查看表结构 | View Structure |
| `sql.tree.ctx.viewRowCount` | 查看行数统计 | View Row Count |
| `sql.tree.ctx.copyTableName` | 复制表名 | Copy Table Name |
| `sql.tree.ctx.selectStar` | SELECT * 查询 | SELECT * Query |
| `sql.tree.ctx.copyColumnName` | 复制列名 | Copy Column Name |
| `sql.tree.ctx.copyColumnType` | 复制列类型 | Copy Column Type |
| `sql.tree.ctx.copyDbName` | 复制数据库名 | Copy Database Name |
| `sql.tree.ctx.refresh` | 刷新 | Refresh |
| `sql.tree.ctx.expandAll` | 全部展开 | Expand All |
| `sql.tree.ctx.collapseAll` | 全部折叠 | Collapse All |
| `sql.tree.ctx.refreshStructure` | 刷新结构 | Refresh Structure |

**查询标签右键菜单（`sql.tab.ctx.*`）：**
| 键 | 中文 | English |
|---|------|---------|
| `sql.tab.ctx.close` | 关闭 | Close |
| `sql.tab.ctx.closeOthers` | 关闭其他 | Close Others |
| `sql.tab.ctx.save` | 保存 | Save |
| `sql.tab.ctx.rename` | 重命名 | Rename |
| `sql.tab.ctx.copySql` | 复制 SQL | Copy SQL |
| `sql.tab.ctx.executeSql` | 执行 SQL | Execute SQL |

---

## 设计核对点

- [x] 库表结构树右键菜单与 PRODUCT.md §3.7 一致（缺少项均为后端 API 未实现，已标注排除）
- [x] 查询标签右键菜单与 PRODUCT.md §3.7 一致（缺少项均为后端 API 未实现，已标注排除）
- [x] i18n 覆盖所有新增文字
- [x] 复制操作使用 navigator.clipboard API
- [x] 右键菜单使用现有 useContextMenu composable
- [x] 视图节点菜单标注为后续实现

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
