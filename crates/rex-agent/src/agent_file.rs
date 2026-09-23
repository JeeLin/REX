//! Agent 侧文件传输执行层（v0.70.6 子任务 #6）。
//!
//! 此前 file 资源在 agent 模式下由 Hub 直接 `SftpConnector`/`S3Connector` 连目标，
//! 根本没走隧道。本模块让 **Agent 在私网内终结 SFTP / S3 协议**（数据不落浏览器，
//! 满足 AGENTS.md 硬性约束），把列目录/分块读写结果经隧道帧回传 Hub。

use std::collections::HashMap;
use std::sync::Arc;

use crate::agent_ws::SshHandlePool;
use tokio::sync::{mpsc, RwLock};

use rex_common::file_transfer::{dispatch_file, FileConnectRequest, FileConnector};

use rex_common::agent_proto::send_session_error;
use rex_common::agent_proto::AgentEvent;

use crate::agent_ws::LocalChannel;

/// Agent 内建立文件连接并接管隧道上的请求/响应。
pub async fn handle_connect_file(
    request_id: String,
    channel_id: String,
    protocol: String,
    cfg: &serde_json::Value,
    evt_tx: mpsc::Sender<AgentEvent>,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
    ssh_handles: SshHandlePool,
) {
    tracing::info!(
        action = "AGENT_FILE_CONNECT",
        request_id = %request_id,
        protocol = %protocol,
        host = %cfg.get("host").and_then(|v| v.as_str()).unwrap_or(""),
        port = %cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(0),
        has_password = cfg.get("password").and_then(|v| v.as_str()).is_some(),
        bucket = %cfg.get("bucket").and_then(|v| v.as_str()).unwrap_or(""),
        "file connection initiated"
    );

    let mut connector: Box<dyn FileConnector> =
        match build_connector(&protocol, cfg, ssh_handles).await {
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

    // 注册 channel（必须在 SessionOpened 之前，否则 Hub 立即下发查询帧导致丢帧）。
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

    let ok = serde_json::to_string(&rex_common::agent_proto::AgentSessionMsg::SessionOpened(
        rex_common::agent_proto::SessionOpened {
            request_id,
            channel_id: channel_id.clone(),
            subtype: None,
        },
    ))
    .unwrap_or_default();
    let _ = evt_tx.send(AgentEvent::Text(ok)).await;

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
        let resp = match dispatch_file(&mut *connector, &msg.kind, &msg.payload).await {
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
    ssh_handles: SshHandlePool,
) -> anyhow::Result<Box<dyn FileConnector>> {
    match protocol {
        "sftp" | "ssh" => {
            let host = cfg
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let port = cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;
            let pool_key = format!("{}:{}", host, port);

            // 检查连接池：优先复用已有的 SSH Handle
            let conn = {
                let handles = ssh_handles.read().await;
                if let Some(handle_arc) = handles.get(&pool_key) {
                    tracing::info!(
                        action = "SFTP_CONNECT",
                        host = %host,
                        port = port,
                        pool_key = %pool_key,
                        "SFTP: reusing existing SSH handle from pool"
                    );
                    let handle = handle_arc.lock().await;
                    rex_ssh::sftp::SftpConnector::connect_from_handle(&handle, &host).await?
                } else {
                    // 池中无可用 Handle，创建新连接
                    drop(handles);
                    tracing::info!(
                        action = "SFTP_CONNECT",
                        host = %host,
                        port = port,
                        "SFTP: no existing handle in pool, creating new SSH connection"
                    );
                    rex_ssh::sftp::SftpConnector::connect_with_config(rex_ssh::SshConfig {
                        host: host.clone(),
                        port,
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
                    .await?
                }
            };
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
