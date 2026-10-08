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
use rex_common::resource_config::config_private_key;

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

/// 探测（S7-F7）：Agent 侧 S3 可达性+凭据验证，不碰数据面。
///
/// 不复用 `handle_connect_file` 的真实会话 —— `build_connector` 会在
/// `connect_from_request` 里构造 client（不发请求），但后者本身无法判定
/// endpoint 可达或 key 有效。这里只建 client 即可调用 `verify()`
/// （`list_buckets`/`head_bucket`，只读不写），认证通过回 `SessionOpened`，
/// 失败回 `SessionError`，随即 drop client。全程不注册 channel，不写共享
/// Handle 池，与 `probe_ssh` 同等是「握手+认证后拆除」的轻探针。
pub async fn probe_s3(
    request_id: String,
    channel_id: String,
    cfg: &serde_json::Value,
    evt_tx: mpsc::Sender<AgentEvent>,
) {
    const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

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

    let endpoint = cfg
        .get("endpoint")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let port = cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(443) as u16;

    let res = tokio::time::timeout(PROBE_TIMEOUT, async {
        let conn = rex_s3::S3Connector::connect_from_request(&req).await?;
        conn.verify().await
    })
    .await;

    match res {
        Ok(Ok(())) => {
            let ok =
                serde_json::to_string(&rex_common::agent_proto::AgentSessionMsg::SessionOpened(
                    rex_common::agent_proto::SessionOpened {
                        request_id: request_id.clone(),
                        channel_id: channel_id.clone(),
                        subtype: None,
                    },
                ))
                .unwrap_or_default();
            let _ = evt_tx.send(AgentEvent::Text(ok)).await;
            tracing::info!(
                action = "AGENT_S3_PROBE_OK",
                request_id = %request_id,
                endpoint = %endpoint,
                port = port,
                "S3 probe verified (credentials OK)"
            );
        }
        Ok(Err(e)) => {
            send_session_error(
                &evt_tx,
                &channel_id,
                Some(&request_id),
                &format!("S3 verify failed: {e}"),
            )
            .await;
            tracing::warn!(
                action = "AGENT_S3_PROBE_FAILED",
                request_id = %request_id,
                endpoint = %endpoint,
                port = port,
                error = %e,
                "S3 probe failed"
            );
        }
        Err(_) => {
            send_session_error(&evt_tx, &channel_id, Some(&request_id), PROBE_TIMEOUT_FMT).await;
            tracing::warn!(
                action = "AGENT_S3_PROBE_FAILED",
                request_id = %request_id,
                endpoint = %endpoint,
                port = port,
                "S3 probe timed out"
            );
        }
    }
}

/// Agent 侧探测超时的用户可见文案（与 Hub `PROBE_TIMEOUT_MSG` 同值）。
const PROBE_TIMEOUT_FMT: &str = "connection timed out";

/// 与 `handle_connect_file` 同源：从 connect config 构造 FileConnector。
async fn build_connector(
    protocol: &str,
    cfg: &serde_json::Value,
    ssh_handles: SshHandlePool,
) -> anyhow::Result<Box<dyn FileConnector>> {
    match protocol {
        "sftp" | "ssh" => {
            // 池键与 `agent_ssh` 同源（含 username）：终端已认证的 Handle 只在
            // 同一用户下被 SFTP 复用，不同用户不共用会话（CR10）。
            let pool_key = crate::agent_ws::ssh_pool_key_from_cfg(cfg);
            let host = cfg
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let port = cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;

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
                        private_key: config_private_key(cfg),
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
