# 代码审查：M51 v0.44.0

## 变更概览

- **变更文件**：8 个（Rust 2 + TS 3 + Vue 3）
- **审查时间**：2026-07-28

## 问题列表

| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 1 | 🟡 | `auth.rs` | 110-120 | `change_password` 无密码强度校验（最短长度等），与 `set_password` 一致但可加固 |
| 2 | 🟢 | `TerminalView.vue` / `SettingsPage.vue` | — | `'terminal-settings-changed'` 事件名硬编码在两处，未抽常量 |
| 3 | 🟢 | `SettingsPage.vue` | 67 | `settings.value.language as 'zh' \| 'en'` 类型断言，若后端返回非法值会运行时异常 |

## 汇总

- 🔴 必须修复：0
- 🟡 应该修复：1
- 🟢 可选改进：2
- **结论**：0 个必须修复项 → ✅ 通过

## 详细分析

### 1. 🟡 密码强度校验缺失（auth.rs）

`change_password` handler 直接透传新密码到 `set_password`，无最短长度检查。当前前端 Button 的 `:disabled` 提供了基本保护（空密码不可提交），但后端无校验。

**评估**：`set_password`（首次设置）同样无校验，行为一致。单用户自托管场景下风险较低。建议后续里程碑统一加固。

### 2. 🟢 事件名硬编码

`SettingsPage.vue` dispatch 和 `TerminalView.vue` listen 使用相同的字符串 `'terminal-settings-changed'`，但未抽取为共享常量。拼写错误会导致静默失败。

**评估**：当前两处均正常工作，单用户应用影响有限。

### 3. 🟢 语言类型断言

`locale.value = settings.value.language as 'zh' | 'en'` 使用了类型断言。若后端存储了非法语言值，`vue-i18n` 会 fallback 到默认语言，不会崩溃。

**评估**：风险极低，设置值通过 `<select>` 组件限定。

## 安全审查

- ✅ `/api/auth/change-password` 路由在 `protected_routes` 中，需 JWT 认证
- ✅ 审计日志正确记录 `AUTH_PASSWORD_CHANGED`
- ✅ 错误消息区分"密码错误"和"内部错误"，返回不同 HTTP 状态码
- ✅ `readToken()` 同时检查 localStorage 和 sessionStorage，401 时清除两者
- ✅ `verify_password` 不泄露密码哈希信息
