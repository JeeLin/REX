# 0.70.2 步骤6：测试验证

## 质量门禁

依据 `AGENTS.md` 质量门禁（Rust 跳过编译检查，因前置 `cargo check --locked` 已验证 `Cargo.lock` 一致）。

### Rust

| 检查项 | 命令 | 结果 | 说明 |
|--------|------|------|------|
| 依赖一致性（前置） | `cargo check --locked` | ✅ | `Cargo.lock` 与 `Cargo.toml` 一致 |
| 格式 | `cargo fmt --check` | ✅ | 退出码 0，无格式漂移 |
| Lint | `cargo clippy --workspace --all-targets` | ✅ | 退出码 0（仅 warning，无 error；warning 不阻断 CI） |
| 测试 | `cargo test --workspace` | ✅ | 全 crate 0 failed |

**关键 crate 测试计数**：
- rex-common：15 passed
- rex-hub：84 passed（含 `api_integration` 集成测试）
- rex-sip：14 passed（含 `video_bridge` round-trip、`sip_media` 视频帧编解码、`capture` pcap、`sip_recording` 落盘）
- rex-agent：14 passed（媒体帧隧道路由）
- 其余 crate（rex-ssh/rex-transfer/rex-mysql/rex-postgresql/rex-sqlite/rex-redis/rex-s3）：0 failed
- 总计：**0 failed，0 error**

> 覆盖率：AGENTS.md 质量门禁未定义 Rust/前端覆盖率命令（默认 JS 90% 门禁被 AGENTS.md 覆盖为空），本步骤不要求覆盖率达标，仅跑通测试。

### 前端（`packages/rex-console-web/`）

| 检查项 | 命令 | 结果 | 说明 |
|--------|------|------|------|
| 类型检查 | `bun run type-check` | ✅ | `vue-tsc --noEmit` 退出码 0 |
| Lint | `bun run lint` | ✅ | 0 error，41 warning（warning 不阻断，按 AGENTS.md 规则可接受） |
| 构建 | `bun run build` | ✅ | `built in 7.15s`，产物正常 |

## 结论

全部门禁通过：Rust 编译/Lint/测试无 error、前端 type-check/lint/build 全绿。无失败项，无新增 🔴/🟡 问题。

→ 勾选步骤6，进入步骤7（设计再确认）。
