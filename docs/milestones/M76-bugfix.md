# M76: Bug Fix

## Context
M75 完成 Bug Fix & UX Polish。本里程碑继续修复已知 bug。

版本类型：patch
版本号：0.65.3

## 产品边界
本阶段做什么：修复已知 bug
本阶段不做什么：不新增功能

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 1 | Bug 修复 | 混合 | ⬜ |

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
| ⬜ | 🔴 | Settings API 泄露敏感数据 | 用户反馈 | GET /api/settings 返回 jwt_secret、password_hash 等敏感字段 |
| ⬜ | 🔴 | Settings API auto_update 类型不匹配 | 用户反馈 | PUT /api/settings 时 auto_update 传 boolean 但后端 HashMap<String, String> 期望 string |
