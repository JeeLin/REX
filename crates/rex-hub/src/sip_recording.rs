//! SIP 通话录音（子任务 #2）。
//!
//! Hub 在通话进行时捕获**下行（远端→本端）PCM 媒体帧**落盘为 WAV（16-bit PCM, 8kHz, 单声道，
//! 与 M82b 媒体通道 `MEDIA_SAMPLE_RATE` 一致），通话结束时写入
//! `<data_dir>/recordings/<cdr_id>.wav` 并回填 CDR 的 `recording_url`，前端 CDR 详情即可回放/下载。
//!
//! 媒体帧不经过浏览器中转（「文件传输不经过浏览器」一致）：Hub 在媒体边界直接 tap 落盘，
//! 前端只取结构化 `recording_url`。
//!
//! 抓包开关（子任务 #3）按 resource 全局开启；录音同样按 resource 开启，但**按 call_id 分文件**——
//! 同一 resource 的多通电话各自独立录音文件，分别与各自 CDR 关联。

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use rex_common::sip_media::decode_media_frame;

/// 录音文件编码参数（与下游媒体通道一致：S16LE、8kHz、单声道）。
pub const REC_SAMPLE_RATE: u32 = 8000;
pub const REC_CHANNELS: u16 = 1;
pub const REC_BITS: u16 = 16;

/// 录音子目录名（相对 `data_dir`）。
pub const REC_DIR: &str = "recordings";

/// Hub 级录音状态：按 resource 开关 + 按 call_id 分缓冲。
///
/// 下行媒体帧经 [`SipRecordingRegistry::append_current`] 写入「当前激活通话」的缓冲；
/// 通话结束（CDR 状态机 `Ended`）时 [`finalize_call`] 把缓冲落盘并回填 CDR。
pub struct SipRecordingRegistry {
    data_dir: PathBuf,
    /// 任一会话开启即激活录音（按 resource，与 capture 同语义）。
    enabled: AtomicBool,
    /// 当前正在录音的 call_id（由 CDR 状态机在 `Active` 时设置、`Ended` 时清空）。
    current_call: Mutex<Option<String>>,
    /// 各 call_id 累积的下行 PCM 样本。
    buffers: Mutex<HashMap<String, Vec<i16>>>,
}

impl SipRecordingRegistry {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            enabled: AtomicBool::new(false),
            current_call: Mutex::new(None),
            buffers: Mutex::new(HashMap::new()),
        }
    }

    /// 开启录音（幂等，按 resource）。
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// 关闭录音（停止承接新帧；已累积缓冲在通话结束时仍会落盘）。
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
        *self.current_call.lock().unwrap() = None;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// CDR 状态机驱动：标记某 call_id 进入录音（确保缓冲就绪）。
    pub fn begin_call(&self, call_id: &str) {
        if !self.enabled.load(Ordering::SeqCst) {
            return;
        }
        *self.current_call.lock().unwrap() = Some(call_id.to_string());
        self.buffers
            .lock()
            .unwrap()
            .entry(call_id.to_string())
            .or_default();
    }

    /// 下行媒体帧回调：写入当前激活通话的缓冲（仅开启且存在激活通话时）。
    pub fn append_current(&self, pcm: &[i16]) {
        if !self.enabled.load(Ordering::SeqCst) {
            return;
        }
        let call_id = self.current_call.lock().unwrap().clone();
        if let Some(id) = call_id {
            self.buffers
                .lock()
                .unwrap()
                .entry(id)
                .or_default()
                .extend_from_slice(pcm);
        }
    }

    /// CDR 状态机驱动：通话结束，落盘并回填 URL（返回录音相对 URL，无缓冲时返回 None）。
    ///
    /// `cdr_id` 即 CDR 主键（形如 `cdr:{call_id}`），用作文件名与下载路径。
    pub fn finalize_call(&self, cdr_id: &str) -> Option<String> {
        let call_id = cdr_id.strip_prefix("cdr:").unwrap_or(cdr_id);
        let samples = self
            .buffers
            .lock()
            .unwrap()
            .remove(call_id)
            .filter(|b| !b.is_empty());
        *self.current_call.lock().unwrap() = None;
        let samples = samples?;

        let dir = self.data_dir.join(REC_DIR);
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{cdr_id}.wav"));
        if write_wav(&path, &samples, REC_SAMPLE_RATE, REC_CHANNELS, REC_BITS).is_err() {
            return None;
        }
        Some(format!("/api/sip/recordings/{cdr_id}.wav"))
    }
}

