//! Agent 侧文件传输执行层（v0.70.6 子任务 #6）。
//!
//! 此前 file 资源在 agent 模式下由 Hub 直接 `SftpConnector`/`S3Connector` 连目标，
//! 根本没走隧道。本模块让 **Agent 在私网内终结 SFTP / S3 协议**（数据不落浏览器，
//! 满足 AGENTS.md 硬性约束），把列目录/分块读写结果经隧道帧回传 Hub。

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use rex_common::file_transfer::{FileConnectRequest, FileConnector};

use crate::agent_ws::{AgentEvent, LocalChannel};

/// Agent 内建立文件连接并接管隧道上的请求/响应。
pub async fn handle_connect_file(
    request_id: String,
    channel_id: String,
    protocol: String,
    cfg: &serde_json::Value,
    evt_tx: mpsc::Sender<AgentEvent>,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
) {
    let mut connector: Box<dyn FileConnector> = match build_connector(&protocol, cfg).await {
        Ok(c) => c,
        Err(e) => {
            send_session_error(
                &evt_tx,
                &channel_id,
                Some(&request_id),
                &format!("file connection failed: {e}"),
            )
            .await;
            return;
        }
    };

    let ok = serde_json::to_string(&rex_common::agent_proto::AgentSessionMsg::SessionOpened(
        rex_common::agent_proto::SessionOpened {
            request_id,
            channel_id: channel_id.clone(),
            subtype: None,
        },
    ))
    .unwrap_or_default();
    let _ = evt_tx.send(AgentEvent::Text(ok)).await;

    let (data_tx, mut data_rx) = mpsc::channel::<Vec<u8>>(512);
    {
        let mut chs = channels.write().await;
        chs.insert(
            channel_id.clone(),
            LocalChannel {
                channel_id: channel_id.clone(),
                data_tx,
                resize_tx: None,
            },
        );
    }

    while let Some(frame) = data_rx.recv().await {
        if frame.is_empty() {
            break;
        }
        let msg: rex_common::agent_proto::SessionRequest = match serde_json::from_slice(&frame) {
            Ok(m) => m,
            Err(e) => {
                send_session_error(
                    &evt_tx,
                    &channel_id,
                    None,
                    &format!("invalid session_request: {e}"),
                )
                .await;
                continue;
            }
        };
        let resp = match dispatch_file(&mut connector, &msg.kind, &msg.payload).await {
            Ok(data) => rex_common::agent_proto::SessionResponse {
                channel_id: channel_id.clone(),
                seq: msg.seq,
                data,
                error: None,
            },
            Err(e) => rex_common::agent_proto::SessionResponse {
                channel_id: channel_id.clone(),
                seq: msg.seq,
                data: serde_json::Value::Null,
                error: Some(e.to_string()),
            },
        };
        let s = serde_json::to_string(&rex_common::agent_proto::AgentSessionMsg::SessionResponse(
            resp,
        ))
        .unwrap_or_default();
        if evt_tx.send(AgentEvent::Text(s)).await.is_err() {
            break;
        }
    }

    let _ = connector.close().await;
    {
        let mut chs = channels.write().await;
        chs.remove(&channel_id);
    }
    tracing::info!(action = "AGENT_FILE_END", channel_id = %channel_id, "agent file session ended");
}

async fn build_connector(
    protocol: &str,
    cfg: &serde_json::Value,
) -> anyhow::Result<Box<dyn FileConnector>> {
    match protocol {
        "sftp" | "ssh" => {
            let conn = rex_ssh::sftp::SftpConnector::connect_with_config(rex_ssh::SshConfig {
                host: cfg
                    .get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                port: cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16,
                username: cfg
                    .get("username")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                password: cfg
                    .get("password")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                private_key: cfg
                    .get("privateKey")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .or_else(|| {
                        cfg.get("private_key")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    }),
                keepalive_interval: cfg
                    .get("keepalive_interval")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32),
                init_script: None,
            })
            .await?;
            Ok(Box::new(conn))
        }
        "s3" => {
            let req = FileConnectRequest {
                protocol: "s3".to_string(),
                host: cfg
                    .get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                port: cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(443) as u16,
                username: None,
                password: None,
                private_key: None,
                keepalive_interval: None,
                bucket: cfg.get("bucket").and_then(|v| v.as_str()).map(String::from),
                region: cfg.get("region").and_then(|v| v.as_str()).map(String::from),
                endpoint: cfg
                    .get("endpoint")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                access_key: cfg
                    .get("access_key")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                secret_key: cfg
                    .get("secret_key")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            };
            let conn = rex_s3::S3Connector::connect_from_request(&req).await?;
            Ok(Box::new(conn))
        }
        other => anyhow::bail!("unsupported file protocol: {other}"),
    }
}

async fn dispatch_file(
    conn: &mut Box<dyn FileConnector>,
    kind: &str,
    payload: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    match kind {
        "list" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("/");
            let entries = conn.list(path).await?;
            Ok(serde_json::json!({ "entries": entries }))
        }
        "stat" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let e = conn.stat(path).await?;
            Ok(serde_json::json!({ "entry": e }))
        }
        "mkdir" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            conn.mkdir(path).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "delete" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            conn.delete(path).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "rename" => {
            let from = payload.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let to = payload.get("to").and_then(|v| v.as_str()).unwrap_or("");
            conn.rename(from, to).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "download" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let offset = payload.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
            let limit = payload.get("limit").and_then(|v| v.as_u64());
            let data = if limit.is_some() || offset > 0 {
                conn.download_range(path, offset, limit).await?
            } else {
                conn.download(path).await?
            };
            // 文件分块走 session_response 的 data.b64；大文件由前端切片下发。
            Ok(serde_json::json!({ "data": base64_chunk(&data), "len": data.len() }))
        }
        "download_meta" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let e = conn.stat(path).await?;
            Ok(serde_json::json!({ "size": e.size }))
        }
        "read_for_edit" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let data = conn.read_for_edit(path).await?;
            Ok(serde_json::json!({ "data": base64_chunk(&data), "len": data.len() }))
        }
        "upload" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let offset = payload.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
            let b64 = payload.get("data").and_then(|v| v.as_str()).unwrap_or("");
            let data = base64_decode(b64)?;
            conn.upload(path, data, offset, None).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "save_from_edit" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let b64 = payload.get("data").and_then(|v| v.as_str()).unwrap_or("");
            let data = base64_decode(b64)?;
            conn.save_from_edit(path, data).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "close" => {
            let _ = conn.close().await;
            Ok(serde_json::json!({ "closed": true }))
        }
        other => anyhow::bail!("unsupported file request kind: {other}"),
    }
}

fn base64_chunk(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn base64_decode(s: &str) -> anyhow::Result<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| anyhow::anyhow!("base64 decode failed: {e}"))
}

async fn send_session_error(
    evt_tx: &mpsc::Sender<AgentEvent>,
    channel_id: &str,
    request_id: Option<&str>,
    error: &str,
) {
    let msg = rex_common::agent_proto::AgentSessionMsg::SessionError(
        rex_common::agent_proto::SessionError {
            channel_id: channel_id.to_string(),
            request_id: request_id.map(|s| s.to_string()),
            error: error.to_string(),
        },
    );
    let s = serde_json::to_string(&msg).unwrap_or_default();
    let _ = evt_tx.send(AgentEvent::Text(s)).await;
}
