# M39 代码精简报告

## 精简项

| # | 文件 | 变更 | 原因 |
|---|------|------|------|
| 1 | CommandPalette.vue | 移除 `resource` category（接口 + 分组标签） | 死代码，从未使用 |
| 2 | useSessionTimeout.ts | `remainingSeconds` 从 `WARNING_BEFORE_MS` 派生 | 消除魔法数字耦合 |
| 3 | AppLayout.vue | 移除 `.sidebar-spacer` CSS | 死 CSS，模板未引用 |
| 4 | AppLayout.vue | 移除 `.resource-panel-desktop` CSS | 死 CSS，模板未引用 |
| 5 | SettingsPage.vue | ref 初始化使用硬编码默认值 `30` | 避免冗余 localStorage 读取（onMounted 会覆盖） |
| 6 | router/index.ts | 提取 `DEFAULT_SESSION_TIMEOUT` 常量 | 消除魔法数字重复 |

## 未处理项（预存问题，非 M39 引入）

| 项 | 原因 |
|----|------|
| `/resource-new` 路由不存在 | 预存问题，CommandPalette 原有命令 |
| SettingsPage theme/lang 同步重复 | 预存代码，M39 未引入 |
| AppLayout fullscreen 按钮 CSS 重复 | 预存样式 |
| CommandPalette indexOf O(n) | 性能优化，资源数量 <100，影响极小 |

## 结论

✅ 精简不改变功能行为。移除死代码和魔法数字，提升可维护性。
