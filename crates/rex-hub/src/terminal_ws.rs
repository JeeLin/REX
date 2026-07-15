//! WebSocket 终端桥接 — 浏览器 ↔ Hub ↔ SSH 服务器。
//!
//! 协议：JSON 消息，数据部分 base64 编码。

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use rex_ssh::{SshConfig, SshSession, TerminalEvent};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};

/// 前端 → 后端的消息
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMsg {
    #[serde(rename = "terminal.connect")]
    Connect {
        host: String,
        port: u16,
        username: String,
        password: Option<String>,
        #[serde(rename = "privateKey")]
        private_key: Option<String>,
    },
    #[serde(rename = "terminal.data")]
    Data { data: String },
    #[serde(rename = "terminal.resize")]
    Resize { cols: u32, rows: u32 },
    #[serde(rename = "terminal.disconnect")]
    Disconnect,
}

/// 后端 → 前端的消息
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMsg {
    #[serde(rename = "terminal.connected")]
    Connected { payload: ConnectedPayload },
    #[serde(rename = "terminal.data")]
    Data { payload: DataPayload },
    #[serde(rename = "terminal.disconnected")]
    Disconnected { payload: DisconnectedPayload },
    #[serde(rename = "terminal.error")]
    Error { payload: ErrorPayload },
}

#[derive(Debug, Serialize)]
struct ConnectedPayload {
    #[serde(rename = "sessionId")]
    session_id: String,
}

#[derive(Debug, Serialize)]
struct DataPayload {
    data: String,
}

#[derive(Debug, Serialize)]
struct DisconnectedPayload {
    reason: String,
}

#[derive(Debug, Serialize)]
struct ErrorPayload {
    message: String,
}

/// 处理 WebSocket 升级请求
pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

/// WebSocket 连接处理主循环
async fn handle_socket(mut ws: WebSocket) {
    let session_id = format!("sess_{}", &uuid::Uuid::new_v4().to_string()[..8]);

    // 等待客户端发送 terminal.connect
    let connect_msg = match ws.recv().await {
        Some(Ok(Message::Text(text))) => serde_json::from_str::<ClientMsg>(&text).ok(),
        _ => None,
    };

    let config = match connect_msg {
        Some(ClientMsg::Connect {
            host,
            port,
            username,
            password,
            private_key,
        }) => SshConfig {
            host,
            port,
            username,
            password,
            private_key,
        },
        _ => {
            let _ = send_ws_error(&mut ws, "expected terminal.connect").await;
            return;
        }
    };

    // 建立 SSH 连接
    let session = match SshSession::connect(config).await {
        Ok(s) => s,
        Err(e) => {
            let _ = send_ws_error(&mut ws, &format!("SSH connection failed: {e}")).await;
            return;
        }
    };

    // 发送 connected 确认
    let _ = ws
        .send(Message::Text(
            serde_json::to_string(&ServerMsg::Connected {
                payload: ConnectedPayload {
                    session_id: session_id.clone(),
                },
            })
            .unwrap(),
        ))
        .await;

    let session = Arc::new(Mutex::new(session));

    // 拆分 WebSocket 为读/写两半
    let (mut ws_sink, mut ws_stream) = ws.split();

    // WS → SSH 命令通道
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<ClientMsg>(64);
    // SSH → WS 数据通道
    let (data_tx, mut data_rx) = mpsc::channel::<String>(512);

    // 任务1：WS 读取 → cmd_tx
    let ws_read_task = tokio::spawn(async move {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) {
                        let is_disconnect = matches!(client_msg, ClientMsg::Disconnect);
                        if cmd_tx.send(client_msg).await.is_err() {
                            break;
                        }
                        if is_disconnect {
                            break;
                        }
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    });

    // 任务2：SSH 收发（cmd_rx + session → data_tx）
    let session_for_ssh = session.clone();
    let ssh_task = tokio::spawn(async move {
        loop {
            let mut session = session_for_ssh.lock().await;
            tokio::select! {
                // 接收来自 WS 的命令
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(ClientMsg::Data { data }) => {
                            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&data) {
                                let _ = session.send_data(bytes::Bytes::from(bytes)).await;
                            }
                        }
                        Some(ClientMsg::Resize { cols, rows }) => {
                            let _ = session.resize(cols, rows).await;
                        }
                        Some(ClientMsg::Disconnect) | None => {
                            let _ = session.disconnect().await;
                            break;
                        }
                        _ => {}
                    }
                }
                // 接收来自 SSH 的事件
                event = session.recv() => {
                    drop(session); // 释放锁
                    match event {
                        Some(TerminalEvent::Data(data)) => {
                            let encoded = base64::engine::general_purpose::STANDARD.encode(data.as_bytes());
                            if data_tx.send(encoded).await.is_err() {
                                break;
                            }
                        }
                        Some(TerminalEvent::Disconnected(reason)) => {
                            let msg = ServerMsg::Disconnected {
                                payload: DisconnectedPayload { reason },
                            };
                            let _ = data_tx.send(serde_json::to_string(&msg).unwrap_or_default()).await;
                            break;
                        }
                        None => break,
                    }
                    continue; // 不需要重新获取锁
                }
            }
            drop(session);
        }
    });

    // 任务3：data_rx → WebSocket 写入
    let ws_write_task = tokio::spawn(async move {
        while let Some(data) = data_rx.recv().await {
            let msg = if data.starts_with('{') {
                // JSON 控制消息（disconnected）直接发送
                Message::Text(data)
            } else {
                // base64 编码的终端数据
                let wrapped = ServerMsg::Data {
                    payload: DataPayload { data },
                };
                Message::Text(serde_json::to_string(&wrapped).unwrap())
            };
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 等待任一任务结束
    tokio::select! {
        _ = ws_read_task => {},
        _ = ssh_task => {},
        _ = ws_write_task => {},
    }

    tracing::debug!(session_id, "terminal session ended");
}

async fn send_ws_error(ws: &mut WebSocket, msg: &str) -> Result<(), axum::Error> {
    ws.send(Message::Text(
        serde_json::to_string(&ServerMsg::Error {
            payload: ErrorPayload {
                message: msg.into(),
            },
        })
        .unwrap(),
    ))
    .await
}
