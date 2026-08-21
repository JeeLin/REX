//! SIP 媒体通道共享编码（Hub 与 Agent 共用）。
//!
//! 音频帧**直接携带原始 S16LE PCM**（小端 i16），视频帧**直接携带原始像素**（RGBA 字节），
//! 均不做线上编解码：单用户自托管场景下浏览器用 Web Audio / WebCodecs 原生消费采集，线上
//! 编码仅带宽优化却带来额外依赖（ffmpeg/libvpx）与每帧编解码延迟；局域网下原始边界完全可接受。
//!
//! 隧道二进制帧布局：`[4B channelId][1B kind][payload]`
//! - `KIND_SIGNAL`（0）：控制/事件帧（SipControl / SipEvent JSON）
//! - `KIND_MEDIA`（1）：媒体帧（原始 S16LE PCM 字节，音频）
//! - `KIND_VIDEO`（2）：媒体帧（原始像素字节，视频；payload 见 [`encode_video_frame`]）
//!
//! 注：当前构建的 baresip 未编译视频编解码器模块（avcodec/vp8/vp9/av1），端到端视频联调
//! 需在本地安装 ffmpeg 重编译 baresip。框架层（驱动桥 + 媒体通道 + 前端解码/采集）在此
//! 子任务打通，单测覆盖帧编解码 round-trip，与里程碑测试标准一致（CI 难含真摄像头/解码器）。

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
        // `bytes[*]` 已是 u8，无需 `as u8`（clippy `cast_same_type`）。
        let lo = bytes[i * 2];
        let hi = bytes[i * 2 + 1];
        out.push(i16::from_le_bytes([lo, hi]));
    }
    out
}

/// 隧道内二进制帧「信令」种类字节（kind=0）。
pub const KIND_SIGNAL: u8 = 0;
/// 隧道内二进制帧「媒体」种类字节（kind=1），与 `KIND_SIGNAL` 区分（音频 PCM）。
pub const KIND_MEDIA: u8 = 1;
/// 隧道内二进制帧「视频」种类字节（kind=2），与音频媒体帧并列（原始像素）。
pub const KIND_VIDEO: u8 = 2;

/// 视频像素格式（与前端 `sipMedia.ts` 约定一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VideoPixFmt {
    /// 32-bit RGBA，每像素 4 字节，行优先。
    Rgba = 0,
}

impl VideoPixFmt {
    /// 每像素字节数。
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            VideoPixFmt::Rgba => 4,
        }
    }
}

/// 将一帧视频像素编码为隧道视频帧字节：
/// `[fmt:u8][w:u16 LE][h:u16 LE][pixels...]`（RGBA 行优先）。
///
/// `pixels` 长度必须等于 `w*h*bpp`，否则返回 `Err`。
pub fn encode_video_frame(
    fmt: VideoPixFmt,
    width: u16,
    height: u16,
    pixels: &[u8],
) -> Result<Vec<u8>, &'static str> {
    let need = width as usize * height as usize * fmt.bytes_per_pixel();
    if pixels.len() != need {
        return Err("pixel buffer size mismatch");
    }
    let mut out = Vec::with_capacity(5 + pixels.len());
    out.push(fmt as u8);
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(pixels);
    Ok(out)
}

/// 解码隧道视频帧字节为 `(fmt, width, height, pixels)`。长度不足或 fmt 未知返回 `Err`。
pub fn decode_video_frame(bytes: &[u8]) -> Result<(VideoPixFmt, u16, u16, Vec<u8>), &'static str> {
    if bytes.len() < 5 {
        return Err("video frame too short");
    }
    let fmt = match bytes[0] {
        0 => VideoPixFmt::Rgba,
        _ => return Err("unknown video pixfmt"),
    };
    let width = u16::from_le_bytes([bytes[1], bytes[2]]);
    let height = u16::from_le_bytes([bytes[3], bytes[4]]);
    let pixels = &bytes[5..];
    let need = width as usize * height as usize * fmt.bytes_per_pixel();
    if pixels.len() != need {
        return Err("pixel buffer size mismatch");
    }
    Ok((fmt, width, height, pixels.to_vec()))
}

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

    #[test]
    fn video_frame_roundtrip_preserves_pixels() {
        let w = 2u16;
        let h = 2u16;
        // 2x2 RGBA：红/绿/蓝/白
        let pixels: Vec<u8> = vec![
            255, 0, 0, 255, // 红
            0, 255, 0, 255, // 绿
            0, 0, 255, 255, // 蓝
            255, 255, 255, 255, // 白
        ];
        let frame = encode_video_frame(VideoPixFmt::Rgba, w, h, &pixels).unwrap();
        assert_eq!(frame[0], VideoPixFmt::Rgba as u8);
        assert_eq!(&frame[1..3], &w.to_le_bytes());
        assert_eq!(&frame[3..5], &h.to_le_bytes());
        let (fmt, dw, dh, got) = decode_video_frame(&frame).unwrap();
        assert_eq!(fmt, VideoPixFmt::Rgba);
        assert_eq!(dw, w);
        assert_eq!(dh, h);
        assert_eq!(got, pixels);
    }

    #[test]
    fn video_frame_rejects_size_mismatch() {
        // 声明 2x2（需 16 字节）但只给 4 字节像素 → 编码失败。
        let small = vec![1, 2, 3, 4];
        assert!(encode_video_frame(VideoPixFmt::Rgba, 2, 2, &small).is_err());
        // 编码成功但解码时截断像素 → 解码失败。
        let good = encode_video_frame(VideoPixFmt::Rgba, 1, 1, &[9, 8, 7, 6]).unwrap();
        let mut truncated = good.clone();
        truncated.truncate(good.len() - 1);
        assert!(decode_video_frame(&truncated).is_err());
    }

    #[test]
    fn video_frame_rejects_unknown_fmt() {
        let mut bad = vec![99u8]; // 未知 fmt
        bad.extend_from_slice(&1u16.to_le_bytes());
        bad.extend_from_slice(&1u16.to_le_bytes());
        bad.extend_from_slice(&[1, 2, 3, 4]);
        assert!(decode_video_frame(&bad).is_err());
    }

    #[test]
    fn video_frame_too_short_rejected() {
        assert!(decode_video_frame(&[0u8; 4]).is_err());
    }
}
