//! Redis 控制台 REST 路由。

use std::collections::HashMap;
use std::sync::Arc;

use crate::AppState;
use crate::resource_conn::load_resource_config;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use rex_common::redis::{RedisConnectRequest, RedisConnector};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// 全局 Redis 连接池状态
pub type RedisState = Arc<Mutex<RedisConnectionPool>>;

pub struct RedisConnectionPool {
    connectors: HashMap<String, Box<dyn RedisConnector>>,
}

impl Default for RedisConnectionPool {
    fn default() -> Self {
        Self::new()
    }
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
    // 从 DB 加载资源连接信息
    let res = match load_resource_config(&state, &body.resource_id) {
        Ok(r) => r,
        Err(e) => return error_response("INVALID_RESOURCE", &e).into_response(),
    };

    let req = RedisConnectRequest {
        host: res.host.clone(),
        port: res.port.unwrap_or(6379),
        password: res.config.get("password").and_then(|v| v.as_str()).map(String::from),
        db: res.config.get("db").and_then(|v| v.as_i64()).map(|v| v as i32),
    };

    tracing::info!(
        action = "REDIS_CONNECT",
        resource_id = %body.resource_id,
        host = %res.host,
        port = res.port.unwrap_or(6379),
        "Redis connecting"
    );

    match rex_redis::RedisConnectorImpl::connect(req).await {
        Ok(conn) => {
            let session_id = format!("redis_{}", &uuid::Uuid::new_v4().to_string()[..8]);
            state
                .redis_pool
                .lock()
                .await
                .insert(session_id.clone(), Box::new(conn));

            tracing::info!(
                action = "REDIS_CONNECT",
                session_id = %session_id,
                resource_id = %body.resource_id,
                "Redis connected"
            );

            // 审计日志写入
            let audit_db = state.db.clone();
            let target = res.host.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "REDIS_CONNECT".into(),
                    target: Some(target),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;

            (StatusCode::OK, Json(ConnectResponse { session_id })).into_response()
        }
        Err(e) => {
            tracing::warn!(
                action = "REDIS_CONNECT",
                resource_id = %body.resource_id,
                error = %e,
                "Redis connect failed"
            );
            error_response("CONNECTION_FAILED", &e.to_string()).into_response()
        }
    }
}
async fn disconnect(
    State(state): State<AppState>,
    Json(body): Json<DisconnectBody>,
) -> axum::response::Response {
    tracing::info!(
        action = "REDIS_DISCONNECT",
        session_id = %body.session_id,
        "Redis disconnecting"
    );

    let mut pool = state.redis_pool.lock().await;
    if let Some(mut conn) = pool.remove(&body.session_id) {
        let _ = conn.close().await;

        tracing::info!(
            action = "REDIS_DISCONNECT",
            session_id = %body.session_id,
            "Redis disconnected"
        );

        // 审计日志写入
        let audit_db = state.db.clone();
        let session_id = body.session_id.clone();
        let _ = tokio::task::spawn_blocking(move || {
            audit_db.write_audit_log(&crate::models::NewAuditEntry {
                action: "REDIS_DISCONNECT".into(),
                target: Some(session_id),
                result: "success".into(),
                ..Default::default()
            })
        })
        .await;

        (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
    } else {
        tracing::warn!(
            action = "REDIS_DISCONNECT",
            session_id = %body.session_id,
            "Redis session not found"
        );
        error_response("SESSION_NOT_FOUND", "session not found").into_response()
    }
}

async fn databases(
    State(state): State<AppState>,
    Query(params): Query<SessionQuery>,
) -> axum::response::Response {
    tracing::debug!(
        action = "REDIS_DATABASES",
        session_id = %params.session_id,
        "Redis listing databases"
    );

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
    tracing::info!(
        action = "REDIS_SELECT",
        session_id = %body.session_id,
        db = body.db,
        "Redis selecting database"
    );

    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.select_db(body.db).await {
        Ok(()) => {
            tracing::info!(
                action = "REDIS_SELECT",
                session_id = %body.session_id,
                db = body.db,
                "Redis database selected"
            );

            // 审计日志写入
            let audit_db = state.db.clone();
            let session_id = body.session_id.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "REDIS_SELECT".into(),
                    target: Some(session_id),
                    detail: Some(format!("db={}", body.db)),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;

            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

async fn scan(
    State(state): State<AppState>,
    Query(params): Query<ScanQuery>,
) -> axum::response::Response {
    tracing::debug!(
        action = "REDIS_SCAN",
        session_id = %params.session_id,
        pattern = %params.pattern,
        count = params.count,
        "Redis scanning keys"
    );

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
    tracing::debug!(
        action = "REDIS_GET_KEY",
        session_id = %params.session_id,
        key = %params.key,
        "Redis reading key"
    );

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
    tracing::info!(
        action = "REDIS_SET_KEY",
        session_id = %body.session_id,
        key = %body.key,
        value_len = body.value.len(),
        "Redis writing key"
    );

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
    tracing::info!(
        action = "REDIS_DEL",
        session_id = %body.session_id,
        keys_count = body.keys.len(),
        "Redis deleting keys"
    );

    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.del(&body.keys).await {
        Ok(count) => {
            tracing::info!(
                action = "REDIS_DEL",
                session_id = %body.session_id,
                deleted = count,
                "Redis keys deleted"
            );

            // 审计日志写入
            let audit_db = state.db.clone();
            let session_id = body.session_id.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "REDIS_DEL".into(),
                    target: Some(session_id),
                    detail: Some(format!("deleted={}", count)),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;

            (StatusCode::OK, Json(serde_json::json!({"deleted": count}))).into_response()
        }
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

async fn get_ttl(
    State(state): State<AppState>,
    Query(params): Query<TtlQuery>,
) -> axum::response::Response {
    tracing::debug!(
        action = "REDIS_GET_TTL",
        session_id = %params.session_id,
        key = %params.key,
        "Redis reading TTL"
    );

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
    tracing::info!(
        action = "REDIS_SET_TTL",
        session_id = %body.session_id,
        key = %body.key,
        seconds = body.seconds,
        "Redis setting TTL"
    );

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
    tracing::debug!(
        action = "REDIS_INFO",
        session_id = %params.session_id,
        "Redis reading server info"
    );

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
    // 脱敏处理：仅记录命令名称，AUTH 等敏感命令不记录参数
    let cmd_name = body
        .args
        .first()
        .map(|s| s.to_uppercase())
        .unwrap_or_default();
    let is_sensitive = matches!(cmd_name.as_str(), "AUTH" | "CONFIG" | "DEBUG");

    tracing::info!(
        action = "REDIS_COMMAND",
        session_id = %body.session_id,
        command = %cmd_name,
        args_count = body.args.len().saturating_sub(1),
        has_sensitive_args = is_sensitive,
        "Redis command executed"
    );

    let mut pool = state.redis_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.command(&body.args).await {
        Ok(result) => {
            // 审计日志写入（脱敏：仅记录命令名）
            let audit_db = state.db.clone();
            let session_id = body.session_id.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "REDIS_COMMAND".into(),
                    target: Some(session_id),
                    detail: Some(format!("command={}", cmd_name)),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;

            (StatusCode::OK, Json(serde_json::json!({"result": result}))).into_response()
        }
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}
