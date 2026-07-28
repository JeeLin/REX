# 代码精简：M51 v0.44.0

## 精简维度

| # | 维度 | 检查结果 | 修复 |
|---|------|----------|------|
| 1 | 重复代码 | `auth.rs` 中 `login()` 和 `change_password()` 重复密码验证逻辑 | 提取 `verify_password()` 私有方法 |
| 2 | 代码组织 | `TerminalView.vue` 两个独立 `onMounted` 可合并 | 合并为单个 `onMounted`，settings listener 移到顶层 |
| 3 | 过度设计 | 无 | — |
| 4 | 文件大小 | 所有文件合理 | — |
| 5 | 项目风格 | 一致 | — |

## 详细修复

### 1. auth.rs：提取密码验证方法

**Before**：`login()` 和 `change_password()` 各自重复实现密码哈希读取 + 验证逻辑（8 行重复代码）。

**After**：提取 `verify_password(&self, password: &str) -> AuthResult<()>` 私有方法，`login()` 和 `change_password()` 均调用此方法。`login()` 返回错误信息 "invalid password"，`change_password()` 将错误映射为 "current password is incorrect"。

### 2. TerminalView.vue：合并生命周期钩子

**Before**：两个独立的 `onMounted` 和一个独立的 `onBeforeUnmount`（settings listener），以及嵌套在 `onMounted` 内部的 `onBeforeUnmount`（resizeObserver + dispose）。

**After**：
- 合并为单个 `onMounted` 处理终端初始化
- 顶层 `onMounted` + `onBeforeUnmount` 处理 settings listener（window 事件）
- 嵌套 `onBeforeUnmount` 仅处理 resizeObserver + dispose

## 验证

- `cargo check --workspace`：✅ 通过
- `bun run type-check`：✅ 通过

## 结论

✅ 无功能变更，仅重构重复代码和改善生命周期组织。
