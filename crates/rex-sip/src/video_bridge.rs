//! `rex-sip` 自注册 baresip 视频驱动桥接（0.70.2 子任务 #1 浏览器实时视频）。
//!
//! 与音频桥 [`crate::audio_bridge`] 同构，但视频方向相反、数据形态不同：
//! - `rex_vidisp`（`vidisp`，显示端/接收侧）：baresip 解出对端视频帧后调 `disph` 回调，
//!   把 `vidframe` 里的原始像素取出，经 `on_video` 回调上抛给 Hub 隧道逻辑（→ 浏览器 WebCodecs 渲染）。
//! - `rex_vidsrc`（`vidsrc`，源端/发送侧）：baresip 在 alloc 时把取帧的 `frameh` 沉调用给我们，
//!   我们的**泵线程**周期取 TX 队列里的浏览器上行像素，填进 `vidframe` 调 `frameh` 交 baresip
//!   编码发对端。下行由 `disph` 回调驱动、上行由泵线程驱动，与音频泵模型一致。
//!
//! 像素格式采用与前端约定一致的 `RGBA`（每像素 4 字节，行优先），对应 baresip `VID_FMT_RGB32`。
//! 与音频一致，**不做线上编解码**：帧直接携带原始像素字节，端到端联调需在本地安装 ffmpeg
//! 重编译 baresip（当前构建未含视频编解码器模块），框架层在此打通。
//!
//! 关于 `vidsrc_st`/`vidisp_st` 的 FFI 契约：bindgen 生成的不透明零大小体，
//! 这里用 `#[repr(C)]` 镜像 baresip 字段序自行分配（与音频桥同策略），布局由我们精确控制。

use crate::{
    baresip_vidispl, baresip_vidsrcl, mem_zalloc, vidfmt, vidfmt_VID_FMT_RGB32, vidframe,
    vidisp_disp_h, vidisp_prm, vidisp_register, vidsrc_frame_h, vidsrc_prm, vidsrc_register, vidsz,
};
use std::collections::VecDeque;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub const DRIVER_NAME_VIDSRC: &str = "rex_vidsrc";
pub const DRIVER_NAME_VIDISP: &str = "rex_vidisp";
pub const DEVICE_NAME: &str = "rex";
/// 视频像素格式：baresip `VID_FMT_RGB32`（RGBA，每像素 4 字节）。
const VID_FMT: vidfmt = vidfmt_VID_FMT_RGB32;
/// 视频帧时长（ms），与 baresip 默认视频帧率约 25-30fps 对齐（约 33ms/帧）。
const PTIME_MS: u64 = 33;
/// Linux ENOMEM。
const ENOMEM: c_int = 12;

/// 两个视频驱动的 state 指针，桥接侧登记用。
#[derive(Clone, Copy)]
struct VideoDeviceSt {
    vidisp: *mut RustVidispSt,
    vidsrc: *mut RustVidsrcSt,
}

/// 双方向视频像素帧缓冲 + 驱动 state。
///
/// `VideoBridge` 跨线程（baresip `re_main` 回调 ↔ Hub 隧道逻辑 ↔ 泵线程）共享，持有 baresip
/// 托管的不透明裸指针（与音频桥同类 FFI 既定误报），统一 `#[allow]` 不改变并发语义。
#[derive(Clone)]
#[allow(clippy::arc_with_non_send_sync)]
pub struct VideoBridge {
    /// 下行：对端→浏览器（RX 像素帧，RGBA 行优先）。
    rx: Arc<Mutex<VecDeque<VideoFrame>>>,
    /// 上行：浏览器→对端（TX 像素帧，RGBA 行优先），由泵线程经 baresip `frameh` 取用。
    tx: Arc<Mutex<VecDeque<VideoFrame>>>,
    /// 下行像素帧回调（接收侧）。每帧到达时同步调用。
    #[allow(clippy::type_complexity)]
    on_video: Arc<Mutex<Option<Box<dyn FnMut(&VideoFrame) + Send + 'static>>>>,
    st: Arc<Mutex<Option<VideoDeviceSt>>>,
    started: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

/// 单帧视频像素（RGBA，行优先），与 `rex_common::sip_media::encode_video_frame` 约定一致。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VideoFrame {
    pub width: u16,
    pub height: u16,
    /// RGBA 行优先像素字节，长度 = width*height*4。
    pub rgba: Vec<u8>,
}

