# M53: Bug fix + UX polish

## Context
M52 完成 Hub 自动更新机制（v0.45.0）后，进入 bug 修复和 UX 打磨阶段。本里程碑修复已知的用户体验问题，提升整体稳定性。

版本类型：patch（bug 修复）
版本号：0.46.0

## 产品边界

**做什么**：
- 修复 SSH 终端显示问题（最后两行截切、切换标签重连）
- 修复资源编辑模态框字段不匹配协议类型
- 修复 agent-token-row 显示为空
- 修复 xterm-char-measure-element 在终端中可见

**不做什么**：
- 不新增功能
- 不修改核心架构

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | SSH 终端修复（最后两行截切 + 切换标签重连） | ✅ |
| 2 | 资源编辑模态框修复 | ✅ |
| 3 | 前端 UX 打磨（token 显示、i18n） | ⬜ |
| 4 | 质量验证 | ⬜ |

## 子任务详细设计

### 1 SSH 终端修复

- **功能目标**：修复 SSH 终端最后两行被截切的问题，以及切换标签时终端重连的问题
- **文件结构**：
  - 修改：`packages/rex-console-web/src/features/terminal/TerminalView.vue`（终端容器样式）
  - 修改：`packages/rex-console-web/src/features/terminal/useTerminal.ts`（连接管理）
- **交互设计**：
  - SSH 终端底部不再截切，所有行可见
  - 切换标签再切回时，SSH 连接保持不断开
- **后端流程**：无后端变更
- **测试标准**：终端显示正常，切换标签不断连
- **提交信息**：`fix(terminal): prevent content clipping and connection reset on tab switch`

### 2 资源编辑模态框修复

- **功能目标**：修复资源编辑时只显示固定字段，不根据协议类型显示对应字段的问题
- **文件结构**：
  - 修改：`packages/rex-console-web/src/pages/EnvironmentDetailPage.vue`（资源编辑逻辑）
- **交互设计**：
  - 编辑资源时，表单字段根据协议类型动态显示（SSH/MySQL/Redis 等各有不同字段）
- **后端流程**：无后端变更
- **测试标准**：编辑不同协议资源时显示正确字段
- **提交信息**：`fix(resources): show protocol-specific fields in resource edit modal`

### 3 前端 UX 打磨

- **功能目标**：修复 agent-token-row 显示为空，以及 i18n 遗漏
- **文件结构**：
  - 修改：`packages/rex-console-web/src/pages/EnvironmentDetailPage.vue`（agent token 显示）
  - 修改：`packages/rex-console-web/src/i18n/locales/zh.json`（中文翻译）
  - 修改：`packages/rex-console-web/src/i18n/locales/en.json`（英文翻译）
- **交互设计**：agent token 正确显示，重置按钮可用
- **后端流程**：无后端变更
- **测试标准**：token 显示正确，重置功能正常
- **提交信息**：`fix(ui): correct agent token display and add i18n keys`

### 4 质量验证

- **功能目标**：确保所有改动通过质量门禁
- **后端流程**：`cargo fmt --check` + `cargo clippy` + `cargo test`
- **前端流程**：`bun run type-check` + `bun run lint` + `bun run build`
- **测试标准**：所有检查通过
- **提交信息**：`chore: quality gate verification for M53`

## 设计核对点

1. SSH 终端不再截切底部行
2. 切换标签保持连接不断开
3. 资源编辑表单根据协议类型动态显示字段
4. agent token 正确显示且可重置

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
