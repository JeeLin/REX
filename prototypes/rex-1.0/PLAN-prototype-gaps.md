# 原型补全计划（前端代码有、原型没有的功能）

> 目标：把 `packages/rex-console-web/` 已实现、但 `rex-1.0` 原型里缺失的**用户可见功能**，以**可交互静态 Mock** 的形式补进现有原型。
> 约束（来自 `DESIGN.md`）：
> - 纯静态 HTML + CSS + 原生 JS，无框架、无构建步骤。
> - 所有浮层用 `REX.*`（`common.js`），不手写 div 弹窗。
> - 协议配色走 `.p-*` / `var(--*)`，禁止 markup 里出现裸 hex。
> - 版本字符串一律 `1.0.x`，禁止 2.0。
> - 新增页面用 NN 前缀、保持侧栏 `nav` 相互链接。
> - `node --check` 校验所有 `<script>` 与 `common.js`。
> - 框架底座（xterm / CodeMirror / Pinia / vue-router / vue-i18n）**只做 UX Mock，不移植库**。

---

## 0. 通用改造（所有页面共用）

1. **命令面板 Cmd/Ctrl+K（真实化）**
   - `common.js` 新增 `REX.commandPalette()`：全屏 overlay，分类命令列表（跳转到各页、执行动作如「新建资源 / 注册 Agent / 刷新」），支持模糊过滤、↑↓ 选择、Enter 执行、Esc 关闭。
   - 把现有各页**装饰性**搜索框（`Search resources` / `Quick connect`）接上：点击即唤起面板（替代纯展示）。
2. **会话超时 / 自动锁定**
   - `common.js` 新增全局空闲计时器（演示用较短间隔，如 2 分钟；真实产品 120 分钟）。
   - 空闲达阈值 → 顶部 warning → 锁定弹窗 → 跳 `00-login.html`（或 `REX.toast` 提示）。
   - 在 `initSidebar` 之后于所有页面挂载；登录/设置页豁免。
3. **Token 过期刷新弹窗**
   - 新增 `REX.tokenRefresh()`：检测到 `localStorage['rex-token']` 为「过期态」时弹出（提供「重新登录 / 刷新令牌」）。`00-login.html` 登录成功后写入 token；可加一个演示入口触发过期态。

---

## 1. 新增独立页面（沿用 §9 骨架 + 全部 `nav` 互链）

| 文件 | 对标代码 | 内容 | 交互 |
|------|----------|------|------|
| `03-setup.html` | `SetupPage.vue` | 首启管理员密码设置（全屏，同登录版式） | 密码 + 确认，≥6 位校验、不一致报错；成功 → `02-workspace.html`。已设置则提示跳登录。 |
| `12-design-preview.html` | `DesignPreview.vue` | 设计系统组件陈列馆 | 展示全部 `.p-*` 配色、`.badge/.btn/.st/.stat/.panel/.table/.tabs/.switch/.select/.input/.card/.alert/.tooltip/.avatar`；亮/暗主题切换（`data-theme`）。 |

> **SIP 通话详单（CDR）不单独成页** —— 折叠进工作区 SIP 面板（见 §2.4「通话记录」抽屉），避免为它单开一页。

---

## 2. 工作区增强（`02-workspace.html`）

### 2.1 SQL 控制台
- **表设计器** `REX.tableDesigner()`：modal 多标签（列 / 索引 / 外键 / DDL），静态样例 DDL 反向展示；纯展示 + 复制。
- **导入/导出向导** `REX.sqlImport()/sqlExport()`：modal 分步（导出选格式 csv/json；导入粘贴/选文件）→ toast。
- **已存查询** `REX.savedQueries()`：下拉/小面板，列表存 `localStorage`，点击回填编辑器。
- **全局查询** `REX.globalQuery()`：modal 跨库查询（静态结果集）。

