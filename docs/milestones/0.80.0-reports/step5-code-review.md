# Step 5: 代码审查报告

## 版本
0.80.0

## 检查时间
2026-07-09

## 审查摘要

**总发现数**: 15
- 🔴 必须修复: 2（已修复）
- 🟡 应该修复: 7（部分修复）
- 🟢 可选改进: 6

## 🔴 必须修复（已修复）

### 1. useThemeObserver 生命周期钩子在 setup 外调用 — 内存泄漏

**文件**: SqlCodeMirror.vue, WorkspaceTerminal.vue

**问题**: `useThemeObserver()` 在 `onMounted()` 回调中调用，但 composable 内部使用 `onBeforeUnmount()` 注册清理函数。由于 `onMounted` 回调在 setup 之后执行，Vue 无法将钩子与组件关联，导致 MutationObserver 永远不会断开。

**修复**: 将 `useThemeObserver()` 调用移到 `<script setup>` 顶层作用域。

### 2. Global Query 缺少 Authorization Header

**文件**: useGlobalQuery.ts

**问题**: `executeGlobalQuery()` 使用原生 `fetch()` 但未添加 Authorization header，而其他 API 调用都使用共享 axios 客户端或手动添加 header。

**修复**: 添加 `Authorization: Bearer ${localStorage.getItem('rex-token') || ''}` header。

## 🟡 应该修复（未修复，记录供后续处理）

### 3. copyToken 上下文菜单项无动作
**文件**: AgentCard.vue:67
**描述**: "Copy Token" 菜单项没有 action 回调

### 4. connectNewTab 与 connect 功能相同
**文件**: AppLayout.vue:387-388
**描述**: 两个菜单项调用相同参数，标签误导

### 5. SQL 格式化复合关键字处理顺序错误
**文件**: SqlCodeMirror.vue:208-213
**描述**: JOIN 在 LEFT JOIN 之前处理，导致 LEFT\nJOIN 而非 \nLEFT JOIN

### 6. EditorView 主题切换时销毁重建丢失状态
**文件**: SqlCodeMirror.vue:121-133
**描述**: 切换主题时整个 EditorView 被销毁重建，丢失撤销历史、选区等状态

### 7. 模态框静默吞掉错误
**文件**: EnvironmentEditModal.vue, ResourceEditModal.vue, Workspace.vue
**描述**: 多个 catch 块静默丢弃错误，用户无反馈

## 🟢 可选改进（未修复）

### 8-15. 可访问性和代码质量改进建议
- 模态框关闭按钮缺少 aria-label
- 硬编码 z-index 未使用设计 token
- Agent 卡片缺少键盘可访问性
- GlobalQueryModal 缺少 role=dialog
- 等

## 结论

🔴 必须修复项已全部修复。🟡 应该修复项记录供后续里程碑处理。审查通过。

**检查项**:
- ✅ TypeScript 类型检查通过
- ✅ ESLint 无 error
- ✅ 构建成功
