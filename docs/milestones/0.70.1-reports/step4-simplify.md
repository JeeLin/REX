# M82b 步骤4（重跑）：代码精简（0.70.1）

> 本文件为重跑版本。初版 step4-simplify.md 因命中 🟡 死代码发现已被打回（重命名为
> `step4-simplify.md.rejected`），开发阶段删除死代码（`3553aed9`）后重跑本步骤。

## 范围

`git diff --name-only milestone-0.70.1-start HEAD` 的 26 个变更文件，按 CLAUDE.md 的 simplify 维度
（重复代码 / 过度设计 / 提前实现 / workspace 依赖 / 大文件拆分 / 风格一致性）逐项排查。

## 发现

### 初版发现（已在开发阶段修复，提交 `3553aed9`）

- 🟡 死代码：`crates/rex-common/src/sip_media.rs` 的 `MediaKind` 枚举与 `MEDIA_SRATE`/`MEDIA_CHANNELS`
  常量全仓库零引用。已在开发阶段删除（连同相关注释），本次重跑前已并入 main。

### 重跑扫描结论

- **死代码**：`grep -rn "MediaKind\|MEDIA_SRATE\|MEDIA_CHANNELS" --include=*.rs crates/` 结果为 0；
  `cargo build -p rex-common -p rex-hub -p rex-agent` 无任何 warning（无 unused / never used 信号）。
  初版唯一 🟴/🟡 发现已消失。
- **重复逻辑**：PCM 编解码（encode/decode）、隧道帧 wrap/unwrap 已集中于 `rex_common::sip_media`，
  Hub（`crates/rex-hub/src/sip_media.rs` 再导出）与 Agent（`agent_ws.rs` 调用）共用，无重复实现。
- **过度设计 / 提前实现**：媒体抽象按「媒体无关」设计（kind 字节区分信令/媒体、payload 为原始 PCM），
  未预埋下一阶段（视频等）未使用接口/字段，符合 M82b 边界。
- **workspace 依赖规则**：未新增 crate 级依赖；既有依赖均在根 `Cargo.toml` 声明、`workspace = true` 复用。
- **大文件拆分**：`audio_bridge.rs` / `baresip_ua.rs` 为 baresip FFI 与泵线程核心，单一职责清晰，无需拆。
- **风格一致性**：Rust 走 `cargo fmt`、前端走 ESLint（`noUncheckedIndexedAccess` 已用 `?? 0` 兜底）。

## 门禁判断

重跑无任何 🔴/🟡 发现（初版 🟡 已在开发阶段修复，无 🟢 入缺陷池项）→ **勾选步骤4**。
