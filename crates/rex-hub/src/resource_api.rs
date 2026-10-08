//! 资源管理 REST API。

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::error::{api_error as err, ErrorBody};
use crate::models::{NewResource, Resource};
use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorBody>)>;

/// 资源路由（嵌套在 /api/environments 下）
pub fn resource_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/{env_id}/resources",
            axum::routing::get(list_resources).post(create_resource),
        )
        .route(
            "/{env_id}/resources/{resource_id}",
            axum::routing::get(get_resource)
                .put(update_resource)
                .delete(delete_resource),
        )
        .route(
            "/{env_id}/resources/{resource_id}/active-account",
            axum::routing::post(set_active_account),
        )
}

// --- API handlers ---

async fn list_resources(
    State(state): State<AppState>,
    Path(env_id): Path<String>,
) -> ApiResult<Vec<Resource>> {
    let db = state.db.clone();
    let mut resources = tokio::task::spawn_blocking(move || db.list_resources_by_env(&env_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    // 解密每条资源的 config_json
    for r in &mut resources {
        if crate::resource_conn::has_config_json(&r.config_json) {
            if let Ok(dec) = state.crypto.decrypt(&r.config_json) {
                r.config_json = dec;
            } else {
                r.config_json = String::new();
                tracing::warn!(
                    resource_id = %r.id,
                    "config_json decrypt failed (data key mismatch)"
                );
            }
        }
    }
    Ok(Json(resources))
}

async fn get_resource(
    State(state): State<AppState>,
    Path((_env_id, resource_id)): Path<(String, String)>,
) -> ApiResult<Resource> {
    let db = state.db.clone();
    let mut resource = tokio::task::spawn_blocking(move || db.get_resource(&resource_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "resource not found"))?;
    // 解密 config_json
    if crate::resource_conn::has_config_json(&resource.config_json) {
        match state.crypto.decrypt(&resource.config_json) {
            Ok(dec) => resource.config_json = dec,
            Err(_) => {
                return Err(err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    crate::error::CREDENTIAL_DECRYPT_MSG,
                ));
            }
        }
    }
    Ok(Json(resource))
}

async fn create_resource(
    State(state): State<AppState>,
    Path(env_id): Path<String>,
    Json(mut body): Json<NewResource>,
) -> ApiResult<Resource> {
    tracing::info!(
        action = "RESOURCE_CREATE",
        env_id = %env_id,
        protocol = %body.protocol,
        name = %body.name,
        host = %body.host,
        "creating resource"
    );

    if body.name.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "name is required"));
    }
    if body.host.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "host is required"));
    }
    // 加密 config_json 中的凭据
    if let Some(ref cfg) = body.config_json {
        match state.crypto.encrypt(cfg) {
            Ok(enc) => body.config_json = Some(enc),
            Err(e) => return Err(err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())),
        }
    }
    // 验证环境存在
    let db = state.db.clone();
    let env_id_check = env_id.clone();
    let env_exists = tokio::task::spawn_blocking(move || db.get_environment(&env_id_check))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .is_some();
    if !env_exists {
        return Err(err(StatusCode::NOT_FOUND, "environment not found"));
    }
    let db = state.db.clone();
    let resource = tokio::task::spawn_blocking(move || db.create_resource(&env_id, &body))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    // 审计日志
    let audit_db = state.db.clone();
    let res_name = resource.name.clone();
    let res_env_id = resource.environment_id.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_CREATE".into(),
            target: Some(res_name),
            environment_id: Some(res_env_id),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;
    Ok(Json(resource))
}

async fn update_resource(
    State(state): State<AppState>,
    Path((env_id, resource_id)): Path<(String, String)>,
    Json(mut body): Json<NewResource>,
) -> ApiResult<Resource> {
    tracing::info!(
        action = "RESOURCE_UPDATE",
        env_id = %env_id,
        resource_id = %resource_id,
        protocol = %body.protocol,
        name = %body.name,
        "updating resource"
    );

    // 加密 config_json 中的凭据
    if let Some(ref cfg) = body.config_json {
        match state.crypto.encrypt(cfg) {
            Ok(enc) => body.config_json = Some(enc),
            Err(e) => return Err(err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())),
        }
    }
    let db = state.db.clone();
    let resource =
        tokio::task::spawn_blocking(move || db.update_resource(&env_id, &resource_id, &body))
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("not found") {
                    err(StatusCode::NOT_FOUND, &msg)
                } else {
                    err(StatusCode::INTERNAL_SERVER_ERROR, &msg)
                }
            })?;

    // 审计日志
    let audit_db = state.db.clone();
    let res_name = resource.name.clone();
    let res_env_id = resource.environment_id.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_UPDATE".into(),
            target: Some(res_name),
            environment_id: Some(res_env_id),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;

    Ok(Json(resource))
}

