# 步骤 4：代码精简报告

## 检查范围

0.11.0 里程碑所有变更文件（3 个子任务提交 + 未提交的精简变更）。

## 检查结果

### 1. 未使用的依赖（已修复）

**文件**: `crates/rex-s3/Cargo.toml`

移除 7 个未使用的依赖：`rex-common`、`tracing`、`reqwest`、`hmac`、`sha2`、`hex`、`time`、`base64`。

`connector.rs` 仅导入 `anyhow`、`async_trait`、`serde`、`tokio`，这些在保留的 5 个依赖中已覆盖。

### 2. 内存泄漏：全局事件监听器未清理（已修复）

**文件**: `packages/rex-console-web/src/features/s3/ObjectBrowser.vue`

`document.addEventListener('click', hideCtxMenu)` 在 `<script setup>` 顶层执行，组件卸载时未移除监听器。添加 `onUnmounted` 清理：

```typescript
onUnmounted(() => {
  document.removeEventListener('click', hideCtxMenu)
})
```

### 3. CSS 重复（保留）

`.s3-btn` 类在 `ObjectBrowser.vue`、`BucketList.vue`、`S3Console.vue` 三处重复定义。

**决策**：保留不提取。原因：
- `<style scoped>` 使重复不会互相影响
- 提取为共享 CSS 变量或公共样式文件会增加文件数量和导入复杂度，不符合当前功能域组织方式
- 三个组件的 `.s3-btn` 样式略有差异（padding、white-space 等），统一可能引入不期望的副作用

## 精简总结

| 项目 | 状态 | 影响 |
|------|------|------|
| 未使用依赖移除 | ✅ 已修复 | 缩小 crate 编译依赖 |
| 全局事件监听器泄漏 | ✅ 已修复 | 防止组件卸载后内存泄漏 |
| CSS 重复 | ⏭ 保留 | scoped 隔离，不影响功能 |

## 结论

精简未改变功能行为。所有修复均为代码质量改进。
