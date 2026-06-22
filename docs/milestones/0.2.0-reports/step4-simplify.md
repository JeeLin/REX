# 0.2.0 步骤4：代码精简报告

## 检查范围

`git diff main~5..main` — 本次里程碑的所有变更（15 个文件，689 行新增）

## 发现与修复

### 1. 审计统计查询重复（Simplification） ✅ 已修复

**问题：** `audit.rs` get_stats 函数执行 3 个独立的 COUNT 查询（total、success、failed），可合并为 1 个带 CASE 表达式的查询。

**修复：** 合并为单条 SQL：
```sql
SELECT COUNT(*),
       SUM(CASE WHEN result='success' THEN 1 ELSE 0 END),
       SUM(CASE WHEN result='failure' THEN 1 ELSE 0 END)
FROM audit_log [WHERE time >= ?1]
```

### 2. 密码哈希逻辑重复（Reuse） ✅ 已修复

**问题：** `auth.rs` login 和 `user.rs` change_password 中的密码哈希查找 + 默认密码生成逻辑完全重复。

**修复：** 提取到 `helpers.rs` 中的 `get_or_create_password_hash(db)` 共享函数，两个模块统一调用。

### 3. Modal CSS 重复（Altitude） ⏭ 跳过

**问题：** `ProfileSection.vue` 和 `Terminal.vue` 中的 modal-overlay/modal/modal-title/modal-actions 样式重复。

**跳过原因：** 两个组件使用 `<style scoped>`，CSS 隔离不影响运行时。提取共享组件需要改动多个文件，超出本次精简范围。

## 结论

精简不改变功能行为，减少 33 行重复代码（-88 行 / +55 行）。
