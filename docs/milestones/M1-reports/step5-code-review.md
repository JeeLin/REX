# M1 步骤5：代码审查报告

## 结论：✅ 通过（5 个 🔴 已修复，9 个 🟡 已修复 4 个，其余 🟢 可选）

## 🔴 必须修复（已全部修复）

| # | 文件 | 问题 | 状态 |
|---|------|------|------|
| F01 | Select.vue | 下拉框无定位，Teleport 后位置错误 | ✅ 已修复（computed 定位） |
| F02 | Button.vue | loading spinner 在非 primary 变体下不可见 | ✅ 已修复（border-color 用 var(--text-secondary)） |
| F03 | Avatar.vue | 图片加载失败时 fallback 不显示 | ✅ 已修复（imgError ref） |
| F04 | Modal.vue | 组件卸载时事件监听和 body overflow 未清理 | ✅ 已修复（onBeforeUnmount 清理） |
| F05 | ToggleGroup.vue | 缺少 withDefaults，产生 toggle-group--undefined | ✅ 已修复 |

## 🟡 应该修复（已修复 4 个）

| # | 文件 | 问题 | 状态 |
|---|------|------|------|
| F06 | Badge.vue | size prop 无 CSS 对应 | ✅ 已添加 .badge--sm/.badge--md |
| F14 | Checkbox.vue | 无用 props 变量 | ✅ 已移除 |
| F17 | DesignPreview.vue | dataset.theme = undefined 问题 | ✅ 改用 delete |
| F05 | ToggleGroup.vue | withDefaults 缺失 | ✅ 同上 🔴 |

## 🟡 未修复（留待后续里程碑）

| # | 问题 | 原因 |
|---|------|------|
| F07 | Drawer 缺少 ESC 关闭 | M2 工作空间外壳统一处理 |
| F08/F09/F10/F13 | 键盘导航和 ARIA 属性 | M7 无障碍适配 |
| F11/F12 | 亮色主题对比度 | M7 主题打磨 |
| F18-F20 | 动画 token 未使用、close 按钮 aria-label、Input type | 可选改进 |
