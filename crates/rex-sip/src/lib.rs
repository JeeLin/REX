//! REX SIP 协议封装：基于 baresip（C 库，进程内 FFI，bindgen 生成绑定）的安全 UA 抽象。
//!
//! M82a（v0.70.0）只打通信令层：register / dial / answer / hangup / hold / unhold / dtmf
//! 与注册状态、来电、通话状态事件流。Hub 与 Agent 共用此 crate，分别当 UA₁ / UA₂。
//! `SipUa` 的安全封装见 [`baresip_ua`]，测试用 Mock 实现见 [`mock`]。
//!
//! 音频链路（RTP→Opus→WebSocket、麦克风回传）属 0.70.1（M82b），本里程碑
//! [`SipUaTrait::on_rtp`] 仅预留空桩与接口，不实现。

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

// baresip / re 的自动生成 FFI 绑定（含 `ua`/`call`/`account` 等 C 类型与函数）。
// 隔离在私有模块内，对所有 clippy/rustc 诊断 blanket allow，避免生成代码噪声。
#[allow(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::style,
    warnings
)]
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use ffi::*;

pub mod audio_bridge;
pub mod baresip_ua;
pub mod mock;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// SIP 传输方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SipTransport {
    Udp,
    Tcp,
    Tls,
}

impl SipTransport {
    pub fn as_str(&self) -> &'static str {
        match self {
            SipTransport::Udp => "udp",
            SipTransport::Tcp => "tcp",
            SipTransport::Tls => "tls",
        }
    }
}

/// SIP UA 配置（对应资源模型里的 `config_json` SIP 段）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SipConfig {
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub display_name: Option<String>,
    pub transport: SipTransport,
}

/// 通话状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallState {
    Ringing,
    Active,
    Held,
    Ended,
}

/// SIP UA 事件（从 baresip `bevent` 事件总线映射）。
///
/// 经隧道在 Agent(UA₂) ↔ Hub(UA₁) 之间序列化传输（见 [`SipControl`]）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SipEvent {
    Registered,
    RegistrationFailed { reason: String },
    IncomingCall { call_id: String, from: String },
    CallState { call_id: String, state: CallState },
    Message { raw: String },
}

/// Agent(UA₂) ↔ Hub(UA₁) 隧道上的控制指令（前端控制帧的线格式等价物）。
///
/// Hub 收到前端 `sip.dial` 等后转封装为 [`SipControl`]，经隧道 binary 帧
/// `[4B channelId][json]` 发给 Agent 的 UA₂；Agent 调用对应 `SipUaTrait` 方法。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SipControl {
    Register,
    Dial {
        destination: String,
    },
    Answer {
        #[serde(rename = "callId")]
        call_id: String,
    },
    Hangup {
        #[serde(rename = "callId")]
        call_id: String,
    },
    Hold {
        #[serde(rename = "callId")]
        call_id: String,
    },
    Unhold {
        #[serde(rename = "callId")]
        call_id: String,
    },
    Dtmf {
        #[serde(rename = "callId")]
        call_id: String,
        digit: char,
    },
}

/// 音频帧格式（内部 baresip↔Rust 边界统一 S16LE；fmt 仅记录协商结果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub srate: u32,
    pub channels: u8,
}

/// 所有 UA 实现的统一抽象。真实现用 baresip FFI（[`baresip_ua::BaresipSipUa`]），
/// 测试用 [`mock::MockSipUa`]。
#[async_trait::async_trait]
pub trait SipUaTrait {
    async fn register(&self) -> anyhow::Result<()>;
    async fn dial(&self, dest: &str) -> anyhow::Result<String>;
    async fn answer(&self, call_id: &str) -> anyhow::Result<()>;
    async fn hangup(&self, call_id: &str) -> anyhow::Result<()>;
    async fn hold(&self, call_id: &str) -> anyhow::Result<()>;
    async fn unhold(&self, call_id: &str) -> anyhow::Result<()>;
    async fn dtmf(&self, call_id: &str, digit: char) -> anyhow::Result<()>;
    /// 接收事件流；多次调用返回独立的 receiver。
    fn events(&self) -> mpsc::UnboundedReceiver<SipEvent>;
    /// 注册远端→浏览器 PCM 回调（接收侧，M82b）。每帧 RX PCM（i16 LE）到达即触发。
    /// 默认 no-op（Mock 可覆盖）；真实现经 baresip 音频驱动桥接上抛。
    fn on_rtp(&self, _cb: Box<dyn FnMut(&[i16]) + Send + 'static>) {
        let _ = _cb;
    }
    /// 浏览器麦克风 PCM 回传（发送侧，M82b）。把 i16 LE PCM 喂回 baresip 发送链路。
    async fn send_audio(&self, _pcm: Vec<i16>) -> anyhow::Result<()> {
        Ok(())
    }
}

/// 安全的 `SipUa` 句柄：包真 baresip 实现，构造时选择真 / Mock。
pub use baresip_ua::SipUa;

pub use mock::{MockAction, MockSipUa};

pub const APP_NAME: &str = "rex-sip";
