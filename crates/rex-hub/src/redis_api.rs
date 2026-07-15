//! Redis 控制台 REST 路由。

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use crate::AppState;
use rex_common::redis::{RedisConnectRequest, RedisConnector};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// 全局 Redis 连接池状态
pub type RedisState = Arc<Mutex<RedisConnectionPool>>;

pub struct RedisConnectionPool {
    connectors: HashMap<String, Box<dyn RedisConnector>>,
}

impl RedisConnectionPool {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: String, conn: Box<dyn RedisConnector>) {
        self.connectors.insert(id, conn);
    }

    pub fn remove(&mut self, id: &str) -> Option<Box<dyn RedisConnector>> {
        self.connectors.remove(id)
    }
}

/// 创建 Redis API 路由
pub fn redis_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/connect", axum::routing::post(connect))
        .route("/disconnect", axum::routing::post(disconnect))
        .route("/databases", axum::routing::get(databases))
        .route("/select", axum::routing::post(select_db))
        .route("/scan", axum::routing::get(scan))
        .route("/key", axum::routing::get(get_key))
        .route("/set", axum::routing::post(set_key))
        .route("/del", axum::routing::post(del_keys))
        .route("/ttl", axum::routing::get(get_ttl))
        .route("/set-ttl", axum::routing::post(set_ttl))
        .route("/info", axum::routing::get(info))
        .route("/command", axum::routing::post(run_command))
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ConnectBody {
    host: String,
    port: u16,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    db: Option<i32>,
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
struct SessionQuery {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct ScanQuery {
    session_id: String,
    #[serde(default = "default_pattern")]
    pattern: String,
    #[serde(default = "default_count")]
    count: u32,
}

fn default_pattern() -> String {
    "*".to_string()
}

fn default_count() -> u32 {
    100
}

#[derive(Debug, Deserialize)]
struct KeyQuery {
    session_id: String,
    key: String,
}

#[derive(Debug, Deserialize)]
struct SetBody {
    session_id: String,
    key: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct DelBody {
    session_id: String,
    keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TtlQuery {
    session_id: String,
    key: String,
}

#[derive(Debug, Deserialize)]
struct SetTtlBody {
    session_id: String,
    key: String,
    seconds: i64,
}

#[derive(Debug, Deserialize)]
struct SelectBody {
    session_id: String,
    db: i32,
}

#[derive(Debug, Deserialize)]
struct CommandBody {
    session_id: String,
    args: Vec<String>,
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

async fn connect(
    State(state): State<AppState>,
    Json(body): Json<ConnectBody>,
) -> axum::response::Response {
    let req = RedisConnectRequest {
        host: body.host,
        port: body.port,
        password: body.password,
        db: body.db,
    };

    match rex_redis::RedisConnectorImpl::connect(req).await {
        Ok(conn) => {
            let session_id = format!("redis_{}", &uuid::Uuid::new_v4().to_string()[..8]);
            state
                .redis_pool
                .lock()
                .await
                .insert(session_id.clone(), Box::new(conn));
            (StatusCode::OK, Json(ConnectResponse { session_id })).into_response()
        }
        Err(e) => error_response("CONNECTION_FAILED", &e.to_string()).into_response(),
    }
}

async fn disconnect(
    State(state): State<AppState>,
    Json(body): Json<DisconnectBody>,
) -> axum::response::Response {
    let mut pool = state.redis_pool.lock().await;
    if let Some(mut conn) = pool.remove(&body.session_id) {
        let _ = conn.close().await;
        (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
    } else {
        error_response("SESSION_NOT_FOUND", "session not found").into_response()
    }
}

async fn databases(
    State(state): State<AppState>,
    Query(params): Query<SessionQuery>,
) -> axum::response::Response {
    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.dbs().await {
        Ok(dbs) => (StatusCode::OK, Json(dbs)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

async fn select_db(
    State(state): State<AppState>,
    Json(body): Json<SelectBody>,
) -> axum::response::Response {
    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.select_db(body.db).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

async fn scan(
    State(state): State<AppState>,
    Query(params): Query<ScanQuery>,
) -> axum::response::Response {
    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.scan(&params.pattern, params.count).await {
        Ok(keys) => (StatusCode::OK, Json(keys)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

async fn get_key(
    State(state): State<AppState>,
    Query(params): Query<KeyQuery>,
) -> axum::response::Response {
    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.get_value(&params.key).await {
        Ok(val) => (StatusCode::OK, Json(val)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

async fn set_key(
    State(state): State<AppState>,
    Json(body): Json<SetBody>,
) -> axum::response::Response {
    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.set_value(&body.key, &body.value).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

async fn del_keys(
    State(state): State<AppState>,
    Json(body): Json<DelBody>,
) -> axum::response::Response {
    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.del(&body.keys).await {
        Ok(count) => (StatusCode::OK, Json(serde_json::json!({"deleted": count}))).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

async fn get_ttl(
    State(state): State<AppState>,
    Query(params): Query<TtlQuery>,
) -> axum::response::Response {
    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.ttl(&params.key).await {
        Ok(ttl) => (StatusCode::OK, Json(serde_json::json!({"ttl": ttl}))).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

async fn set_ttl(
    State(state): State<AppState>,
    Json(body): Json<SetTtlBody>,
) -> axum::response::Response {
    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.set_ttl(&body.key, body.seconds).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

async fn info(
    State(state): State<AppState>,
    Query(params): Query<SessionQuery>,
) -> axum::response::Response {
    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.info().await {
        Ok(info) => (StatusCode::OK, Json(info)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

async fn run_command(
    State(state): State<AppState>,
    Json(body): Json<CommandBody>,
) -> axum::response::Response {
    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.command(&body.args).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!({"result": result}))).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}
