# Step 4: Code Simplification — 0.83.1

## 变更文件

| 文件 | 变更 | 行数 |
|------|------|------|
| `useTabs.ts` | reorderTab 修复 splice 索引 | +5 -2 |
| `TabBar.vue` | 拖拽 drop position 跟踪 | +8 -0 |
| `Workspace.vue` | overlay guard + i18n key 重命名 + contenteditable | +14 -10 |
| `AppLayout.vue` | contenteditable 检查 | +2 -1 |
| `Terminal.vue` | Ctrl+C/V 智能处理 + copyWithFallback | +23 -7 |
| `en.ts` / `zh.ts` | i18n key 重命名 newTab→newConnection | +2 -2 |
| `useTabs.test.ts` | 测试断言适配修正后行为 | +2 -1 |

## 精简结论

- 所有修改均为最小必要改动，无冗余抽象
- Terminal.vue 中 Ctrl+C/V handler 结构与 WorkspaceTerminal.vue 保持一致，无需合并（两个独立组件）
- i18n key 重命名是破坏性清理，正确做法
- 无多余注释、无死代码、无新增依赖
- 结论：**无需精简**，代码已是最简形式