### 2.2 Redis
- **CLI 模式切换**：redis leaf 顶部加 toggle（键值树 / `redis>` CLI）。CLI 为输入框 + 命令历史（↑↓ 回溯）+ 静态回显日志（沿用现有 `redis>` 风格）。

### 2.3 文件 / 对象存储
- **内联文件编辑器** `REX.fileEditor(name, content)`：双击文件行 → modal 内 `textarea`（样式化，模拟 CodeMirror）→ 保存 toast。
- **文件抽屉** `REX.filesDrawer()`：侧滑面板，列出最近/收藏路径（`localStorage`）。
- **文件夹同步对话框** `REX.folderSync()`：modal 配源/目标/方向 → toast（数据不经过浏览器，仅建任务）。

### 2.4 SIP 软电话
- **呼叫状态机**：在 `sipBody` 增加 CallState（振铃 → 通话中（计时器）→ 结束），由拨号盘驱动；挂断按钮。
- **媒体占位**：音频/视频走浏览器 WebRTC，静态原型用「通话中计时 + 占位 canvas/头像」模拟，标注「真实媒体由浏览器 getUserMedia 提供」。
- **通话记录（CDR）抽屉**：SIP 面板顶部加「通话记录」按钮 → 右侧抽屉，表格（时间 / 主叫 / 被叫 / 方向 / 状态 / 时长），按方向+状态筛选 chips，详情展开，分页（20/50/100）。复用 audit 表样式。**不新增独立页面**。

---

## 3. 侧栏 / 全局组件

- **收藏 / 最近**（`common.js` + 侧栏）：`.tree` 顶部加「收藏」分组，资源行加星标 toggle（`localStorage`）；命令面板/快速连接可写入最近。
- **资源创建保持单弹窗**：沿用现有 `REX.newResource()`（类型感知字段 + 环境选择 + 一次填写提交），**不做分步向导**（参数不多，单弹窗更省事）。
- **更新检查（设置页）** `09-settings.html`：Updates 段加「检查更新」→ 模拟 latest 版本 + 进度条；保留「Hub/Agent 版本必须一致」状态文案。

---

## 4. 一致性收尾（对照 DESIGN.md §13 清单）

- [ ] 所有 `:root` 新增 token 同步到 8 个页面。
- [ ] 新浮层全部走 `REX.*`。
- [ ] 协议色只用 `.p-*` / `var(--*)`，markup 无裸 hex。
- [ ] 版本字符串保持 `1.0.x`。
- [ ] `node --check` 通过每个 `<script>` 与 `common.js`。
- [ ] 新增页面被所有页 `nav` 链接。
- [ ] `open-design lint` 仍只有预期的 `--purple` P0。

---

## 5. 暂不处理（说明）

- **AI 助手抽屉**：本次不做（代码现状也是本地模拟，无真实 LLM）。若以后做，用抽屉形态而非独立页。
- **SIP CDR 独立页**：明确不做，折叠进工作区 SIP 面板（见 §2.4）。
- **资源创建分步向导**：不做，保持单弹窗（§3）。
- **框架底座**：xterm / CodeMirror / Pinia / vue-router / vue-i18n 以静态 UX 模拟，不引入库、不加构建步骤（DESIGN.md §1）。
- **版本号对齐**：原型继续标 `1.0.x`（DESIGN.md 红线）；真实代码是 `0.70.x`。若要原型改标 0.7x 请单独指示。
- **多 Agent / 环境限制**：维持上一轮结论，本次不动。

---

## 6. 建议实施顺序（确认后再动手）

1. 通用改造：命令面板、会话超时、Token 刷新弹窗（影响所有页）。
2. 新页面：Setup、Design Preview。
3. 工作区：SQL 表设计器/导入导出/已存查询/全局查询 → Redis CLI → 文件内联编辑/抽屉/同步 → SIP 状态机 + CDR 抽屉。
4. 侧栏/全局：收藏/最近、资源单弹窗（不动）、设置更新检查。
5. 一致性收尾 + `node --check` + 互链校验。
