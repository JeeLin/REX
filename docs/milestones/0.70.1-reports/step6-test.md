# 测试验证：0.70.1（M82b）

## 验证时间

2026-08-18

## 验证命令与结果

### Rust — 编译一致性

| 检查项 | 命令 | 结果 |
|--------|------|------|
| Cargo.lock 一致性 | `cargo check --locked` | ✅ Finished（无 dependency 漂移） |

> 通过后跳过 dev-flow 质量门禁中的「编译检查」项（已验证 `Cargo.lock` 与 `Cargo.toml` 一致）。

### Rust — 测试

| Crate | 命令 | 结果 |
|-------|------|------|
| rex-common | `cargo test -p rex-common` | ✅ 15 passed |
| rex-hub（sip_ws 模块） | `cargo test -p rex-hub --lib sip_ws` | ✅ 21 passed |
| rex-sip | `cargo test -p rex-sip` | ✅ 7 passed |
| rex-agent | `cargo test -p rex-agent` | ✅ 13 passed |

合计划分四条链路（Hub 直连下行/上行、Agent 链式下行/上行、隧道帧 kind 区分、子任务 1-3 单测）全部绿。

### Rust — Lint

| 检查项 | 命令 | 结果 |
|--------|------|------|
| Clippy（里程碑 4 crate） | `cargo clippy -p rex-common -p rex-hub -p rex-agent -p rex-sip --all-targets` | ✅ 无 error（仅 2+6 条 warning，均为步骤5 已登记的 🟢 缺陷池项：`cast_same_type` 与 baresip FFI 样式建议，不阻断） |

### 前端 — 类型 / Lint / 构建

| 检查项 | 命令 | 结果 |
|--------|------|------|
| 类型检查 | `bun run type-check`（vue-tsc --noEmit） | ✅ 0 error |
| Lint | `bun run lint`（ESLint） | ✅ 0 error（35 warning，均为存量 style 提示，无新增 error） |
| 构建 | `bun run build`（vue-tsc -b && vite build） | ✅ built in 7.18s |

### 前端 — 测试与覆盖率

| 检查项 | 命令 | 结果 |
|--------|------|------|
| 全量 vitest | `bunx vitest run` | ✅ 15 文件 / 147 tests passed（含本次新增覆盖补测） |

**里程碑文件覆盖率**（`--coverage.include` 聚焦本里程碑变更文件）：

| 文件 | % Stmts | % Branch | % Funcs | % Lines |
|------|---------|----------|---------|---------|
| `src/api/sip.ts` | 98.03 | 70.58 | 95 | **100** |
| `src/api/sipMedia.ts` | 96.84 | 79.06 | 100 | **100** |
| `src/features/sip/CallState.vue` | 100 | 96.29 | 100 | **100** |
| `src/features/sip/SipPage.vue` | 90.21 | 69.04 | 100 | **91.66** |
| **聚合** | 87.84 | — | — | **90.62** |

门禁阈值 90%：聚合 Lines 90.62% ≥ 90%，全部单文件 Lines ≥ 90% ✅。

## 覆盖率补测说明

步骤6 初测覆盖率低于 90% 门禁（SipPage.vue 76%、sipMedia.ts 85%），已补充以下针对性单测把两条文件拉到门禁之上：

- `sipMedia.test.ts`：`playback onaudioprocess` 灌流播放队列到输出缓冲（覆盖 89-104 播放 drain 循环）；`startMic` 浮点→i16 截断转换（mock `navigator.mediaDevices.getUserMedia`）。
- `SipPage.test.ts`：dtmf 按键 → `sip.dtmf`；ws close/error → status emit；麦克风开关 → 上行 PCM 二进制帧发送 + 关闭；decline → `sip.hangup` 清 incoming；hold→held→unhold 双向信令。

> 注：`0.5 * 0x7fff` 经 `Int16Array` 赋值时**向零截断**为 16383（非四舍五入 16384），属 PCM 正确行为，测试断言已对齐该实现。

## 门禁判定

| 门禁项 | 结果 |
|--------|------|
| 测试全部通过（Rust 56 + 前端 147） | ✅ |
| 编译无 error（`cargo check --locked`） | ✅ |
| Lint 无 error（clippy / eslint） | ✅ |
| 覆盖率达标（聚合 ≥ 90%） | ✅ |

**结论**：步骤6 门禁全部通过 → 勾选步骤6。
