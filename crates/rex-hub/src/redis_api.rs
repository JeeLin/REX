//! Redis 控制台 REST 路由。

use std::collections::HashMap;
use std::sync::Arc;

use crate::resource_conn::load_resource_config;
use crate::AppState;
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use futures_util::{SinkExt, StreamExt};
use rex_common::redis::{RedisConnectRequest, RedisConnector};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// 全局 Redis 连接池状态
pub type RedisState = Arc<Mutex<RedisConnectionPool>>;

/// 每个 session 存储的连接器 + 创建时的连接参数（用于 Pub/Sub 新建连接）
pub struct SessionEntry {
    pub connector: Box<dyn RedisConnector>,
    pub connect_request: RedisConnectRequest,
}

pub struct RedisConnectionPool {
    entries: HashMap<String, SessionEntry>,
}

impl Default for RedisConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl RedisConnectionPool {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: String, conn: Box<dyn RedisConnector>, req: RedisConnectRequest) {
        self.entries.insert(
            id,
            SessionEntry {
                connector: conn,
                connect_request: req,
            },
        );
    }

    pub fn remove(&mut self, id: &str) -> Option<SessionEntry> {
        self.entries.remove(id)
    }

    /// 获取连接器的可变引用
    pub fn get_connector_mut(&mut self, id: &str) -> Option<&mut Box<dyn RedisConnector>> {
        self.entries.get_mut(id).map(|e| &mut e.connector)
    }

    /// 获取会话的连接参数（用于创建新的 Pub/Sub 连接）
    pub fn get_connect_request(&self, id: &str) -> Option<&RedisConnectRequest> {
        self.entries.get(id).map(|e| &e.connect_request)
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
        .route("/pubsub/poll", axum::routing::post(pubsub_poll))
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

    // v0.70.6 子任务 #7：agent 模式 —— 协议在 Agent 私网内终结，Hub 仅做隧道中转。
    if res.use_agent {
        let agent_id = match res.agent_id.clone() {
            Some(id) => id,
            None => {
                return error_response("AGENT_UNAVAILABLE", "no online agent for environment")
                    .into_response()
            }
        };
        let mut cfg = serde_json::json!({
            "host": res.host,
            "port": res.port.unwrap_or(6379),
        });
        if let serde_json::Value::Object(m) = res.config.clone() {
            for (k, v) in m {
                cfg[k] = v;
            }
        }
        let channel_id = match crate::agent_ws::open_agent_session(
            &state,
            &agent_id,
            &body.resource_id,
            "redis",
            cfg,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                return error_response("AGENT_CONNECT_FAILED", &e.to_string()).into_response()
            }
        };
        let session_id = format!("redis_{}", &uuid::Uuid::new_v4().to_string()[..8]);
        // Agent 模式的 Pub/Sub 不支持（代理不支持订阅），存储一个占位连接参数
        let agent_req = RedisConnectRequest {
            host: "agent-proxy".into(),
            port: 0,
            password: None,
            db: None,
        };
        state.redis_pool.lock().await.insert(
            session_id.clone(),
            Box::new(crate::agent_proxy::AgentRedisProxy::new(
                state.clone(),
                channel_id,
            )),
            agent_req,
        );
        tracing::info!(action = "REDIS_CONNECT_AGENT", session_id = %session_id, resource_id = %body.resource_id, resource_name = %res.name, agent_id = %agent_id, "Redis connected via agent");
        return (StatusCode::OK, Json(ConnectResponse { session_id })).into_response();
    }

    let connect_req = RedisConnectRequest {
        host: res.host.clone(),
        port: res.port.unwrap_or(6379),
        password: res
            .config
            .get("password")
            .and_then(|v| v.as_str())
            .map(String::from),
        db: res
            .config
            .get("db")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
    };

    tracing::info!(
        action = "REDIS_CONNECT",
        resource_id = %body.resource_id,
        resource_name = %res.name,
        host = %res.host,
        port = res.port.unwrap_or(6379),
        "Redis connecting"
    );

    match rex_redis::RedisConnectorImpl::connect(connect_req.clone()).await {
        Ok(conn) => {
            let session_id = format!("redis_{}", &uuid::Uuid::new_v4().to_string()[..8]);
            state
                .redis_pool
                .lock()
                .await
                .insert(session_id.clone(), Box::new(conn), connect_req);

            tracing::info!(
                action = "REDIS_CONNECT",
                session_id = %session_id,
                resource_id = %body.resource_id,
                resource_name = %res.name,
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
    if let Some(mut entry) = pool.remove(&body.session_id) {
        let _ = entry.connector.close().await;

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
    let conn = match pool.get_connector_mut(&params.session_id) {
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
    let conn = match pool.get_connector_mut(&body.session_id) {
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
    let conn = match pool.get_connector_mut(&params.session_id) {
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
    let conn = match pool.get_connector_mut(&params.session_id) {
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
    let conn = match pool.get_connector_mut(&body.session_id) {
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
    let conn = match pool.get_connector_mut(&body.session_id) {
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
    let conn = match pool.get_connector_mut(&params.session_id) {
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
    let conn = match pool.get_connector_mut(&body.session_id) {
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
    let conn = match pool.get_connector_mut(&params.session_id) {
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
    let conn = match pool.get_connector_mut(&body.session_id) {
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

// ---------------------------------------------------------------------------
// Pub/Sub polling
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct PubSubPollBody {
    pub session_id: String,
    pub channels: Vec<String>,
    #[serde(default = "default_poll_timeout")]
    pub timeout_ms: u64,
}

fn default_poll_timeout() -> u64 {
    5000
}

#[derive(Debug, Serialize)]
pub struct PubSubPollResult {
    pub messages: Vec<PubSubMessage>,
}

#[derive(Debug, Serialize)]
pub struct PubSubMessage {
    pub channel: String,
    pub data: String,
}

/// POST /api/redis/pubsub/poll
/// 创建临时订阅连接，拉取指定时间窗口内的消息后返回。
/// 前端通过定时轮询实现"实时"效果。
pub async fn pubsub_poll(
    State(state): State<AppState>,
    Json(body): Json<PubSubPollBody>,
) -> impl IntoResponse {
    if body.channels.is_empty() {
        return (StatusCode::OK, Json(PubSubPollResult { messages: vec![] })).into_response();
    }

    // 获取连接参数
    let url = {
        let pool = state.redis_pool.lock().await;
        match pool.get_connect_request(&body.session_id) {
            Some(req) => {
                let host = rex_common::bracket_host(&req.host);
                if let Some(ref password) = req.password {
                    format!("redis://:{}@{host}:{}", password, req.port)
                } else {
                    format!("redis://{host}:{}", req.port)
                }
            }
            None => {
                return error_response("SESSION_NOT_FOUND", "session not found").into_response();
            }
        }
    };

    // 创建临时连接
    let client = match redis::Client::open(url.as_str()) {
        Ok(c) => c,
        Err(e) => return error_response("CLIENT_ERROR", &e.to_string()).into_response(),
    };

    let mut conn = match client.get_async_connection().await {
        Ok(c) => c,
        Err(e) => return error_response("CONNECT_ERROR", &e.to_string()).into_response(),
    };

    // 转为 PubSub 模式
    let mut pubsub = conn.into_pubsub();

    // 订阅频道（async）
    for ch in &body.channels {
        if let Err(e) = pubsub.subscribe(ch.as_str()).await {
            return error_response("SUBSCRIBE_ERROR", &e.to_string()).into_response();
        }
    }

    // 获取消息流
    let mut stream = pubsub.on_message();

    // 在超时时间内尽可能多地读取消息
    let timeout = std::time::Duration::from_millis(body.timeout_ms.min(30_000));
    let deadline = tokio::time::Instant::now() + timeout;
    let mut messages = Vec::new();

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(msg)) => {
                let channel = msg.get_channel_name().to_string();
                let data: String = msg.get_payload().unwrap_or_default();
                messages.push(PubSubMessage { channel, data });
                if messages.len() >= 1000 {
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => break, // 超时
        }
    }

    (StatusCode::OK, Json(PubSubPollResult { messages })).into_response()
}