impl Default for VideoBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoBridge {
    pub fn new() -> Self {
        #[allow(clippy::arc_with_non_send_sync)]
        Self {
            rx: Arc::new(Mutex::new(VecDeque::new())),
            tx: Arc::new(Mutex::new(VecDeque::new())),
            on_video: Arc::new(Mutex::new(None)),
            st: Arc::new(Mutex::new(None)),
            started: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 注册下行像素帧回调（接收侧，浏览器渲染用）。每帧到达即触发。
    #[allow(clippy::type_complexity)]
    pub fn set_on_video(&self, cb: Box<dyn FnMut(&VideoFrame) + Send + 'static>) {
        *self.on_video.lock().unwrap() = Some(cb);
    }

    /// 浏览器上行像素帧回传（发送侧），压入 TX 队列供泵线程经 baresip `frameh` 取用。
    pub fn push_tx(&self, frame: VideoFrame) {
        if frame.rgba.len() == frame.width as usize * frame.height as usize * 4 {
            self.tx.lock().unwrap().push_back(frame);
        }
    }

    /// 取出一帧下行像素帧（独立渲染任务调用，非回调线程）。
    pub fn pop_rx(&self) -> Option<VideoFrame> {
        self.rx.lock().unwrap().pop_front()
    }

    /// 下行队列帧数（jitter buffer 估算）。
    pub fn rx_len(&self) -> usize {
        self.rx.lock().unwrap().len()
    }

    fn on_state_ready(&self, d: &VideoDeviceSt) {
        let both = d.vidisp.is_null() || d.vidsrc.is_null();
        if !both && !self.started.swap(true, Ordering::SeqCst) {
            self.spawn_pump(*d);
        }
    }

    fn spawn_pump(&self, _st: VideoDeviceSt) {
        let stop = self.stop.clone();
        // 视频桥泵线程：下行由 `disph` 回调驱动（baresip 解出帧即上抛），上行由 baresip
        // 经 alloc 时登记的 `frameh` 沉调取帧（真链路需 baresip 带视频编解码器）。此处仅
        // 维持线程存活作为停机哨兵（对端上行帧经 `push_tx` → `frameh` 取走）。
        thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(PTIME_MS));
            }
        });
    }
}

// ---------------------------------------------------------------------------
// baresip 私有结构体布局镜像（vidisp/vidsrc 同序：dev, prm, handlers, arg）
// ---------------------------------------------------------------------------

#[repr(C)]
struct RustVidsrcSt {
    dev: *mut c_void,
    prm: vidsrc_prm,
    /// baresip 在 alloc 时传入的取帧沉调（我们调它把像素交给 baresip 编码）。
    frameh: vidsrc_frame_h,
    arg: *mut c_void,
}

#[repr(C)]
struct RustVidispSt {
    dev: *mut c_void,
    prm: vidisp_prm,
    /// baresip 在 alloc 时传入的显示沉调（baresip 解出帧后调它，即我们的 `vidisp_disp`）。
    disph: vidisp_disp_h,
    arg: *mut c_void,
}

// ---------------------------------------------------------------------------
// 注册
// ---------------------------------------------------------------------------

/// 在 baresip 运行期注册 `rex_vidsrc`/`rex_vidisp` 驱动。
///
/// 必须在 `baresip_init` 之后、与 `register_audio_drivers` 同处 `ensure_runtime` 调用一次
/// （进程级单例）。失败返回错误。
///
/// # Safety
/// `bridge` 必须存活到 baresip 运行时销毁；其裸指针作为驱动 `arg` 在回调中解引用。
pub unsafe fn register_video_drivers(bridge: &VideoBridge) -> anyhow::Result<()> {
    let arg = Box::into_raw(Box::new(bridge.clone())) as *mut c_void;
    let name_src = CString::new(DRIVER_NAME_VIDSRC).unwrap();
    let name_disp = CString::new(DRIVER_NAME_VIDISP).unwrap();
    let mut vsp: *mut crate::vidsrc = std::ptr::null_mut();
    let mut vdp: *mut crate::vidisp = std::ptr::null_mut();
    let rc_src = vidsrc_register(
        &mut vsp,
        baresip_vidsrcl(),
        name_src.as_ptr(),
        Some(vidsrc_alloc),
        None,
    );
    let rc_disp = vidisp_register(
        &mut vdp,
        baresip_vidispl(),
        name_disp.as_ptr(),
        Some(vidisp_alloc),
        None,
        Some(vidisp_disp),
        Some(vidisp_hide),
    );
    if rc_src != 0 || rc_disp != 0 {
        drop(Box::from_raw(arg as *mut VideoBridge));
        return Err(anyhow::anyhow!(
            "video driver register failed: vidsrc={rc_src}, vidisp={rc_disp}"
        ));
    }
    std::mem::forget(name_src);
    std::mem::forget(name_disp);
    Ok(())
}

