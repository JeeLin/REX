use axum::extract::ws::{Message, WebSocket};
use futures_util::sink::SinkExt;
use futures_util::stream::SplitSink;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::routes::AppState;

// ── 公共消息类型 ────────────────────────────────────────────

/// WebSocket 客户端消息（通用）
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum WsClientMsg {
    /// 执行操作
    #[serde(rename = "command")]
    Command {
        id: String,
        action: String,
        #[serde(default)]
        params: serde_json::Value,
    },
    /// 心跳
    #[serde(rename = "ping")]
    Ping,
}

/// WebSocket 服务端消息（通用）
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum WsServerMsg {
    /// 操作结果
    #[serde(rename = "response")]
    Response { id: String, data: serde_json::Value },
    /// 操作错误
    #[serde(rename = "error")]
    Error { id: String, message: String },
    /// 心跳响应
    #[serde(rename = "pong")]
    Pong,
    /// 已连接
    #[serde(rename = "connected")]
    Connected { server: HashMap<String, String> },
    /// 连接断开
    #[serde(rename = "disconnected")]
    Disconnected { reason: String },
}

/// WebSocket 查询参数
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

// ── 工具函数 ──────────────────────────────────────────────

/// 发送 WebSocket 消息
pub async fn send_ws_msg(
    write: &mut SplitSink<WebSocket, Message>,
    msg: &WsServerMsg,
) -> Result<(), ()> {
    let json = serde_json::to_string(msg).map_err(|_| ())?;
    write.send(Message::Text(json)).await.map_err(|_| ())
}

/// 发送 WebSocket 错误消息
pub async fn send_ws_error(write: &mut SplitSink<WebSocket, Message>, msg: &str) -> Result<(), ()> {
    let err = WsServerMsg::Error {
        id: String::new(),
        message: msg.to_string(),
    };
    send_ws_msg(write, &err).await
}

/// 从数据库读取资源配置 JSON
pub async fn read_resource_config(state: &Arc<AppState>, resource_id: &str) -> Result<String, ()> {
    let db = state.db.clone();
    let rid = resource_id.to_string();
    match tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| "pool error")?;
        conn.query_row(
            "SELECT config_json FROM resources WHERE id = ?1",
            rusqlite::params![rid],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| "resource not found")
    })
    .await
    {
        Ok(Ok(json)) => Ok(json),
        _ => Err(()),
    }
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_client_msg_command_deserialize() {
        let json =
            r#"{"type":"command","id":"cmd-1","action":"execute","params":{"sql":"SELECT 1"}}"#;
        let msg: WsClientMsg = serde_json::from_str(json).unwrap();
        match msg {
            WsClientMsg::Command { id, action, params } => {
                assert_eq!(id, "cmd-1");
                assert_eq!(action, "execute");
                assert_eq!(params["sql"], "SELECT 1");
            }
            _ => panic!("expected Command variant"),
        }
    }

    #[test]
    fn ws_client_msg_ping_deserialize() {
        let json = r#"{"type":"ping"}"#;
        let msg: WsClientMsg = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, WsClientMsg::Ping));
    }

    #[test]
    fn ws_server_msg_response_serialize() {
        let msg = WsServerMsg::Response {
            id: "cmd-1".into(),
            data: serde_json::json!({ "columns": [], "rows": [] }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("response"));
        assert!(json.contains("cmd-1"));
    }

    #[test]
    fn ws_server_msg_error_serialize() {
        let msg = WsServerMsg::Error {
            id: "cmd-2".into(),
            message: "connection refused".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "error");
        assert_eq!(parsed["message"].as_str().unwrap(), "connection refused");
    }

    #[test]
    fn ws_server_msg_connected_serialize() {
        let mut server = HashMap::new();
        server.insert("host".into(), "127.0.0.1".into());
        let msg = WsServerMsg::Connected { server };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("connected"));
        assert!(json.contains("host"));
    }

    #[test]
    fn ws_server_msg_pong_serialize() {
        let msg = WsServerMsg::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"pong"}"#);
    }
}
