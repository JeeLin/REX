//! SIP 信令抓包（子任务 #3）。
//!
//! 两条捕获流在 Hub 汇聚：
//!
//! - **UA₁（Hub 本地直连）**：baresip 核心库内置报文追踪钩子 `sip_set_trace_handler`
//!   （见 `rex_sip::capture`）提供**真实 SIP 信令字节**。该钩子挂在进程内唯一的 baresip
//!   runtime 上，范围是 **Hub 全局**（无法按 resource 区分），报文为真实 SIP 文本，
//!   可经 `encode_pcap` 导出为 Wireshark 可解析的 libpcap（`LINKTYPE_RAW`）。
//! - **UA₂（Agent 链式路径）**：SIP 在远端，经 `/ws/agent` 隧道二进制帧回 Hub 时已重建为
//!   `SipEvent` JSON，无法经 baresip 钩子抓。此处按 resource 在中继层捕获其 JSON，
//!   拼接入同一个 pcap（Wireshark 中该流为 JSON 文本）。
//!
//! 抓包以 **Hub 全局开关** 驱动（任一 resource 发起 start 即开启 UA₁ 钩子与 UA₂ 中继记录），
//! 停止时合并两条流的报文导出。

use std::sync::Mutex;

use rex_sip::capture::{self, CapturedPacket};

/// Hub 级抓包状态：是否开启 + 按 resource 累积的 UA₂ 中继层报文。
#[derive(Default)]
pub struct SipCaptureRegistry {
    /// 任一会话激活即开启全局 UA₁ 钩子 + UA₂ 中继记录。
    active: Mutex<bool>,
    /// UA₂ 链式路径按 resource 累积的 `SipEvent` JSON 报文。
    ua2: Mutex<std::collections::HashMap<String, Vec<CapturedPacket>>>,
}

impl SipCaptureRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 开启抓包（幂等）。UA₁ 为 Hub 全局钩子；UA₂ 中继记录随 stop 时按 resource 归属。
    pub fn start(&self, _resource_id: &str) {
        *self.active.lock().unwrap() = true;
        // 确保 baresip runtime 已初始化，使 UA₁ 全局钩子生效（首个 UA 之前也可抓）。
        let _ = capture::start();
    }

    /// 停止抓包：合并 UA₁（全局真实 SIP 字节）+ 当前 resource 的 UA₂（中继 JSON）。
    pub fn stop(&self, resource_id: &str) -> Vec<CapturedPacket> {
        let ua1 = capture::stop();
        let ua2 = self
            .ua2
            .lock()
            .unwrap()
            .remove(resource_id)
            .unwrap_or_default();
        *self.active.lock().unwrap() = false;
        merge(ua1, ua2)
    }

    pub fn is_active(&self, _resource_id: &str) -> bool {
        *self.active.lock().unwrap()
    }

    /// 记录 UA₂ 中继层报文（Agent 回传的 `SipEvent` JSON）。仅全局激活时累积。
    pub fn record_ua2(&self, resource_id: &str, direction: &str, raw: &str) {
        if !*self.active.lock().unwrap() {
            return;
        }
        let pkt = CapturedPacket {
            // ts_us 由 rex_sip 钩子侧使用真实时间；此处 UA₂ 同样需时间戳——复用模型字段。
            ts_us: now_us(),
            direction: direction.to_string(),
            raw: raw.to_string(),
        };
        self.ua2
            .lock()
            .unwrap()
            .entry(resource_id.to_string())
            .or_default()
            .push(pkt);
    }

    /// 当前累积报文快照（UA₁ 全局 + 该 resource 的 UA₂），无论抓包是否仍在进行。
    pub fn snapshot(&self, resource_id: &str) -> Vec<CapturedPacket> {
        let ua1 = capture::snapshot();
        let ua2 = self
            .ua2
            .lock()
            .unwrap()
            .get(resource_id)
            .cloned()
            .unwrap_or_default();
        merge(ua1, ua2)
    }
}

/// 按时间戳升序合并两条流（UA₁ 真实 SIP 在前，UA₂ 中继在后仅作保序兜底）。
fn merge(mut a: Vec<CapturedPacket>, mut b: Vec<CapturedPacket>) -> Vec<CapturedPacket> {
    a.append(&mut b);
    a.sort_by_key(|p| p.ts_us);
    a
}

fn now_us() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}
