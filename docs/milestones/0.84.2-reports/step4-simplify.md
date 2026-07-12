# Step 4: Code Simplification Report — 0.84.2

## 变更分析

| 文件 | 变更 | 精简操作 |
|------|------|---------|
| `AppLayout.vue` | 移除死代码 `globalShortcutsEnabled`，简化 TEXTAREA 守卫 | ✅ 已精简 |
| `WorkspaceTerminal.vue` | 添加 Ctrl+Shift+F 处理 | ✅ 无冗余 |
| `useWorkspacePersistence.ts` | 添加 layout 持久化 | ✅ 无冗余 |

## 精简结果

- 移除了未使用的 `globalShortcutsEnabled` ref 和 `provide` 调用
- 将 TEXTAREA 守卫逻辑从嵌套 if 改为扁平结构
- 无功能变更

## 结论

✅ 精简完成，功能行为不变。