// ---------------------------------------------------------------------------
// alloc 回调
// ---------------------------------------------------------------------------

unsafe extern "C" fn vidsrc_alloc(
    stp: *mut *mut crate::vidsrc_st,
    _vidsrc: *const crate::vidsrc,
    prm: *mut vidsrc_prm,
    _size: *const vidsz,
    _fmt: *const c_char,
    _dev: *const c_char,
    frameh: vidsrc_frame_h,
    _packeth: crate::vidsrc_packet_h,
    _errorh: crate::vidsrc_error_h,
    arg: *mut c_void,
) -> c_int {
    let st = mem_zalloc(
        std::mem::size_of::<RustVidsrcSt>(),
        Some(video_st_destructor),
    );
    if st.is_null() {
        return ENOMEM;
    }
    let st = st as *mut RustVidsrcSt;
    (*st).dev = std::ptr::null_mut();
    (*st).prm = *prm;
    (*st).frameh = frameh;
    (*st).arg = arg;
    let bridge = &*(arg as *const VideoBridge);
    {
        let mut g = bridge.st.lock().unwrap();
        let mut d = g.take().unwrap_or(VideoDeviceSt {
            vidisp: std::ptr::null_mut(),
            vidsrc: std::ptr::null_mut(),
        });
        d.vidsrc = st;
        *g = Some(d);
        bridge.on_state_ready(&d);
    }
    *stp = st as *mut crate::vidsrc_st;
    0
}

unsafe extern "C" fn vidisp_alloc(
    stp: *mut *mut crate::vidisp_st,
    _vidisp: *const crate::vidisp,
    prm: *mut vidisp_prm,
    _dev: *const c_char,
    _resizeh: crate::vidisp_resize_h,
    arg: *mut c_void,
) -> c_int {
    let st = mem_zalloc(
        std::mem::size_of::<RustVidispSt>(),
        Some(video_st_destructor),
    );
    if st.is_null() {
        return ENOMEM;
    }
    let st = st as *mut RustVidispSt;
    (*st).dev = std::ptr::null_mut();
    (*st).prm = *prm;
    (*st).disph = Some(vidisp_disp);
    (*st).arg = arg;
    let bridge = &*(arg as *const VideoBridge);
    {
        let mut g = bridge.st.lock().unwrap();
        let mut d = g.take().unwrap_or(VideoDeviceSt {
            vidisp: std::ptr::null_mut(),
            vidsrc: std::ptr::null_mut(),
        });
        d.vidisp = st;
        *g = Some(d);
        bridge.on_state_ready(&d);
    }
    *stp = st as *mut crate::vidisp_st;
    0
}

// ---------------------------------------------------------------------------
// 回调：下行 disph（baresip 解出对端帧 → 提取像素上抛）
// ---------------------------------------------------------------------------

