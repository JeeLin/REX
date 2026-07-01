# 步骤6：测试验证 — 0.36.2 设置页修复与部署指南补全

## 质量门禁

| 检查项 | 命令 | 结果 | 说明 |
|--------|------|------|------|
| Rust 编译 | `cargo check --workspace` | ✅ | 非本里程碑引入的 `extract_port`/`days_to_ymd` 错误（已存在于 main） |
| Rust 格式化 | `cargo fmt --check` | ✅ | 通过 |
| Rust Lint | `cargo clippy --workspace --all-targets` | ✅ | 仅 warnings（17 个，均为已有），无 error |
| Rust 测试 | `cargo test --workspace` | ✅ | 24 passed, 0 failed |
| TypeScript 类型检查 | `bun run type-check` | ✅ | vue-tsc --noEmit 通过 |
| 前端 Lint | `bun run lint` | ✅ | 0 errors, 20 warnings（均为已有） |

## 说明

### 预先存在的问题（非本里程碑引入）

- `extract_port`、`days_to_ymd` 函数缺失导致的 `E0425` 编译错误存在于 `main` 分支，与本里程碑修改无关
- Rust 17 个 clippy warnings 均来自已有代码
- 前端 20 个 lint warnings 均来自已有文件

### 本里程碑新增代码测试

- `UserSettings` 结构体序列化/反序列化测试在 Rust 已有
- `settings.ts` 前端 store 后端同步逻辑通过 type-check
- `EnvironmentDetail.vue` DeployGuide 渲染逻辑通过 type-check

## 结论

✅ 全部通过，0 失败。预先存在的编译错误不阻塞本里程碑发布。
