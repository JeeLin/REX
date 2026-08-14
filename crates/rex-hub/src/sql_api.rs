//! SQL 控制台 REST + WebSocket 路由。
//!
//! 提供数据库连接、查询执行、元数据获取等 API。

use std::collections::HashMap;
use std::sync::Arc;

use crate::models::SavedQuery;
use crate::resource_conn::load_resource_config;
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use rex_common::sql::{ConnectRequest, SqlConnector};
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
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ConnectBody {
    #[serde(rename = "type")]
    db_type: String,
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

    let db_type = body.db_type.clone();
    let req = match db_type.to_lowercase().as_str() {
        "mysql" | "postgresql" | "postgres" => ConnectRequest {
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
        _ => {
            return error_response(
                "INVALID_DB_TYPE",
                &format!("unsupported database type: {}", db_type),
            )
            .into_response();
        }
    };

    let conn_result = match db_type.to_lowercase().as_str() {
        "mysql" => rex_mysql::MySqlConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "postgresql" | "postgres" => rex_postgresql::PostgresConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "sqlite" => rex_sqlite::SqliteConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        _ => unreachable!(),
    };

    match conn_result {
        Ok(conn) => {
            let session_id = format!("sql_{}", &uuid::Uuid::new_v4().to_string()[..8]);
            tracing::info!(
                action = "SQL_CONNECT",
                db_type = %db_type,
                resource_id = %body.resource_id,
                resource_name = %res.name,
                session_id = %session_id,
                "SQL connection established"
            );
            state.sql_pool.lock().await.insert(session_id.clone(), conn);
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
            error_response("CONNECTION_FAILED", &e.to_string()).into_response()
        }
    }
}

/// POST /api/sql/disconnect
async fn disconnect(
    State(state): State<AppState>,
    Json(body): Json<DisconnectBody>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    if let Some(mut conn) = pool.remove(&body.session_id) {
        let _ = conn.close().await;
        tracing::info!(action = "SQL_DISCONNECT", session_id = %body.session_id, "SQL session disconnected");
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
            error_response("QUERY_FAILED", &e.to_string()).into_response()
        }
        Err(_) => {
            tracing::warn!(
                action = "SQL_QUERY",
                session_id = %body.session_id,
                query_length = query_len,
                "SQL query timed out"
            );
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

/// DELETE /api/sql/saved-queries/:id
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
