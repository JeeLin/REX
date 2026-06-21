# M18 设计核对报告

**日期：** 2026-06-21
**审核人：** Claude Code
**核对对象：** `docs/milestones/M18-terminal-sftp-sql-contextmenu.md`
**对照基准：** `docs/PRODUCT.md` 3.5-3.8

---

## 检查概要

| # | 检查维度 | 结果 | 说明 |
|---|----------|------|------|
| 1 | SFTP 面板与 PRODUCT.md 3.6 对齐 | 🟡 | 缺少「拖拽文件到终端自动粘贴路径」交互 |
| 2 | SQL 右键菜单与 PRODUCT.md 3.7 对齐 | 🟡 | 缺少大小写转换、插入模板、历史记录等菜单项 |
| 3 | 终端右键菜单与 PRODUCT.md 3.6 对齐 | 🟡 | 缺少 SFTP 面板打开、新建连接、复制地址菜单项 |
| 4 | 产品边界（单用户/自托管/数据不经浏览器） | ✅ | 无违规 |
| 5 | 产品边界排除项合理性 | ✅ | 排除的 4 项均有明确依赖，合理 |
| 6 | 遗漏 PRODUCT.md 功能 | 🟡 | 库表结构树右键菜单、查询标签右键菜单、工具栏右键菜单均未纳入 |
| 7 | 超范围功能 | ✅ | 未发现超范围内容 |
| 8 | 文件结构（Vue 功能域） | ✅ | 符合 `features/{domain}/` 组织 |
| 9 | i18n 键命名一致性 | 🟡 | 未定义具体 i18n 键；现有命名规范可复用但需补充说明 |
| 10 | 提交信息格式 | ✅ | 符合 `feat: <scope> <description>` 规范 |

---

## 详细发现

### 1. SFTP 面板与 PRODUCT.md 3.6 对齐

**✅ 一致：**
- 工具栏「📁 SFTP」按钮 toggle 显隐
- 右侧 split view 布局
- 可拖拽调整宽度（280px 默认）
- 面包屑路径 + 文件列表
- 双击文件夹进入、双击 `..` 返回上级
- 复用现有 `FileList.vue` 组件

**🟡 缺失交互：** PRODUCT.md 3.6 明确定义「从 SFTP 面板拖拽文件到终端区域，自动将文件路径粘贴到终端」。M18 未包含此功能。该交互属于前端 DOM 交互，不依赖后端，应在本里程碑内实现。

**🟡 缺失布局变体：** PRODUCT.md 3.6 描述为「在右侧/下方打开内嵌 SFTP 面板」，M18 仅实现右侧 split view。下方布局可视为后续增强，影响较小。

---

### 2. SQL 编辑器/结果右键菜单与 PRODUCT.md 3.7 对齐

**✅ 一致：**
- 执行选中 SQL / 执行全部 SQL
- 剪切 / 复制 / 粘贴
- 格式化 SQL
- 注释/取消注释
- 保存为查询文件
- 复制行 / 复制单元格 / 复制为 JSON
- 按列升序/降序排序
- 导出当前行

**🟡 缺失菜单项（SQL 编辑器）：**
- `大小写转换 ▸`（全部大写/全部小写/首字母大写）— PRODUCT.md 明确定义
- `插入模板 ▸`（SELECT/INSERT/UPDATE/DELETE/CREATE TABLE）— PRODUCT.md 明确定义
- `历史记录` — PRODUCT.md 明确定义

**🟡 缺失菜单项（结果表格）：**
- `复制整列` — PRODUCT.md 明确定义
- `在编辑器中生成 UPDATE` — PRODUCT.md 明确定义
- `在编辑器中生成 DELETE` — PRODUCT.md 明确定义

---

### 3. 终端右键菜单与 PRODUCT.md 3.6 对齐

**✅ 一致：**
- 复制 / 粘贴 / 全选
- 清屏 / 重连
- 断开连接（含确认框）

**🟡 缺失菜单项（终端区右键菜单）：**
- `打开 SFTP 面板` — 与 18.1 SFTP 功能直接相关，应在本里程碑包含
- `在新标签中打开 SFTP` — PRODUCT.md 定义
- `新建 SSH 连接` — PRODUCT.md 定义
- `复制连接地址` — PRODUCT.md 定义

