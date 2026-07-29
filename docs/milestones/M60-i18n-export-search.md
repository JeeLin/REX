# M60: i18n Completion, Data Export & Search Enhancement

## Context
M59 完成了生产加固和集成测试。本里程碑从国际化完善、数据导出增强、全局搜索三个维度进一步提升产品质量。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段完善现有功能，不涉及新功能模块。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 国际化：补全所有硬编码文本 | ⬜ |
| 2 | 数据导出：审计日志 CSV 导出增强 | ⬜ |
| 3 | 搜索增强：全局资源搜索优化 | ⬜ |

## 子任务详细设计

### 1 国际化：补全所有硬编码文本

- **功能目标**：扫描并补全所有硬编码的中英文文本
- **文件结构**（修改）：
  - `packages/rex-console-web/src/i18n/locales/en.json`
  - `packages/rex-console-web/src/i18n/locales/zh.json`
  - `packages/rex-console-web/src/` 各组件文件
- **提交信息**：`fix(i18n): complete all hardcoded text translations`

### 2 数据导出：审计日志 CSV 导出增强

- **功能目标**：增强审计日志导出，支持筛选条件导出
- **文件结构**（修改）：
  - `crates/rex-hub/src/audit_api.rs` — 导出端点增强
  - `packages/rex-console-web/src/pages/AuditLogPage.vue` — 导出按钮
- **提交信息**：`feat(export): enhance audit log CSV export with filters`

### 3 搜索增强：全局资源搜索优化

- **功能目标**：提升侧栏资源搜索的响应速度和准确性
- **文件结构**（修改）：
  - `packages/rex-console-web/src/features/resource-panel/ResourcePanel.vue` — 搜索优化
- **提交信息**：`perf(search): optimize resource search performance`

## 设计核对点

- 符合产品文档描述
- 不引入多用户/RBAC 概念
- 前端命令使用 `bun`

## Flow Status

- [ ] 步骤1：编写里程碑文档
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

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
