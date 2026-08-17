//! Agent WebSocket 客户端 — 连接 Hub，处理认证、心跳、资源连接代理、更新。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use rex_sip::SipUaTrait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

// ═══════════════════════════════════════
// TLS Insecure 模式（自签名证书跳过验证）
// ═══════════════════════════════════════

#[derive(Debug)]
struct InsecureVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dcsa: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dcsa: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}

// ═══════════════════════════════════════
// 消息协议（与 Hub 侧 agent_ws.rs 对称）
// ═══════════════════════════════════════

/// Hub → Agent 消息
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum HubMsg {
    #[serde(rename = "auth_ok")]
    AuthOk { payload: AuthOkPayload },
    #[serde(rename = "auth_fail")]
    AuthFail { payload: AuthFailPayload },
    #[serde(rename = "heartbeat_ack")]
    HeartbeatAck,
    #[serde(rename = "connect")]
    Connect { payload: ConnectRequest },
    #[serde(rename = "close")]
    Close { payload: ChannelPayload },
    #[serde(rename = "update")]
    Update {
        payload: rex_common::update::UpdateCommand,
    },
}

#[derive(Debug, Deserialize)]
struct AuthOkPayload {
    agent_id: String,
}

#[derive(Debug, Deserialize)]
struct AuthFailPayload {
    reason: String,
}

