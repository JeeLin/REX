# M10 步骤4 代码精简报告

## 精简内容

### 1. 提取共享 SettingsSection 组件 ✅

**问题**：AppearanceSection、TerminalSection、SecuritySection 三个组件各自重复定义了相同的 CSS 变量类（`.settings-section`、`.settings-section-header`、`.settings-section-body`、`.settings-row`、`.settings-toggle`、`.form-select`），约 120 行重复样式。

**修复**：创建 `SettingsSection.vue` 共享壳组件，将重复样式提取为非 scoped 全局类。三个子组件只需保留各自的特有样式（主题选择器、语言切换按钮等）。

**结果**：删除约 120 行重复 CSS，三个子组件体积显著缩小。

### 2. UpdateSection 保持不动 ✅

UpdateSection 的布局模式（version-info、update-available 等）与其他 settings section 不同，不纳入共享组件，避免改变已有功能。

## 未修改项

- AuditLog.vue 的样式全部为页面特有，无重复可提取
- i18n keys 结构清晰，无需调整
- 路由和侧边栏逻辑简洁

## 结论

精简不改变功能行为，只消除重复代码。
