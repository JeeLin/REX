//! SQL 控制台 REST + WebSocket 路由。
//!
//! 提供数据库连接、查询执行、元数据获取等 API。

use std::collections::HashMap;
use std::sync::Arc;

use crate::db::{audit_log, audit_log_with_detail};
use crate::models::SavedQuery;
use crate::resource_conn::load_resource_config;
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use rex_common::sql::{ConnectRequest, DatabaseType, SqlConnector};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// 全局 SQL 连接池状态
pub type SqlState = Arc<Mutex<SqlConnectionPool>>;

/// 连接池 — sessionId → 连接器
pub struct SqlConnectionPool {
    connectors: HashMap<String, Box<dyn SqlConnector>>,
}

impl Default for SqlConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlConnectionPool {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: String, conn: Box<dyn SqlConnector>) {
        self.connectors.insert(id, conn);
    }

    pub fn remove(&mut self, id: &str) -> Option<Box<dyn SqlConnector>> {
        self.connectors.remove(id)
    }
}

/// 创建 SQL API 路由
pub fn sql_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/connect", axum::routing::post(connect))
        .route("/disconnect", axum::routing::post(disconnect))
        .route("/query", axum::routing::post(query))
        .route("/databases", axum::routing::get(databases))
        .route("/tables", axum::routing::get(tables))
        .route("/columns", axum::routing::get(columns))
        .route("/indexes", axum::routing::get(indexes))
        .route("/foreign_keys", axum::routing::get(foreign_keys))
        .route("/ddl", axum::routing::get(ddl))
        .route("/saved-queries", axum::routing::get(list_saved_queries))
        .route("/saved-queries", axum::routing::post(upsert_saved_query))
        .route(
            "/saved-queries/{id}",
            axum::routing::delete(delete_saved_query),
        )
        .route("/compare", axum::routing::post(compare))
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ConnectBody {
    #[serde(rename = "type", default)]
    subtype: Option<String>,
    resource_id: String,
}

