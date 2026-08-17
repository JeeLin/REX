//! SIP 媒体通道（M82b）：Hub/Agent 侧把 baresip 抽出的 S16LE PCM 经 WebSocket 二进制帧
//! 实时推流；反向把收到的 PCM 帧喂回 baresip 发送链路。
//!
//! 编码选择：媒体帧**直接携带原始 S16LE PCM**（小端 i16），不做 opus 等线上编码。
//! 理由：本产品是单用户自托管，浏览器侧用 Web Audio 原生消费/采集 PCM，opus 仅是带宽
//! 优化却带来额外依赖与每帧编解码延迟；局域网下 8kHz 单声道 PCM（≈128kbps）完全可接受。
//! 与里程碑「baresip↔Rex 边界统一 AUFMT_S16LE」设定一致——线上/边界同为 S16LE i16。

/// 默认采样率（Hz），与 baresip 窄带语音一致。
pub const MEDIA_SRATE: u32 = 8000;
/// 默认声道数（单声道）。
pub const MEDIA_CHANNELS: u8 = 1;

/// 将一帧 S16LE PCM 样本编码为 WebSocket 二进制帧字节（直接小端 i16 拼接）。
pub fn encode_pcm_frame(pcm: &[i16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pcm.len() * 2);
    for &s in pcm {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// 将 WebSocket 二进制帧字节解码为一帧 S16LE PCM 样本（小端 i16）。
///
/// 长度非偶数时丢弃末尾孤立字节（避免越界/错位）。
pub fn decode_media_frame(bytes: &[u8]) -> Vec<i16> {
    let n = bytes.len() / 2;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let lo = bytes[i * 2] as u8;
        let hi = bytes[i * 2 + 1] as u8;
        out.push(i16::from_le_bytes([lo, hi]));
    }
    out
}

/// 媒体帧方向：仅用于隧道内区分媒体帧与控制帧，直连 WebSocket 帧直接为 PCM 字节、
/// 方向由数据流隐含（Hub→浏览器为下行、浏览器→Hub 为上行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// Hub→浏览器下行（远端 PCM）。
    Down,
    /// 浏览器→Hub 上行（麦克风 PCM）。
    Up,
}

/// 隧道内二进制帧的「种类」字节，用来在同一 WebSocket 隧道上区分信令帧与媒体帧。
///
/// 帧布局：`[4B channelId][1B kind][payload]`。
/// - `KIND_SIGNAL`：控制/事件帧（SipControl / SipEvent JSON）。
/// - `KIND_MEDIA`：媒体帧（原始 S16LE PCM 字节）。
pub const KIND_SIGNAL: u8 = 0;
/// 隧道媒体帧种类字节（与 `KIND_SIGNAL` 区分）。
pub const KIND_MEDIA: u8 = 1;

/// 在 payload 前加 1 字节 kind 标记，形成隧道内区分信令/媒体的封装。
pub fn wrap_tunnel_frame(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut f = Vec::with_capacity(payload.len() + 1);
    f.push(kind);
    f.extend_from_slice(payload);
    f
}

/// 剥去隧道帧首字节 kind，返回 `(kind, payload)`。长度不足 1 字节时返回 `(KIND_SIGNAL, 空)`。
pub fn unwrap_tunnel_frame(data: &[u8]) -> (u8, &[u8]) {
    if data.is_empty() {
        return (KIND_SIGNAL, &[]);
    }
    (data[0], &data[1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_frame_roundtrip_preserves_samples() {
        let pcm: Vec<i16> = (0..160).map(|i| (i as i16) * 7 - 1000).collect();
        let frame = encode_pcm_frame(&pcm);
        assert_eq!(frame.len(), pcm.len() * 2);
        let decoded = decode_media_frame(&frame);
        assert_eq!(decoded, pcm);
    }

    #[test]
    fn decode_drops_trailing_odd_byte() {
        let pcm: Vec<i16> = vec![-1234, 5678, 0, -1];
        let mut frame = encode_pcm_frame(&pcm);
        frame.push(0xAB); // 孤立字节
        let decoded = decode_media_frame(&frame);
        assert_eq!(decoded, pcm);
        // 空输入安全。
        assert!(decode_media_frame(&[]).is_empty());
    }

    #[test]
    fn encode_empty_is_empty() {
        assert!(encode_pcm_frame(&[]).is_empty());
    }
}
