# M82b 步骤4：代码精简（0.70.1）

## 范围

`git diff --name-only milestone-0.70.1-start HEAD` 的 26 个变更文件，按 CLAUDE.md 的 simplify 维度逐项排查：
重复代码 / 过度设计 / 提前实现下一阶段能力 / workspace 依赖规则 / 大文件是否可拆 / 风格一致性。

## 发现

### 🟡 删除死代码：`sip_media.rs` 未引用的枚举与常量（步骤4代码精简）

**位置**：`crates/rex-common/src/sip_media.rs`

- `pub enum MediaKind { Down, Up }`（第 41-47 行）：全仓库 `grep -rn MediaKind --include=*.rs` 仅命中其自身定义，零使用点。
- `pub const MEDIA_SRATE: u32 = 8000;`（第 12 行）与 `pub const MEDIA_CHANNELS: u8 = 1;`（第 14 行）：仅定义处命中；前端 `sipMedia.ts` 自行定义了同源 TS 常量（`MEDIA_SAMPLE_RATE` / `MEDIA_CHANNELS` / `PCM_FRAME_SAMPLES`），Rust 侧这两个常量无任何引用。

**分析**：媒体方向（下行/上行）实际由数据流向隐含（直连 WebSocket 帧无 kind 字节，隧道帧靠 `KIND_SIGNAL`/`KIND_MEDIA` 区分），`MediaKind` 枚举属设计期预留但未落地，属死代码。`MEDIA_SRATE`/`MEDIA_CHANNELS` 与前端常量重复定义、Rust 侧未消费，属无用导出。

**走向**：🟡（可维护性差：死代码）→ 登记入里程碑 Bugs 表（状态 ⬜），按打回动作取消步骤3/4/5/6/7 勾选，回开发阶段删除后重跑 4→5→6→7。

### 其余维度结论

- **重复逻辑**：PCM 编解码（encode/decode）、隧道帧 wrap/unwrap 已集中于 `rex_common::sip_media`，Hub（`crates/rex-hub/src/sip_media.rs` 再导出）与 Agent（`agent_ws.rs` 调用）共用，无重复实现。
- **过度设计 / 提前实现**：媒体抽象已按「媒体无关」设计（kind 字节区分信令/媒体、payload 为原始 PCM），但未为视频等下一阶段预埋未使用的接口/字段，符合 M82b 边界。
- **workspace 依赖规则**：未新增 crate 级依赖；既有依赖均在根 `Cargo.toml` 声明、`workspace = true` 复用，无子 crate 重复声明。
- **大文件拆分**：`audio_bridge.rs`(386) / `baresip_ua.rs`(690) 为 baresip FFI 与泵线程核心，单一职责清晰，当前无需拆分。
- **风格一致性**：Rust 走 `cargo fmt`、前端走 ESLint（`noUncheckedIndexedAccess` 已用 `?? 0` 兜底），命名与既有模块一致。

## 门禁判断

存在 🟡 发现（死代码）→ **不勾选步骤4**，按打回动作处理：登记 Bugs 表、取消步骤3/4/5/6/7 勾选、回开发阶段修复后重跑 4→5→6→7。
