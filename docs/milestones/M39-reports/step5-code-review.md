# M39 代码审查报告

## 审查发现

### 🔴 必须修复（已修复）

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 1 | 警告对话框「登出」按钮不清除 auth token，路由守卫将用户重定向回 workspace | AppLayout.vue | 改用 `authStore.logout()` + `window.location.href` |
| 2 | i18n `{seconds}` 参数从未传递，显示为字面量 `{seconds}` | zh.json, en.json | 移除 `{seconds}` 参数 |
| 3 | `NaN` timeout 禁用会话超时安全功能 | useSessionTimeout.ts | 添加 `Number.isFinite` 验证 |
| 4 | CommandPalette 全局 keydown 在面板隐藏时仍然触发 | CommandPalette.vue | 添加 `if (!props.visible) return` |

### 🟡 应该修复（已修复）

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 5 | 语言切换命令无 action，点击不生效 | CommandPalette.vue | 添加 `locale.value` 设置 |

### 🟢 可选改进（未处理，预存问题或低影响）

| 项 | 原因 |
|----|------|
| O(n) indexOf 性能 | 资源数量 <100，影响极小 |
| 警告对话框无 focus trap | 单用户工具，无障碍优先级低 |
| setTimeout 未清理 | 2 秒超时，组件卸载风险极低 |
| 硬编码英文字符串 | 预存问题，非 M39 引入 |

## 结论

✅ 无 🔴 必须修复项。所有关键问题已修复。
