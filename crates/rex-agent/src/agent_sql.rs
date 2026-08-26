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
    subtype: String,
    cfg: &serde_json::Value,
    evt_tx: mpsc::Sender<AgentEvent>,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
) {
    let req = ConnectRequest {
        host: cfg
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        port: cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
        username: cfg
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        password: cfg
            .get("password")
            .and_then(|v| v.as_str())
            .map(String::from),
        database: cfg
            .get("database")
            .and_then(|v| v.as_str())
            .or_else(|| cfg.get("database_name").and_then(|v| v.as_str()))
            .map(String::from),
    };

    let db_type = match subtype.to_lowercase().as_str() {
        "mysql" | "sql" => Some(DatabaseType::MySQL),
        "postgresql" | "postgres" => Some(DatabaseType::PostgreSQL),
        "sqlite" => Some(DatabaseType::SQLite),
        "auto" | "" => None,
        other => {
            send_session_error(
                &evt_tx,
                &channel_id,
                Some(&request_id),
                &format!("unsupported subtype: {other}"),
            )
            .await;
            return;
        }
    };

    // v0.70.7：db_type 缺省（auto）时，由 Agent 私网内探测 dialect 后回传 Hub 持久化。
    let (mut connector, detected): (Box<dyn SqlConnector>, Option<String>) = match db_type {
        Some(dt) => match connect_by_type(dt, &req).await {
            Ok(c) => (c, None),
            Err(e) => {
                send_session_error(
                    &evt_tx,
                    &channel_id,
                    Some(&request_id),
                    &format!("SQL connection failed: {e}"),
                )
                .await;
                return;
            }
        },
        None => match detect_dialect(&req).await {
            Ok((c, detected)) => (c, detected),
            Err(e) => {
                send_session_error(&evt_tx, &channel_id, Some(&request_id), &e.to_string()).await;
                return;
            }
        },
    };

    // 通知 Hub 连接成功（协议已在 Agent 终结）。探测模式下回传 detected dialect。
    let ok = serde_json::to_string(&rex_common::agent_proto::AgentSessionMsg::SessionOpened(
        rex_common::agent_proto::SessionOpened {
            request_id,
            channel_id: channel_id.clone(),
            subtype: detected,
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
    tracing::info!(action = "AGENT_SQL_END", channel_id = %channel_id, "agent SQL session ended");
}

async fn connect_by_type(
    db_type: DatabaseType,
    req: &ConnectRequest,
) -> anyhow::Result<Box<dyn SqlConnector>> {
    match db_type {
        DatabaseType::MySQL => Ok(Box::new(
            rex_mysql::MySqlConnector::connect(req.clone()).await?,
        )),
        DatabaseType::PostgreSQL => Ok(Box::new(
            rex_postgresql::PostgresConnector::connect(req.clone()).await?,
        )),
        DatabaseType::SQLite => Ok(Box::new(
            rex_sqlite::SqliteConnector::connect(req.clone()).await?,
        )),
    }
}

/// v0.70.7 dialect 探测：db_type 缺省时，按端口预判 → 双线缆协议握手回退 →
/// `SELECT VERSION()` 确认，最终解析出 dialect 并连上对应连接器。
///
/// 返回探测出的连接器，以及探测 dialect 的 db_type 字符串（mysql/postgresql/sqlite），
/// 供 `handle_connect_sql` 经 `SessionOpened.db_type` 回传 Hub 持久化。
/// 与 Hub 直连侧 `detect_dialect` 共用同一套规则（见 rex-hub sql_api.rs）。
async fn detect_dialect(
    req: &ConnectRequest,
) -> anyhow::Result<(Box<dyn SqlConnector>, Option<String>)> {
    use rex_common::sql::DatabaseType;

    // SQLite：无 host 或 port 为 0 视为本地文件库。
    if req.host.is_empty() || req.port == 0 {
        let conn = Box::new(rex_sqlite::SqliteConnector::connect(req.clone()).await?);
        return Ok((conn, Some("sqlite".to_string())));
    }

    // 端口预判。
    let candidates: &[DatabaseType] = match req.port {
        3306 => &[DatabaseType::MySQL, DatabaseType::PostgreSQL],
        5432 => &[DatabaseType::PostgreSQL, DatabaseType::MySQL],
        _ => &[DatabaseType::MySQL, DatabaseType::PostgreSQL],
    };

    for &dt in candidates {
        match connect_by_type(dt, req).await {
            Ok(mut conn) => {
                // `SELECT VERSION()` 确认 dialect（消除线缆协议握手歧义）。
                match conn.execute("SELECT VERSION()").await {
                    Ok(result) => {
                        let version = result
                            .rows
                            .first()
                            .and_then(|r| r.first())
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        let confirmed = if version.to_uppercase().contains("POSTGRESQL") {
                            DatabaseType::PostgreSQL
                        } else {
                            dt
                        };
                        tracing::info!(
                            action = "AGENT_SQL_DETECT",
                            port = req.port,
                            version = %version,
                            dialect = ?confirmed,
                            "dialect detected"
                        );
                        // 已连上 confirmed 类型的连接器；若确认结果与握手类型不同，
                        // 重新以确认类型连接（端口预判 + 握手可能匹配到错误协议）。
                        let final_conn = if confirmed == dt {
                            conn
                        } else {
                            connect_by_type(confirmed, req).await?
                        };
                        return Ok((final_conn, Some(detected_to_str(confirmed))));
                    }
                    Err(_) => continue,
                }
            }
            Err(_) => continue,
        }
    }

    anyhow::bail!("无法识别 dialect，请在创建资源时指定 subtype")
}

/// v0.70.7：将探测确认的 [`DatabaseType`] 转成持久化用的 db_type 字符串。
fn detected_to_str(dt: DatabaseType) -> String {
    match dt {
        DatabaseType::MySQL => "mysql".to_string(),
        DatabaseType::PostgreSQL => "postgresql".to_string(),
        DatabaseType::SQLite => "sqlite".to_string(),
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
