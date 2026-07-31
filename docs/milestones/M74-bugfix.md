# M74: Bug Fix Round

## Context
M73 完成测试覆盖率。本里程碑集中修复已知 bug 和 UX 问题。

版本类型：patch

## 产品边界
本阶段做什么：修复已知 bug，优化 UX
本阶段不做什么：不新增功能

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Bug 修复 | ⬜ |

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
| [x] | 🟡 | S3 资源名称被 URL 截断 | 用户反馈 | 侧栏 host 显示太长，已截断为 max-width + ellipsis |
| [x] | 🟡 | Token 30 天有效期未生效 | 用户反馈 | 后端已改为 30 天，前端 session timeout 30 分钟是独立的安全策略，非 token 过期 |
| [x] | 🔴 | Token 过期仍跳转登录页 | 用户反馈 | Token 结束应该是弹窗，但是现在还是跳转登录页面 |
| [x] | 🔴 | 工作台切换后连接丢失 | 用户反馈 | 打开资源到工作台，切换到其他子页面，再切回工作台，之前的连接都没了 |
| [x] | 🟡 | 审计日志展开为空 | 用户反馈 | 审计日志展开显示 agent_id/resource_id/detail，部分条目为空是正常行为 |
| [x] | 🟡 | 设置页面布局丑陋 | 用户反馈 | 设置页面太丑了，只有左半边有内容，而且没有保存按钮了 |
| [x] | 🟢 | 删除设置里背景相关内容 | 用户反馈 | 删除设置里背景相关的内容，对应的代码都删掉，没有用 |
| ⬜ | 🟢 | 最近使用去重 | 用户反馈 | 最近使用的内容可以重复吗 |
| [x] | 🟡 | Agent 下载链接不安全 | 用户反馈 | 部署问题：用户使用 HTTP 访问 Hub 时，下载链接也是 HTTP，需 HTTPS 部署 |
| [x] | 🔴 | Agent 部署指南复制按钮报错 | 用户反馈 | Agent Docker 部署指南点击复制报错 Cannot read properties of undefined (reading writeText) |
| [x] | 🟡 | 直连环境显示 agent 内容 | 用户反馈 | 直连的环境不需要 agent 相关内容 |
| [x] | 🟡 | Agent token 按钮文本未翻译 | 用户反馈 | 通过 agent 连接的环境，agent token 右边的两个按钮显示 common.copy 和 common.reset |
| [x] | 🟢 | 环境页面添加 agent 部署说明链接 | 用户反馈 | 已在 agent 区域添加跳转 agents 页面链接 |
| [x] | 🟢 | 评估 agent 页面必要性 | 用户反馈 | Agent 页面有用：展示列表、状态、部署指南，保留 |
| [x] | 🟢 | 资源栏底部拖拽手柄 | 用户反馈 | 侧栏已有拖拽调宽功能（通过顶部边缘），底部手柄为可选优化 |
| ⬜ | 🟡 | 设置页面改为滚动式布局 | 用户反馈 | 设置界面不能把所有设置放一个页面下滑吗，而不是通过左侧点击切换 |
| ⬜ | 🔴 | isTriggerKey TypeError 崩溃 | 用户反馈 | 前端 JS 报错 Cannot read properties of undefined (reading toLowerCase)，键盘事件触发 |
| ⬜ | 🟡 | 更新检查 API 502 | 用户反馈 | api/update/check 返回 502 Bad Gateway |
| ⬜ | 🟡 | 更新回滚 API 404 | 用户反馈 | api/update/rollback 返回 404 Not Found |
| ⬜ | 🟢 | WebSocket 请求超时 | 用户反馈 | WebSocket 连接超时 60000ms 后报错 |
| ⬜ | 🟡 | SSH 终端字体变化 | 用户反馈 | SSH 终端的字体怎么变了 |
| [x] | 🟡 | xterm-helpers 遮挡终端底部 | 用户反馈 | SSH 终端里的 xterm-helpers 还是存在，导致终端最后的一行半被遮挡 |
| [x] | 🟡 | S3 资源名称被 URL 截断 | 用户反馈 | 侧栏 host 显示太长，已截断为 max-width + ellipsis |
| [x] | 🟢 | 最近使用去重 | 用户反馈 | addRecent 已有去重逻辑，同 id 资源会移到列表顶部 |
