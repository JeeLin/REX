# M71: i18n Completion

## Context
M70 完成数据导出与备份。本里程碑确保所有界面文本都支持 i18n，补全遗漏的硬编码英文。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段做什么：
- 审计并修复所有硬编码英文文本
- 补全缺失的翻译 key（en/zh）
- 新增功能的 i18n（M65-M70）
- 统一错误消息翻译

本阶段不做什么：
- 不新增语言（仅 en/zh）
- 不修改翻译框架
- 不改变 UI 布局

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 审计硬编码文本 | ⬜ |
| 2 | 补全缺失的翻译 key | ⬜ |
| 3 | 新增功能 i18n 补全（M65-M70） | ⬜ |
| 4 | 错误消息翻译统一 | ⬜ |

## 子任务详细设计

### 1 审计硬编码文本

- **功能目标**：找出所有未翻译的硬编码文本
- **文件结构**（创建）：
  - `docs/milestones/M71-reports/audit-hardcoded.md` — 审计报告
- **方法**：
  - grep 搜索 `.vue` 和 `.ts` 文件中的英文字符串
  - 检查 `title=`, `label=`, `placeholder=`, `>text<` 模式
- **提交信息**：`docs(i18n): audit hardcoded text in components`

### 2 补全缺失的翻译 key

- **功能目标**：补充所有遗漏的 en/zh 翻译
- **文件结构**（修改）：
  - `packages/rex-console-web/src/i18n/locales/en.json`
  - `packages/rex-console-web/src/i18n/locales/zh.json`
- **提交信息**：`feat(i18n): add missing translations for en/zh`

### 3 新增功能 i18n 补全

- **功能目标**：确保 M65-M70 新增功能的 i18n 完整
- **检查范围**：
  - M65: TokenRefreshModal（已添加）
  - M66: 移动端底部导航（已存在）
  - M67: 安全相关文本
  - M68: 无新增 UI 文本
  - M69: 调试面板文本
  - M70: 导出/备份相关文本
- **提交信息**：`feat(i18n): complete i18n for M65-M70 features`

### 4 错误消息翻译统一

- **功能目标**：统一所有错误消息的翻译
- **文件结构**（修改）：
  - `packages/rex-console-web/src/i18n/locales/en.json`
  - `packages/rex-console-web/src/i18n/locales/zh.json`
- **提交信息**：`feat(i18n): unify error message translations`

## 设计核对点

- en/zh 翻译 key 数量一致
- 无遗漏的硬编码英文
- 切换语言时界面无未翻译文本

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

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
