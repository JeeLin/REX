# 代码审查：0.70.1（M82b）

## 变更概览

- **变更文件**：26（含 4 个测试文件、3 个 i18n、2 个文档、21 个代码/测试）
- **审查时间**：2026-08-18
- **审查维度**：AGENTS.md 无 `## 代码审查维度`，回退至 devflow-review 内置默认集
  （正确性 / 安全性 / 健壮性 / 可维护性 / 性能 / 规范）

## 正确性核查（端到端媒体管线）

逐跳读取四个链路的帧路由，确认逻辑闭环、无方向错乱、无 kind 错配：

1. **Hub 直连下行**：`handle_sip_session` 的 `on_rtp` 回调 → `encode_pcm_frame` →
   `Outbound::Binary` → 单一 writer → 浏览器（`sip_ws.rs:502-505`）。✅
2. **Hub 直连上行**：浏览器 `Message::Binary` → `decode_media_frame` → `send_audio`
   （`sip_ws.rs:537-544`，空帧跳过）。✅
3. **Agent 链式下行**：`run_sip_ua2` 的 `on_rtp` → `wrap_tunnel_frame(KIND_MEDIA)` →
   叠加 channelId 前缀 → 隧道；Hub `handle_agent_sip` `unwrap_tunnel_frame` →
   `KIND_MEDIA` → `Outbound::Binary` 原样推浏览器（`agent_ws.rs:683-703` +
   `sip_ws.rs:345-356`）。✅
4. **Agent 链式上行**：浏览器 `Message::Binary` → `wrap_tunnel_frame(KIND_MEDIA)` +
   channelId → Agent `dispatch_sip_tunnel_frame` → `KIND_MEDIA` → `decode_media_frame`
   → `send_audio`（`sip_ws.rs:308-319` + `agent_ws.rs:789-807`）。✅
5. **前端**：`sip.ts` `binaryType='arraybuffer'`，`ArrayBuffer` → `onMedia` →
   `decodeMediaFrame` → `playPcm`；上行 `encodePcmFrame` → `sendMediaFrame`
   （`sip.ts:62-92` + `SipPage.vue:62-97`）。✅
6. **共享编码**：PCM 编解码 / 隧道帧 wrap/unwrap 集中于 `rex_common::sip_media`，
   Hub（`crates/rex-hub/src/sip_media.rs` 再导出）与 Agent 共用，line 格式一致。✅
7. **挂断清理**：`ended` 状态 `teardownAudio`（`close` 重置 playQueue/playOffset +
   停 mic + 关 AudioContext），`onBeforeUnmount` 同样清理（`SipPage.vue:68-74,99-103,148-152`）。✅

安全/健壮性：WebSocket 鉴权复用既有 JWT query token；二进制帧解码对非偶数长度
丢弃孤立字节（`decode_media_frame`）、对空帧跳过（`pcm.is_empty()` 守卫），
无越界/panic 风险；baresip 泵线程内 `on_rtp` 同步回调仅 `try_send` 不阻塞。无注入/
权限/敏感信息泄露点。

## 问题列表

| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 1 | 🟢 | `crates/rex-common/src/sip_media.rs` | 27-28 | `bytes[i*2] as u8` 冗余类型转换（`bytes` 为 `&[u8]`，索引即 `u8`）；clippy `cast_same_type`。纯风格，无功能影响。 |
| 2 | 🟢 | `crates/rex-sip/src/audio_bridge.rs` / `baresip_ua.rs` | 71/78/83/90、194 | clippy 对 baresip FFI 封装的样式建议：`very complex type` / `consider Default for AudioBridge` / `Arc not Send+Sync`（裸指针 `Box<dyn>` 包裹的已知误报）。属 FFI 既定模式，非缺陷。 |

> 注：`rex-hub/src/middleware.rs:234` 与 `rex-hub/tests/api_integration.rs:209` 的 clippy
> 告警**不在本里程碑变更文件内**（`git diff milestone-0.70.1-start` 未含），属历史存量，
> 不计入本审查处置。

## 汇总

- 🔴 必须修复：0
- 🟡 应该修复：0
- 🟢 可选改进：2（均入缺陷池，不阻断）
- **结论**：无 🔴/🟡 发现 → **通过**（🟢 记入缺陷池 `docs/BUGS.md`，下个版本统一规划）。