#[derive(Debug, Deserialize)]
struct ConnectRequest {
    request_id: String,
    #[allow(dead_code)]
    resource_id: String,
    protocol: String,
    config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChannelPayload {
    #[allow(dead_code)]
    channel_id: String,
}

/// Agent → Hub 消息
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum AgentMsg {
    #[serde(rename = "auth")]
    Auth { payload: AuthPayload },
    #[serde(rename = "heartbeat")]
    Heartbeat { payload: HeartbeatPayload },
    #[serde(rename = "connected")]
    Connected { payload: ConnectedPayload },
    #[serde(rename = "connect_error")]
    ConnectError { payload: ConnectErrorPayload },
    #[serde(rename = "closed")]
    Closed { payload: ChannelPayload },
    #[serde(rename = "update_progress")]
    UpdateProgress { payload: UpdateProgressPayload },
}

#[derive(Debug, Serialize)]
struct UpdateProgressPayload {
    phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuthPayload {
    token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct HeartbeatPayload {
    version: String,
    os: String,
    arch: String,
    hostname: String,
}

#[derive(Debug, Serialize)]
struct ConnectedPayload {
    request_id: String,
    channel_id: String,
}

#[derive(Debug, Serialize)]
struct ConnectErrorPayload {
    request_id: String,
    error: String,
}

// ═══════════════════════════════════════
// Agent 配置
// ═══════════════════════════════════════

pub struct AgentConfig {
    pub hub_url: String,
    pub agent_token: String,
    pub auto_update: bool,
    pub tls_insecure: bool,
    pub heartbeat_interval: u64,
}

impl AgentConfig {
    pub fn from_env() -> Result<Self, String> {
        let hub_url =
            std::env::var("REX_HUB_URL").map_err(|_| "REX_HUB_URL not set".to_string())?;
        let agent_token =
            std::env::var("REX_AGENT_TOKEN").map_err(|_| "REX_AGENT_TOKEN not set".to_string())?;
        let auto_update = std::env::var("REX_AUTO_UPDATE")
            .unwrap_or_else(|_| "true".into())
            .parse::<bool>()
            .unwrap_or(true);
        let tls_insecure = std::env::var("REX_TLS_INSECURE")
            .map(|v| v == "true")
            .unwrap_or(false);
        let heartbeat_interval = std::env::var("REX_HEARTBEAT_INTERVAL")
            .unwrap_or_else(|_| "30".into())
            .parse::<u64>()
            .unwrap_or(30);
        Ok(Self {
            hub_url,
            agent_token,
            auto_update,
            tls_insecure,
            heartbeat_interval,
        })
    }
}

// ═══════════════════════════════════════
// Channel 管理
// ═══════════════════════════════════════

struct LocalChannel {
    #[allow(dead_code)]
    channel_id: String,
    data_tx: mpsc::Sender<Vec<u8>>,
}

// ═══════════════════════════════════════
// 主入口
// ═══════════════════════════════════════

pub async fn run_agent(config: AgentConfig) {
    let channels: Arc<RwLock<HashMap<String, LocalChannel>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let mut backoff = 1u64; // 初始退避 1 秒
    const MAX_BACKOFF: u64 = 30;

    loop {
        tracing::info!(hub_url = %config.hub_url, "connecting to hub");

        match connect_and_run(&config, channels.clone()).await {
            Ok(()) => {
                tracing::info!("connection closed cleanly");
                backoff = 1; // 正常关闭，重置退避
            }
            Err(e) => {
                tracing::error!(error = %e, "connection failed");
            }
        }

        // 指数退避重连：1s → 2s → 4s → 8s → 16s → 30s（最大）
        tracing::info!(seconds = backoff, "reconnecting");
        tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

async fn connect_and_run(
    config: &AgentConfig,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 构建 WebSocket URL
    let ws_url = build_ws_url(&config.hub_url, &config.agent_token)?;
    tracing::info!(url = %ws_url, "connecting");

    // 根据 TLS 配置选择连接方式
    let (ws_stream, _) = if config.tls_insecure && ws_url.starts_with("wss://") {
        use tokio_tungstenite::connect_async_tls_with_config;
        use tokio_tungstenite::Connector;

        let request = ws_url.as_str();
        let connector = Connector::Rustls(Arc::new(
            rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(InsecureVerifier))
                .with_no_client_auth(),
        ));
        connect_async_tls_with_config(request, None, false, Some(connector)).await?
    } else {
        connect_async(&ws_url).await?
    };
    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // 1. 发送 auth（token + name，Hub 通过 token 查找环境并注册 agent）
    let agent_name = std::env::var("REX_AGENT_NAME").unwrap_or_else(|_| "agent".into());
    let auth_msg = serde_json::to_string(&AgentMsg::Auth {
        payload: AuthPayload {
            token: config.agent_token.clone(),
            name: Some(agent_name),
        },
    })?;
    ws_sink.send(Message::Text(auth_msg)).await?;

    // 2. 等待 auth_ok
    match ws_stream.next().await {
        Some(Ok(Message::Text(text))) => {
            let msg: HubMsg = serde_json::from_str(&text)?;
            match msg {
                HubMsg::AuthOk { payload } => {
                    tracing::info!(agent_id = %payload.agent_id, "authenticated");
                }
                HubMsg::AuthFail { payload } => {
                    return Err(format!("auth failed: {}", payload.reason).into());
                }
                _ => {
                    return Err("expected auth_ok or auth_fail".into());
                }
            }
        }
        _ => {
            return Err("connection closed during auth".into());
        }
    }

    // 3. 创建事件通道（用于从 channel 任务发数据到 ws_sink）
    let (evt_tx, mut evt_rx) = mpsc::channel::<AgentEvent>(256);

    // 4. 启动写入任务（evt_rx → ws_sink）
    let ws_write_task = tokio::spawn(async move {
        while let Some(evt) = evt_rx.recv().await {
            let msg = match evt {
                AgentEvent::Text(t) => Message::Text(t),
                AgentEvent::Binary(b) => Message::Binary(b),
                AgentEvent::Close => break,
            };
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 5. 心跳任务
    let evt_tx_hb = evt_tx.clone();
    let heartbeat_interval = config.heartbeat_interval;
    let heartbeat_task = tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(heartbeat_interval));
        loop {
            interval.tick().await;
            let hostname = hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_default();
            let hb = serde_json::to_string(&AgentMsg::Heartbeat {
                payload: HeartbeatPayload {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    os: std::env::consts::OS.to_string(),
                    arch: std::env::consts::ARCH.to_string(),
                    hostname,
                },
            })
            .unwrap();
            if evt_tx_hb.send(AgentEvent::Text(hb)).await.is_err() {
                break;
            }
        }
    });

    // 7. 读取循环（ws_stream → 处理消息）
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(hub_msg) = serde_json::from_str::<HubMsg>(&text) {
                    match hub_msg {
                        HubMsg::Connect { payload } => {
                            let evt_tx2 = evt_tx.clone();
                            let channels = channels.clone();
                            tokio::spawn(async move {
                                handle_connect(payload, evt_tx2, channels).await;
                            });
                        }
                        HubMsg::Close { payload } => {
                            let mut chs = channels.write().await;
                            if let Some(ch) = chs.remove(&payload.channel_id) {
                                let _ = ch.data_tx.send(vec![]).await; // signal close
                            }
                        }
                        HubMsg::HeartbeatAck => {
                            tracing::trace!("heartbeat ack");
                        }
                        HubMsg::Update { payload } => {
                            if !config.auto_update {
                                tracing::info!("auto_update disabled, ignoring update command");
                                continue;
                            }
                            let version = payload.version.clone();
                            let evt_tx_clone = evt_tx.clone();
                            tokio::spawn(async move {
                                handle_update(payload, evt_tx_clone).await;
                            });
                            tracing::info!(version = %version, "update started");
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                // 二进制帧：前 4 字节 channelId
                if data.len() >= 4 {
                    let ch_id = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                    let ch_id_str = ch_id.to_string();
                    let payload = data[4..].to_vec();
                    let chs = channels.read().await;
                    if let Some(ch) = chs.get(&ch_id_str) {
                        let _ = ch.data_tx.send(payload).await;
                    }
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    // 8. 清理
    heartbeat_task.abort();
    ws_write_task.abort();
    channels.write().await.clear();

    Ok(())
}

/// 处理 Hub 发来的 connect 请求
async fn handle_connect(
    req: ConnectRequest,
    evt_tx: mpsc::Sender<AgentEvent>,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
) {
    // SIP 资源走 Agent 内网 UA₂：不建 TCP，由 UA₂ 直接对内网 SIP server 信令。
    if req.protocol == "sip" {
        handle_connect_sip(req, evt_tx, channels).await;
        return;
    }

    // channel_id 必须为数值：隧道二进制帧以「4B u32 channelId」前缀路由，
    // Hub/Agent 两侧均用 `u32::from_be_bytes` 解前缀后 `to_string()` 查表。
    // 若为非数值（如旧 "ch_{uuid}"），`parse::<u32>()` 失败回退为 0，会导致
    // 回传帧全部命中键 "0" 而丢帧——终端/SQL/Redis/S3 等协议因此无返回。
    let channel_id = AGENT_CHANNEL_SEQ.fetch_add(1, Ordering::SeqCst).to_string();

    // 解析目标地址
    let host = req
        .config
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let port = req
        .config
        .get("port")
        .and_then(|v| v.as_u64())
        .unwrap_or(22) as u16;

    if host.is_empty() {
        let err_msg = serde_json::to_string(&AgentMsg::ConnectError {
            payload: ConnectErrorPayload {
                request_id: req.request_id,
                error: "missing host".into(),
            },
        })
        .unwrap();
        let _ = evt_tx.send(AgentEvent::Text(err_msg)).await;
        return;
    }

    // 建立本地 TCP 连接
    // IPv6 addresses need brackets: [::1]:22
    let addr = if host.contains(':') && !host.starts_with('[') {
        format!("[{}]:{}", host, port)
    } else {
        format!("{}:{}", host, port)
    };
    match TcpStream::connect(&addr).await {
        Ok(tcp_stream) => {
            tracing::info!(
                channel_id = %channel_id,
                request_id = %req.request_id,
                protocol = %req.protocol,
                addr = %addr,
                "local connection established"
            );

            // 通知 Hub 连接成功
            let ok_msg = serde_json::to_string(&AgentMsg::Connected {
                payload: ConnectedPayload {
                    request_id: req.request_id,
                    channel_id: channel_id.clone(),
                },
            })
            .unwrap();
            let _ = evt_tx.send(AgentEvent::Text(ok_msg)).await;

            // 注册 channel
            let (data_tx, mut data_rx) = mpsc::channel::<Vec<u8>>(512);
            {
                let mut chs = channels.write().await;
                chs.insert(
                    channel_id.clone(),
                    LocalChannel {
                        channel_id: channel_id.clone(),
                        data_tx,
                    },
                );
            }

            // 拆分 TCP 为读/写
            let (mut tcp_reader, mut tcp_writer) = tcp_stream.into_split();

            // TCP 读取 → Hub（通过 evt_tx）
            let evt_tx_read = evt_tx.clone();
            let ch_id = channel_id.clone();
            let tcp_read_task = tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                loop {
                    match tcp_reader.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            // 构造二进制帧：[4B channelId][data]
                            let mut frame = Vec::with_capacity(4 + n);
                            let ch_id_bytes = ch_id.parse::<u32>().unwrap_or(0).to_be_bytes();
                            frame.extend_from_slice(&ch_id_bytes);
                            frame.extend_from_slice(&buf[..n]);
                            if evt_tx_read.send(AgentEvent::Binary(frame)).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });

            // Hub 数据 → TCP 写入
            let tcp_write_task = tokio::spawn(async move {
                while let Some(data) = data_rx.recv().await {
                    if data.is_empty() {
                        break; // close signal
                    }
                    if tcp_writer.write_all(&data).await.is_err() {
                        break;
                    }
                }
            });

            // 等待任一方向结束
            tokio::select! {
                _ = tcp_read_task => {},
                _ = tcp_write_task => {},
            }

            // 清理 channel
            {
                let mut chs = channels.write().await;
                chs.remove(&channel_id);
            }

            // 通知 Hub 关闭 channel
            let close_msg = serde_json::to_string(&AgentMsg::Closed {
                payload: ChannelPayload {
                    channel_id: channel_id.clone(),
                },
            })
            .unwrap();
            let _ = evt_tx.send(AgentEvent::Text(close_msg)).await;

            tracing::info!(channel_id = %channel_id, "local connection closed");
        }
        Err(e) => {
            tracing::warn!(
                request_id = %req.request_id,
                error = %e,
                "local connection failed"
            );
            let err_msg = serde_json::to_string(&AgentMsg::ConnectError {
                payload: ConnectErrorPayload {
                    request_id: req.request_id,
                    error: e.to_string(),
                },
            })
            .unwrap();
            let _ = evt_tx.send(AgentEvent::Text(err_msg)).await;
        }
    }
}

/// 全资源共享的 channel 计数：所有 protocol（ssh/sql/redis/s3/sip…）统一用
/// 数值 channel_id，使隧道二进制帧的「4B u32 前缀」在 Hub/Agent 两侧正确往返
/// （见 `crates/rex-hub/src/agent_ws.rs` 的 `from_be_bytes` 路由）。非数值
/// channel_id 会导致 `parse::<u32>()` 失败回退为 0、回传帧全部丢帧。
static AGENT_CHANNEL_SEQ: AtomicU32 = AtomicU32::new(1);

/// 处理 SIP 资源的 connect 请求：Agent 内起真实 SipUa（UA₂）作为最终 SIP 终端，
/// 直接对内网 SIP server 信令。Hub 与前端仅做 JSON 控制/事件的中继（见 Hub 侧
/// `sip_ws::handle_agent_sip`），不跑任何 UA、不搬运原始 SIP 字节。
async fn handle_connect_sip(
    req: ConnectRequest,
    evt_tx: mpsc::Sender<AgentEvent>,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
) {
    // channel_id 必须为数值，否则隧道二进制帧的 u32 前缀无法路由。
    let channel_id = AGENT_CHANNEL_SEQ.fetch_add(1, Ordering::SeqCst).to_string();

    let sip_cfg = match parse_sip_config(&req.config) {
        Ok(c) => c,
        Err(e) => {
            let err_msg = serde_json::to_string(&AgentMsg::ConnectError {
                payload: ConnectErrorPayload {
                    request_id: req.request_id,
                    error: e,
                },
            })
            .unwrap();
            let _ = evt_tx.send(AgentEvent::Text(err_msg)).await;
            return;
        }
    };

    let ua = match rex_sip::SipUa::real(sip_cfg) {
        Ok(ua) => ua,
        Err(e) => {
            let err_msg = serde_json::to_string(&AgentMsg::ConnectError {
                payload: ConnectErrorPayload {
                    request_id: req.request_id,
                    error: format!("SIP UA init failed: {e}"),
                },
            })
            .unwrap();
            let _ = evt_tx.send(AgentEvent::Text(err_msg)).await;
            return;
        }
    };

    // 通知 Hub 连接成功（UA₂ 已就绪）。
    let ok_msg = serde_json::to_string(&AgentMsg::Connected {
        payload: ConnectedPayload {
            request_id: req.request_id,
            channel_id: channel_id.clone(),
        },
    })
    .unwrap();
    let _ = evt_tx.send(AgentEvent::Text(ok_msg)).await;

    // 注册 channel（用于接收 Hub 经隧道发来的 SipControl 帧）。
    let (data_tx, mut data_rx) = mpsc::channel::<Vec<u8>>(512);
    {
        let mut chs = channels.write().await;
        chs.insert(
            channel_id.clone(),
            LocalChannel {
                channel_id: channel_id.clone(),
                data_tx,
            },
        );
    }

    // UA₂ 事件流 → 隧道 SipEvent 帧；Hub 经隧道发来的 SipControl 帧 → UA₂。
    run_sip_ua2(ua, channel_id, evt_tx, channels, data_rx).await;
}

/// 驱动 UA₂：把 baresip 事件封装为 `SipEvent` JSON 经隧道回 Hub；
/// 把 Hub 经隧道发来的 `SipControl` 帧转调 UA₂ 方法。任一方向结束即清理。
async fn run_sip_ua2(
    ua: rex_sip::SipUa,
    channel_id: String,
    evt_tx: mpsc::Sender<AgentEvent>,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
    mut data_rx: mpsc::Receiver<Vec<u8>>,
) {
    let ch_id_num = channel_id.parse::<u32>().unwrap_or(0);
    let ua = Arc::new(ua);

    // 注册 UA₂，错误直接上报并结束。
    if let Err(e) = ua.register().await {
        let _ = send_sip_event(
            &evt_tx,
            ch_id_num,
            &rex_sip::SipEvent::RegistrationFailed {
                reason: e.to_string(),
            },
        )
        .await;
        cleanup_sip_channel(&channels, &channel_id).await;
        return;
    }

    // UA₂ 事件 → 隧道帧。
    let mut events = ua.events();
    let evt_tx_event = evt_tx.clone();
    let event_task = tokio::spawn(async move {
        while let Some(ev) = events.recv().await {
            if send_sip_event(&evt_tx_event, ch_id_num, &ev).await.is_err() {
                break;
            }
        }
    });

    // 隧道帧 → UA₂ 控制。
    let ua_ctrl = ua.clone();
    let ctrl_channel_id = channel_id.clone();
    let ctrl_task = tokio::spawn(async move {
        while let Some(frame) = data_rx.recv().await {
            if frame.is_empty() {
                break; // 关闭信号
            }
            let ctrl: rex_sip::SipControl = match serde_json::from_slice(&frame) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(action = "SIP_CTRL_PARSE", channel_id = %ctrl_channel_id, error = %e, "invalid SipControl frame");
                    continue;
                }
            };
            if let Err(e) = dispatch_sip_control(ua_ctrl.as_ref(), &ctrl).await {
                tracing::warn!(action = "SIP_CTRL_DISPATCH", channel_id = %ctrl_channel_id, error = %e, "SipControl dispatch failed");
            }
        }
    });

    tokio::select! {
        _ = event_task => {},
        _ = ctrl_task => {},
    }

    cleanup_sip_channel(&channels, &channel_id).await;
    tracing::info!(action = "SIP_UA2_END", channel_id = %channel_id, "agent SIP UA2 session ended");
}

/// 将 UA₂ 事件封成 `[4B channelId][SipEvent JSON]` 二进制帧经隧道发往 Hub。
async fn send_sip_event(
    evt_tx: &mpsc::Sender<AgentEvent>,
    ch_id_num: u32,
    ev: &rex_sip::SipEvent,
) -> Result<(), mpsc::error::SendError<AgentEvent>> {
    let payload = match serde_json::to_vec(ev) {
        Ok(p) => p,
        Err(_) => return Ok(()),
    };
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&ch_id_num.to_be_bytes());
    frame.extend_from_slice(&payload);
    evt_tx.send(AgentEvent::Binary(frame)).await
}

/// 将 Hub 发来的 `SipControl` 转调 UA₂ 对应方法。
async fn dispatch_sip_control(
    ua: &rex_sip::SipUa,
    ctrl: &rex_sip::SipControl,
) -> Result<(), String> {
    match ctrl {
        rex_sip::SipControl::Register => ua.register().await.map_err(|e| e.to_string()),
        rex_sip::SipControl::Dial { destination } => ua
            .dial(destination)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string()),
        rex_sip::SipControl::Answer { call_id } => {
            ua.answer(call_id).await.map_err(|e| e.to_string())
        }
        rex_sip::SipControl::Hangup { call_id } => {
            ua.hangup(call_id).await.map_err(|e| e.to_string())
        }
        rex_sip::SipControl::Hold { call_id } => ua.hold(call_id).await.map_err(|e| e.to_string()),
        rex_sip::SipControl::Unhold { call_id } => {
            ua.unhold(call_id).await.map_err(|e| e.to_string())
        }
        rex_sip::SipControl::Dtmf { call_id, digit } => {
            ua.dtmf(call_id, *digit).await.map_err(|e| e.to_string())
        }
    }
}

/// 从 channel 表移除 SIP 会话，并通知 Hub 关闭。
async fn cleanup_sip_channel(
    channels: &Arc<RwLock<HashMap<String, LocalChannel>>>,
    channel_id: &str,
) {
    {
        let mut chs = channels.write().await;
        chs.remove(channel_id);
    }
}

/// 从 connect config 解析 SIP 配置（对应 Hub 侧 `load_sip_conn` 的字段约定）。
fn parse_sip_config(cfg: &serde_json::Value) -> Result<rex_sip::SipConfig, String> {
    let server = cfg
        .get("server")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "sip: missing server".to_string())?
        .to_string();
    let port = cfg
        .get("port")
        .and_then(|v| v.as_u64())
        .map(|p| p as u16)
        .unwrap_or(5060);
    let username = cfg
        .get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "sip: missing username".to_string())?
        .to_string();
    let password = cfg
        .get("password")
        .and_then(|v| v.as_str())
        .map(String::from);
    let display_name = cfg
        .get("displayName")
        .and_then(|v| v.as_str())
        .map(String::from);
    let transport = match cfg
        .get("transport")
        .and_then(|v| v.as_str())
        .unwrap_or("udp")
    {
        "tcp" => rex_sip::SipTransport::Tcp,
        "tls" => rex_sip::SipTransport::Tls,
        _ => rex_sip::SipTransport::Udp,
    };
    Ok(rex_sip::SipConfig {
        server,
        port,
        username,
        password,
        display_name,
        transport,
    })
}

enum AgentEvent {
    Text(String),
    Binary(Vec<u8>),
    #[allow(dead_code)]
    Close,
}

/// 处理 Hub 发来的更新指令
async fn handle_update(cmd: rex_common::update::UpdateCommand, evt_tx: mpsc::Sender<AgentEvent>) {
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "failed to get current exe path");
            return;
        }
    };

    let report: crate::updater::ProgressReporter = Box::new(move |progress| {
        let _evt_tx = evt_tx.clone();
        Box::pin(async move {
            let phase_str = match progress.phase {
                rex_common::update::UpdatePhase::Idle => "idle",
                rex_common::update::UpdatePhase::Downloading => "downloading",
                rex_common::update::UpdatePhase::Verifying => "verifying",
                rex_common::update::UpdatePhase::Replacing => "replacing",
                rex_common::update::UpdatePhase::Restarting => "restarting",
                rex_common::update::UpdatePhase::Error => "error",
                _ => "unknown",
            };
            let msg = serde_json::to_string(&AgentMsg::UpdateProgress {
                payload: UpdateProgressPayload {
                    phase: phase_str.to_string(),
                    progress: if progress.progress > 0.0 {
                        Some(progress.progress)
                    } else {
                        None
                    },
                    error: progress.error,
                },
            })
            .unwrap();
            let _ = _evt_tx.send(AgentEvent::Text(msg)).await;
        })
    });

    match crate::updater::run_update(cmd, current_exe, &report).await {
        Ok(()) => {
            tracing::info!("update prepared, exiting for supervisor");
            // 等一小段时间让进度消息发出去
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            std::process::exit(42);
        }
        Err(e) => {
            tracing::error!(error = %e, "update failed");
        }
    }
}

