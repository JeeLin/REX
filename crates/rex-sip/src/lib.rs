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
pub mod capture;
pub mod mock;
pub mod video_bridge;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// SIP 传输方式。
///
/// config_json（前端与 `SipProfile`）使用小写 `udp`/`tcp`/`tls`，故序列化用小写。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SipTransport {
    #[default]
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
///
/// `SipConfig` 是**解析后生效配置**：Hub/Agent 的 UA 仍以单份 `SipConfig`
/// 注册/拨号（FFI 边界与隧道帧不变）。多账户切换在存储层与解析层完成，
/// 不触及 UA 实现（见 [`SipProfile`] / [`SipAccount`]）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SipConfig {
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub display_name: Option<String>,
    pub transport: SipTransport,
}

/// SIP 账户（名称下的一个注册身份）。
///
/// 每个账户自带完整 server profile（`server`/`port`/`transport`）与登录凭据，
/// 名称仅是展示分组，不绑定服务器。一个 `SipProfile` 可挂多个账户，
/// 前端切换「当前生效账户」即改变 [`SipProfile::active_account`] 指向的 `id`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SipAccount {
    /// 账户本地标识（同一 profile 内唯一）。
    pub id: String,
    /// 注册域（SIP 服务器地址），缺省时回退资源顶层 `host`。
    #[serde(default)]
    pub server: String,
    /// 注册端口，默认 5060。
    #[serde(default = "default_sip_port")]
    pub port: u16,
    /// 传输方式，默认 udp。
    #[serde(default)]
    pub transport: SipTransport,
    /// 注册用户名（SIP AOR 的用户部分）。
    pub username: String,
    /// 注册密码（可选，匿名注册为 None）。
    #[serde(default)]
    pub password: Option<String>,
    /// 显示名（可选）。
    #[serde(default)]
    pub display_name: Option<String>,
}

/// SIP 资源存储层模型：「名称（= 资源名，仅展示分组）+ 多账户」。
///
/// 这是 `resources.config_json` 中 SIP 段的形状。名称不进 `config_json`，
/// 取 `Resource.name`；注册/拨号所需服务器与凭据全部在生效账户的 [`SipAccount`] 中。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SipProfile {
    /// 名称下挂载的多个账户（各自带 server profile + 凭据）。
    #[serde(default)]
    pub accounts: Vec<SipAccount>,
    /// 当前生效账户 id（指向 [`SipAccount::id`]）。
    #[serde(default)]
    pub active_account: String,
}

/// SIP 默认注册端口。
pub const DEFAULT_SIP_PORT: u16 = 5060;

fn default_sip_port() -> u16 {
    DEFAULT_SIP_PORT
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
    #[allow(clippy::type_complexity)]
    fn on_rtp(&self, _cb: Box<dyn FnMut(&[i16]) + Send + 'static>) {
        let _ = _cb;
    }
    /// 浏览器麦克风 PCM 回传（发送侧，M82b）。把 i16 LE PCM 喂回 baresip 发送链路。
    async fn send_audio(&self, _pcm: Vec<i16>) -> anyhow::Result<()> {
        Ok(())
    }
    /// 注册下行视频像素帧回调（接收侧，0.70.2 子任务 #1）。每帧对端视频像素（RGBA）到达即触发。
    /// 默认 no-op（Mock 可覆盖）；真实现经 baresip 视频驱动桥接上抛。
    #[allow(clippy::type_complexity)]
    fn on_video(&self, _cb: Box<dyn FnMut(&crate::video_bridge::VideoFrame) + Send + 'static>) {
        let _ = _cb;
    }
    /// 浏览器上行视频像素帧回传（发送侧，0.70.2 子任务 #1）。把 RGBA 像素喂回 baresip 发送链路。
    async fn send_video(&self, _frame: crate::video_bridge::VideoFrame) -> anyhow::Result<()> {
        let _ = _frame;
        Ok(())
    }
    /// 实时媒体质量快照（子任务 #5）。默认零值（Mock 可覆盖）。
    #[allow(clippy::type_complexity)]
    fn quality(&self) -> crate::audio_bridge::QualitySnapshot {
        crate::audio_bridge::QualitySnapshot::default()
    }
}

/// 安全的 `SipUa` 句柄：包真 baresip 实现，构造时选择真 / Mock。
pub use baresip_ua::SipUa;

pub use mock::{MockAction, MockSipUa};

pub const APP_NAME: &str = "rex-sip";
