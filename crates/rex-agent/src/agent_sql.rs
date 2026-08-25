//! Agent 侧 SQL 协议执行层（v0.70.6 子任务 #4）。
//!
//! 此前 sql 资源在 agent 模式下由 Hub 直接 `rex_mysql::connect` 连目标（根本没走隧道）。
//! 本模块让 **Agent 在私网内用 sqlx 终结 SQL 协议**，把查询结果经 `session_response`
//! 结构化帧回传 Hub，Hub 仅做代理转发。直连模式不受影响（Hub 侧仍直连）。

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use rex_common::sql::{ConnectRequest, DatabaseType, QueryResult, SqlConnector};

use crate::agent_ws::{AgentEvent, LocalChannel};

/// 在 Agent 内建立 SQL 连接并接管隧道上的请求/响应。
pub async fn handle_connect_sql(
    request_id: String,
    channel_id: String,
    db_type: String,
    cfg: &serde_json::Value,
    evt_tx: mpsc::Sender<AgentEvent>,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
) {
    let req = ConnectRequest {
        host: cfg.get("host").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        port: cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
        username: cfg.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        password: cfg.get("password").and_then(|v| v.as_str()).map(String::from),
        database: cfg
            .get("database")
            .and_then(|v| v.as_str())
            .or_else(|| cfg.get("database_name").and_then(|v| v.as_str()))
            .map(String::from),
    };

    let db_type = match db_type.to_lowercase().as_str() {
        "mysql" | "sql" => DatabaseType::MySQL,
        "postgresql" | "postgres" => DatabaseType::PostgreSQL,
        "sqlite" => DatabaseType::SQLite,
        other => {
            send_session_error(&evt_tx, &channel_id, Some(&request_id), &format!("unsupported db_type: {other}")).await;
            return;
        }
    };

    let mut connector: Box<dyn SqlConnector> = match connect_by_type(db_type, &req).await {
        Ok(c) => c,
        Err(e) => {
            send_session_error(&evt_tx, &channel_id, Some(&request_id), &format!("SQL connection failed: {e}")).await;
            return;
        }
    };

    // 通知 Hub 连接成功（协议已在 Agent 终结）。
    let ok = serde_json::to_string(&rex_common::agent_proto::AgentSessionMsg::SessionOpened(
        rex_common::agent_proto::SessionOpened {
            request_id,
            channel_id: channel_id.clone(),
        },
    ))
    .unwrap_or_default();
    let _ = evt_tx.send(AgentEvent::Text(ok)).await;

    // 注册 channel（接收 Hub 经隧道下发的 session_request 帧字节）。
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

    // 隧道帧（Hub 经 `[4B channelId][json]` 下发）即 session_request JSON；
    // 主循环持有 connector，逐帧处理并按 seq 回传 session_response。
    while let Some(frame) = data_rx.recv().await {
        if frame.is_empty() {
            break; // 关闭信号
        }
        let msg: rex_common::agent_proto::SessionRequest = match serde_json::from_slice(&frame) {
            Ok(m) => m,
            Err(e) => {
                send_session_error(&evt_tx, &channel_id, None, &format!("invalid session_request: {e}")).await;
                continue;
            }
        };
        let resp = match dispatch_sql(&mut connector, &msg.kind, &msg.payload).await {
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
        let s = serde_json::to_string(&rex_common::agent_proto::AgentSessionMsg::SessionResponse(resp))
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
    tracing::info!(action = "AGENT_SQL_END", channel_id = %channel_id, "agent SQL session ended");
}

async fn connect_by_type(
    db_type: DatabaseType,
    req: &ConnectRequest,
) -> anyhow::Result<Box<dyn SqlConnector>> {
    match db_type {
        DatabaseType::MySQL => Ok(Box::new(rex_mysql::MySqlConnector::connect(req.clone()).await?)),
        DatabaseType::PostgreSQL => {
            Ok(Box::new(rex_postgresql::PostgresConnector::connect(req.clone()).await?))
        }
        DatabaseType::SQLite => Ok(Box::new(rex_sqlite::SqliteConnector::connect(req.clone()).await?)),
    }
}

async fn dispatch_sql(
    conn: &mut Box<dyn SqlConnector>,
    kind: &str,
    payload: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    match kind {
        "query" | "exec" => {
            let sql = payload.get("sql").and_then(|v| v.as_str()).unwrap_or("");
            let res: QueryResult = conn.execute(sql).await?;
            Ok(serde_json::json!({
                "columns": res.columns,
                "rows": res.rows,
                "affected_rows": res.affected_rows,
                "elapsed_ms": res.elapsed_ms,
            }))
        }
        "databases" => {
            let dbs = conn.databases().await?;
            Ok(serde_json::json!({ "databases": dbs }))
        }
        "tables" => {
            let db = payload.get("db").and_then(|v| v.as_str()).unwrap_or("");
            let t = conn.tables(db).await?;
            Ok(serde_json::json!({ "tables": t }))
        }
        "columns" => {
            let db = payload.get("db").and_then(|v| v.as_str()).unwrap_or("");
            let table = payload.get("table").and_then(|v| v.as_str()).unwrap_or("");
            let c = conn.columns(db, table).await?;
            Ok(serde_json::json!({ "columns": c }))
        }
        "close" => {
            let _ = conn.close().await;
            Ok(serde_json::json!({ "closed": true }))
        }
        other => anyhow::bail!("unsupported sql request kind: {other}"),
    }
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
