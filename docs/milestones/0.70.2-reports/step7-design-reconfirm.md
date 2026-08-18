# 0.70.2 步骤7：设计再确认（代码 vs 里程碑文档）

## 审查维度

| # | 维度 | 结论 | 说明 |
|---|------|------|------|
| 1 | 单用户、自托管定位 | ✅ | 录音/抓包/CDR 均为单一用户本地 Hub 产物，无多租户/账户隔离；浏览器仅取 `recording_url`/`pcap` 下载链接。 |
| 2 | 架构一致性 | ✅ | 媒体通道复用 M82b `[kind][payload]`：音频 `KIND_MEDIA=1`（S16LE PCM）、视频 `KIND_VIDEO=2`（RGBA 像素）在 `sip_media.rs` 定义并在 `sip_ws.rs`/`agent_ws.rs` 四向路由一致解复用。`Cargo.toml` 仍 `0.70.x`，Hub/Agent 版本一致模型未破坏。 |
| 3 | 产品边界 | ✅ | 未引入多用户/RBAC/企业协作；M82b 音频契约未改（`KIND_MEDIA=1` 上行/下行 PCM 路径不变）；视频仅扩展媒体类型（`KIND_VIDEO=2`），不破坏既有音频帧结构。 |
| 4 | 设计 token 使用 | ✅（🟢 备注） | 前端主用设计 token：`SipPage.vue` 状态色用 `var(--danger, #e5484d)`/`var(--accent, #4a9eff)`（token + 回退，合规）。仅视频 `<canvas>` 背景写死 `background: #000`（中性黑，媒体画布惯用），属 🟢 可选改进，不阻断。 |
| 5 | 缺陷收口向后兼容 | ✅ | `## Bugs` 9 条全部 `[x]`：🔴 更新检查（已 `compare_version` 语义化比较）、🟡 审计分页/分栏聚焦/S3 单栏/SQL 子类归并、🟢 移动端/转换清理均已处置或经验证无需改动；S3 单栏落在 `FilesPage.vue:532`（`v-if="!(isS3 && side==='right')"`）。均为内部重构，无对外破坏性变更。 |

## 关键实现核对点

- **视频框架层（#1）**：`video_bridge.rs` 与 `audio_bridge` 同构；`sip_media.rs` 视频帧编解码 round-trip 单测覆盖；隧道帧路由 kind=2 上行/下行单测（`sip_ws.rs`/`agent_ws.rs`）。
- **录音（#2）**：`sip_recording.rs` `write_wav` 头部断言 + `SipRecordingRegistry` 按 call 分文件落盘，URL 回填 CDR。
- **抓包（#3）**：`capture.rs` `sip_trace` 钩子 + `encode_pcap`（LINKTYPE_RAW），`sip_capture.rs` 合并 UA₁ 全局 + UA₂ 中继。
- **CDR（#4）**：`db.rs` 增 `calls` 表，`cdr_api.rs` 列表/详情 API（稳定排序 `ORDER BY time DESC, id DESC`）。
- **质量监控（#5）**：`sip.quality` 事件（loss/jitter/rtt）经 WebSocket 推送，前端指标卡片。

## 汇总

- **通过维度**：5/5
- **结论**：✅ 通过（实现与里程碑文档一致；1 项 🟢 风格备注不阻断）

## 发现的问题

- 🟢 `SipPage.vue` 视频画布 `background: #000` 硬编码，可改用设计 token 或 `var(--xxx, #000)` 回退。非阻断，留待后续打磨。
