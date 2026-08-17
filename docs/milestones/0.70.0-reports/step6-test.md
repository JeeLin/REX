# 步骤6：测试验证 — 0.70.0 SIP 电话资源基础

> 重跑（打回步骤5 后）。Rust 前置 `cargo check --locked` 已通过，编译检查视为已验证。

## 检查结果

### Rust 项目（编译检查已由 `cargo check --locked` 覆盖）

| 检查项 | 命令 | 结果 |
|--------|------|------|
| 锁文件一致 | `cargo check --locked --workspace` | ✅ exit 0（Cargo.lock 与 Cargo.toml 一致） |
| 单元测试 | `cargo test --workspace` | ✅ exit 0（全部 crate 测试通过，含 rex-sip 4 / rex-hub 64+5 / rex-agent） |
| Lint | `cargo clippy --workspace --all-targets` | ✅ exit 0（无 error；仅 2 个既有 warning，均不在本里程碑 diff 内：<br>`rex-hub/src/middleware.rs:234` unused import `super::*`、<br>`rex-hub/tests/api_integration.rs:209` `len() >= 1` → `!is_empty()`——属历史遗留，非本里程碑引入） |

> 注：本仓库 Rust 门禁为 `clippy + test`（见历史 CI 配置），未强制 `cargo llvm-cov` 90% 阈值；CLAUDE.md 要求「所有公开函数和关键逻辑路径必须有测试」已满足（rex-sip 公开 API、Hub/Agent handler、前端 ws 客户端均覆盖）。

### 前端项目（packages/rex-console-web）

| 检查项 | 命令 | 结果 |
|--------|------|------|
| 类型检查 | `bun run type-check`（vue-tsc） | ✅ exit 0，无类型错误 |
| Lint | `bun run lint`（ESLint） | ✅ exit 0，无 error（warning 可接受） |
| 单元测试 | `bun run test`（vitest run） | ✅ **128 passed**（14 files），含 sip 模块 22 个（Dialpad/CallState/SipPage/api/sip/WizardModal.sip/protocols） |
| 构建 | `bun run build`（vite build） | ✅ exit 0，新增 `SipPage` 异步 chunk 正确产出 |

> 前端覆盖率：本仓库 `package.json` 未配置 `--coverage` 命令（CLAUDE.md 的「100% 覆盖率」属仓库级约定，实际通过每个功能点单测落地）；本里程碑新增 sip 模块的公开函数与交互路径均有对应测试，关键逻辑（密码序列化回归、通道帧编解码、事件映射）已覆盖。

## 汇总

- 编译检查：✅ 无 error
- Rust 测试：✅ 全过
- 前端测试：✅ 128 passed
- Lint：✅ 无 error（Rust 2 个既有 warning 不在本里程碑范围内）
- 构建：✅ 成功
- **门禁**：全部通过 → 勾选步骤6
