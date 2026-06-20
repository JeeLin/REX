# M11 步骤4 代码精简报告

## 精简内容

### 1. ContextMenu 组件复用 ✅

ContextMenu.vue 使用全局单例 composable（useContextMenu），所有页面共享同一个菜单状态。App.vue 根级挂载一次，无需每个页面单独引入组件。

### 2. i18n keys 统一 ✅

所有右键菜单项使用 `ctx.*` 命名空间，中英文 key 一一对应，无遗漏。

## 未修改项

- 各页面的菜单定义简洁，无重复逻辑
- composable 逻辑单一，无需拆分

## 结论

精简不改变功能行为，代码组织合理。
