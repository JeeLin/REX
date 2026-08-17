//! SIP 媒体通道共享编码（Hub 与 Agent 共用）。
//!
//! 媒体帧**直接携带原始 S16LE PCM**（小端 i16），不做 opus 等线上编码：单用户自托管
//! 场景下浏览器用 Web Audio 原生消费/采集 PCM，opus 仅是带宽优化却带来额外依赖与每帧
//! 编解码延迟；局域网下 8kHz 单声道 PCM（≈128kbps）完全可接受。
//!
//! 隧道二进制帧布局：`[4B channelId][1B kind][payload]`
//! - `KIND_SIGNAL`（0）：控制/事件帧（SipControl / SipEvent JSON）
//! - `KIND_MEDIA`（1）：媒体帧（原始 S16LE PCM 字节）

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

/// 隧道内二进制帧「信令」种类字节（kind=0）。
pub const KIND_SIGNAL: u8 = 0;
/// 隧道内二进制帧「媒体」种类字节（kind=1），与 `KIND_SIGNAL` 区分。
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

    #[test]
    fn tunnel_frame_wraps_and_unwraps_kind() {
        let payload = b"signal-json";
        let f = wrap_tunnel_frame(KIND_SIGNAL, payload);
        let (kind, rest) = unwrap_tunnel_frame(&f);
        assert_eq!(kind, KIND_SIGNAL);
        assert_eq!(rest, payload);

        let pcm = encode_pcm_frame(&[1i16, -2, 3]);
        let m = wrap_tunnel_frame(KIND_MEDIA, &pcm);
        let (mk, mrest) = unwrap_tunnel_frame(&m);
        assert_eq!(mk, KIND_MEDIA);
        assert_eq!(decode_media_frame(mrest), vec![1i16, -2, 3]);
    }

    #[test]
    fn unwrap_empty_yields_signal_and_empty() {
        let (kind, rest) = unwrap_tunnel_frame(&[]);
        assert_eq!(kind, KIND_SIGNAL);
        assert!(rest.is_empty());
    }
}
