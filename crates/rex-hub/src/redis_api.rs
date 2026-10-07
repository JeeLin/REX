//! Redis 控制台 REST 路由。

use std::collections::HashMap;
use std::sync::Arc;
use url::form_urlencoded;

use crate::db::{audit_log_scoped, AuditScope};
use crate::resource_conn::{load_resource_config, ResourceConnInfo};
use crate::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use futures_util::StreamExt;
use rex_common::redis::{RedisConnectRequest, RedisConnector};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::error::{
    connect_error_response_with_stage, error_with_status, redact_secrets, ErrorBody, ProtoKind,
};

/// 全局 Redis 连接池状态
pub type RedisState = Arc<Mutex<RedisConnectionPool>>;

/// 每个 session 存储的连接器 + 创建时的连接参数（用于 Pub/Sub 新建连接）
pub struct SessionEntry {
    pub connector: Box<dyn RedisConnector>,
    pub connect_request: RedisConnectRequest,
    /// 建连时的审计归属：`select` / `del` / `command` / `disconnect` 只有
    /// session id，得靠它还原资源与环境，否则按资源过滤时查不到这些事件。
    pub audit_scope: AuditScope,
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

    pub fn insert(
        &mut self,
        id: String,
        conn: Box<dyn RedisConnector>,
        req: RedisConnectRequest,
        scope: AuditScope,
    ) {
        self.entries.insert(
            id,
            SessionEntry {
                connector: conn,
                connect_request: req,
                audit_scope: scope,
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

    /// 获取会话的审计归属；会话不存在时给空归属，不让审计写入失败。
    pub fn audit_scope(&self, id: &str) -> AuditScope {
        self.entries
            .get(id)
            .map(|e| e.audit_scope.clone())
            .unwrap_or_default()
    }
}

/// Redis 事件的归属：资源与环境取自资源记录，agent 维度取
/// `load_resource_config` 解析出的在线 Agent（直连为 None）。
fn resource_scope(state: &AppState, res: &ResourceConnInfo, resource_id: &str) -> AuditScope {
    AuditScope {
        environment_id: state
            .db
            .get_resource(resource_id)
            .ok()
            .flatten()
            .map(|r| r.environment_id),
        resource_id: Some(resource_id.to_string()),
        agent_id: res.agent_id.clone(),
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

fn error_response(code: &str, message: &str) -> (StatusCode, Json<ErrorBody>) {
    error_with_status(StatusCode::BAD_REQUEST, code, message)
}

/// Redis 建连失败的结构化响应：`code` 带 `REDIS_` 前缀、`stage` 给出连接阶段、
/// `message` 保留完整错误链并抹掉密码。
fn connect_error(context: &str, raw: &str, chain: &str) -> Json<ErrorBody> {
    connect_error_response_with_stage(context, raw, chain, ProtoKind::Redis)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// 组装发给 Agent 的 Redis 连接配置（host/port + config_json 合并）。
///
/// 口径与 `file_api::agent_file_config`、`sql_api::agent_sql_config` 一致：
/// 先合并 `config_json`，再以资源顶层 `username` 强制覆盖——前端只写顶层字段，
/// `config_json` 中的历史 `username` 键不是权威值（Agent 侧当前不消费该键，
/// 下发为三 API 口径统一）。
fn agent_redis_config(res: &ResourceConnInfo) -> serde_json::Value {
    let mut cfg = serde_json::json!({
        "host": res.host,
        "port": res.port.unwrap_or(6379),
    });
    if let serde_json::Value::Object(m) = res.config.clone() {
        for (k, v) in m {
            cfg[k] = v;
        }
    }
    if let serde_json::Value::Object(m) = &mut cfg {
        // 资源顶层 username 为权威字段，不被 config_json 中的历史键覆盖
        m.insert("username".to_string(), res.username.clone().into());
    }
    cfg
}

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
        let cfg = agent_redis_config(&res);
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
            resource_scope(&state, &res, &body.resource_id),
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
            let audit_scope = resource_scope(&state, &res, &body.resource_id);
            state.redis_pool.lock().await.insert(
                session_id.clone(),
                Box::new(conn),
                connect_req,
                audit_scope.clone(),
            );

            tracing::info!(
                action = "REDIS_CONNECT",
                session_id = %session_id,
                resource_id = %body.resource_id,
                resource_name = %res.name,
                "Redis connected"
            );

            audit_log_scoped(
                &state.db,
                "REDIS_CONNECT",
                "success",
                Some(res.host.clone()),
                audit_scope.clone(),
            );

            (StatusCode::OK, Json(ConnectResponse { session_id })).into_response()
        }
        Err(e) => {
            tracing::warn!(
                action = "REDIS_CONNECT",
                resource_id = %body.resource_id,
                host = %res.host,
                error = %e,
                "Redis connect failed"
            );
            audit_log_scoped(
                &state.db,
                "REDIS_CONNECT",
                "failure",
                Some(res.host.clone()),
                resource_scope(&state, &res, &body.resource_id),
            );
            connect_error(
                &format!(
                    "failed to connect to Redis at {}:{}",
                    res.host,
                    res.port.unwrap_or(6379)
                ),
                &e.to_string(),
                &redact_secrets(
                    &format!("{e:#}"),
                    &[connect_req.password.as_deref().unwrap_or("")],
                ),
            )
            .into_response()
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
        let audit_scope = entry.audit_scope.clone();

        tracing::info!(
            action = "REDIS_DISCONNECT",
            session_id = %body.session_id,
            "Redis disconnected"
        );

        audit_log_scoped(
            &state.db,
            "REDIS_DISCONNECT",
            "success",
            Some(body.session_id.clone()),
            audit_scope,
        );

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
    let audit_scope = pool.audit_scope(&body.session_id);
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
                    environment_id: audit_scope.environment_id,
                    resource_id: audit_scope.resource_id,
                    agent_id: audit_scope.agent_id,
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
    let audit_scope = pool.audit_scope(&body.session_id);
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
                    environment_id: audit_scope.environment_id,
                    resource_id: audit_scope.resource_id,
                    agent_id: audit_scope.agent_id,
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
    let audit_scope = pool.audit_scope(&body.session_id);
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
                    environment_id: audit_scope.environment_id,
                    resource_id: audit_scope.resource_id,
                    agent_id: audit_scope.agent_id,
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
                    let enc_pass =
                        form_urlencoded::byte_serialize(password.as_bytes()).collect::<String>();
                    format!("redis://:{}@{host}:{}", enc_pass, req.port)
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

    #[allow(deprecated)]
    let conn = match client.get_async_connection().await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rex_common::redis::{DbInfo, KeyInfo, RedisInfo, RedisValue};

    fn info(username: &str, config: &str) -> ResourceConnInfo {
        ResourceConnInfo {
            resource_id: "r1".into(),
            name: "cache".into(),
            protocol: "redis".into(),
            host: "10.0.0.2".into(),
            port: Some(6380),
            username: username.to_string(),
            config: serde_json::from_str(config).unwrap(),
            subtype: None,
            use_agent: true,
            agent_id: Some("agent-1".into()),
        }
    }

    #[test]
    fn agent_redis_config_carries_username_and_merged_credentials() {
        let cfg = agent_redis_config(&info("alice", r#"{"password":"pw","db":3}"#));
        assert_eq!(
            cfg.get("username").and_then(|v| v.as_str()),
            Some("alice"),
            "top-level username is always forwarded, same as file/sql agent configs"
        );
        assert_eq!(cfg.get("host").and_then(|v| v.as_str()), Some("10.0.0.2"));
        assert_eq!(cfg.get("port").and_then(|v| v.as_u64()), Some(6380));
        assert_eq!(cfg.get("password").and_then(|v| v.as_str()), Some("pw"));
        assert_eq!(cfg.get("db").and_then(|v| v.as_i64()), Some(3));
    }

    #[test]
    fn agent_redis_config_username_beats_stale_config_key() {
        let cfg = agent_redis_config(&info("alice", r#"{"username":"stale"}"#));
        assert_eq!(
            cfg.get("username").and_then(|v| v.as_str()),
            Some("alice"),
            "top-level username must stay authoritative after config_json merge"
        );
    }

    #[test]
    fn redis_connect_error_codes_are_prefixed_by_root_cause() {
        let cases = [
            ("Connection refused", "REDIS_CONNECTION_REFUSED", "tcp"),
            (
                "connection timed out",
                "REDIS_CONNECTION_TIMEOUT",
                "timeout",
            ),
            ("no such host", "REDIS_DNS_FAILURE", "dns"),
            ("certificate verify failed", "REDIS_TLS_FAILURE", "tls"),
            ("invalid credentials", "REDIS_AUTH_FAILED", "auth"),
            ("something odd", "REDIS_CONNECTION_FAILED", "connect"),
        ];
        for (raw, code, stage) in cases {
            let resp = connect_error("failed to connect to Redis at 10.0.0.2:6380", raw, raw);
            assert_eq!(resp.0.error.code, code);
            assert_eq!(resp.0.error.stage.as_deref(), Some(stage));
            assert!(resp
                .0
                .error
                .message
                .starts_with("failed to connect to Redis at 10.0.0.2:6380: "));
        }
    }

    #[test]
    fn redis_connect_error_masks_password_in_error_chain() {
        let raw = "error";
        let chain = redact_secrets(
            "error: Connection refused for redis://:s3cr3t@10.0.0.2:6380/0",
            &["s3cr3t", ""],
        );
        let resp = connect_error("failed to connect to Redis at 10.0.0.2:6380", raw, &chain);
        assert_eq!(resp.0.error.code, "REDIS_CONNECTION_FAILED");
        assert!(!resp.0.error.message.contains("s3cr3t"));
        assert!(resp.0.error.message.contains("Connection refused"));
    }

    /// `disconnect` / `select` / `del` / `command` 只有 session id，归属必须
    /// 随会话条目一起保存并能取回；会话不存在时给空归属而非编造。
    #[test]
    fn session_scope_round_trips_and_defaults_when_absent() {
        let mut pool = RedisConnectionPool::new();
        let scope = AuditScope {
            environment_id: Some("env-redis".into()),
            resource_id: Some("res-redis".into()),
            agent_id: Some("agent-4".into()),
        };
        pool.insert(
            "redis_1".into(),
            Box::new(DummyRedis),
            RedisConnectRequest {
                host: "10.0.0.2".into(),
                port: 6380,
                password: None,
                db: None,
            },
            scope,
        );

        let got = pool.audit_scope("redis_1");
        assert_eq!(got.resource_id.as_deref(), Some("res-redis"));
        assert_eq!(got.environment_id.as_deref(), Some("env-redis"));
        assert_eq!(got.agent_id.as_deref(), Some("agent-4"));

        assert!(pool.remove("redis_1").is_some());
        assert!(
            pool.audit_scope("redis_1").resource_id.is_none(),
            "a removed session must not leave a stale scope behind"
        );
        assert!(pool.audit_scope("never-opened").agent_id.is_none());
    }

    /// `resource_scope` 只取资源记录里真实存在的环境 id 与 agent，不编造。
    #[test]
    fn resource_scope_reads_env_and_agent_from_resource_record() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());
        let env = state
            .db
            .create_environment(&crate::models::NewEnvironment {
                name: format!("env-{}", uuid::Uuid::new_v4()),
                description: None,
                connection_mode: Some("direct".into()),
            })
            .unwrap();
        let resource_id = state
            .db
            .create_resource(
                &env.id,
                &crate::models::NewResource {
                    name: "cache".into(),
                    protocol: "redis".into(),
                    host: "10.0.0.2".into(),
                    port: Some(6380),
                    username: Some("default".into()),
                    config_json: None,
                    subtype: None,
                    color: None,
                    sort_order: None,
                },
            )
            .unwrap()
            .id;
        let res = load_resource_config(&state, &resource_id).unwrap();

        let scope = resource_scope(&state, &res, &resource_id);
        assert_eq!(scope.resource_id.as_deref(), Some(&*resource_id));
        assert_eq!(scope.environment_id.as_deref(), Some(&*env.id));
        assert!(
            scope.agent_id.is_none(),
            "a direct environment has no agent — leave the dimension unset"
        );
    }

    /// Redis 事件按 resource_id 过滤可查：`REDIS_CONNECT` 与走会话归属的
    /// `REDIS_SELECT` / `REDIS_DEL` / `REDIS_COMMAND` 共用同一份 scope。
    #[tokio::test]
    async fn redis_audit_events_are_visible_when_filtering_by_resource_id() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());
        let scope = AuditScope {
            environment_id: Some("env-redis".into()),
            resource_id: Some("res-redis".into()),
            agent_id: Some("agent-4".into()),
        };
        state.redis_pool.lock().await.insert(
            "redis_live".into(),
            Box::new(DummyRedis),
            RedisConnectRequest {
                host: "10.0.0.2".into(),
                port: 6380,
                password: None,
                db: None,
            },
            scope.clone(),
        );

        // connect 侧（写审计后回查）
        crate::db::audit_log_scoped(
            &state.db,
            "REDIS_CONNECT",
            "success",
            Some("10.0.0.2".into()),
            scope,
        );
        // 会话侧：归属从连接池取，与 connect 写入的是同一份
        let session_scope = state.redis_pool.lock().await.audit_scope("redis_live");
        for action in ["REDIS_SELECT", "REDIS_DEL", "REDIS_COMMAND"] {
            crate::db::audit_log_scoped(
                &state.db,
                action,
                "success",
                Some("redis_live".into()),
                session_scope.clone(),
            );
        }

        let mut seen = Vec::new();
        for _ in 0..50 {
            seen = state
                .db
                .query_audit_log(&crate::models::AuditFilter {
                    resource_id: Some("res-redis".into()),
                    ..Default::default()
                })
                .unwrap();
            if seen.len() >= 4 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let actions: Vec<&str> = seen.iter().map(|e| e.action.as_str()).collect();
        for want in [
            "REDIS_CONNECT",
            "REDIS_SELECT",
            "REDIS_DEL",
            "REDIS_COMMAND",
        ] {
            assert!(actions.contains(&want), "{want} missing, got {actions:?}");
        }
        assert!(seen
            .iter()
            .all(|e| e.resource_id.as_deref() == Some("res-redis")));
        assert!(state
            .db
            .query_audit_log(&crate::models::AuditFilter {
                resource_id: Some("res-unrelated".into()),
                ..Default::default()
            })
            .unwrap()
            .is_empty());
    }

    /// 归属表只关心键，连接行为用空实现占位。
    struct DummyRedis;

    #[async_trait::async_trait]
    impl RedisConnector for DummyRedis {
        async fn info(&mut self) -> anyhow::Result<RedisInfo> {
            Ok(RedisInfo {
                redis_version: String::new(),
                os: String::new(),
                process_id: String::new(),
                connected_clients: String::new(),
                used_memory: String::new(),
                used_memory_peak: String::new(),
                total_commands_processed: String::new(),
                keyspace: Vec::new(),
            })
        }

        async fn dbs(&mut self) -> anyhow::Result<Vec<DbInfo>> {
            Ok(Vec::new())
        }

        async fn select_db(&mut self, _db: i32) -> anyhow::Result<()> {
            Ok(())
        }

        async fn scan(&mut self, _pattern: &str, _count: u32) -> anyhow::Result<Vec<KeyInfo>> {
            Ok(Vec::new())
        }

        async fn get_type(&mut self, _key: &str) -> anyhow::Result<String> {
            Ok(String::new())
        }

        async fn get_value(&mut self, _key: &str) -> anyhow::Result<RedisValue> {
            Ok(RedisValue::String {
                value: String::new(),
                format: None,
            })
        }

        async fn set_value(&mut self, _key: &str, _value: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn del(&mut self, _keys: &[String]) -> anyhow::Result<u64> {
            Ok(0)
        }

        async fn ttl(&mut self, _key: &str) -> anyhow::Result<i64> {
            Ok(-1)
        }

        async fn set_ttl(&mut self, _key: &str, _seconds: i64) -> anyhow::Result<()> {
            Ok(())
        }

        async fn command(&mut self, _args: &[String]) -> anyhow::Result<String> {
            Ok(String::new())
        }

        async fn close(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
    }
}
