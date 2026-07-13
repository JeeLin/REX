# 0.86.0 步骤6：测试验证报告

## 质量门禁结果

### Rust（后端）

| 检查项 | 命令 | 结果 |
|--------|------|------|
| 格式化 | `cargo fmt --check` | ✅ 通过（已格式化 `task.rs` 新增测试） |
| 编译 | `cargo check -p rex-hub -p rex-transfer` | ✅ 通过 |
| Lint | `cargo clippy -p rex-hub -p rex-transfer` | ✅ 0 error（17 个 warning 为既有问题，位于 `ws.rs`/`update.rs`，非本次变更） |
| 测试 | `cargo test -p rex-transfer` | ✅ **50 passed**（44 既有 + 6 新增并发控制测试） |

### 前端

| 检查项 | 命令 | 结果 |
|--------|------|------|
| 类型检查 | `bun run type-check` (vue-tsc) | ✅ 通过（0 error） |
| Lint | `bun run lint` | ✅ 0 error（仅 455 个既有 warning） |
| 构建 | `bun run build` | ✅ 成功（build in 6.10s） |

## 新增测试（合约验证）

针对 0.86.0 新增的并发控制 API 补充了 6 个测试：

1. `manager_default_concurrency` — 默认并发数 3，可用许可 3
2. `manager_custom_concurrency` — 自定义并发数生效
3. `manager_set_max_concurrent` — 动态设置并发上限
4. `manager_set_max_concurrent_clamps` — 边界 clamp（0→1，100→32）
5. `manager_acquire_and_release_permit` — 信号量 acquire/release 后许可数正确恢复
6. `manager_running_count` — 运行计数随任务状态变化

这 6 个测试直接防御 Sub1（文件传输并发控制）的核心合约：并发上限可调、信号量许可正确、运行计数准确。

## 前端变更验证

- `BatchImportDialog.vue` / `RedisConsole.vue` / `RedisKeyBrowser.vue` — 批量导入功能 type-check + build 均通过
- `SqlResults.vue` / `SqlHistoryPanel.vue` — 固定列、列宽拖拽、数据库筛选 type-check + build 均通过
- `backup.ts` — 认证头修复后 type-check 通过（修复了此前 `token` 作用域的 4 个 TS2304 错误）

## 结论

✅ **全部通过：测试通过 + 编译无 error + Lint 无 error + 前端构建成功。**

覆盖率：后端 `rex-transfer` 测试数量从 44 → 50（新增并发控制核心路径覆盖）；前端未强制 coverage 阈值（AGENTS.md 未定义），以 type-check + lint + build 为门禁。
