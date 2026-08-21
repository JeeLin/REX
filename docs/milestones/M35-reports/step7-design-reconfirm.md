# M35 Step 7: 设计再确认报告

## 实现 vs 里程碑文档

| 子任务 | 设计要求 | 实现情况 |
|--------|----------|----------|
| 1 快捷键面板 | F1 触发，分组展示，Esc 关闭 | ✅ ShortcutPanel.vue + WorkspacePage.vue F1 绑定，Escape 键监听已修复 |
| 2 编码子菜单 | 右键菜单 → 编码 ▸ UTF-8/GBK/ISO-8859-1 | ✅ TerminalContextMenu + TerminalView 编码状态管理 |
| 3 状态栏增强 | 编码显示+动态切换 | ✅ 状态栏读取 tab.encoding，默认 UTF-8 |
| 4 删除死代码 | 删除 QuickConnect.vue 及引用 | ✅ 已删除，无残留引用 |

## 路由修复

resource_routes 合并到 env_routes nest 块，顺序：resource → agent → env，避免 `/{id}` 拦截。

## 结论

✅ 实现与里程碑文档一致，产品语义无变更。
