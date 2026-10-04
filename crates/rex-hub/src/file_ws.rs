//! WebSocket 传输进度广播 — 浏览器 ↔ Hub 广播通道。
//!
//! 统一入口：`/ws/files?token=jwt`
//! 前端订阅特定任务的进度变更：发送 `{"type":"subscribe","task_id":"..."}`，
//! 后端转发 `TransferProgressEvent` JSON。取消订阅：`{"type":"unsubscribe",...}`
//! 或直接关闭连接。JWT 由路由上的 `AuthUser` 中间件校验。

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::{broadcast, Mutex};

use crate::app::AppState;

/// WS 查询参数（JWT 通过 query `token` 传入，由中间件校验）。
#[derive(Debug, Deserialize)]
pub struct FileWsQuery {
    /// 可选：一次性订阅的任务 id，方便前端省去握手后发送 subscribe。
    pub task_id: Option<String>,
}

/// 前端 → 后端的控制消息。
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMsg {
    #[serde(rename = "subscribe")]
    Subscribe { task_id: String },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { task_id: String },
}

/// GET `/ws/files?token=jwt`
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<FileWsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, query.task_id))
}

async fn handle_socket(ws: WebSocket, state: AppState, initial_task: Option<String>) {
    let mut rx = state.transfer_bcast.subscribe();
    let subscribed: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    if let Some(tid) = &initial_task {
        subscribed.lock().await.insert(tid.clone());
    }

    let (mut ws_sink, mut ws_stream) = ws.split();

    // 广播接收任务 → WS 发送
    let bcast_to_ws = async {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let subs = subscribed.lock().await;
                    let forward = subs.is_empty() || subs.contains(&event.task_id);
                    drop(subs);
                    if forward {
                        let msg = json!({
                            "type": "progress",
                            "payload": event,
                        });
                        if ws_sink
                            .send(Message::Text(msg.to_string().into()))
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    };

    // 客户端消息处理任务 → 更新订阅集合
    let client_to_ctrl = async {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) {
                        let mut subs = subscribed.lock().await;
                        match client_msg {
                            ClientMsg::Subscribe { task_id } => {
                                subs.insert(task_id);
                            }
                            ClientMsg::Unsubscribe { task_id } => {
                                subs.remove(&task_id);
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    };

    tokio::select! {
        _ = bcast_to_ws => {},
        _ = client_to_ctrl => {},
    }
    tracing::debug!("file_ws connection ended");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::TransferProgressEvent;

    #[test]
    fn client_msg_subscribe_parses() {
        let msg: ClientMsg =
            serde_json::from_str(r#"{"type":"subscribe","task_id":"t1"}"#).unwrap();
        match msg {
            ClientMsg::Subscribe { task_id } => assert_eq!(task_id, "t1"),
            _ => panic!("expected subscribe"),
        }
    }

    #[test]
    fn client_msg_unsubscribe_parses() {
        let msg: ClientMsg =
            serde_json::from_str(r#"{"type":"unsubscribe","task_id":"t2"}"#).unwrap();
        match msg {
            ClientMsg::Unsubscribe { task_id } => assert_eq!(task_id, "t2"),
            _ => panic!("expected unsubscribe"),
        }
    }

    #[test]
    fn transfer_progress_event_serializes() {
        let ev = TransferProgressEvent {
            task_id: "t1".into(),
            transferred_bytes: 100,
            total_bytes: 1000,
            speed_bytes_per_sec: 50,
            status: "running".into(),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("\"task_id\":\"t1\""));
        assert!(json.contains("\"status\":\"running\""));
    }
}
