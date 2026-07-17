//! Agent WebSocket 客户端 — 连接 Hub，处理认证、心跳、资源连接代理。

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

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
}

#[derive(Debug, Serialize)]
struct AuthPayload {
    agent_id: String,
    token: String,
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
    pub agent_id: String,
}

impl AgentConfig {
    pub fn from_env() -> Result<Self, String> {
        let hub_url =
            std::env::var("REX_HUB_URL").map_err(|_| "REX_HUB_URL not set".to_string())?;
        let agent_token = std::env::var("REX_AGENT_TOKEN")
            .map_err(|_| "REX_AGENT_TOKEN not set".to_string())?;
        let agent_id =
            std::env::var("REX_AGENT_ID").map_err(|_| "REX_AGENT_ID not set".to_string())?;
        Ok(Self {
            hub_url,
            agent_token,
            agent_id,
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
    let channels: Arc<RwLock<HashMap<String, LocalChannel>>> = Arc::new(RwLock::new(HashMap::new()));

    loop {
        tracing::info!(hub_url = %config.hub_url, "connecting to hub");

        match connect_and_run(&config, channels.clone()).await {
            Ok(()) => {
                tracing::info!("connection closed cleanly");
            }
            Err(e) => {
                tracing::error!(error = %e, "connection failed");
            }
        }

        // 断线重连（指数退避）
        tracing::info!("reconnecting in 5s...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

async fn connect_and_run(
    config: &AgentConfig,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 构建 WebSocket URL
    let ws_url = build_ws_url(&config.hub_url, &config.agent_token)?;
    tracing::info!(url = %ws_url, "connecting");

    let (ws_stream, _) = connect_async(&ws_url).await?;
    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // 1. 发送 auth
    let auth_msg = serde_json::to_string(&AgentMsg::Auth {
        payload: AuthPayload {
            agent_id: config.agent_id.clone(),
            token: config.agent_token.clone(),
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
    let heartbeat_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
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
                            let evt_tx = evt_tx.clone();
                            let channels = channels.clone();
                            tokio::spawn(async move {
                                handle_connect(payload, evt_tx, channels).await;
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
    let channel_id = format!("ch_{}", &uuid::Uuid::new_v4().to_string()[..8]);

    // 解析目标地址
    let host = req.config.get("host").and_then(|v| v.as_str()).unwrap_or("");
    let port = req.config.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;

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
    let addr = format!("{}:{}", host, port);
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

enum AgentEvent {
    Text(String),
    Binary(Vec<u8>),
    #[allow(dead_code)]
    Close,
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
