//! SIP 信令抓包（子任务 #3）。
//!
//! baresip v4.10 **没有 pcap / siprtp 运行时模块**,但其**核心库内置报文追踪钩子**
//! `sip_set_trace_handler`(`re_sip.h`):每收/发一条 SIP 信令都会回调真实报文字节
//! (`pkt`/`len`),带 TX/RX 方向与端点。本模块接该钩子,把 UA₁(Hub 本地跑的所有 UA)
//! 的真实 SIP 报文捕获进全局缓冲区,导出为 libpcap(`LINKTYPE_RAW`,包体 = SIP 文本)。
//!
//! 范围说明:
//! - 抓包是 **Hub 全局**的——baresip runtime 在进程内唯一单例,`uag_sip()` 是唯一的 sip 栈,
//!   勾子无法区分某条报文属于哪个 resource,故不按 resource_id 维度抓。
//! - UA₂(Agent 链式路径)的 SIP 在远端、经隧道二进制帧回 Hub 时已重建,不能经此钩子抓,
//!   由 Hub 中继层单独捕获(见 `rex_hub::sip_capture` 的 UA₂ 段),Wireshark 中体现为 JSON 文本。

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ffi::{sa, sa_ntop, sip_set_trace_handler, uag_sip};

/// 单条捕获报文(真实 SIP 信令字节)。
#[derive(Debug, Clone)]
pub struct CapturedPacket {
    /// Unix 微秒时间戳。
    pub ts_us: u64,
    /// 方向:UA₁ 出站 `ua1_out` / UA₁ 入站 `ua1_in`。
    pub direction: String,
    /// 原始 SIP 报文文本。
    pub raw: String,
}

#[derive(Default)]
struct CaptureInner {
    active: bool,
    packets: Vec<CapturedPacket>,
}

static CAPTURE: Mutex<CaptureInner> = Mutex::new(CaptureInner {
    active: false,
    packets: Vec::new(),
});

/// baresip `sip_trace_h` 回调:运行在 baresip `re_main` 单线程上。`pkt`/`len` 即真实 SIP 报文字节。
///
/// 仅在抓包激活时记录,临界区仅复制字节,不持有任何锁跨 await。
unsafe extern "C" fn sip_trace_cb(
    tx: bool,
    _tp: crate::sip_transp,
    src: *const sa,
    dst: *const sa,
    pkt: *const u8,
    len: usize,
    _arg: *mut std::os::raw::c_void,
) {
    let active = CAPTURE.lock().map(|g| g.active).unwrap_or(false);
    if !active {
        return;
    }
    // 复制真实 SIP 报文(允许非 UTF-8,lossy)。
    let bytes = std::slice::from_raw_parts(pkt, len);
    let raw = String::from_utf8_lossy(bytes).into_owned();
    let direction = if tx { "ua1_out" } else { "ua1_in" };
    let endpoint = endpoint_label(src, dst);
    let entry = CapturedPacket {
        ts_us: now_us(),
        direction: direction.to_string(),
        raw: if endpoint.is_empty() {
            raw
        } else {
            format!("{endpoint}\n{raw}")
        },
    };
    if let Ok(mut g) = CAPTURE.lock() {
        g.packets.push(entry);
    }
}

/// 由 `sa_ntop` 拼出 `src -> dst` 端点标签(用于无 Wireshark 时文本回看)。
unsafe fn endpoint_label(src: *const sa, dst: *const sa) -> String {
    let mut buf = [0i8; 64];
    let mut s = String::new();
    if !src.is_null() && sa_ntop(src, buf.as_mut_ptr(), buf.len() as i32) > 0 {
        s.push_str(
            std::ffi::CStr::from_ptr(buf.as_ptr())
                .to_str()
                .unwrap_or(""),
        );
    }
    s.push_str(" -> ");
    let mut buf = [0i8; 64];
    if !dst.is_null() && sa_ntop(dst, buf.as_mut_ptr(), buf.len() as i32) > 0 {
        s.push_str(
            std::ffi::CStr::from_ptr(buf.as_ptr())
                .to_str()
                .unwrap_or(""),
        );
    }
    s
}

fn now_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

/// 在 `re_main` 初始化时注册一次报文追踪钩子(幂等:仅设置函数指针)。
///
/// 必须在 `baresip_init` 之后、`uag_sip()` 返回有效实例后调用。
pub(crate) unsafe fn install_trace() {
    let sip = uag_sip();
    if !sip.is_null() {
        sip_set_trace_handler(sip, Some(sip_trace_cb));
    }
}

/// 开始全局抓包(幂等)。同时确保 baresip runtime 已初始化,使 `uag_sip()` 有效。
pub fn start() -> anyhow::Result<()> {
    crate::baresip_ua::ensure_runtime_initialized()?;
    if let Ok(mut g) = CAPTURE.lock() {
        g.active = true;
    }
    Ok(())
}

