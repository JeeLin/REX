# M18: 终端内置 SFTP + SQL/终端右键菜单

## Context

M17 完成了工作区标签右键菜单增强（移动到面板子菜单 + 全部断开）。工作区外壳（M13）、面板集成（M14）、Agent 部署指南（M15）、资源创建向导（M16）均已完成。

PRODUCT.md §3.6 定义了 SSH 终端内置 SFTP 面板（split view），§3.7 定义了 SQL 编辑器/结果/表结构右键菜单，§3.6 定义了终端右键菜单。这些功能在前序里程碑中被明确排除，本里程碑实现。

## 产品边界

**做什么：**
- SSH 终端工具栏增加「📁 SFTP」按钮，点击后在右侧打开内嵌 SFTP 面板（split view）
- SFTP 文件拖拽到终端自动粘贴路径
- SQL 编辑器右键菜单（执行选中/全部 SQL、格式化、大小写转换、注释切换、插入模板、保存）
- SQL 结果表格右键菜单（复制行/单元格/整列、排序、生成 UPDATE/DELETE、导出）
- 终端右键菜单（复制、粘贴、全选、清屏、重连、打开 SFTP、新建连接、复制地址、断开连接）

**不做什么：**
- 不实现跨连接文件传输的「发送到…」（需要后端 TransferCoordinator，属于后续里程碑）
- 不实现 SQL AI 助手面板（需要 LLM 集成）
- 不实现全局查询 modal（需要多数据库执行能力）
- 不实现工作空间面板间拖拽（交互复杂度高，单独里程碑）
- 不实现库表结构树右键菜单（5 类节点菜单，复杂度高，单独里程碑）
- 不实现查询标签右键菜单（后续增强）
- 不实现终端工具栏右键菜单（后续增强）
- 不实现移动端浮动工具栏（已有独立实现，后续优化）

---

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 18.1 | SSH 终端内置 SFTP 面板 | ✅ |
| 18.2 | SQL 编辑器/结果右键菜单 | ✅ |
| 18.3 | 终端右键菜单 | ✅ |

---

## 子任务详细设计

### 18.1 SSH 终端内置 SFTP 面板

**功能目标：**
SSH 终端工具栏增加「📁 SFTP」按钮，点击后在终端右侧打开内嵌 SFTP 文件管理面板（split view），共享同一个 SSH 连接的上下文。SFTP 面板可独立浏览远程文件系统。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/workspace/panels/
│   └── WorkspaceTerminal.vue    修改：增加 SFTP 按钮和 split view
├── features/files/
│   └── FileList.vue             复用：SFTP 面板内的文件列表
├── i18n/zh.ts, en.ts            修改：添加 SFTP 相关 i18n
```

**交互设计（参考 PRODUCT.md §3.6）：**

```text
┌─────────────────────────────────────────────┐
│ ● Web服务器  │  清屏  粘贴  📁SFTP  断开    │  ← 工具栏
├──────────────────────────┬──────────────────┤
│                          │  📁 /home/pi     │  ← SFTP 面板
│   $ ls -la               │  ─────────────── │
│   total 32               │  📁 ..           │
│   drwxr-xr-x 4 pi pi    │  📁 Documents    │
│   -rw-r--r-- 1 pi pi    │  📄 file.tar.gz  │
│   ...                    │  📄 script.sh    │
│                          │                  │
│   $ _                    │                  │
├──────────────────────────┴──────────────────┤
│ SSH · UTF-8              已连接              │  ← 状态栏
└─────────────────────────────────────────────┘
```

**SFTP 面板行为：**
- 点击「📁 SFTP」按钮切换显示/隐藏（toggle）
- SFTP 面板在终端右侧，宽度约 280px，可拖拽调整
- SFTP 面板显示面包屑路径 + 文件列表
- 双击文件夹进入，双击 `..` 返回上级
- 文件列表显示名称、大小
- SFTP 面板复用现有 `FileList.vue` 组件

**SFTP 文件拖拽到终端：**
- 从 SFTP 文件列表拖拽文件到终端区域，自动将文件路径粘贴到终端输入
- 拖拽时终端区域显示虚线边框高亮（drop zone）
- 松开后将文件的远程路径（如 `/home/pi/file.tar.gz`）作为文本发送到终端

**WorkspaceTerminal.vue 修改：**
- 新增 `showSftp` ref 控制 SFTP 面板显隐
- 新增 `sftpWidth` ref 控制 SFTP 面板宽度（默认 280px）
- 工具栏增加「📁 SFTP」toggle 按钮
- 主体区域分为终端区 + SFTP 面板（v-if 控制）
- SFTP 面板使用内嵌 FileList，通过 WebSocket 或 REST API 获取文件列表
- 拖拽分隔条调整 SFTP 面板宽度

**测试标准：**
- 工具栏显示 SFTP 按钮
- 点击按钮 toggle SFTP 面板
- SFTP 面板显示文件列表
- 双击文件夹可导航
- 双击 `..` 可返回上级
- 关闭 SFTP 面板后终端恢复全宽

**提交信息：**
```
feat: add embedded SFTP panel in SSH terminal workspace
```

---

### 18.2 SQL 编辑器/结果右键菜单

**功能目标：**
为 SQL 编辑器和结果表格添加右键上下文菜单，提供常用操作快捷入口。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/sql/
│   ├── SqlEditor.vue            修改：添加右键菜单
│   └── SqlResults.vue           修改：添加右键菜单
├── i18n/zh.ts, en.ts            修改：添加 SQL 菜单 i18n
```

