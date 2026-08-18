# 代码审查：0.70.2

## 变更概览

- **变更文件**：34（含文档/报告；代码文件约 28）
- **审查时间**：2026-08-18
- **审查维度**：dev-flow 内置默认集（正确性 / 安全性 / 健壮性 / 可维护性 / 性能 / 规范）；`AGENTS.md` 无 `## 代码审查维度`，按技能规则不传 dimensions，使用内置默认集。

## 重点审查对象

- `crates/rex-sip/src/video_bridge.rs`（新增 FFI 视频桥）
- `crates/rex-common/src/sip_media.rs`（视频帧编解码 + KIND_VIDEO）
- `crates/rex-hub/src/sip_ws.rs` / `crates/rex-agent/src/agent_ws.rs`（四向媒体帧路由增 kind=2 分支）
- 前端 `sipMedia.ts` / `sip.ts` / `SipPage.vue`（视频帧编解码、kind 分流、开关）

## 维度结论

| # | 维度 | 结论 | 说明 |
|---|------|------|------|
| 1 | 正确性 | ✅ | 编解码边界完整：`encode_video_frame`/`decode_video_frame` 校验尺寸与 fmt，畸形帧返回 Err；四向路由 kind 解复用一致（Hub UA₁/浏览器 ↔ Agent UA₂/浏览器）。`dispatch_sip_tunnel_frame` 与 `sip_ws` 上行分支对称。 |
| 2 | 安全性 | ✅ | 无 SQL 拼接（CDR 用参数化/rusqlite）；媒体帧为二进制字节流无注入面；`decode_video_frame` 不越界（切片 `&bytes[5..]` 受 len<5 前置保护）。 |
| 3 | 健壮性 | ✅ | 畸形视频/音频帧静默丢弃并打 debug 日志，不 panic；`on_video`/`dispatch` 失败仅 debug 记录；FFI 裸指针经 `arg` 回取并判 null。 |
| 4 | 可维护性 | ✅ | 视频桥与 `audio_bridge` 同构，删除行后无死代码；编解码前后端约定对称（Rust `sip_media` ↔ TS `sipMedia`）。 |
| 5 | 性能 | ✅ | 视频帧零拷贝编码（直接切片 extend），无 N+1/额外分配；泵线程仅作哨兵（真链路需 baresip 编解码器），无忙等。 |
| 6 | 规范 | ✅ | `cargo fmt` 干净；`cargo clippy` 退出码 0；提交粒度按子功能拆分；依赖走 `workspace = true`。 |

## 问题列表

| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 1 | 🟢 | crates/rex-sip/src/{audio_bridge,baresip_ua,capture}.rs | 构造处 | baresip FFI `arc_with_non_send_sync` clippy 警告（前序子任务遗留，非本里程碑引入）；CI 默认门禁 exit 0 不受影响。已在步骤4 记入缺陷池 `docs/BUGS.md`，下版本统一清理。 |

## 汇总

- 🔴 必须修复：0
- 🟡 应该修复：0
- 🟢 可选改进：1（前序遗留，已入缺陷池，不阻断）
- **结论**：无 🔴 且无 🟡 → 通过；🟢 入缺陷池，不打回。