/// 停止抓包并清空缓冲区(返回停止前累积的报文副本)。
pub fn stop() -> Vec<CapturedPacket> {
    let mut g = CAPTURE.lock().unwrap();
    g.active = false;
    std::mem::take(&mut g.packets)
}

pub fn is_active() -> bool {
    CAPTURE.lock().map(|g| g.active).unwrap_or(false)
}

/// 当前累积报文快照(无论抓包是否仍在进行)。
pub fn snapshot() -> Vec<CapturedPacket> {
    CAPTURE
        .lock()
        .map(|g| g.packets.clone())
        .unwrap_or_default()
}

/// libpcap 全局头（magic 0xa1b2c3d4，微秒精度，链路类型 RAW=101）。
const PCAP_MAGIC: u32 = 0xa1b2c3d4;
const PCAP_VERSION_MAJOR: u16 = 2;
const PCAP_VERSION_MINOR: u16 = 4;
const PCAP_LINKTYPE_RAW: u32 = 101;

/// 把 SIP 报文文本编码为 libpcap 字节流（链路类型 RAW，包体 = SIP 文本）。
///
/// 真实 SIP 字节(来自 `sip_trace` 钩子)经此后可被 Wireshark 直接解析。
pub fn encode_pcap(packets: &[CapturedPacket]) -> Vec<u8> {
    let mut out = Vec::new();
    // 全局头(24 字节)
    out.extend_from_slice(&PCAP_MAGIC.to_le_bytes());
    out.extend_from_slice(&PCAP_VERSION_MAJOR.to_le_bytes());
    out.extend_from_slice(&PCAP_VERSION_MINOR.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // thiszone
    out.extend_from_slice(&0u32.to_le_bytes()); // sigfigs
    out.extend_from_slice(&65535u32.to_le_bytes()); // snaplen
    out.extend_from_slice(&PCAP_LINKTYPE_RAW.to_le_bytes()); // network/linktype

    for p in packets {
        let body = p.raw.as_bytes();
        let ts_sec = (p.ts_us / 1_000_000) as u32;
        let ts_usec = (p.ts_us % 1_000_000) as u32;
        out.extend_from_slice(&ts_sec.to_le_bytes());
        out.extend_from_slice(&ts_usec.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes()); // incl_len
        out.extend_from_slice(&(body.len() as u32).to_le_bytes()); // orig_len
        out.extend_from_slice(body);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkt(ts_us: u64, direction: &str, raw: &str) -> CapturedPacket {
        CapturedPacket {
            ts_us,
            direction: direction.to_string(),
            raw: raw.to_string(),
        }
    }

    #[test]
    fn encode_pcap_writes_magic_and_linktype() {
        let packets = vec![pkt(
            1_000_001,
            "ua1_out",
            "INVITE sip:bob@example.com SIP/2.0",
        )];
        let bytes = encode_pcap(&packets);
        let body = packets[0].raw.as_bytes();
        assert_eq!(bytes.len(), 24 + 16 + body.len());
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(magic, PCAP_MAGIC);
        let major = u16::from_le_bytes([bytes[4], bytes[5]]);
        let minor = u16::from_le_bytes([bytes[6], bytes[7]]);
        assert_eq!((major, minor), (2, 4));
        let linktype = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        assert_eq!(linktype, 101);
        let rec_off = 24;
        let ts_sec = u32::from_le_bytes([
            bytes[rec_off],
            bytes[rec_off + 1],
            bytes[rec_off + 2],
            bytes[rec_off + 3],
        ]);
        let ts_usec = u32::from_le_bytes([
            bytes[rec_off + 4],
            bytes[rec_off + 5],
            bytes[rec_off + 6],
            bytes[rec_off + 7],
        ]);
        assert_eq!(ts_sec, 1);
        assert_eq!(ts_usec, 1);
        let incl = u32::from_le_bytes([
            bytes[rec_off + 8],
            bytes[rec_off + 9],
            bytes[rec_off + 10],
            bytes[rec_off + 11],
        ]);
        let orig = u32::from_le_bytes([
            bytes[rec_off + 12],
            bytes[rec_off + 13],
            bytes[rec_off + 14],
            bytes[rec_off + 15],
        ]);
        assert_eq!(incl, body.len() as u32);
        assert_eq!(orig, body.len() as u32);
        assert_eq!(&bytes[rec_off + 16..], body);
    }

    #[test]
    fn encode_pcap_empty_has_only_global_header() {
        let bytes = encode_pcap(&[]);
        assert_eq!(bytes.len(), 24);
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(magic, PCAP_MAGIC);
    }
}
