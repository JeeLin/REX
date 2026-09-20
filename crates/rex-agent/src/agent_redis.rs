//! Agent 侧 Redis 协议执行层（v0.70.6 子任务 #5）。
//!
//! 此前 redis 资源在 agent 模式下由 Hub 直接 `RedisConnectorImpl::connect` 连目标，
//! 根本没走隧道。本模块让 **Agent 在私网内用 redis crate 终结协议**，把结果经
//! `session_response` 结构化帧回传 Hub，Hub 仅做代理转发。直连模式不受影响。

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use rex_common::redis::dispatch_redis;
use rex_common::redis::RedisConnectRequest;
use rex_common::redis::RedisConnector;

use rex_common::agent_proto::send_session_error;
use rex_common::agent_proto::AgentEvent;

use crate::agent_ws::LocalChannel;

/// Agent 内建立 Redis 连接并接管隧道上的请求/响应。
pub async fn handle_connect_redis(
    request_id: String,
    channel_id: String,
    cfg: &serde_json::Value,
    evt_tx: mpsc::Sender<AgentEvent>,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
) {
    tracing::info!(action = "AGENT_REDIS_CONNECT", request_id = %request_id, host = %cfg.get("host").and_then(|v| v.as_str()).unwrap_or(""), port = %cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(6379), has_password = cfg.get("password").and_then(|v| v.as_str()).is_some(), db = %cfg.get("db").and_then(|v| v.as_u64()).unwrap_or(0), "Redis connection initiated");
    let req = RedisConnectRequest {
        host: cfg
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        port: cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(6379) as u16,
        password: cfg
            .get("password")
            .and_then(|v| v.as_str())
            .map(String::from),
        db: cfg.get("db").and_then(|v| v.as_i64()).map(|v| v as i32),
    };

    let mut connector = match rex_redis::RedisConnectorImpl::connect(req).await {
        Ok(c) => c,
        Err(e) => {
            send_session_error(
                &evt_tx,
                &channel_id,
                Some(&request_id),
                &format!("Redis connection failed: {e}"),
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
        let resp = match dispatch_redis(
            &mut connector as &mut dyn RedisConnector,
            &msg.kind,
            &msg.payload,
        )
        .await
        {
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
    tracing::info!(action = "AGENT_REDIS_END", channel_id = %channel_id, "agent Redis session ended");
}
