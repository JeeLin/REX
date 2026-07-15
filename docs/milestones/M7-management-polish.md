# 0.8.0: M7 管理模块 + 打磨收尾

## Context
M0-M6 已完成全部核心功能模块（骨架、设计系统、工作空间、SSH、SQL、Redis、文件管理）。M7 是最后一个里程碑，统一管理页面的视觉风格，补全 i18n、移动端适配、性能优化和测试收尾。

前序：M0-M6 全部核心功能模块完成。
后续：无（最终里程碑）。

版本类型：minor

## 产品边界
- **做**：仪表盘/环境管理/Agent 管理/审计日志/设置页面的视觉统一、i18n 补全、移动端适配、测试收尾
- **不做**：新功能模块（核心功能已在 M3-M6 完成）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 仪表盘（统计卡片 + 快速连接 + 环境卡片） | ⬜ |
| 2 | 环境管理（列表/详情/创建向导） | ⬜ |
| 3 | Agent 管理（卡片/配置/日志/令牌/部署指南） | ⬜ |
| 4 | 审计日志（筛选/统计/详情/导出） | ⬜ |
| 5 | 设置（外观/终端/安全/更新版本总览） | ⬜ |
| 6 | i18n 补全 + 移动端适配 | ⬜ |
| 7 | 测试与收尾 | ⬜ |

## 子任务详细设计

### 1 仪表盘

- **功能目标**：系统概览入口，统计卡片 + 快速连接 + 环境卡片网格
- **文件结构**：
  - `src/pages/DashboardPage.vue`（重写：使用 REX 设计系统组件）
- **内容**：
  - 统计卡片：环境数 / 资源数 / Agent 在线数 / 今日操作数
  - 快速连接：最近使用资源列表
  - 环境卡片网格：名称、Agent 状态、资源类型分布
- **提交**：`feat(web): redesign dashboard with REX design system`

### 2 环境管理

- **功能目标**：环境列表/详情/创建向导
- **文件结构**：
  - `src/pages/EnvironmentsPage.vue`（重写）
  - `src/pages/EnvironmentDetailPage.vue`（新增）
  - `src/pages/EnvironmentNewPage.vue`（新增：创建向导）
- **交互设计**：
  - 列表：卡片式展示（名称、Agent 状态、描述、资源数）
  - 详情：Agent 面板 + 资源表格
  - 创建：步骤向导（基本信息 → 连接方式 → 完成）
- **提交**：`feat(web): redesign environment management pages`

### 3 Agent 管理

- **功能目标**：Agent 卡片/配置/日志/令牌/部署指南
- **文件结构**：
  - `src/pages/AgentsPage.vue`（重写）
- **内容**：
  - 卡片式展示（名称+状态、环境、设备信息）
  - 配置弹窗（环境/Agent ID/版本/服务器/令牌）
  - 日志查看器（级别筛选、自动滚动）
  - 部署指南（二进制/Docker/Compose）
- **提交**：`feat(web): redesign agent management page`

### 4 审计日志

- **功能目标**：操作记录查询（筛选/统计/详情/导出）
- **文件结构**：
  - `src/pages/AuditLogPage.vue`（重写）
- **内容**：
  - 统计卡片（总数/成功/失败）
  - 筛选栏（时间范围/操作类型/结果）
  - 表格（时间/操作/摘要/结果）+ 行展开详情
  - CSV 导出
- **提交**：`feat(web): redesign audit log page`

### 5 设置

- **功能目标**：个人设置页面
- **文件结构**：
  - `src/pages/SettingsPage.vue`（重写）
- **内容**：
  - 外观设置（主题切换、语言）
  - 终端设置（字体、光标闪烁）
  - 安全设置（会话超时）
  - 更新版本总览（当前版本、Agent 版本列表）
- **提交**：`feat(web): redesign settings page`

### 6 i18n 补全 + 移动端适配

- **功能目标**：补全所有页面的中英文翻译，移动端响应式适配
- **文件结构**：
  - `src/i18n/zh.ts`（补全缺失翻译）
  - `src/i18n/en.ts`（补全缺失翻译）
  - 各页面组件（添加移动端响应式样式）
- **提交**：`feat(web): complete i18n and mobile responsiveness`

### 7 测试与收尾

- **功能目标**：全量验证，确保所有质量门禁通过
- **测试标准**：
  - `cargo fmt --check` 通过
  - `cargo clippy --workspace --all-targets` 无 error
  - `cargo test --workspace` 通过
  - `bun run type-check` 通过
  - `bun run lint` 无 error
  - `bun run build` 通过
- **提交**：`chore: final polish and cleanup for 0.8.0 release`

## 设计核对点
- [ ] 仪表盘统计卡片数据正确展示
- [ ] 环境管理 CRUD 流程完整
- [ ] Agent 管理页面信息展示正确
- [ ] 审计日志筛选和导出功能正常
- [ ] 设置页面各配置项可操作
- [ ] 所有页面 i18n 中英文切换正常
- [ ] 移动端响应式布局正确
- [ ] 全部质量门禁通过

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

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