**🟡 完全未覆盖（§3.6 定义）：**
- `工具栏右键菜单` — 右键工具栏区域的独立菜单（复制延迟信息、打开连接详情、切换全屏），M18 未提及
- `移动端浮动工具栏` — 底部浮动面板的方向键、Tab、历史等按钮，M18 未提及（可视为独立里程碑，但应在边界中说明）

---

### 4. 产品边界检查

**✅ 单用户：** M18 未引入多用户、RBAC、团队协作等概念。

**✅ 自托管：** 所有功能均在自托管架构内。

**✅ 数据不经过浏览器：**
- SFTP 面板通过 WebSocket/REST API 获取文件列表（非浏览器直传）
- 未涉及文件传输数据流
- 符合 PRODUCT.md「文件传输数据不经过浏览器」原则

---

### 5. 排除项合理性

M18 明确排除的 4 项均合理：
- 跨连接文件传输「发送到…」→ 需要后端 TransferCoordinator，正确排除
- SQL AI 助手 → 需要 LLM 集成，正确排除
- 全局查询 modal → 需要多数据库执行能力，正确排除
- 工作空间面板间拖拽 → 交互复杂度高，正确排除

---

### 6. 遗漏 PRODUCT.md 功能

M18 未覆盖的 §3.7 定义菜单：

| 菜单 | 状态 | 说明 |
|------|------|------|
| 库表结构树右键菜单（表节点、列节点、数据库节点、视图节点、空白区域） | 未纳入 | 涉及 5 类右键菜单，复杂度较高，可拆为独立里程碑 |
| 查询标签右键菜单（关闭、保存、另存为、重命名、复制 SQL、执行 SQL） | 未纳入 | 可拆为独立里程碑 |

建议在 M18 边界中明确标注这些菜单在后续里程碑实现，避免遗漏。

---

### 7. 超范围功能

**✅ 未发现。** M18 所有功能均在 PRODUCT.md 3.6/3.7 范围内。

---

### 8. 文件结构

**✅ 符合 Vue 功能域组织：**

```
features/workspace/panels/WorkspaceTerminal.vue  ← 终端面板（SFTP 按钮 + 右键菜单）
features/sql/SqlEditor.vue                      ← SQL 编辑器（右键菜单）
features/sql/SqlResults.vue                     ← SQL 结果（右键菜单）
features/files/FileList.vue                      ← SFTP 面板复用
i18n/zh.ts, en.ts                               ← 国际化
```

修改文件范围合理，复用策略清晰（复用 `FileList.vue`、`useContextMenu` composable）。

---

### 9. i18n 键命名一致性

**🟡 未定义具体 i18n 键。** M18 仅标注「修改 i18n/zh.ts, en.ts，添加 SFTP/SQL/终端菜单 i18n」，未列出具体键值。

**现有命名规范（供参考）：**
- 上下文菜单项：`ctx.xxx`（如 `ctx.copy`, `ctx.paste`, `ctx.refresh`）
- 工作区：`ws.tab.xxx`, `ws.layout.xxx`
- SQL：`sql.xxx`
- 文件：`files.xxx`

**建议：** SFTP 面板可使用 `files.sftp.xxx`；终端右键菜单建议使用 `ws.terminal.ctx.xxx`；SQL 右键菜单建议使用 `sql.ctx.xxx`。设计文档应明确列出待添加的 i18n 键。

---

### 10. 提交信息格式

**✅ 三个子任务的提交信息均符合规范：**
- `feat: add embedded SFTP panel in SSH terminal workspace`
- `feat: add context menus for SQL editor and results table`
- `feat: add terminal context menu with copy/paste/reconnect`

---

## 总体结论

**🟡 需修订后通过**

M18 覆盖了 PRODUCT.md §3.6 和 §3.7 的核心功能，产品边界正确，文件结构合理。但存在以下需补充的要点：

1. **必须补充：** SQL 编辑器右键菜单缺少「大小写转换」「插入模板」「历史记录」；结果表格缺少「复制整列」「生成 UPDATE/DELETE」
2. **必须补充：** 终端右键菜单缺少「打开 SFTP 面板」（与 18.1 直接关联）；「新建连接」「复制地址」
3. **建议明确：** 库表结构树右键菜单、查询标签右键菜单是否纳入本里程碑或后续里程碑
4. **建议补充：** SFTP 文件拖拽到终端自动粘贴路径的交互设计
5. **建议补充：** i18n 键的具体命名清单