#[derive(serde::Deserialize)]
struct SetActiveAccountBody {
    account_id: String,
}

// 专用端点：仅切换 SIP 资源的生效账户，前端无需先 get 全量再 update。
async fn set_active_account(
    State(state): State<AppState>,
    Path((env_id, resource_id)): Path<(String, String)>,
    Json(body): Json<SetActiveAccountBody>,
) -> ApiResult<Resource> {
    tracing::info!(
        action = "RESOURCE_SET_ACTIVE_ACCOUNT",
        env_id = %env_id,
        resource_id = %resource_id,
        account_id = %body.account_id,
        "switching active sip account"
    );

    let db = state.db.clone();
    let crypto = state.crypto.clone();
    let account_id = body.account_id.clone();
    let resource = tokio::task::spawn_blocking(move || {
        db.set_resource_active_account(&crypto, &env_id, &resource_id, &account_id)
    })
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("not found") {
            err(StatusCode::NOT_FOUND, &msg)
        } else {
            err(StatusCode::BAD_REQUEST, &msg)
        }
    })?;

    // 审计日志：后台异步写入，不阻塞响应返回（fire-and-forget）。
    let audit_db = state.db.clone();
    let res_name = resource.name.clone();
    let res_env_id = resource.environment_id.clone();
    tokio::task::spawn_blocking(move || {
        let _ = audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_SET_ACTIVE_ACCOUNT".into(),
            target: Some(res_name),
            environment_id: Some(res_env_id),
            result: "success".into(),
            ..Default::default()
        });
    });

    Ok(Json(resource))
}

