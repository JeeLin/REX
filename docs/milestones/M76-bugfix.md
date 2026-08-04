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
| [x] | 🔴 | Settings API 泄露敏感数据 | 用户反馈 | 修复 get_settings 过滤 SENSITIVE_KEYS（jwt_secret、password_hash） |
| [x] | 🔴 | Settings API auto_update 类型不匹配 | 用户反馈 | 修复 settingsApi.update 将 auto_update boolean 转为 string |
| [x] | 🟡 | 设置页面各区块最后一项被截断 | 用户反馈 | 修复 Card overflow:hidden 导致内容裁切，save-bar 改为 sticky 定位 |
| [x] | 🔴 | SSH 连接失败后无限重连 | 用户反馈 | 修复：移除 ws.onopen 中的 reconnectAttempts 重置，仅手动重连时重置 |
| [x] | 🟡 | Docker 数据目录与程序文件混用 | 用户反馈 | agent 二进制移至 /app/agent-binaries，通过 REX_AGENT_BINARIES_DIR 环境变量配置 |
| [x] | 🟡 | 切换页面后工作区重连 | 用户反馈 | 修复：AppLayout RouterView 添加 KeepAlive，排除 login/setup 页面 |