#[derive(Debug, Serialize)]
struct ConnectResponse {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct DisconnectBody {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct QueryBody {
    session_id: String,
    sql: String,
    #[serde(default)]
    #[allow(dead_code)]
    database: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionQuery {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct TablesQuery {
    session_id: String,
    db: String,
}

#[derive(Debug, Deserialize)]
struct ColumnsQuery {
    session_id: String,
    db: String,
    table: String,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

fn error_response(code: &str, message: &str) -> (StatusCode, Json<ErrorBody>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorBody {
            error: ErrorDetail {
                code: code.to_string(),
                message: message.to_string(),
            },
        }),
    )
}

#[derive(Debug, Deserialize)]
struct CompareBody {
    session_id: String,
    sql_left: String,
    sql_right: String,
    #[serde(default)]
    key_columns: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct CompareResult {
    left: rex_common::sql::QueryResult,
    right: rex_common::sql::QueryResult,
    diffs: Vec<DiffRow>,
    summary: CompareSummary,
}

#[derive(Debug, Serialize)]
struct DiffRow {
    row_index: usize,
    diff_type: String,
    column: String,
    left_value: serde_json::Value,
    right_value: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct CompareSummary {
    left_rows: usize,
    right_rows: usize,
    identical_rows: usize,
    modified_rows: usize,
    only_in_left: usize,
    only_in_right: usize,
}
// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/sql/connect
async fn connect(
    State(state): State<AppState>,
    Json(body): Json<ConnectBody>,
) -> axum::response::Response {
    // 从 DB 加载资源连接信息
    let res = match load_resource_config(&state, &body.resource_id) {
        Ok(r) => r,
        Err(e) => return error_response("INVALID_RESOURCE", &e).into_response(),
    };

    // v0.70.7：SQL 资源合并后，subtype 取自资源探测结果（res.subtype），
    // 缺省时进入探测分支（直连侧下方 detect_dialect，agent 侧由 Agent 私网内探测）。
    let db_type = body
        .subtype
        .clone()
        .or_else(|| res.subtype.clone())
        .unwrap_or_else(|| "auto".to_string());

    // v0.70.6 子任务 #7：agent 模式 —— 协议在 Agent 私网内终结，Hub 仅做隧道中转。
    if res.use_agent {
        let agent_id = match res.agent_id.clone() {
            Some(id) => id,
            None => {
                return error_response("AGENT_UNAVAILABLE", "no online agent for environment")
                    .into_response()
            }
        };
        let agent_db_type = if db_type == "auto" {
            "auto"
        } else {
            db_type.as_str()
        };
        let mut cfg = serde_json::json!({
            "host": res.host,
            "port": res.port.unwrap_or(0),
            "username": res.username,
            "subtype": agent_db_type,
        });
        if let serde_json::Value::Object(m) = res.config.clone() {
            for (k, v) in m {
                cfg[k] = v;
            }
        }
        tracing::debug!(action = "SQL_AGENT_CONFIG", resource_id = %body.resource_id, host = %res.host, port = %res.port.unwrap_or(0), subtype = %agent_db_type, has_password = res.config.get("password").and_then(|v| v.as_str()).is_some(), has_database = res.config.get("database").and_then(|v| v.as_str()).is_some(), "SQL agent config forwarded to agent");
        let channel_id = match crate::agent_ws::open_agent_session(
            &state,
            &agent_id,
            &body.resource_id,
            agent_db_type,
            cfg,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                return error_response("AGENT_CONNECT_FAILED", &e.to_string()).into_response()
            }
        };
        // 探测模式下 Agent 会回传 detected dialect（SessionOpened.subtype），在此持久化。
        let resolved_subtype: Option<String> = if db_type == "auto" {
            match crate::agent_ws::take_session_subtype(&state, &channel_id).await {
                Some(detected) => {
                    let _ = state.db.set_resource_subtype(&body.resource_id, &detected);
                    Some(detected)
                }
                None => None,
            }
        } else {
            None
        };
        let session_id = format!("sql_{}", &uuid::Uuid::new_v4().to_string()[..8]);
        state.sql_pool.lock().await.insert(
            session_id.clone(),
            Box::new(crate::agent_proxy::AgentSqlProxy::new(
                state.clone(),
                channel_id,
                resolved_subtype.or_else(|| {
                    if db_type == "auto" {
                        None
                    } else {
                        Some(db_type.clone())
                    }
                }),
            )),
        );
        tracing::info!(action = "SQL_CONNECT_AGENT", session_id = %session_id, resource_id = %body.resource_id, resource_name = %res.name, agent_id = %agent_id, db_type = %db_type, "SQL connected via agent");
        return (StatusCode::OK, Json(ConnectResponse { session_id })).into_response();
    }

    let req = match db_type.to_lowercase().as_str() {
        "mysql" | "postgresql" | "postgres" | "clickhouse" | "sqlserver" | "mssql" | "oracle"
        | "mariadb" => ConnectRequest {
            host: res.host,
            port: res.port.unwrap_or(0),
            username: res.username,
            password: res
                .config
                .get("password")
                .and_then(|v| v.as_str())
                .map(String::from),
            database: res
                .config
                .get("database_name")
                .and_then(|v| v.as_str())
                .map(String::from),
        },
        "sqlite" => {
            let file_path = res
                .config
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            ConnectRequest {
                host: file_path.to_string(),
                port: 0,
                username: String::new(),
                password: None,
                database: None,
            }
        }
        "auto" => ConnectRequest {
            host: res.host,
            port: res.port.unwrap_or(0),
            username: res.username,
            password: res
                .config
                .get("password")
                .and_then(|v| v.as_str())
                .map(String::from),
            database: res
                .config
                .get("database_name")
                .and_then(|v| v.as_str())
                .map(String::from),
        },
        _ => {
            return error_response(
                "INVALID_DB_TYPE",
                &format!("unsupported database type: {}", db_type),
            )
            .into_response();
        }
    };

