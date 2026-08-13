# M80: M78 重设计收尾 — feature 组件全量 token 化迁移

## Context
M78 完成了「全系统 UX/UI 重设计」，但审计发现只重做了 9 个顶层页面（pages/），feature 组件层（files/sql/redis/terminal/workspace/resource）基本未迁移：这些组件仍使用硬编码 hex 色、自带平行「伪设计系统」（`.btn`/`.btn-primary` 等），并引用了设计系统中不存在的 token（`--bg-secondary`/`--border-default`/`--color-primary`/`--surface-2`），导致 light/高对比主题下部分元素渲染无样式。M80 将这些 feature 组件统一迁移到 `styles/tokens.css` 设计系统与 `components/ui/*` 共享组件库，使 M78 的重设计真正彻底。

版本类型：minor（纯前端样式/组件收尾，无新功能、无后端变更、无数据模型变更）
版本号：0.68.0

## 产品边界
本阶段做什么：
- 全量迁移 feature 组件样式到设计 token（移除硬编码 hex、`#fff` 改 `--text-on-accent`、不存在的 token 名改映射到现有 token）
- 原生 `<table>`/`<button>`/`<input>` 标记改用 `components/ui/*`（Table/Button/Select/Input）
- 统一协议色使用 `--proto-*`（ssh/sftp/mysql/postgresql/redis/sqlite/s3）
- 补齐缺失语义 token（`--text-on-accent` 已在 token 中；`--bg-secondary`/`--border-default`/`--color-primary`/`--surface-2` 直接映射现有 token，不新增无意义 token）

本阶段不做什么：
- 不新增页面、不新增后端功能、不修改数据模型
- 不改变任何交互行为/功能（仅视觉与组件统一）
- 不引入多用户/RBAC/企业概念

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 1 | 设计系统补充与清理：确认 `--text-on-accent`、协议色 `--proto-*`、映射不存在 token 的等价名 | 前端 | ⬜ |
| 2 | terminal/WorkspaceTerminal.vue：38 处裸 hex → token；状态栏徽章 `#fff`/`#000` → `--text-on-accent` | 前端 | ⬜ |
| 3 | files/FilesPage.vue：移除自身 `.btn`、裸 `<button>` 改用 Button、内联色改用 token、修复 `--bg-secondary` | 前端 | ✅ |
| 4 | files/FileEditorDialog.vue：删除自带 `.btn` 伪系统，改用 Button；移除 `--border-default`/`--color-primary`；离谱色 → token | 前端 | ✅ |
| 5 | files/FilesDrawer.vue + MobileFilesBar.vue：裸 `<button>` 改用 Button/IconButton | 前端 | ✅ |
| 6 | files/FolderSyncDialog.vue：硬编码协议色板 → `--proto-*` | 前端 | ✅ |
| 7 | sql/ 系列：SqlPage/TableDesigner/ImportWizard/SqlResultGrid/ColumnEditor/GlobalQueryModal/ExportWizard/SqlFormView/AiAssistantDrawer 原生 table/btn → ui 库 + token | 前端 | ✅ |
| 8 | redis/ 系列：RedisPage/FormatViewer/RedisStatus 硬编码类型色板 → `--proto-*` | 前端 | ✅ |
| 9 | workspace/ResourceProperties.vue + resource/WizardModal.vue：硬编码协议色/资源色 → `--proto-*` | 前端 | ✅ |
| 10 | resource-panel/ResourcePanel.vue 离谱色 `#f5a623` → `--accent`；components/TokenRefreshModal.vue 移除不存在 token 回退 | 前端 | ✅ |

## 子任务详细设计

### 1 设计系统补充与清理
- **功能目标**：保证设计系统自洽，让 feature 组件迁移有统一依据。
- **文件结构**：`packages/rex-console-web/src/styles/tokens.css`
- **接口设计**：无。仅补充 token：
  - 确认 `--text-on-accent: #0d1117`（深色）/ `#fff`（浅色）已存在（用于白字按钮/徽章）。
  - 协议色已存在：`--proto-ssh/--proto-sftp/--proto-mysql/--proto-postgresql/--proto-redis/--proto-sqlite/--proto-s3`。
  - 不存在的 token 名处理：**不新增**，迁移时直接映射——`--bg-secondary`→`--bg-surface`、`--surface-2`→`--bg-elevated`、`--border-default`→`--border`、`--color-primary`→`--accent`。
- **前端交互设计**：无。
- **后端流程**：无。
- **测试标准**：`bun run type-check`、`bun run lint`（0 error）、`bun run build` 全绿；浅色/高对比主题下抽样组件无无样式元素（grep 确认无 `var(--bg-secondary`/`--border-default`/`--color-primary`/`--surface-2` 残留）。
- **提交信息**：`style: consolidate design tokens for feature-components migration (M80 #1)`

### 2 WorkspaceTerminal.vue token 化
- **功能目标**：移除 38 处裸 hex，状态栏徽章/背景预设改用 token。
- **文件结构**：`packages/rex-console-web/src/features/terminal/WorkspaceTerminal.vue`
- **关键改动**：`BG_PRESETS` 字面量（`#0D1117`/`#161B22`）→ `var(--bg-deep)`/`var(--bg-page)`；状态徽章 `#fff`/`#000` → `var(--text-on-accent)`/`var(--text-primary)`；离谱 `#8b949e` → `var(--text-muted)`。
- **测试标准**：type-check/lint/build 全绿；终端可正常渲染（手动/构建验证）。
- **提交信息**：`style: tokenize WorkspaceTerminal hard-coded colors (M80 #2)`

### 3-10 feature 组件迁移
- 每个子任务：用 `components/ui/*`（Button/Input/Select/Table）替换原生标记；硬编码 hex/协议色板统一为 `--proto-*` 与设计 token；移除不存在 token 名的回退写法。
- **文件结构**：见各子任务表。
- **测试标准**：各子任务后 `type-check`/`lint`/`build` 全绿；lint 0 error。

## 设计核对点
- 不引入多用户/RBAC 概念
- 所有组件 `cursor: pointer`、hover 过渡 150-300ms（沿用设计系统）
- 深色优先；`--text-on-accent` 用于白字按钮/徽章；`--proto-*` 用于协议着色
- 不新增无意义 token；不存在 token 名映射到现有 token
- 迁移不改变任何交互行为

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

## Bugs
| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
