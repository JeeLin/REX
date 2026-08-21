# 步骤7：设计再确认 — 0.70.0 SIP 电话资源基础

> 重跑（打回步骤5 后）。审查对象：已实现代码（变更文件）vs 里程碑文档。
> AGENTS.md 无 `## 审查维度`，按 dev-flow 步骤7 通用框架：实现是否符合里程碑设计、产品语义是否变化、用户可见行为是否变化。

## 设计核对点逐项确认

| # | 设计核对点 | 实现状态 | 结论 |
|---|-----------|----------|------|
| 1 | 单用户、自托管定位不被破坏（SIP 资源仍属单一用户本地管理，无多租户） | `sip_ws.rs` 复用现有 `AppState`/JWT 鉴权，无多用户/RBAC 概念 | ✅ |
| 2 | 不引入 RBAC/多用户/企业协作 | 前端 `features/sip/*`、向导均无角色/协作逻辑 | ✅ |
| 3 | 资源模型仅在既有 `protocol` 字符串枚举上增 `sip` 值，不改 schema，向后兼容 | `resource_conn.rs` 仅新增 `load_sip_conn` 按 `protocol=="sip"` 分支；DB schema 未改；`useTabs.ts`/`protocols.ts` 仅增枚举值 | ✅ |
| 4 | baresip 进程内 FFI + bindgen 直接绑 baresip 本体 + git submodule 固定 v4.10.0 + CMake 构建 | `rex-sip/build.rs` 调 CMake 编 libre/libbaresip 静态链 + bindgen；`.gitmodules` 固定 baresip v4.10.0 + re 对应 release | ✅ |
| 5 | 满足「必须支持 Windows」：baresip 官方全平台（Linux/Win10+/macOS/iOS/Android） | 构建用 CMake（baresip v4.x 官方构建系统），无平台专有代码 | ✅ |
| 6 | baresip 进程内 FFI 运行于 Hub/Agent worker，音频回调可在事件线程内接管原始 RTP 帧（M82b 前置） | `BaresipSipUa::on_rtp` 接口预留（M82a 留空桩，符合设计「留空桩但接口预留」） | ✅ |
| 7 | 音频链路（RTP→Opus→WebSocket、麦克风回传）明确留到 0.70.1，本里程碑 `SipUa::on_rtp` 留空桩 | `lib.rs` `on_rtp` 默认 no-op 注释明确「0.70.0 不实现」 | ✅ |
| 8 | Agent 侧 `sip` 复用现有 `agent_ws.rs` channel_id 多路复用与 binary 帧，不新造隧道；Agent 嵌 `rex-sip` 当 UA₂ | Agent `handle_connect` 对 `protocol:"sip"` 起 UA₂，复用 `AGENT_CHANNEL_SEQ` 数值 channel_id + binary 帧 `[4B channelId][payload]`；`rex-agent/Cargo.toml` 加 `rex-sip` | ✅ |
| 9 | 前端统一用设计 token，不硬编码 hex | `CallState.vue`/`Dialpad.vue`/`SipPage.vue` 全程 `var(--space-*)`/`--text-*`/`--border`/`--accent`/`--danger`；协议色 `#2DD4BF` 青色与既有 7 色区分（核对点第 10 项） | ✅ |
| 10 | 协议图标/色：新增 `sip` 与现有 7 色区分（建议青色 `#2DD4BF`） | `protocols.ts`：`sip: '#2DD4BF'`（青）、图标 `☎`；与 S3 橙 `#E8912D` 等无冲突 | ✅ |

## 实现 vs 里程碑文档一致性

- **子任务 1-7**：全部 ✅（代码已落，单测/集成测试覆盖）。
- **产品边界「本阶段不做什么」**：音频流（浏览器实时听/说）、CDR/录音/信令抓包/展示图 均未实现（属 0.70.1/0.70.2），本里程碑仅信令层——与文档一致。
- **用户可见行为**：前端拨号盘 + 通话状态组件，经 `/ws/sip` 控制通话；行为未偏离文档描述。
- **打回修复一致性**：5 个 🔴（password 未传 / 递归死锁 / 重复 init / UAF / 跨线程竞争）+ 2 个用户反馈 bug（Agent 回传帧丢弃）已全部修复并验证，实现与文档「修复后状态」一致。

## 汇总

- **通过维度**：10/10（设计核对点）
- **结论**：✅ 通过（实现符合里程碑设计，产品语义与用户可见行为未变）

## 发现的问题

无（无 ❌ 维度）。
