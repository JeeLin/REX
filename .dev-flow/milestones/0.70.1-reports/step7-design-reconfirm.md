# 设计再确认：0.70.1（M82b）

## 审查类型

设计再确认（dev-flow 步骤7）：已实现代码 vs 里程碑文档。

## 审查维度（对照 AGENTS.md + 里程碑「设计核对点」）

| # | 维度 | 结论 | 说明 |
|---|------|------|------|
| 1 | 产品边界一致性 | ✅ | 仅增媒体通道，信令层（register/dial/answer/hangup/hold/unhold/dtmf）行为未变；未引入多用户/RBAC/企业协作；未改动 Hub/Agent 版本一致模型。 |
| 2 | 单用户 / 自托管定位 | ✅ | 浏览器作为实时音频端点（用户明确要求的「可听可说」例外），所有帧仍经 Hub/Agent 服务端中转，浏览器不接触远端 SIP 服务器；与 PRODUCT.md 430 行「媒体不经过浏览器」例外条款一致（M82b 已确认）。 |
| 3 | 子任务实现覆盖 | ✅ | 5 个子任务全部 ⬛→✅：①rex-sip 音频桥（ausrc/auplay + send_audio）；②Hub `/ws/sip` 媒体通道（binary PCM 帧）；③Agent UA₂ 媒体转发（tunnel kind）；④前端 Web Audio 播放 + getUserMedia 回传；⑤延迟/回声基础优化（gain=0 回声抑制、jitter buffer 由队列自然平滑）+ 联调。 |
| 4 | 媒体格式一致性 | ✅ | 线上为原始 S16LE i16 LE 小端 PCM，不引入 opus；前端 `sipMedia.ts` 与后端 `rex_common::sip_media` 编解码对称（round-trip 单测互验）。 |
| 5 | 隧道帧 kind 区分 | ✅ | Agent 链式隧道帧 `[4B channelId][1B kind][payload]`，`KIND_SIGNAL=0`/`KIND_MEDIA=1`；复用既有 agent_ws 隧道，未新造通道（步骤4 死代码 `MediaKind` 已删除）。 |
| 6 | 帧路由端到端闭环 | ✅ | 四链路逐跳核对无方向错乱、无 kind 错配：Hub 直连下行（on_rtp→Binary→浏览器）/上行（Binary→decode→send_audio）；Agent 链式下行（on_rtp→wrap_tunnel_frame(KIND_MEDIA)+channelId→隧道→Hub 推浏览器）/上行（浏览器 Binary→wrap+channelId→Agent dispatch→send_audio）。 |
| 7 | 挂断 / 卸载清理 | ✅ | `call_state:ended` → teardownAudio（`close` 重置 playQueue/playOffset + 停 mic + 关 AudioContext）；`onBeforeUnmount` 同样清理。 |
| 8 | 设计核对点（媒体无关抽象） | ✅ | `on_rtp`/`send_audio` 与隧道帧以 kind 区分、payload 为原始 PCM，不写死编码；仅实装 Audio 分支，视频（vidbridge/vidsrc 同构）留后续里程碑复用。 |
| 9 | 前端设计 token | ✅ | 组件样式统一引用 `--bg-deep`/`--border`/`--space-*`/`--text-*` 等变量，无硬编码 hex。 |

## 已验证的实现-文档一致点（本次步骤6 实跑佐证）

- `cargo test` rex-common 15 / rex-hub sip_ws 21 / rex-sip 7 / rex-agent 13 全绿 → 四条链路帧路由函数级契约稳定。
- 前端 vitest 147 全绿，里程碑文件覆盖率 90.62%（门禁 ≥ 90%）→ 媒体帧路由、`initPlayback`/`startMic`/`playPcm`/`teardownAudio` 关键路径均有覆盖。
- `bun run type-check` / `build` / `lint` 均 0 error。

## 发现的问题

无。实现与里程碑文档设计逐项一致，无 🔴/🟡/🟢 偏差。

## 汇总

- **通过维度**：9/9
- **结论**：✅ 通过
