//! SQL 控制台 REST + WebSocket 路由。
//!
//! 提供数据库连接、查询执行、元数据获取等 API。

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use crate::AppState;
use rex_common::sql::{
    ConnectRequest, DatabaseType, SqlConnector, SqlConnectorFactory,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// 全局 SQL 连接池状态
pub type SqlState = Arc<Mutex<SqlConnectionPool>>;

/// 连接池 — sessionId → 连接器
pub struct SqlConnectionPool {
    connectors: HashMap<String, Box<dyn SqlConnector>>,
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
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ConnectBody {
    #[serde(rename = "type")]
    db_type: String,
    #[serde(flatten)]
    req: ConnectRequest,
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
    let db_type = match body.db_type.to_lowercase().as_str() {
        "mysql" => DatabaseType::MySQL,
        "postgresql" | "postgres" => DatabaseType::PostgreSQL,
        "sqlite" => DatabaseType::SQLite,
        _ => {
            return error_response(
                "INVALID_DB_TYPE",
                &format!("unsupported database type: {}", body.db_type),
            )
            .into_response();
        }
    };

    let factory = SqlConnectorFactory::new(db_type);
    match factory.connect(body.req).await {
        Ok(conn) => {
            let session_id = format!("sql_{}", &uuid::Uuid::new_v4().to_string()[..8]);
            state.sql_pool.lock().await.insert(session_id.clone(), conn);
            (StatusCode::OK, Json(ConnectResponse { session_id })).into_response()
        }
        Err(e) => error_response("CONNECTION_FAILED", &e.to_string()).into_response(),
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
        (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
    } else {
        error_response("SESSION_NOT_FOUND", "session not found").into_response()
    }
}

/// POST /api/sql/query
async fn query(
    State(state): State<AppState>,
    Json(body): Json<QueryBody>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.execute(&body.sql).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
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
