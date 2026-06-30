# 0.32.0 步骤4：代码精简报告

## 检查范围

本次修改涉及 11 个文件（含 Cargo.lock、DEVELOPMENT.md、里程碑文档），重点检查前端代码精简。

## 精简项目

### ✅ 1. 提取重复 watch 逻辑为 composable

**问题**：Files.vue 和 TerminalSftp.vue 都包含相同的 transferTasks watch 代码，用于检测传输状态变化并触发 Toast。

**处理**：提取为 `packages/rex-console-web/src/features/files/useTransferToast.ts`，两页面统一调用 `useTransferToast(transferTasks, prevTasks)`。

### ✅ 2. 补全 TransferQueuePanel props 声明

**问题**：模板中已使用 `speeds?.get(task.id)` 和 `etas?.get(task.id)`，但 `defineProps` 未声明这两个 props。

**处理**：添加 `speeds?: Map<string, number>` 和 `etas?: Map<string, number>` 到 props 声明。

### ✅ 3. useTransferQueue 结构检查

`useTransferQueue.ts` 当前结构：
- 速度/ETA 计算封装在 `computeSpeedAndEta()` 中，职责单一
- `prevBytes` 使用 Map 而非 reactive ref（正确，无需 Vue 响应式追踪）
- `prevTasks` 作为 ref 导出，供 `useTransferToast` 消费

无进一步精简空间。

### ✅ 4. i18n key 组织

zh.ts / en.ts 中新增的 `files.transfer.*` key 统一放在 `files` 命名空间下，与现有 `files.transferFailed` 等扁平 key 并存。本次新增采用嵌套结构，更清晰。

## 未发现问题

- [x] 无重复代码
- [x] 无过度设计（速度计算基于现有轮询，无额外定时器）
- [x] 未提前实现下一阶段能力
- [x] 符合 Vue 功能域结构（features/files/ 下集中管理）
- [x] 未照搬原型交互（按现有组件风格实现）

## 结论

精简不改变功能行为，代码组织更合理。✅