/// 把下行媒体帧（S16LE PCM 二进制）解码为 i16 并写入当前激活通话缓冲。
///
/// 媒体帧在隧道上带 1 字节 kind 前缀（见 `unwrap_tunnel_frame`），`rest` 即纯 PCM 字节。
pub fn append_tunnel_media(reg: &SipRecordingRegistry, media_bytes: &[u8]) {
    if !reg.is_enabled() {
        return;
    }
    let pcm = decode_media_frame(media_bytes);
    if !pcm.is_empty() {
        reg.append_current(&pcm);
    }
}

/// 写 16-bit PCM WAV 文件（标准 RIFF 头 + 数据块）。
///
/// 返回 `io::Result`，失败由调用方决定（此处静默返回 None，不阻断通话）。
pub fn write_wav(
    path: &std::path::Path,
    samples: &[i16],
    sample_rate: u32,
    channels: u16,
    bits: u16,
) -> std::io::Result<()> {
    let mut f = File::create(path)?;
    let byte_rate = sample_rate * channels as u32 * (bits as u32 / 8);
    let block_align = channels * (bits / 8);
    let data_len = (samples.len() * 2) as u32;
    let riff_len = 36 + data_len;

    f.write_all(b"RIFF")?;
    f.write_all(&riff_len.to_le_bytes())?;
    f.write_all(b"WAVE")?;
    // fmt 子块
    f.write_all(b"fmt ")?;
    f.write_all(&16u32.to_le_bytes())?; // PCM 块大小
    f.write_all(&1u16.to_le_bytes())?; // audioFormat = 1 (PCM)
    f.write_all(&channels.to_le_bytes())?;
    f.write_all(&sample_rate.to_le_bytes())?;
    f.write_all(&byte_rate.to_le_bytes())?;
    f.write_all(&block_align.to_le_bytes())?;
    f.write_all(&bits.to_le_bytes())?;
    // data 子块
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for s in samples {
        f.write_all(&s.to_le_bytes())?;
    }
    f.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn write_wav_emits_valid_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.wav");
        let samples: Vec<i16> = vec![-32000, 0, 1234, 7];
        write_wav(&path, &samples, 8000, 1, 16).unwrap();
        let mut buf = Vec::new();
        File::open(&path).unwrap().read_to_end(&mut buf).unwrap();
        // RIFF/WAVE 魔法 + fmt PCM + data 长度 = 8 + 24 + 8 + 8 = 48。
        assert_eq!(buf.len(), 44 + samples.len() * 2);
        assert_eq!(&buf[0..4], b"RIFF");
        assert_eq!(&buf[8..12], b"WAVE");
        assert_eq!(&buf[12..16], b"fmt ");
        let audio_format = u16::from_le_bytes([buf[20], buf[21]]);
        assert_eq!(audio_format, 1); // PCM
        let ch = u16::from_le_bytes([buf[22], buf[23]]);
        assert_eq!(ch, 1);
        let sr = u32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]);
        assert_eq!(sr, 8000);
        assert_eq!(&buf[36..40], b"data");
        let data_len = u32::from_le_bytes([buf[40], buf[41], buf[42], buf[43]]);
        assert_eq!(data_len, (samples.len() * 2) as u32);
    }

    #[test]
    fn registry_only_appends_when_enabled_and_current_set() {
        let r = SipRecordingRegistry::new(tempfile::tempdir().unwrap().path().to_path_buf());
        // 未开启：append 静默丢弃。
        r.append_current(&[1, 2, 3]);
        r.begin_call("call-1"); // 开启后 begin 才生效。
        r.enable();
        r.begin_call("call-1");
        r.append_current(&[10, 20]);
        r.append_current(&[30]);
        let url = r.finalize_call("cdr:call-1").unwrap();
        assert!(url.ends_with("/cdr:call-1.wav"));
        // 二次 finalize 无缓冲 → None。
        assert!(r.finalize_call("cdr:call-1").is_none());
    }

    #[test]
    fn registry_disabled_does_not_record() {
        let r = SipRecordingRegistry::new(tempfile::tempdir().unwrap().path().to_path_buf());
        r.disable();
        r.begin_call("call-9");
        r.append_current(&[5, 6]);
        assert!(r.finalize_call("cdr:call-9").is_none());
    }

    #[test]
    fn append_tunnel_media_decodes_and_records() {
        let r = SipRecordingRegistry::new(tempfile::tempdir().unwrap().path().to_path_buf());
        r.enable();
        r.begin_call("call-x");
        // 模拟隧道媒体帧：kind 已剥掉，rest 为纯 PCM 字节（与 encode_pcm_frame 同契约）。
        let pcm: Vec<i16> = vec![-100, 256, -512, 1];
        let bytes = rex_common::sip_media::encode_pcm_frame(&pcm);
        append_tunnel_media(&r, &bytes);
        let url = r.finalize_call("cdr:call-x").unwrap();
        assert!(url.contains("cdr:call-x.wav"));
    }
}