**交互设计（参考 PRODUCT.md §3.7）：**

SQL 编辑器右键菜单：
```text
执行选中 SQL
执行全部 SQL
─── 分隔线 ───
剪切
复制
粘贴
─── 分隔线 ───
格式化 SQL
大小写转换 ▸  全部大写 / 全部小写 / 首字母大写
注释/取消注释
─── 分隔线 ───
保存为查询文件
插入模板 ▸  SELECT / INSERT / UPDATE / DELETE / CREATE TABLE
历史记录
```

SQL 结果表格右键菜单：
```text
复制行
复制单元格
复制整列
复制为 JSON
─── 分隔线 ───
按此列升序排列
按此列降序排列
─── 分隔线 ───
导出当前行
在编辑器中生成 UPDATE
在编辑器中生成 DELETE
```

**SqlEditor.vue 修改：**
- 添加 `@contextmenu.prevent` 事件处理
- 右键时调用 `useContextMenu().show()` 展示菜单
- 菜单项 action：
  - 执行选中 SQL：获取 textarea 选中文本，emit `executeSelection` 事件
  - 执行全部 SQL：emit `execute` 事件（已有）
  - 格式化 SQL：简单格式化（关键字大写、缩进）
  - 大小写转换：子菜单，转换选中文本大小写
  - 注释/取消注释：切换选中行的 `--` 前缀
  - 保存为查询文件：emit `save` 事件
  - 插入模板：子菜单，插入 SELECT/INSERT/UPDATE/DELETE/CREATE TABLE 模板
  - 历史记录：emit `showHistory` 事件（展示最近执行的 SQL 列表）

**SqlResults.vue 修改：**
- 表格行添加 `@contextmenu.prevent` 事件处理
- 右键时调用 `useContextMenu().show()` 展示菜单
- 菜单项 action：
  - 复制行：将当前行数据转为 Tab 分隔文本写入剪贴板
  - 复制单元格：将当前单元格值写入剪贴板
  - 复制整列：将当前列所有值写入剪贴板
  - 复制为 JSON：将当前行转为 JSON 写入剪贴板
  - 排序：emit `sort` 事件，父组件处理排序逻辑
  - 在编辑器中生成 UPDATE：根据当前行数据生成 UPDATE 语句，emit `generateSql` 事件
  - 在编辑器中生成 DELETE：根据当前行主键生成 DELETE 语句，emit `generateSql` 事件

**测试标准：**
- SQL 编辑器右键弹出菜单
- 结果表格行右键弹出菜单
- 复制操作写入剪贴板
- 排序操作触发事件
- 菜单点击后自动关闭

**提交信息：**
```
feat: add context menus for SQL editor and results table
```

---

### 18.3 终端右键菜单

