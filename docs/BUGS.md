# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.88.0 | 🟡 | ctrl+K 同时打开两个搜索面板 | 用户反馈 | AppLayout.vue:72 监听 components/CommandPalette.vue、WorkspacePage.vue:212 监听 features/workspace/CommandPalette.vue，同一 Ctrl+K 双触发；处置方向：保留 palette、移除 command-palette |
| v0.88.0 | 🟡 | Ctrl+N 新建连接与浏览器冲突 | 用户反馈 | WorkspacePage 绑定 Ctrl+N 新建连接，与浏览器新建窗口保留键冲突且无法拦截；处置已定（用户决策 2026-09-23）：移除应用绑定、浏览器为准，替代键位由快捷键治理里程碑统一规划 |
| v0.88.0 | 🟡 | Ctrl+T 新建标签（同协议）与浏览器冲突 | 用户反馈 | WorkspacePage 绑定 Ctrl+T 新建标签，与浏览器新建标签保留键冲突且无法拦截；处置已定：移除应用绑定、浏览器为准，同上统一规划 |
| v0.88.0 | 🟡 | Ctrl+W 关闭当前标签与浏览器冲突 | 用户反馈 | WorkspacePage 绑定 Ctrl+W 关闭标签，与浏览器关闭标签保留键冲突且无法拦截；处置已定：移除应用绑定、浏览器为准，同上统一规划 |
| v0.88.0 | 🟡 | Ctrl+Tab / Ctrl+Shift+Tab 切换标签与浏览器冲突 | 用户反馈 | WorkspacePage 绑定 Ctrl+Tab/Ctrl+Shift+Tab 切换标签，与浏览器标签切换保留键冲突且无法拦截；处置已定：移除应用绑定、浏览器为准，同上统一规划 |
| v0.88.0 | 🟡 | Alt+1~9 面板宣称跳转标签、实测切换布局 | 用户反馈 | 快捷键面板宣称 Alt+1~9 跳转标签 N；实测 Alt+1~5 为布局面板（single/LR/TB/grid/main），Alt+6~9 才是跳标签，面板文案与实现矛盾 |
| v0.88.0 | 🟡 | Ctrl+Shift+\ 垂直分屏无效 | 用户反馈 | WorkspacePage.vue:481 Ctrl+Shift+\ 垂直分屏无响应，Ctrl+\ 水平分屏正常 |
| v0.88.0 | 🟡 | Agent 连接日志明文含 token | 步骤5审查 | agent_ws.rs `tracing::info!(url = %ws_url)` 将带 token 的 ws URL 打入 info 日志；base 既有问题（v0.88.0 审查备注发现），需脱敏（log token 以外部分或降级/移除 URL 字段） |
