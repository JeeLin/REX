# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.88.0 | 🟡 | ctrl+K 同时打开两个搜索面板 | 用户反馈 | AppLayout.vue:72 监听 components/CommandPalette.vue、WorkspacePage.vue:212 监听 features/workspace/CommandPalette.vue，同一 Ctrl+K 双触发；处置方向：保留 palette、移除 command-palette |
| v0.88.0 | 🟡 | Ctrl+N 新建连接与浏览器冲突 | 用户反馈 | WorkspacePage 绑定 Ctrl+N 新建连接，与浏览器新建窗口保留快捷键冲突，页面层无法可靠接管，需改键位 |
| v0.88.0 | 🟡 | Ctrl+T 新建标签（同协议）与浏览器冲突 | 用户反馈 | WorkspacePage 绑定 Ctrl+T 新建标签，与浏览器新建标签保留快捷键冲突 |
| v0.88.0 | 🟡 | Ctrl+W 关闭当前标签与浏览器冲突 | 用户反馈 | WorkspacePage 绑定 Ctrl+W 关闭标签，与浏览器关闭标签保留快捷键冲突 |
| v0.88.0 | 🟡 | Ctrl+Tab / Ctrl+Shift+Tab 切换标签与浏览器冲突 | 用户反馈 | WorkspacePage 绑定 Ctrl+Tab/Ctrl+Shift+Tab 切换标签，与浏览器标签切换保留快捷键冲突 |
| v0.88.0 | 🟡 | Alt+1~9 面板宣称跳转标签、实测切换布局 | 用户反馈 | 快捷键面板宣称 Alt+1~9 跳转标签 N；实测 Alt+1~5 为布局面板（single/LR/TB/grid/main），Alt+6~9 才是跳标签，面板文案与实现矛盾 |
| v0.88.0 | 🟡 | Ctrl+Shift+\ 垂直分屏无效 | 用户反馈 | WorkspacePage.vue:481 Ctrl+Shift+\ 垂直分屏无响应，Ctrl+\ 水平分屏正常 |