**功能目标：**
为 SSH 终端区域添加右键上下文菜单，提供复制、粘贴、清屏等常用操作。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/workspace/panels/
│   └── WorkspaceTerminal.vue    修改：添加终端右键菜单
├── i18n/zh.ts, en.ts            修改：添加终端菜单 i18n
```

**交互设计（参考 PRODUCT.md §3.6）：**

```text
复制
粘贴
全选
─── 分隔线 ───
清屏
重连
─── 分隔线 ───
打开 SFTP 面板
新建 SSH 连接
复制连接地址
─── 分隔线 ───
断开连接
```

**WorkspaceTerminal.vue 修改：**
- 终端容器 `ws-term-container` 添加 `@contextmenu.prevent` 事件处理
- 右键时获取终端选中文本（`terminal.getSelection()`）
- 调用 `useContextMenu().show()` 展示菜单
- 菜单项 action：
  - 复制：`navigator.clipboard.writeText(terminal.getSelection())`
  - 粘贴：复用现有 `handlePaste()` 逻辑
  - 全选：`terminal.selectAll()`
  - 清屏：复用现有 `clearTerminal()` 逻辑
  - 重连：调用 `doDisconnect()` + `connectSession()`
  - 打开 SFTP 面板：`showSftp.value = true`（与 18.1 联动）
  - 新建 SSH 连接：emit `newConnection` 事件（打开连接菜单）
  - 复制连接地址：将 `resourceName` 写入剪贴板
  - 断开连接：`showDisconnectDialog.value = true`

**测试标准：**
- 终端区域右键弹出菜单
- 复制操作获取终端选中文本
- 粘贴操作发送到终端
- 清屏清除终端显示
- 重连断开后重新连接
- 菜单点击后自动关闭

**提交信息：**
```
feat: add terminal context menu with copy/paste/reconnect
```

---

### i18n 键清单

**终端右键菜单（`ws.terminal.ctx.*`）：**
| 键 | 中文 | English |
|---|------|---------|
| `ws.terminal.ctx.copy` | 复制 | Copy |
| `ws.terminal.ctx.paste` | 粘贴 | Paste |
| `ws.terminal.ctx.selectAll` | 全选 | Select All |
| `ws.terminal.ctx.clear` | 清屏 | Clear |
| `ws.terminal.ctx.reconnect` | 重连 | Reconnect |
| `ws.terminal.ctx.openSftp` | 打开 SFTP 面板 | Open SFTP Panel |
| `ws.terminal.ctx.newConnection` | 新建 SSH 连接 | New SSH Connection |
| `ws.terminal.ctx.copyAddress` | 复制连接地址 | Copy Connection Address |
| `ws.terminal.ctx.disconnect` | 断开连接 | Disconnect |

**SQL 编辑器右键菜单（`sql.ctx.*`）：**
| 键 | 中文 | English |
|---|------|---------|
| `sql.ctx.executeSelection` | 执行选中 SQL | Execute Selection |
| `sql.ctx.executeAll` | 执行全部 SQL | Execute All |
| `sql.ctx.cut` | 剪切 | Cut |
| `sql.ctx.copy` | 复制 | Copy |
| `sql.ctx.paste` | 粘贴 | Paste |
| `sql.ctx.format` | 格式化 SQL | Format SQL |
| `sql.ctx.caseConvert` | 大小写转换 | Case Convert |
| `sql.ctx.caseUpper` | 全部大写 | UPPERCASE |
| `sql.ctx.caseLower` | 全部小写 | lowercase |
| `sql.ctx.caseTitle` | 首字母大写 | Title Case |
| `sql.ctx.toggleComment` | 注释/取消注释 | Toggle Comment |
| `sql.ctx.save` | 保存为查询文件 | Save as Query File |
| `sql.ctx.insertTemplate` | 插入模板 | Insert Template |
| `sql.ctx.templateSelect` | SELECT | SELECT |
| `sql.ctx.templateInsert` | INSERT | INSERT |
| `sql.ctx.templateUpdate` | UPDATE | UPDATE |
| `sql.ctx.templateDelete` | DELETE | DELETE |
| `sql.ctx.templateCreateTable` | CREATE TABLE | CREATE TABLE |
| `sql.ctx.history` | 历史记录 | History |

**SQL 结果右键菜单（`sql.result.ctx.*`）：**
| 键 | 中文 | English |
|---|------|---------|
| `sql.result.ctx.copyRow` | 复制行 | Copy Row |
| `sql.result.ctx.copyCell` | 复制单元格 | Copy Cell |
| `sql.result.ctx.copyColumn` | 复制整列 | Copy Column |
| `sql.result.ctx.copyJson` | 复制为 JSON | Copy as JSON |
| `sql.result.ctx.sortAsc` | 按此列升序排列 | Sort Ascending |
| `sql.result.ctx.sortDesc` | 按此列降序排列 | Sort Descending |
| `sql.result.ctx.exportRow` | 导出当前行 | Export Row |
| `sql.result.ctx.generateUpdate` | 在编辑器中生成 UPDATE | Generate UPDATE |
| `sql.result.ctx.generateDelete` | 在编辑器中生成 DELETE | Generate DELETE |

**SFTP 面板（`files.sftp.*`）：**
| 键 | 中文 | English |
|---|------|---------|
| `files.sftp.title` | SFTP | SFTP |
| `files.sftp.toggle` | 切换 SFTP 面板 | Toggle SFTP Panel |

---

## 设计核对点

- [ ] SFTP 面板与 PRODUCT.md §3.6 一致
- [ ] SQL 右键菜单与 PRODUCT.md §3.7 一致
- [ ] 终端右键菜单与 PRODUCT.md §3.6 一致
- [ ] i18n 覆盖所有新增文字
- [ ] 复制操作使用 navigator.clipboard API
- [ ] 右键菜单使用现有 useContextMenu composable

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
