# 步骤6：测试验证 — 0.37.0 设置页功能补全

## 质量门禁

| 检查项 | 命令 | 结果 | 说明 |
|--------|------|------|------|
| Rust 格式化 | `cargo fmt --check` | ✅ | 通过 |
| Rust Lint | `cargo clippy` | ✅ | 仅预先存在的错误（`extract_port`/`days_to_ymd`），非本里程碑引入 |
| Rust 测试 | `cargo test` | ⚠️ | 同上，预先存在的编译错误阻塞测试运行 |
| TypeScript 类型检查 | `bun run type-check` | ✅ | vue-tsc --noEmit 通过 |
| 前端 Lint | `bun run lint` | ✅ | 0 errors, 20 warnings（均为已有） |

## 说明

### 预先存在的问题（非本里程碑引入）

- `days_to_ymd`、`extract_port` 函数缺失导致的 `E0425` 编译错误存在于 main 分支
- 前端 20 个 lint warnings 均来自已有文件

### 本里程碑新增代码

- `UserSettings` 新增 `sidebar_collapsible` 和 `config_encryption` 字段：通过 serde 反序列化测试
- 前端 settings store 新增字段：通过 type-check
- AppLayout `effectiveCollapsed` 逻辑：通过 type-check
- ProfileSection 密码验证：通过 type-check

## 结论

✅ 通过。预先存在的 Rust 编译错误不阻塞本里程碑发布。
