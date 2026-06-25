# 步骤4：代码精简报告

## 精简对象

`crates/rex-hub/src/backup.rs` + `crates/rex-hub/Cargo.toml` + 根 `Cargo.toml`

## 检查维度

### 1. 重复代码 ✅ 已修复

- 提取 `decode_backup_data()` 辅助函数，消除 `preview_backup` 和 `import_backup` 中重复的解密/解析逻辑（原 6 行 × 2 处 → 1 行 × 2 处 + 1 个辅助函数）
- 精简 `export_backup` 中 `BackupFile` 构建的重复字段赋值（两个分支共享字段统一构建）

### 2. 依赖规则 ✅ 已修复

- `chrono = "0.4"` 和 `getrandom = "0.2"` 从 rex-hub/Cargo.toml 移至根 Cargo.toml workspace.dependencies
- rex-hub/Cargo.toml 改为 `chrono = { workspace = true }` / `getrandom = { workspace = true }`

### 3. 过度设计 ✅ 无

- 没有提前实现下一阶段能力
- 没有引入不必要的抽象

### 4. 文件结构 ✅ 合理

- 单文件 900+ 行，但逻辑集中（导出/导入/加密/预览/HTTP handler），暂不需要拆分
- 前端按功能域组织：`api/backup.ts` + `features/settings/BackupSection.vue`

### 5. 功能不变 ✅

- 所有精简均为内部重构，外部 API 行为和数据格式不变

## 结论

**✅ 精简完成**，不改变功能行为。