async fn delete_resource(
    State(state): State<AppState>,
    Path((env_id, resource_id)): Path<(String, String)>,
) -> ApiResult<serde_json::Value> {
    let db = state.db.clone();
    let check_id = resource_id.clone();
    let resource = tokio::task::spawn_blocking(move || db.get_resource(&check_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    if resource.is_none() {
        return Err(err(StatusCode::NOT_FOUND, "resource not found"));
    }
    let res_name = resource.map(|r| r.name).unwrap_or_default();

    let db = state.db.clone();
    tracing::info!(
        action = "RESOURCE_DELETE",
        env_id = %env_id,
        resource_id = %resource_id,
        resource_name = %res_name,
        "deleting resource"
    );

    let del_env_id = env_id.clone();
    let del_id = resource_id.clone();
    tokio::task::spawn_blocking(move || db.delete_resource(&del_env_id, &del_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    // 审计日志
    let audit_db = state.db.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_DELETE".into(),
            target: Some(res_name),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// --- Test connection ---

/// 探测类请求的默认超时：Hub 直连 TCP、Agent connect 回帧、redis PING 共用同一口径。
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// 探测超时的用户可见文案。
const PROBE_TIMEOUT_MSG: &str = "connection timed out";

/// 下发给 Agent 的探测请求 id 前缀（与 `agent_ws::open_agent_session` 同口径）。
const AGENT_REQUEST_ID_PREFIX: &str = "req_";

#[derive(serde::Deserialize)]
pub struct TestConnectionRequest {
    pub protocol: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub config_json: Option<String>,
    pub environment_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct TestConnectionResult {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// 下发给 Agent 的 connect config：host/port + 合并 `config_json`（凭证等）。
///
/// 顶层 `username` 在 merge **之后**写入为权威字段（空 → `root`，
/// [`crate::resource_conn::normalize_username`]），口径与
/// `file_api::agent_file_config` 同源：Agent 侧 `agent_ssh::parse_ssh_config`
/// 以下发 config 的 username 认证，缺字段即以空用户认证，测试连接必被拒（CR15）。
///
/// Agent 侧真正的认证是 ssh/sftp 的 `handle_connect_ssh` / `handle_connect_file`
/// 与 sql/redis 各自的 connector 建连；`probe` 请求的轻量 ssh 探测**也读凭据**
/// （`connect_with_handle` 在 `Ok` 前已跑完 TCP → KEX → authenticate，
/// 错凭据回 `connect_error` 而非 `Connected`），但 config 仍按同一形状下发，
/// 便于 Agent 侧失败文案统一 redact。
fn agent_test_connect_config(
    host: &str,
    port: u16,
    username: Option<&str>,
    config_json: Option<&str>,
) -> serde_json::Value {
    // config_json 解析失败按空配置处理（静默丢弃），合并逻辑归口
    // `merge_resource_config`（与 `file_api::agent_file_config` 同源）。
    let config = match config_json {
        Some(cfg_str) => {
            serde_json::from_str::<serde_json::Value>(cfg_str).unwrap_or(serde_json::Value::Null)
        }
        None => serde_json::Value::Null,
    };
    crate::resource_conn::merge_resource_config(host, port, username.unwrap_or(""), &config)
}

/// 环境是否走 Agent 隧道：仅 `connection_mode == "agent"` 为真。
///
/// 真实数据路径按此判定选 Agent 连接器，测试连接必须同口径，否则内网目标
/// 在 Hub 直连探测上必然 `No route to host`。取值失败（DB 报错或 spawn_blocking
/// panic）记 warn 后回退直连 —— 静默 `unwrap_or(false)` 会把基础设施故障伪装成
/// 「非 agent 环境」，排查时看不出探测为何走了直连。
async fn env_uses_agent(state: &crate::AppState, env_id: Option<&str>) -> bool {
    let Some(env_id) = env_id else { return false };
    let db = state.db.clone();
    let eid = env_id.to_string();
    let lookup_eid = eid.clone();
    match tokio::task::spawn_blocking(move || db.get_environment(&lookup_eid)).await {
        Ok(Ok(env)) => env.is_some_and(|e| e.connection_mode == "agent"),
        Ok(Err(e)) => {
            tracing::warn!(
                env_id = %eid,
                error = %e,
                "environment lookup failed, falling back to direct probe"
            );
            false
        }
        Err(e) => {
            tracing::warn!(
                env_id = %eid,
                error = %e,
                "environment lookup task failed, falling back to direct probe"
            );
            false
        }
    }
}

/// Hub 直连 TCP 可达性探测（ssh / sftp / sql / mysql / postgresql 共用）。
///
/// 只验地址可达，不验凭据。Agent 侧走隧道探测：sql/redis 连真库验凭据，
/// ssh 走 `probe_ssh` 握手+认证验凭据；sftp 走真实 SFTP 连接器验凭据。
/// 深度不同，但两者都只回答「这个资源现在能不能连」。
async fn direct_tcp_probe(host: &str, port: u16) -> Result<(), String> {
    let addr = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    match tokio::time::timeout(PROBE_TIMEOUT, tokio::net::TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!("TCP connect failed: {e}")),
        Err(_) => Err(PROBE_TIMEOUT_MSG.to_string()),
    }
}

/// 从下发给 Agent 的 connect config 中取出凭据片段。
///
/// Agent 侧驱动会把连接串回显到错误里（`agent_sql` 的
/// `SQL connection failed: {e}`），该文案经 `connect_error` 进用户可见 toast
/// 又进日志；凭据只应留在内存里。
fn connect_config_secrets(cfg: &serde_json::Value) -> Vec<String> {
    let mut secrets: Vec<String> = cfg
        .get("password")
        .and_then(|v| v.as_str())
        .map(String::from)
        .into_iter()
        .collect();
    secrets.extend(rex_common::resource_config::config_private_key(cfg));
    secrets
}

/// 抹掉 Agent 回传错误中的凭据明文（用户可见文案与日志共用这一份 redact 结果）。
fn redact_agent_error(message: &str, cfg: &serde_json::Value) -> String {
    let secrets = connect_config_secrets(cfg);
    let refs: Vec<&str> = secrets.iter().map(String::as_str).collect();
    crate::error::redact_secrets(message, &refs)
}

/// 一次 `pending_requests` 登记的所有权句柄：Drop 时反注册。
///
/// `pending_requests` 是无界 `HashMap`（`agent_ws::AgentTunnelState`），且只在
/// 收到 Agent 回帧时按 request_id remove。发送失败 / oneshot 通道关闭 / 超时
/// 这些提前退出路径若各自补 `remove`，新增分支极易再漏 —— 每漏一次就永久残留
/// 一个 `req_*` 键与 oneshot Sender，反复「测试连接」即无界增长。Drop 兜底让
/// 清理与控制流无关（对照 `tunnel_ws` 握手超时分支的手工 remove）。
struct PendingRequestSlot {
    tunnel: Arc<crate::agent_ws::AgentTunnelState>,
    request_id: String,
}

impl PendingRequestSlot {
    async fn register(
        tunnel: Arc<crate::agent_ws::AgentTunnelState>,
        request_id: String,
    ) -> (
        Self,
        tokio::sync::oneshot::Receiver<crate::agent_ws::ConnectResponse>,
    ) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        tunnel
            .pending_requests
            .write()
            .await
            .insert(request_id.clone(), tx);
        (Self { tunnel, request_id }, rx)
    }
}

impl Drop for PendingRequestSlot {
    fn drop(&mut self) {
        let tunnel = self.tunnel.clone();
        let request_id = std::mem::take(&mut self.request_id);
        // 正常路径上回帧消费方已 remove（remove 缺失键是 no-op）；抢不到写锁时
        // 退到后台任务，避免因锁竞争再漏一次清理。
        if let Ok(mut pending) = tunnel.pending_requests.try_write() {
            pending.remove(&request_id);
            return;
        }
        tokio::spawn(async move {
            tunnel.pending_requests.write().await.remove(&request_id);
        });
    }
}

/// agent 模式下把探测请求转发给 Agent（Agent 侧按 protocol 分发）。
///
/// 返回 `None` 表示不适用（无环境 / 非 agent 模式 / 无在线 Agent / WS 未建立），
/// 调用方回退到 Hub 直连；`Some(Err(..))` 表示已尝试但探测失败。
///
/// 探测请求带 `probe` 标记：ssh 在 Agent 侧走 `probe_ssh` 完成**真实认证**后即时
/// 拆连（`connect_with_handle` 跑完 TCP → KEX → authenticate；错凭据回
/// `connect_error` 不入共享会话池 `ssh_handles`）；sftp 不走 `probe`（其
/// `connect_with_handle` 会启动 shell，纯 SFTP 服务端会拒绝），改走 Agent 侧
/// 真实 SFTP 连接器 `handle_connect_file`，用 SessionOpened/SessionError 回传
/// 结果。其余协议无此轻量路径，标记被 Agent 忽略，走各自原有的 connect 流程。
async fn test_connect_via_agent(
    state: &crate::AppState,
    protocol: &str,
    host: &str,
    port: u16,
    username: Option<&str>,
    config_json: Option<&str>,
    environment_id: Option<&str>,
) -> Option<Result<(), String>> {
    if !env_uses_agent(state, environment_id).await {
        return None;
    }

    let db = state.db.clone();
    let eid = environment_id.unwrap_or("").to_string();
    let agent_id = tokio::task::spawn_blocking(move || {
        crate::resource_conn::resolve_agent_mode(&db, &eid, "agent").agent_id
    })
    .await
    .ok()
    .flatten()?;

    let conn = {
        let conns = state.agent_tunnel.connections.read().await;
        conns.get(&agent_id).cloned()
    };
    let conn = match conn {
        Some(c) => c,
        None => return Some(Err("agent not connected".into())),
    };

    let request_id = format!(
        "{AGENT_REQUEST_ID_PREFIX}{}",
        &uuid::Uuid::new_v4().to_string()[..8]
    );
    let connect_config = agent_test_connect_config(host, port, username, config_json);
    let connect_msg = serde_json::json!({
        "type": "connect",
        "payload": {
            "request_id": request_id,
            "resource_id": "test",
            "protocol": protocol,
            "config": connect_config,
            // 探测是「握手+认证后拆除」的轻探针：ssh 走 Agent 侧 `probe_ssh`
            // （`connect_with_handle` 完成认证）；s3 走 Agent 侧 `S3Connector::verify`
            // （list_buckets/head_bucket 真正验凭据 —— `connect_from_request` 只建
            // client 不验凭据，测试时无 bucket 会假阳性）；sip 走 Agent 侧 UA₂ 的
            // 真实 REGISTER（`handle_connect_sip` probe 分支），验完即 drop UA 释放
            // 注册。sftp/sql/redis/mysql/postgresql/sqlite 不走探测（sftp 的
            // probe_ssh 会启动 shell，纯 SFTP 服务端拒绝；其余需要真会话）。
            // sftp 走 Agent 侧 `handle_connect_file` 的真实 SFTP 连接器
            // （`channel_open_session` + sftp subsystem，完成认证且不开 PTY/shell），
            // `probe=false` 让 Hub 把 Test Connection 当一次「即开即关」的真实 sftp
            // 会话处理，借助已有的 SessionOpened/SessionError 回传回路回收结果。
            "probe": matches!(protocol, "ssh" | "s3" | "sip"),
        }
    });

    // 句柄活到本函数返回：发送失败 / 通道关闭 / 超时等提前 return 路径一律由
    // Drop 反注册，新增分支无需记得补 remove。
    let (_slot, resp_rx) =
        PendingRequestSlot::register(state.agent_tunnel.clone(), request_id.clone()).await;

    if conn
        .sender
        .send(crate::agent_ws::AgentEvent::Text(connect_msg.to_string()))
        .await
        .is_err()
    {
        return Some(Err("failed to send connect request to agent".into()));
    }

    // 等待 Agent 响应
    let resp = match tokio::time::timeout(PROBE_TIMEOUT, resp_rx).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(_)) => return Some(Err("agent response channel closed".into())),
        Err(_) => return Some(Err("agent connection timed out".into())),
    };
    if let Some(e) = resp.error {
        return Some(Err(redact_agent_error(&e, &connect_config)));
    }
    // 探测成功：关闭通道。Hub 侧 `channels` 映射交给 Agent 回帧（`closed`）清理，
    // 本地不删 —— Agent 尚未处理 close 的窗口内必须留着映射，否则后续帧无处可路由。
    if let Some(channel_id) = resp.channel_id {
        let close_msg = serde_json::json!({
            "type": "close",
            "payload": { "channel_id": channel_id }
        });
        let _ = conn
            .sender
            .send(crate::agent_ws::AgentEvent::Text(close_msg.to_string()))
            .await;
    }
    Some(Ok(()))
}

/// agent 环境经隧道探测，非 agent 环境回退 Hub 直连 TCP 探测。
async fn probe_target(
    state: &crate::AppState,
    protocol: &str,
    host: &str,
    port: u16,
    username: Option<&str>,
    config_json: Option<&str>,
    environment_id: Option<&str>,
) -> Result<(), String> {
    match test_connect_via_agent(
        state,
        protocol,
        host,
        port,
        username,
        config_json,
        environment_id,
    )
    .await
    {
        Some(r) => r,
        None => direct_tcp_probe(host, port).await,
    }
}

pub async fn test_connection(
    State(state): State<crate::AppState>,
    Json(body): Json<TestConnectionRequest>,
) -> ApiResult<TestConnectionResult> {
    // For S3, log endpoint from config_json instead of empty host
    let log_host = if body.protocol == "s3" {
        body.config_json
            .as_ref()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
            .and_then(|v| v.get("endpoint")?.as_str().map(String::from))
            .unwrap_or_default()
    } else {
        body.host.clone()
    };
    tracing::info!(
        action = "TEST_CONNECTION",
        protocol = %body.protocol,
        host = %log_host,
        port = body.port.unwrap_or(0),
        "testing connection"
    );

    let start = std::time::Instant::now();
    let result = match body.protocol.as_str() {
        "ssh" | "sftp" => {
            let port = body.port.unwrap_or(22);
            probe_target(
                &state,
                &body.protocol,
                &body.host,
                port,
                body.username.as_deref(),
                body.config_json.as_deref(),
                body.environment_id.as_deref(),
            )
            .await
        }
        "redis" => {
            let port = body.port.unwrap_or(6379);
            let result = if let Some(r) = test_connect_via_agent(
                &state,
                &body.protocol,
                &body.host,
                port,
                body.username.as_deref(),
                body.config_json.as_deref(),
                body.environment_id.as_deref(),
            )
            .await
            {
                r
            } else {
                let redis_host = if body.host.contains(':') {
                    format!("[{}]", body.host)
                } else {
                    body.host.clone()
                };
                let addr = format!("redis://{}:{}/", redis_host, port);
                match tokio::time::timeout(PROBE_TIMEOUT, async {
                    let client = redis::Client::open(addr.as_str())
                        .map_err(|e| format!("redis error: {e}"))?;
                    let mut conn = client
                        .get_multiplexed_async_connection()
                        .await
                        .map_err(|e| format!("redis connect error: {e}"))?;
                    redis::Cmd::new()
                        .arg("PING")
                        .query_async::<String>(&mut conn)
                        .await
                        .map_err(|e| format!("redis PING failed: {e}"))?;
                    Ok::<(), String>(())
                })
                .await
                {
                    Ok(r) => r,
                    Err(_) => Err(PROBE_TIMEOUT_MSG.to_string()),
                }
            };
            result
        }
        "sql" => {
            // v0.73.2：统一 SQL 协议迁移后，protocol='sql' + subtype 携带方言。
            // 从 config_json 读取 subtype 路由到对应的直连检测分支。
            let subtype = body
                .config_json
                .as_ref()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                .and_then(|v| v.get("subtype").and_then(|s| s.as_str()).map(String::from))
                .or_else(|| {
                    body.config_json.as_ref().and_then(|c| {
                        let v: serde_json::Value = serde_json::from_str(c).ok()?;
                        v.get("database_type")
                            .and_then(|s| s.as_str())
                            .map(String::from)
                    })
                })
                .unwrap_or_else(|| "mysql".to_string());
            match subtype.as_str() {
                "sqlite" => {
                    // sqlite 以文件路径作为 host 传给 Agent 侧 SqliteConnector
                    // （rex_sqlite::connect 使用 ConnectRequest.host 作为 db_path）；
                    // agent-first-then-fallback，mirroring the `_` branch below.
                    let path = body
                        .config_json
                        .as_ref()
                        .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                        .and_then(|v| v.get("file_path")?.as_str().map(String::from))
                        .unwrap_or_else(|| ":memory:".into());
                    let cfg_json = {
                        let mut v: serde_json::Value = body
                            .config_json
                            .as_deref()
                            .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                            .unwrap_or_else(|| serde_json::json!({}));
                        if let serde_json::Value::Object(m) = &mut v {
                            m.insert("subtype".to_string(), serde_json::json!("sqlite"));
                        }
                        v.to_string()
                    };
                    if let Some(r) = test_connect_via_agent(
                        &state,
                        &body.protocol,
                        &path,
                        0,
                        body.username.as_deref(),
                        Some(&cfg_json),
                        body.environment_id.as_deref(),
                    )
                    .await
                    {
                        r
                    } else {
                        match rusqlite::Connection::open(&path) {
                            Ok(conn) => {
                                if conn.execute_batch("SELECT 1").is_ok() {
                                    Ok(())
                                } else {
                                    Err("SQLite query failed".into())
                                }
                            }
                            Err(e) => Err(format!("SQLite open failed: {e}")),
                        }
                    }
                }
                _ => {
                    let host = body.host.clone();
                    let port = body
                        .port
                        .unwrap_or(if subtype == "mysql" { 3306 } else { 5432 });
                    // Agent 以 config.subtype 选方言下发（agent_ws handle_connect_sql），
                    // 探测出的 subtype 需显式带入，否则回退按 protocol='sql' 解析失败。
                    let cfg_json = {
                        let mut v: serde_json::Value = body
                            .config_json
                            .as_deref()
                            .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                            .unwrap_or_else(|| serde_json::json!({}));
                        if let serde_json::Value::Object(m) = &mut v {
                            m.insert("subtype".to_string(), serde_json::json!(subtype.clone()));
                        }
                        v.to_string()
                    };
                    let result = if let Some(r) = test_connect_via_agent(
                        &state,
                        &body.protocol,
                        &host,
                        port,
                        body.username.as_deref(),
                        Some(&cfg_json),
                        body.environment_id.as_deref(),
                    )
                    .await
                    {
                        r
                    } else {
                        direct_tcp_probe(&host, port).await
                    };
                    result
                }
            }
        }
        "sqlite" => {
            // sqlite 以文件路径作为 host 传给 Agent 侧 SqliteConnector；
            // agent-first-then-fallback，mirroring the sql subtype="sqlite" arm.
            let path = body
                .config_json
                .as_ref()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                .and_then(|v| v.get("file_path")?.as_str().map(String::from))
                .unwrap_or_else(|| ":memory:".into());
            if let Some(r) = test_connect_via_agent(
                &state,
                "sqlite",
                &path,
                0,
                body.username.as_deref(),
                body.config_json.as_deref(),
                body.environment_id.as_deref(),
            )
            .await
            {
                r
            } else {
                match rusqlite::Connection::open(&path) {
                    Ok(conn) => {
                        if conn.execute_batch("SELECT 1").is_ok() {
                            Ok(())
                        } else {
                            Err("SQLite query failed".into())
                        }
                    }
                    Err(e) => Err(format!("SQLite open failed: {e}")),
                }
            }
        }
        "mysql" | "postgresql" => {
            let port = body
                .port
                .unwrap_or(if body.protocol == "mysql" { 3306 } else { 5432 });
            probe_target(
                &state,
                &body.protocol,
                &body.host,
                port,
                body.username.as_deref(),
                body.config_json.as_deref(),
                body.environment_id.as_deref(),
            )
            .await
        }
        "s3" => {
            // agent-first-then-fallback：agent-mode 时路由探测到 Agent 侧
            // `S3Connector::verify`（真正验凭据），直连模式回退 Hub-local list_buckets。
            let result = if let Some(r) = test_connect_via_agent(
                &state,
                "s3",
                &body.host,
                body.port.unwrap_or(0),
                body.username.as_deref(),
                body.config_json.as_deref(),
                body.environment_id.as_deref(),
            )
            .await
            {
                r
            } else {
                match body.config_json {
                    Some(ref cfg) => {
                        let v: serde_json::Value =
                            serde_json::from_str(cfg).unwrap_or(serde_json::Value::Null);
                        let endpoint = v.get("endpoint").and_then(|e| e.as_str()).unwrap_or("");
                        let access_key = v.get("access_key").and_then(|e| e.as_str()).unwrap_or("");
                        let secret_key = v.get("secret_key").and_then(|e| e.as_str()).unwrap_or("");
                        let region = v
                            .get("region")
                            .and_then(|e| e.as_str())
                            .unwrap_or("us-east-1");
                        if endpoint.is_empty() || access_key.is_empty() || secret_key.is_empty() {
                            Err("missing endpoint, access_key, or secret_key".into())
                        } else {
                            let config = aws_sdk_s3::Config::builder()
                                .endpoint_url(endpoint)
                                .region(aws_sdk_s3::config::Region::new(region.to_string()))
                                .credentials_provider(aws_sdk_s3::config::Credentials::new(
                                    access_key.to_string(),
                                    secret_key.to_string(),
                                    None,
                                    None,
                                    "rex-hub-test",
                                ))
                                .behavior_version_latest()
                                .build();
                            let client = aws_sdk_s3::Client::from_conf(config);
                            match tokio::time::timeout(PROBE_TIMEOUT, client.list_buckets().send())
                                .await
                            {
                                Ok(Ok(_)) => Ok(()),
                                Ok(Err(e)) => Err(format!("S3 ListBuckets failed: {e}")),
                                Err(_) => Err("S3 request timed out".into()),
                            }
                        }
                    }
                    None => Err("missing config_json for S3".into()),
                }
            };
            result
        }
        "sip" => match &body.config_json {
            None => Err("missing config_json for SIP".into()),
            Some(cfg) => match serde_json::from_str::<serde_json::Value>(cfg) {
                Err(e) => Err(format!("invalid config_json: {e}")),
                Ok(value) => {
                    // SIP 的 server/port 完全下沉到账户层，load_sip_conn 不读取
                    // 顶层 host/port/username（子任务 #1 已移除回退），故 info 仅带
                    // config。先用与信令注册一致的 load_sip_conn 校验 SipProfile
                    // 能选出生效账户；匿名注册（无 password）是合法的。
                    let info = crate::resource_conn::ResourceConnInfo {
                        resource_id: String::new(),
                        name: String::new(),
                        protocol: "sip".into(),
                        host: String::new(),
                        port: None,
                        username: String::new(),
                        config: value,
                        subtype: None,
                        use_agent: false,
                        agent_id: None,
                    };
                    match crate::resource_conn::load_sip_conn(&info) {
                        Err(e) => Err(format!("invalid SIP config: {e}")),
                        Ok(sip_cfg) => {
                            // agent-first-then-fallback：agent-mode 时把生效账户的
                            // 平坦 SipConfig 下发到 Agent，由 UA₂ 做真实 REGISTER
                            //（验凭据可达内网 SIP server）；直连模式回退 Hub 本地
                            // 仅做配置校验（真正的信令拨测联调见 /ws/sip）。
                            let flat = serde_json::json!({
                                "server": sip_cfg.server,
                                "port": sip_cfg.port,
                                "username": sip_cfg.username,
                                "password": sip_cfg.password,
                                "displayName": sip_cfg.display_name,
                                "transport": sip_cfg.transport.as_str(),
                            });
                            if let Some(r) = test_connect_via_agent(
                                &state,
                                "sip",
                                "",
                                0,
                                Some(&sip_cfg.username),
                                Some(&flat.to_string()),
                                body.environment_id.as_deref(),
                            )
                            .await
                            {
                                r
                            } else {
                                Ok(())
                            }
                        }
                    }
                }
            },
        },
        _ => Err(format!("unsupported protocol: {}", body.protocol)),
    };
    let latency = start.elapsed().as_millis() as u64;
    let ok = result.is_ok();
    let err_msg = match &result {
        Ok(()) => None,
        Err(e) => Some(e.clone()),
    };

    tracing::info!(
        action = "TEST_CONNECTION",
        protocol = %body.protocol,
        host = %body.host,
        ok = ok,
        latency_ms = latency,
        error = err_msg.as_deref().unwrap_or(""),
        "connection test completed"
    );

    match result {
        Ok(()) => Ok(Json(TestConnectionResult {
            ok: true,
            latency_ms: Some(latency),
            error: None,
        })),
        Err(e) => Ok(Json(TestConnectionResult {
            ok: false,
            latency_ms: Some(latency),
            error: Some(e),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CR15 回归：agent 模式测试连接的 connect config 必须透出顶层 username，
    /// 空值归一为 `root`（与 `file_api::agent_file_config` 同源），
    /// 否则 Agent 以空用户名认证，测试连接必被拒。
    #[test]
    fn agent_test_config_normalizes_empty_username() {
        let cfg = agent_test_connect_config("10.0.0.1", 22, Some(""), Some(r#"{"password":"pw"}"#));
        assert_eq!(
            cfg.get("username").and_then(|v| v.as_str()),
            Some(crate::resource_conn::normalize_username("").as_str()),
            "test connection and file entries must share one normalization source"
        );
        assert_eq!(cfg.get("host").and_then(|v| v.as_str()), Some("10.0.0.1"));
        assert_eq!(cfg.get("port").and_then(|v| v.as_u64()), Some(22));
        assert_eq!(cfg.get("password").and_then(|v| v.as_str()), Some("pw"));
    }

    /// 顶层 username 为权威：显式值保留，不被 config_json 中的历史键覆盖；
    /// 缺失字段（`None`）同样按空值归一。
    #[test]
    fn agent_test_config_keeps_explicit_username() {
        let cfg = agent_test_connect_config(
            "10.0.0.1",
            22,
            Some("alice"),
            Some(r#"{"username":"stale"}"#),
        );
        assert_eq!(cfg.get("username").and_then(|v| v.as_str()), Some("alice"));

        let cfg = agent_test_connect_config("10.0.0.1", 22, None, None);
        assert_eq!(cfg.get("username").and_then(|v| v.as_str()), Some("root"));
    }

    /// env_uses_agent 判定 agent 转发口径：仅 `connection_mode == "agent"` 为真，
    /// direct 环境与环境缺失都回退 Hub 直连。
    #[tokio::test]
    async fn env_uses_agent_only_for_agent_mode() {
        let (_dir, state) = crate::testutil::make_state();
        let direct = state
            .db
            .create_environment(&crate::models::NewEnvironment {
                name: "direct-env".into(),
                description: None,
                connection_mode: Some("direct".into()),
            })
            .unwrap();
        let agent = state
            .db
            .create_environment(&crate::models::NewEnvironment {
                name: "agent-env".into(),
                description: None,
                connection_mode: Some("agent".into()),
            })
            .unwrap();

        assert!(
            !env_uses_agent(&state, None).await,
            "no environment means direct connect"
        );
        assert!(!env_uses_agent(&state, Some("env_missing")).await);
        assert!(!env_uses_agent(&state, Some(&direct.id)).await);
        assert!(env_uses_agent(&state, Some(&agent.id)).await);
    }

    /// Agent 侧 sqlx 会把连接串回显进错误文案；该文案既进用户可见 toast 又进
    /// 日志，凭据必须被 redact（password / 私钥两种键名都算）。
    #[test]
    fn agent_error_is_redacted_for_both_secret_fields() {
        let cfg = agent_test_connect_config(
            "10.0.0.1",
            3306,
            Some("ops"),
            Some(r#"{"password":"s3cr3t-pw","private_key":"PEM-SECRET"}"#),
        );

        let pw = redact_agent_error(
            "SQL connection failed: mysql://ops:s3cr3t-pw@10.0.0.1",
            &cfg,
        );
        assert!(
            !pw.contains("s3cr3t-pw"),
            "password must not reach response or log: {pw}"
        );
        assert!(pw.contains("mysql://ops:***@10.0.0.1"));

        let key = redact_agent_error("failed to decode private key PEM-SECRET", &cfg);
        assert!(!key.contains("PEM-SECRET"));

        // 无凭据时原文原样返回（空密码不应把整条文案抹成 ***）
        let bare = agent_test_connect_config("10.0.0.1", 22, Some("ops"), None);
        assert_eq!(
            redact_agent_error("Connection refused", &bare),
            "Connection refused"
        );
    }
}