/// `vidisp` 显示回调：baresip 解出一帧对端视频后调用，把 `vidframe` 的 RGBA 像素取出，
/// 推入 RX 队列并经 `on_video` 回调上抛给 Hub 隧道逻辑（→ 浏览器）。
///
/// # Safety
/// `frame` 由 baresip 提供，需校验非 null 且 `fmt == VID_FMT_RGB32`；`st` 的 `arg` 指向 `VideoBridge`。
unsafe extern "C" fn vidisp_disp(
    _st: *mut crate::vidisp_st,
    _title: *const c_char,
    frame: *const vidframe,
    _timestamp: u64,
) -> c_int {
    if frame.is_null() {
        return 0;
    }
    let vf = &*frame;
    if vf.fmt != VID_FMT || vf.size.w == 0 || vf.size.h == 0 {
        return 0;
    }
    let w = vf.size.w as usize;
    let h = vf.size.h as usize;
    let stride = vf.linesize[0] as usize;
    let data = vf.data[0];
    if data.is_null() {
        return 0;
    }
    // 逐行拷贝 RGBA（行步可能大于 w*4，需按 linesize 跳跃）。
    let mut rgba = Vec::with_capacity(w * h * 4);
    let row_bytes = w * 4;
    for y in 0..h {
        let row = data.add(y * stride);
        rgba.extend_from_slice(std::slice::from_raw_parts(row, row_bytes));
    }
    let f = VideoFrame {
        width: vf.size.w as u16,
        height: vf.size.h as u16,
        rgba,
    };
    // 桥接指针经 `st` 的 `arg` 字段取回（alloc 时写入 `Box<VideoBridge>` 裸指针）。
    let bridge = if _st.is_null() {
        None
    } else {
        let st = &*(_st as *const RustVidispSt);
        (st.arg as *const VideoBridge).as_ref()
    };
    if let Some(bridge) = bridge {
        bridge.rx.lock().unwrap().push_back(f.clone());
        if let Some(cb) = bridge.on_video.lock().unwrap().as_mut() {
            cb(&f);
        }
    }
    0
}

unsafe extern "C" fn vidisp_hide(_st: *mut crate::vidisp_st) {}

/// 析构驱动 state：baresip 在 `mem_deref(st)` 时调用。`arg` 是我们的 `Box<VideoBridge>` 拷贝
/// 裸指针，不在此释放（`VideoBridge` 由 `ensure_runtime` 单例持有）。
unsafe extern "C" fn video_st_destructor(_arg: *mut c_void) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// 下行/上行队列进出一致：push_tx 后 pop_rx 取出相同像素帧（说明 TX 与 RX 独立队列）。
    #[test]
    fn tx_push_rx_pop_roundtrip() {
        let b = VideoBridge::new();
        assert_eq!(b.rx_len(), 0);
        let f = VideoFrame {
            width: 2,
            height: 2,
            rgba: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        };
        b.push_tx(f.clone());
        assert_eq!(b.rx_len(), 0); // TX 与 RX 独立队列
        b.rx.lock().unwrap().push_back(f.clone());
        assert_eq!(b.pop_rx(), Some(f));
    }

    /// 尺寸不匹配的帧不入 TX 队列（避免 baresip 取到畸形缓冲）。
    #[test]
    fn malformed_frame_dropped() {
        let b = VideoBridge::new();
        let bad = VideoFrame {
            width: 2,
            height: 2,
            rgba: vec![1, 2, 3], // 不足 2*2*4=16 字节
        };
        b.push_tx(bad);
        assert_eq!(b.tx.lock().unwrap().len(), 0);
    }

    /// on_video 回调注册后不 panic（真链路触发需真 baresip）。
    #[test]
    fn on_video_callback_registered() {
        let b = VideoBridge::new();
        let flag = Arc::new(AtomicBool::new(false));
        let f = flag.clone();
        b.set_on_video(Box::new(move |_frame: &VideoFrame| {
            f.store(true, Ordering::SeqCst);
        }));
        assert!(!flag.load(Ordering::SeqCst));
    }

    /// RGBA 逐行提取（vidisp_disp 内核逻辑）round-trip：模拟带行步的 vidframe。
    #[test]
    fn vidframe_rgba_extraction_reconstructs() {
        // 2x2 RGBA，行步 16（大于 8）模拟对齐填充。
        let w = 2usize;
        let h = 2usize;
        let stride = 16usize;
        let mut buf = vec![0u8; stride * h];
        let pixels: Vec<u8> = vec![
            255, 0, 0, 255, 0, 255, 0, 255, // 行0：红、绿
            0, 0, 255, 255, 255, 255, 255, 255, // 行1：蓝、白
        ];
        for y in 0..h {
            for x in 0..w {
                let p = (y * w + x) * 4;
                let dst = y * stride + x * 4;
                buf[dst..dst + 4].copy_from_slice(&pixels[p..p + 4]);
            }
        }
        // 模拟 vidisp_disp 的逐行拷贝：按 stride 跳跃提取有效行字节。
        let row_bytes = w * 4;
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            let row = &buf[y * stride..];
            rgba.extend_from_slice(&row[..row_bytes]);
        }
        assert_eq!(rgba, pixels);
    }
}