fn build_ws_url(hub_url: &str, token: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut url = Url::parse(hub_url)?;
    // 替换 scheme: http → ws, https → wss
    match url.scheme() {
        "http" => url.set_scheme("ws").unwrap(),
        "https" => url.set_scheme("wss").unwrap(),
        "ws" | "wss" => {}
        _ => {
            url.set_scheme("ws").unwrap();
        }
    }
    url.set_path("/ws/agent");
    url.query_pairs_mut().append_pair("token", token);
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sip_config_full() {
        let cfg = serde_json::json!({
            "server": "sip.example.com",
            "port": 5061,
            "username": "1000",
            "password": "secret",
            "displayName": "Alice",
            "transport": "tls"
        });
        let sip = parse_sip_config(&cfg).unwrap();
        assert_eq!(sip.server, "sip.example.com");
        assert_eq!(sip.port, 5061);
        assert_eq!(sip.username, "1000");
        assert_eq!(sip.password.as_deref(), Some("secret"));
        assert_eq!(sip.display_name.as_deref(), Some("Alice"));
        assert_eq!(sip.transport, rex_sip::SipTransport::Tls);
    }

    #[test]
    fn parse_sip_config_defaults_and_optional_password() {
        let cfg = serde_json::json!({
            "server": "sip.x",
            "username": "u"
        });
        let sip = parse_sip_config(&cfg).unwrap();
        assert_eq!(sip.port, 5060);
        assert_eq!(sip.transport, rex_sip::SipTransport::Udp);
        assert!(sip.password.is_none());
        assert!(sip.display_name.is_none());
    }

    #[test]
    fn parse_sip_config_missing_server_errors() {
        let cfg = serde_json::json!({ "username": "u" });
        assert!(parse_sip_config(&cfg).is_err());
    }

    #[test]
    fn agent_channel_seq_is_numeric() {
        // 隧道二进制帧前缀要求所有 channel_id 为数值，否则 `parse::<u32>()`
        // 失败回退为 0，回传帧会以键 "0" 落空而丢帧。
        let id = AGENT_CHANNEL_SEQ.fetch_add(1, Ordering::SeqCst).to_string();
        assert!(id.parse::<u32>().is_ok());
    }

    #[test]
    fn non_numeric_channel_id_drops_return_frame() {
        // 回归：非数值 channel_id 经 `parse::<u32>().unwrap_or(0)` 后前缀为 0，
        // 而 map 以原始串为键，导致回传帧丢帧——证明改用数值 channel_id 的必要性。
        let bad = "ch_1234abcd";
        assert_eq!(bad.parse::<u32>().unwrap_or(0), 0);
    }

    #[test]
    fn numeric_channel_id_round_trips_through_tunnel_prefix() {
        // 修复后契约：数值 channel_id 经「本端 parse::<u32>() → 4B 前缀」编码，
        // Hub 侧 `u32::from_be_bytes` 解前缀后 `to_string()` 得到的键，必须等于
        // 本端注册 channel 时用的 channel_id——否则回传帧落空被丢弃。
        let channel_id = AGENT_CHANNEL_SEQ.fetch_add(1, Ordering::SeqCst).to_string();
        let ch_id_num = channel_id.parse::<u32>().unwrap();

        // 模拟 Agent TCP→Hub 编码：前缀 = channel_id 的 u32 大端。
        let mut frame = ch_id_num.to_be_bytes().to_vec();
        frame.extend_from_slice(b"shell output");

        // 模拟 Hub 解码：读前 4 字节 → 键。
        let decoded = u32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]);
        assert_eq!(decoded.to_string(), channel_id);
        assert_eq!(&frame[4..], b"shell output");
    }
}
