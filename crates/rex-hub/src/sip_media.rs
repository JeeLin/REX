//! SIP 媒体通道（M82b）：Hub 侧把 baresip 抽出的 S16LE PCM 经 WebSocket 二进制帧
//! 实时推流；反向把收到的 PCM 帧喂回 baresip 发送链路。
//!
//! 编码选择：媒体帧**直接携带原始 S16LE PCM**（小端 i16），不做 opus 等线上编码。
//! 理由：本产品是单用户自托管，浏览器侧用 Web Audio 原生消费/采集 PCM，opus 仅是带宽
//! 优化却带来额外依赖与每帧编解码延迟；局域网下 8kHz 单声道 PCM（≈128kbps）完全可接受。
//! 与里程碑「baresip↔Rex 边界统一 AUFMT_S16LE」设定一致——线上/边界同为 S16LE i16。
//!
//! 编码/隧道封装的权威实现位于 `rex_common::sip_media`（Hub 与 Agent 共用），
//! 本模块仅做再导出，保持既有 `crate::sip_media::*` 调用点不变。

pub use rex_common::sip_media::*;
