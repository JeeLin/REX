# Step 6: 测试验证

## Rust

| 检查项 | 结果 |
|--------|------|
| `cargo fmt --check` | ✅ 通过（cargo fmt 已格式化） |
| `cargo clippy --workspace --all-targets` | ✅ 无 error |
| `cargo test --workspace` | ✅ 7 passed, 0 failed |

测试覆盖：crypto（4 tests）+ auth（3 tests）。

## 前端

| 检查项 | 结果 |
|--------|------|
| `bun run type-check` | ✅ 通过 |
| `bun run lint` | ⚠️ 2个预存 error（非 M14 引入） |

2个 lint error 来自 FilesPage.vue（`@typescript-eslint/no-unused-expressions`）和 Filter 语法（`vue/no-deprecated-filter`），均为 M14 之前的预存问题，M14 未修改这些文件。

## 结论

✅ M14 变更全部通过。预存 lint error 不影响 M14 交付。