    // v0.70.7：db_type 缺省（auto）或显式给出都走探测/连接；探测成功回写资源 db_type。
    let conn_result = match db_type.to_lowercase().as_str() {
        "mysql" | "mariadb" => rex_mysql::MySqlConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "postgresql" | "postgres" => rex_postgresql::PostgresConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "sqlite" => rex_sqlite::SqliteConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "clickhouse" => rex_clickhouse::ClickHouseConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "sqlserver" | "mssql" => rex_mssql::SqlServerConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "oracle" => rex_oracle::OracleConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "auto" => detect_dialect(req).await,
        _ => unreachable!(),
    };

    match conn_result {
        Ok(conn) => {
            // 探测模式下回写 dialect 到资源（后续连接读缓存，无额外往返）。
            if db_type == "auto" {
                if let Some(detected) =
                    rex_common::sql::DetectedDialect::from_connector(conn.as_ref())
                {
                    let _ = state
                        .db
                        .set_resource_subtype(&body.resource_id, detected.as_str());
                }
            }
            let session_id = format!("sql_{}", &uuid::Uuid::new_v4().to_string()[..8]);
            let final_db_type = conn.database_type();
            tracing::info!(
                action = "SQL_CONNECT",
                db_type = ?final_db_type,
                resource_id = %body.resource_id,
                resource_name = %res.name,
                session_id = %session_id,
                "SQL connection established"
            );
            state.sql_pool.lock().await.insert(session_id.clone(), conn);
            audit_log(&state.db, "SQL_CONNECT", "success", Some(res.name.clone()));
            (StatusCode::OK, Json(ConnectResponse { session_id })).into_response()
        }
        Err(e) => {
            tracing::warn!(
                action = "SQL_CONNECT",
                db_type = %db_type,
                resource_id = %body.resource_id,
                resource_name = %res.name,
                error = %e,
                "SQL connection failed"
            );
            audit_log_with_detail(
                &state.db,
                "SQL_CONNECT",
                "failure",
                Some(res.name),
                Some(e.to_string()),
            );
            error_response("CONNECTION_FAILED", &e.to_string()).into_response()
        }
    }
}

/// 通过共享方言探测函数连接，返回已连接的 [`SqlConnector`]。
async fn detect_dialect(req: ConnectRequest) -> anyhow::Result<Box<dyn SqlConnector>> {
    let result = rex_common::sql::detect_dialect(req, |dt, r| connect_by_dialect(dt, r)).await?;
    Ok(result.conn)
}

async fn connect_by_dialect(
    db_type: DatabaseType,
    req: ConnectRequest,
) -> anyhow::Result<Box<dyn SqlConnector>> {
    match db_type {
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            Ok(Box::new(rex_mysql::MySqlConnector::connect(req).await?))
        }
        DatabaseType::PostgreSQL => Ok(Box::new(
            rex_postgresql::PostgresConnector::connect(req).await?,
        )),
        DatabaseType::SQLite => Ok(Box::new(rex_sqlite::SqliteConnector::connect(req).await?)),
        DatabaseType::ClickHouse => Ok(Box::new(
            rex_clickhouse::ClickHouseConnector::connect(req).await?,
        )),
        DatabaseType::SqlServer => Ok(Box::new(rex_mssql::SqlServerConnector::connect(req).await?)),
        DatabaseType::Oracle => Ok(Box::new(rex_oracle::OracleConnector::connect(req).await?)),
    }
}

/// POST /api/sql/disconnect
async fn disconnect(
    State(state): State<AppState>,
    Json(body): Json<DisconnectBody>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let session_id = body.session_id.clone();
    if let Some(mut conn) = pool.remove(&body.session_id) {
        let _ = conn.close().await;
        tracing::info!(action = "SQL_DISCONNECT", session_id = %session_id, "SQL session disconnected");
        audit_log(&state.db, "SQL_DISCONNECT", "success", Some(session_id));
        (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
    } else {
        error_response("SESSION_NOT_FOUND", "session not found").into_response()
    }
}

/// POST /api/sql/query
async fn query(State(state): State<AppState>, Json(body): Json<QueryBody>) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    // Apply query timeout (30 seconds)
    let timeout = std::time::Duration::from_secs(30);
    let execute_future = conn.execute(&body.sql);
    let query_len = body.sql.len();

    let start = std::time::Instant::now();
    match tokio::time::timeout(timeout, execute_future).await {
        Ok(Ok(mut result)) => {
            let elapsed = start.elapsed().as_millis() as u64;
            tracing::info!(
                action = "SQL_QUERY",
                session_id = %body.session_id,
                query_length = query_len,
                row_count = result.rows.len(),
                duration_ms = elapsed,
                "SQL query executed"
            );
            audit_log(&state.db, "SQL_QUERY", "success", Some(body.session_id));
            // Apply row limit (10000 rows)
            if result.rows.len() > 10000 {
                result.rows.truncate(10000);
            }
            (StatusCode::OK, Json(result)).into_response()
        }
        Ok(Err(e)) => {
            tracing::warn!(
                action = "SQL_QUERY",
                session_id = %body.session_id,
                query_length = query_len,
                error = %e,
                "SQL query failed"
            );
            audit_log_with_detail(
                &state.db,
                "SQL_QUERY",
                "failure",
                Some(body.session_id),
                Some(e.to_string()),
            );
            error_response("QUERY_FAILED", &e.to_string()).into_response()
        }
        Err(_) => {
            tracing::warn!(
                action = "SQL_QUERY",
                session_id = %body.session_id,
                query_length = query_len,
                "SQL query timed out"
            );
            audit_log(&state.db, "SQL_QUERY", "timeout", Some(body.session_id));
            error_response("QUERY_TIMEOUT", "query timed out after 30 seconds").into_response()
        }
    }
}

/// GET /api/sql/databases?session_id=xxx
async fn databases(
    State(state): State<AppState>,
    Query(params): Query<SessionQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.databases().await {
        Ok(dbs) => (StatusCode::OK, Json(dbs)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

/// GET /api/sql/tables?session_id=xxx&db=xxx
async fn tables(
    State(state): State<AppState>,
    Query(params): Query<TablesQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.tables(&params.db).await {
        Ok(tables) => (StatusCode::OK, Json(tables)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

/// GET /api/sql/columns?session_id=xxx&db=xxx&table=xxx
async fn columns(
    State(state): State<AppState>,
    Query(params): Query<ColumnsQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.columns(&params.db, &params.table).await {
        Ok(cols) => (StatusCode::OK, Json(cols)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

/// GET /api/sql/indexes?session_id=xxx&db=xxx&table=xxx
async fn indexes(
    State(state): State<AppState>,
    Query(params): Query<ColumnsQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.indexes(&params.db, &params.table).await {
        Ok(idx) => (StatusCode::OK, Json(idx)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

/// GET /api/sql/foreign_keys?session_id=xxx&db=xxx&table=xxx
async fn foreign_keys(
    State(state): State<AppState>,
    Query(params): Query<ColumnsQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.foreign_keys(&params.db, &params.table).await {
        Ok(fks) => (StatusCode::OK, Json(fks)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

/// GET /api/sql/ddl?session_id=xxx&db=xxx&table=xxx
async fn ddl(
    State(state): State<AppState>,
    Query(params): Query<ColumnsQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.ddl(&params.db, &params.table).await {
        Ok(d) => (StatusCode::OK, Json(d)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Saved SQL Queries (命名查询，持久化于 settings 表)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SavedQueryBody {
    #[serde(default)]
    id: String,
    name: String,
    #[serde(default)]
    sql: String,
    #[serde(default)]
    db_type: Option<String>,
}

/// GET /api/sql/saved-queries
async fn list_saved_queries(State(state): State<AppState>) -> impl IntoResponse {
    let db = state.db.clone();
    match db.list_saved_queries() {
        Ok(list) => (StatusCode::OK, Json(list)).into_response(),
        Err(e) => error_response("DB_ERROR", &e.to_string()).into_response(),
    }
}

/// POST /api/sql/saved-queries
async fn upsert_saved_query(
    State(state): State<AppState>,
    Json(body): Json<SavedQueryBody>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() {
        return error_response("INVALID_NAME", "query name must not be empty").into_response();
    }
    let db = state.db.clone();
    let q = SavedQuery {
        id: body.id,
        name: body.name,
        sql: body.sql,
        db_type: body.db_type,
        updated_at: None,
    };
    match db.upsert_saved_query(&q) {
        Ok(stored) => (StatusCode::OK, Json(stored)).into_response(),
        Err(e) => error_response("DB_ERROR", &e.to_string()).into_response(),
    }
}

/// DELETE /api/sql/saved-queries/{id}
async fn delete_saved_query(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.db.clone();
    match db.delete_saved_query(&id) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response("DB_ERROR", &e.to_string()).into_response(),
    }
}

/// POST /api/sql/compare
/// Execute two SQL queries and compare results, highlighting differences.
async fn compare(
    State(state): State<AppState>,
    Json(body): Json<CompareBody>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    let timeout = std::time::Duration::from_secs(30);

    // Execute left query
    let left_result = match tokio::time::timeout(timeout, conn.execute(&body.sql_left)).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return error_response("QUERY_FAILED_LEFT", &e.to_string()).into_response(),
        Err(_) => return error_response("QUERY_TIMEOUT", "left query timed out").into_response(),
    };

    // Execute right query
    let right_result = match tokio::time::timeout(timeout, conn.execute(&body.sql_right)).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return error_response("QUERY_FAILED_RIGHT", &e.to_string()).into_response(),
        Err(_) => return error_response("QUERY_TIMEOUT", "right query timed out").into_response(),
    };

    // Compare results
    let diffs = compare_results(&left_result, &right_result, &body.key_columns);
    let summary = build_summary(&left_result, &right_result, &diffs);

    tracing::info!(
        action = "SQL_COMPARE",
        session_id = %body.session_id,
        left_rows = left_result.rows.len(),
        right_rows = right_result.rows.len(),
        diff_count = diffs.len(),
        "SQL compare executed"
    );

    (
        StatusCode::OK,
        Json(CompareResult {
            left: left_result,
            right: right_result,
            diffs,
            summary,
        }),
    )
        .into_response()
}

/// Compare two query results row by row.
fn compare_results(
    left: &rex_common::sql::QueryResult,
    right: &rex_common::sql::QueryResult,
    key_columns: &Option<Vec<String>>,
) -> Vec<DiffRow> {
    let mut diffs = Vec::new();

    // Build column index maps
    let left_cols: std::collections::HashMap<&str, usize> = left
        .columns
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.as_str(), i))
        .collect();
    let right_cols: std::collections::HashMap<&str, usize> = right
        .columns
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.as_str(), i))
        .collect();

    // Common columns
    let common_cols: Vec<&str> = left_cols
        .keys()
        .filter(|k| right_cols.contains_key(*k))
        .copied()
        .collect();

    // Determine key column indices
    let key_indices: Vec<(usize, usize)> = if let Some(keys) = key_columns {
        keys.iter()
            .filter_map(|k| {
                let li = left_cols.get(k.as_str())?;
                let ri = right_cols.get(k.as_str())?;
                Some((*li, *ri))
            })
            .collect()
    } else {
        Vec::new()
    };

    if key_indices.is_empty() {
        // Row-by-row comparison (by index)
        let max_rows = left.rows.len().max(right.rows.len());
        for i in 0..max_rows {
            let left_row = left.rows.get(i);
            let right_row = right.rows.get(i);

            match (left_row, right_row) {
                (Some(lr), Some(rr)) => {
                    for &col_name in &common_cols {
                        let li = left_cols[col_name];
                        let ri = right_cols[col_name];
                        if lr[li] != rr[ri] {
                            diffs.push(DiffRow {
                                row_index: i,
                                diff_type: "modified".to_string(),
                                column: col_name.to_string(),
                                left_value: lr[li].clone(),
                                right_value: rr[ri].clone(),
                            });
                        }
                    }
                }
                (Some(_), None) => {
                    diffs.push(DiffRow {
                        row_index: i,
                        diff_type: "only_in_left".to_string(),
                        column: "*".to_string(),
                        left_value: serde_json::Value::Bool(true),
                        right_value: serde_json::Value::Null,
                    });
                }
                (None, Some(_)) => {
                    diffs.push(DiffRow {
                        row_index: i,
                        diff_type: "only_in_right".to_string(),
                        column: "*".to_string(),
                        left_value: serde_json::Value::Null,
                        right_value: serde_json::Value::Bool(true),
                    });
                }
                (None, None) => {}
            }
        }
    } else {
        // Key-based comparison
        let mut right_matched: std::collections::BTreeSet<usize> =
            std::collections::BTreeSet::new();

        for (li, left_row) in left.rows.iter().enumerate() {
            let mut found = None;
            for (ri, right_row) in right.rows.iter().enumerate() {
                if right_matched.contains(&ri) {
                    continue;
                }
                let mut key_match = true;
                for &(lki, rki) in &key_indices {
                    if left_row[lki] != right_row[rki] {
                        key_match = false;
                        break;
                    }
                }
                if key_match {
                    found = Some(ri);
                    break;
                }
            }

            match found {
                Some(ri) => {
                    right_matched.insert(ri);
                    let right_row = &right.rows[ri];
                    for &col_name in &common_cols {
                        let li_idx = left_cols[col_name];
                        let ri_idx = right_cols[col_name];
                        if left_row[li_idx] != right_row[ri_idx] {
                            diffs.push(DiffRow {
                                row_index: li,
                                diff_type: "modified".to_string(),
                                column: col_name.to_string(),
                                left_value: left_row[li_idx].clone(),
                                right_value: right_row[ri_idx].clone(),
                            });
                        }
                    }
                }
                None => {
                    diffs.push(DiffRow {
                        row_index: li,
                        diff_type: "only_in_left".to_string(),
                        column: "*".to_string(),
                        left_value: serde_json::Value::Bool(true),
                        right_value: serde_json::Value::Null,
                    });
                }
            }
        }

        for ri in 0..right.rows.len() {
            if !right_matched.contains(&ri) {
                diffs.push(DiffRow {
                    row_index: ri,
                    diff_type: "only_in_right".to_string(),
                    column: "*".to_string(),
                    left_value: serde_json::Value::Null,
                    right_value: serde_json::Value::Bool(true),
                });
            }
        }
    }

    diffs
}

fn build_summary(
    left: &rex_common::sql::QueryResult,
    right: &rex_common::sql::QueryResult,
    diffs: &[DiffRow],
) -> CompareSummary {
    use std::collections::HashSet;

    let only_left = diffs
        .iter()
        .filter(|d| d.diff_type == "only_in_left")
        .count();
    let only_right = diffs
        .iter()
        .filter(|d| d.diff_type == "only_in_right")
        .count();
    // Count unique row indices that have at least one "modified" diff
    let modified_row_indices: HashSet<usize> = diffs
        .iter()
        .filter(|d| d.diff_type == "modified")
        .map(|d| d.row_index)
        .collect();
    let modified = modified_row_indices.len();
    let total_rows = left.rows.len().max(right.rows.len());
    let identical = total_rows.saturating_sub(only_left + only_right + modified);

    CompareSummary {
        left_rows: left.rows.len(),
        right_rows: right.rows.len(),
        identical_rows: identical,
        modified_rows: modified,
        only_in_left: only_left,
        only_in_right: only_right,
    }
}
