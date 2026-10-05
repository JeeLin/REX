//! 文件管理 REST 路由。

use std::collections::HashMap;
use std::sync::Arc;

use crate::db::audit_log;
use crate::models::{NewSyncTask, NewTransferTask};
use crate::resource_conn::{load_resource_config, normalize_username, ResourceConnInfo};
use crate::sync_coordinator::SyncCoordinator;
use crate::transfer_coordinator::TransferOp;
use crate::AppState;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use base64::Engine;
use rex_common::file_transfer::{
    FileCapabilitySet, FileConnectRequest, FileConnector, UnsupportedProtocolError,
};
use rex_common::resource_config::config_private_key;
use rex_transfer::ConflictPolicy;
use serde::{Deserialize, Serialize};
use std::fmt;
use tokio::sync::Mutex;

use crate::error::{error_with_status, ErrorBody};

pub type FileState = Arc<Mutex<FileConnectionPool>>;

pub struct FileConnectionPool {
    connectors: HashMap<String, Box<dyn FileConnector>>,
}

impl Default for FileConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl FileConnectionPool {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }
    pub fn insert(&mut self, id: String, conn: Box<dyn FileConnector>) {
        self.connectors.insert(id, conn);
    }
    pub fn remove(&mut self, id: &str) -> Option<Box<dyn FileConnector>> {
        self.connectors.remove(id)
    }
}

pub fn file_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/connect", axum::routing::post(connect))
        .route("/disconnect", axum::routing::post(disconnect))
        .route("/list", axum::routing::get(list))
        .route("/upload", axum::routing::post(upload))
        .route("/download", axum::routing::get(download))
        .route("/delete", axum::routing::post(delete))
        .route("/rename", axum::routing::post(rename))
        .route("/mkdir", axum::routing::post(mkdir))
        .route("/chmod", axum::routing::post(chmod))
        .route("/presigned-url", axum::routing::post(presigned_url))
        .route("/acl", axum::routing::get(get_acl).put(put_acl))
        .route(
            "/connector/{resource_id}/capability",
            axum::routing::get(connector_capability),
        )
        .route("/read-for-edit", axum::routing::get(read_for_edit))
        .route("/save-from-edit", axum::routing::post(save_from_edit))
        .route(
            "/transfer",
            axum::routing::get(list_transfer_tasks).post(create_transfer_task),
        )
        .route("/transfer/action", axum::routing::post(transfer_action))
        .route("/transfer/{id}", axum::routing::get(get_transfer_task))
        .route(
            "/transfer/{id}/cancel",
            axum::routing::post(cancel_transfer_task),
        )
        // v0.92.0：目录同步任务（create/preview/get/cancel）
        // 静态路径必须先于 `/sync/{id}` 注册，避免 `preview` 被当作任务 id。
        .route("/sync/preview", axum::routing::post(preview_sync))
        .route("/sync", axum::routing::post(create_sync_task))
        .route(
            "/sync/{id}",
            axum::routing::get(get_sync_task).delete(cancel_sync_task),
        )
}

// ---------------------------------------------------------------------------
// Transfer task API (v0.91.0, T1)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TransferListQuery {
    limit: Option<u64>,
    offset: Option<u64>,
}

async fn list_transfer_tasks(
    State(state): State<AppState>,
    Query(q): Query<TransferListQuery>,
) -> axum::response::Response {
    let limit = q.limit.unwrap_or(50).min(200);
    let offset = q.offset.unwrap_or(0);
    match state.db.list_transfer_tasks(limit, offset) {
        Ok(items) => (StatusCode::OK, Json(items)).into_response(),
        Err(e) => error_response("TRANSFER_LIST_FAILED", &e.to_string()).into_response(),
    }
}

/// 取任务记录并序列化为响应（transfer / sync 共用；未命中回 404）。
async fn task_response(state: &AppState, task_id: &str) -> axum::response::Response {
    match state.db.get_transfer_task(task_id) {
        Ok(Some(item)) => (StatusCode::OK, Json(item)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "transfer task not found" })),
        )
            .into_response(),
        Err(e) => error_response("TRANSFER_GET_FAILED", &e.to_string()).into_response(),
    }
}

async fn get_transfer_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> axum::response::Response {
    task_response(&state, &task_id).await
}

async fn create_transfer_task(
    State(state): State<AppState>,
    Json(body): Json<NewTransferTask>,
) -> axum::response::Response {
    let task_id = match state.db.create_transfer_task(&body) {
        Ok(id) => id,
        Err(e) => return error_response("TRANSFER_CREATE_FAILED", &e.to_string()).into_response(),
    };
    tracing::info!(
        action = "FILE_TRANSFER_CREATED",
        transfer_task_id = %task_id,
        source_resource_id = %body.source_resource_id,
        target_resource_id = %body.target_resource_id,
        "transfer task created"
    );
    audit_log(
        &state.db,
        "FILE_TRANSFER_CREATED",
        "success",
        Some(task_id.clone()),
    );
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": task_id, "status": "pending" })),
    )
        .into_response()
}

/// 取消任务（transfer / sync 共用）：持久化取消标记 → 中止进程内后台流。
async fn cancel_task(
    state: &AppState,
    task_id: &str,
    audit_action: &'static str,
) -> axum::response::Response {
    // T1：持久化取消标记；T2 驱动引擎实际中止。
    match state.db.set_transfer_task_status(task_id, "canceled", None) {
        Ok(_) => {
            // T2：持久化取消标记后，中止进程内的后台传输流。run_stream 亦轮询 DB
            // 状态作为协作式中止的兜底。
            state.coordinator.abort(task_id);
            // v0.92.0：同步引擎同样轮询 DB 取消标记，这里一并中止后台任务。
            state.sync_coordinator.abort(task_id);
            tracing::info!(
                action = audit_action,
                transfer_task_id = %task_id,
                "transfer task canceled"
            );
            audit_log(
                &state.db,
                audit_action,
                "success",
                Some(task_id.to_string()),
            );
            (
                StatusCode::OK,
                Json(serde_json::json!({ "id": task_id, "status": "canceled" })),
            )
                .into_response()
        }
        Err(e) => error_response("TRANSFER_CANCEL_FAILED", &e.to_string()).into_response(),
    }
}

async fn cancel_transfer_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> axum::response::Response {
    cancel_task(&state, &task_id, "FILE_TRANSFER_CANCELED").await
}

// ---------------------------------------------------------------------------
// Sync task API (v0.92.0, 子任务 1)
// ---------------------------------------------------------------------------

/// `POST /api/files/sync` 与 `POST /api/files/sync/preview` 共用请求体：
/// 源/目标端点 + 同步选项 + 可选冲突策略。
#[derive(Debug, Clone, Deserialize)]
pub struct SyncRequestBody {
    pub source: TransferEndpointRef,
    pub target: TransferEndpointRef,
    pub options: rex_transfer::SyncOptions,
    /// 冲突策略：overwrite|skip|rename|fail（缺省 overwrite），沿用 v0.91.0 语义。
    #[serde(default)]
    pub conflict: Option<String>,
}

/// 校验同步请求参数（create 与 preview 共用同一份逻辑）：冲突策略合法 +
/// 源/目标路径非空。返回归一后的冲突策略（缺省 `overwrite`）。
///
/// 预览与执行必须对同一组选项给出一致的判定，否则预览承诺会与实际落盘不符。
fn validate_sync_request(body: &SyncRequestBody) -> Result<String, (&'static str, &'static str)> {
    let conflict = body
        .conflict
        .clone()
        .unwrap_or_else(|| "overwrite".to_string());
    if ConflictPolicy::from_str(&conflict).is_none() {
        return Err(("INVALID_CONFLICT_POLICY", "invalid conflict policy"));
    }
    if body.source.path.is_empty() || body.target.path.is_empty() {
        return Err(("SYNC_PATH_REQUIRED", "source and target path are required"));
    }
    Ok(conflict)
}

/// `POST /api/files/sync/preview`（dry-run）：开两侧连接器 → 扫描 → `diff` →
/// 返回 `SyncPlan`。**不落盘、不入库、不推进任务状态机**（无任务即无 running）。
///
/// 计划与真实执行同源（同一扫描 + 同一 `diff`），因此「预览即所得」；扫描 /
/// 连接失败只回错误码，不落任何副作用。
async fn preview_sync(
    State(state): State<AppState>,
    Json(body): Json<SyncRequestBody>,
) -> axum::response::Response {
    if let Err((code, message)) = validate_sync_request(&body) {
        return error_response(code, message).into_response();
    }
    let mut source = match connect_resource(&state, &body.source.resource_id).await {
        Ok(c) => c,
        Err(e) => {
            return error_response("SYNC_PREVIEW_CONNECT_FAILED", &e.to_string()).into_response()
        }
    };
    let mut target = match connect_resource(&state, &body.target.resource_id).await {
        Ok(c) => c,
        Err(e) => {
            return error_response("SYNC_PREVIEW_CONNECT_FAILED", &e.to_string()).into_response()
        }
    };

    let plan = SyncCoordinator::preview_plan(
        &mut *source,
        &mut *target,
        &body.source.path,
        &body.target.path,
        &body.options,
    )
    .await;
    let _ = source.close().await;
    let _ = target.close().await;

    match plan {
        Ok(plan) => (StatusCode::OK, Json(plan)).into_response(),
        Err(e) => error_response("SYNC_PREVIEW_FAILED", &e.to_string()).into_response(),
    }
}

/// 创建同步任务：校验参数 → 入库（kind=sync）→ 审计 → 返回 201 pending。
///
/// 同步 diff/apply 全部在服务端完成，浏览器只负责创建任务与查看进度。
async fn create_sync_task(
    State(state): State<AppState>,
    Json(body): Json<SyncRequestBody>,
) -> axum::response::Response {
    let conflict = match validate_sync_request(&body) {
        Ok(c) => c,
        Err((code, message)) => return error_response(code, message).into_response(),
    };
    let sync_options = match serde_json::to_string(&body.options) {
        Ok(s) => s,
        Err(e) => return error_response("INVALID_SYNC_OPTIONS", &e.to_string()).into_response(),
    };
    let new_task = NewSyncTask {
        source_resource_id: body.source.resource_id.clone(),
        target_resource_id: body.target.resource_id.clone(),
        source_path: body.source.path.clone(),
        target_path: body.target.path.clone(),
        sync_options,
        conflict_policy: Some(conflict),
    };
    let task_id = match state.db.create_sync_task(&new_task) {
        Ok(id) => id,
        Err(e) => return error_response("SYNC_CREATE_FAILED", &e.to_string()).into_response(),
    };
    tracing::info!(
        action = "FILE_SYNC_CREATED",
        transfer_task_id = %task_id,
        source_resource_id = %body.source.resource_id,
        target_resource_id = %body.target.resource_id,
        "sync task created"
    );
    audit_log(
        &state.db,
        "FILE_SYNC_CREATED",
        "success",
        Some(task_id.clone()),
    );
    // 子任务 2：提交同步引擎（scanning → planning → running → verifying）。
    // 文件字节仅在服务端 source/target 连接器之间搬运，不经过浏览器。
    state
        .sync_coordinator
        .submit(state.clone(), task_id.clone());
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": task_id, "status": "pending" })),
    )
        .into_response()
}

async fn get_sync_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> axum::response::Response {
    task_response(&state, &task_id).await
}

async fn cancel_sync_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> axum::response::Response {
    cancel_task(&state, &task_id, "FILE_SYNC_CANCELED").await
}

/// 传输端点引用（resource + path）。
#[derive(Debug, Clone, Deserialize)]
pub struct TransferEndpointRef {
    pub resource_id: String,
    pub path: String,
}
/// POST `/transfer/action` 请求体：op（copy/move）+ 源/目标端点 + 冲突策略。
#[derive(Debug, Deserialize)]
pub struct TransferActionRequest {
    pub op: TransferOp,
    pub src: TransferEndpointRef,
    pub dst: TransferEndpointRef,
    /// 冲突策略：overwrite|skip|rename|fail（缺省 overwrite）。
    #[serde(default)]
    pub conflict: Option<String>,
}

/// T2 直连传输入口：创建任务 → 审计 → 提交后台传输流 → 立即返回 201 pending。
///
/// 文件数据在服务端 source-connector → target-connector 之间流式传输，不经过浏览器。
async fn transfer_action(
    State(state): State<AppState>,
    Json(body): Json<TransferActionRequest>,
) -> axum::response::Response {
    let conflict = body
        .conflict
        .clone()
        .unwrap_or_else(|| "overwrite".to_string());
    if ConflictPolicy::from_str(&conflict).is_none() {
        return error_response("INVALID_CONFLICT_POLICY", "invalid conflict policy")
            .into_response();
    }
    let new_task = NewTransferTask {
        source_resource_id: body.src.resource_id.clone(),
        target_resource_id: body.dst.resource_id.clone(),
        source_path: body.src.path.clone(),
        target_path: body.dst.path.clone(),
        conflict_policy: Some(conflict),
    };
    let task_id = match state.db.create_transfer_task(&new_task) {
        Ok(id) => id,
        Err(e) => return error_response("TRANSFER_CREATE_FAILED", &e.to_string()).into_response(),
    };
    tracing::info!(
        action = "FILE_TRANSFER_CREATED",
        transfer_task_id = %task_id,
        source_resource_id = %body.src.resource_id,
        target_resource_id = %body.dst.resource_id,
        "transfer task created"
    );
    audit_log(
        &state.db,
        "FILE_TRANSFER_CREATED",
        "success",
        Some(task_id.clone()),
    );
    // T2：提交后台传输流（打开两个连接器并流式传输）。
    state
        .coordinator
        .submit(state.clone(), task_id.clone(), body.op);
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": task_id, "status": "pending" })),
    )
        .into_response()
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
struct PathQuery {
    session_id: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct DeleteBody {
    session_id: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct RenameBody {
    session_id: String,
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct MkdirBody {
    session_id: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct ChmodBody {
    session_id: String,
    path: String,
    mode: String,
}

#[derive(Debug, Deserialize)]
struct SaveFromEditBody {
    session_id: String,
    path: String,
    content: String, // base64 encoded
}

fn error_response(code: &str, message: &str) -> (StatusCode, Json<ErrorBody>) {
    error_with_status(StatusCode::BAD_REQUEST, code, message)
}

/// S3-only 操作的统一错误映射：`UnsupportedProtocolError` 保持历史
/// `UNSUPPORTED_PROTOCOL` 错误码 + 原文案，其余错误走各自业务码。
///
/// 取代原先的 `downcast::<S3Connector>` 失败分支（v0.91.0 T3）。
fn connector_op_error(code: &str, e: &anyhow::Error) -> (StatusCode, Json<ErrorBody>) {
    match e.downcast_ref::<UnsupportedProtocolError>() {
        Some(u) => error_response("UNSUPPORTED_PROTOCOL", &u.message),
        None => error_response(code, &e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// 能力查询 API（v0.91.0 T3）
// ---------------------------------------------------------------------------

/// 能力查询响应：`{"protocol": "s3", "capabilities": {...}}`。
///
/// `protocol` 供前端替代 `isS3` 判断（T6）。
#[derive(Debug, Serialize)]
pub struct ConnectorCapabilityResponse {
    pub protocol: String,
    pub capabilities: FileCapabilitySet,
}

/// 协议 → 静态能力集：**不建立连接**，只按资源配置推导。
///
/// agent 与直连同源 —— `AgentFileProxy::capability` 也走这里，因此 agent 模式
/// 下 S3 专属能力不再是恒 false（修复 downcast 根因）。
///
/// - `s3`：presigned URL / ACL / multipart 为 true；chmod 无实现保持 false。
/// - `sftp` / `ssh`：chmod 为 true（SFTP 实现，T4）；其余 S3 专属能力保持 false。
/// - 其它协议：全 false。
pub fn capabilities_for_protocol(protocol: &str) -> FileCapabilitySet {
    match protocol {
        "s3" => FileCapabilitySet {
            chmod: false,
            presigned_url: true,
            acl: true,
            multipart: true,
        },
        "sftp" | "ssh" => FileCapabilitySet {
            chmod: true,
            presigned_url: false,
            acl: false,
            multipart: false,
        },
        _ => FileCapabilitySet::default(),
    }
}

/// 只读能力查询：读资源配置 → protocol → 静态能力集，全程不打开连接。
///
/// 错误：`(状态码, 错误码, 文案)` —— 资源不存在回 404。
fn load_connector_capability(
    state: &AppState,
    resource_id: &str,
) -> Result<ConnectorCapabilityResponse, (StatusCode, &'static str, String)> {
    let res = load_resource_config(state, resource_id).map_err(|e| {
        if e.starts_with("resource not found") {
            (StatusCode::NOT_FOUND, "RESOURCE_NOT_FOUND", e)
        } else {
            (StatusCode::BAD_REQUEST, "INVALID_RESOURCE", e)
        }
    })?;
    Ok(ConnectorCapabilityResponse {
        protocol: res.protocol.clone(),
        capabilities: capabilities_for_protocol(&res.protocol),
    })
}

/// GET `/connector/{resource_id}/capability`。
async fn connector_capability(
    State(state): State<AppState>,
    Path(resource_id): Path<String>,
) -> axum::response::Response {
    tracing::debug!(action = "FILE_CONNECTOR_CAPABILITY", resource_id = %resource_id, "file connector capability");
    match load_connector_capability(&state, &resource_id) {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err((status, code, msg)) => error_with_status(status, code, &msg).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Build the file connection config sent to the Agent (host/port/username +
/// merged config_json).
///
/// `username` must be sent and normalized with the same source as the direct
/// leg (empty → `root`, [`normalize_username`]): the agent side `agent_file.rs`
/// falls back with `unwrap_or("")`, so an empty value authenticates as an empty
/// SSH user (same origin as the terminal config in `agent_ssh.rs`).
fn agent_file_config(res: &ResourceConnInfo) -> serde_json::Value {
    let mut cfg = serde_json::json!({
        "host": res.host,
        "port": res.port.unwrap_or(22),
    });
    if let serde_json::Value::Object(m) = res.config.clone() {
        for (k, v) in m {
            cfg[k] = v;
        }
    }
    if let serde_json::Value::Object(m) = &mut cfg {
        // 资源顶层 username 为权威字段，不被 config_json 中的历史键覆盖
        m.insert(
            "username".to_string(),
            normalize_username(&res.username).into(),
        );
    }
    cfg
}

/// 直连腿的 SSH/SFTP 连接参数。
///
/// `username` 走 SSH/SFTP 专属归一（空 → `root`，[`normalize_username`]），
/// 与 `terminal_ws::load_resource_conn` 同源，保证连接池键 `user@host:port`
/// 不分叉；其他协议不经过这里，空 username 保持原值。
fn ssh_connect_config(res: &ResourceConnInfo) -> rex_ssh::SshConfig {
    rex_ssh::SshConfig {
        host: res.host.clone(),
        port: res.port.unwrap_or(22),
        username: normalize_username(&res.username),
        password: res
            .config
            .get("password")
            .and_then(|v| v.as_str())
            .map(String::from),
        private_key: config_private_key(&res.config),
        keepalive_interval: res
            .config
            .get("keepalive_interval")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        init_script: res
            .config
            .get("initScript")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(String::from),
    }
}

/// 连接资源过程中可能产生的错误（供 `connect` 与 `connect_resource` 共用分类）。
#[derive(Debug)]
pub enum ConnectError {
    AgentUnavailable,
    AgentConnect(String),
    SftpConnect(String),
    S3Connect(String),
    UnsupportedProtocol,
    InvalidResource(String),
}

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AgentUnavailable => write!(f, "no online agent for environment"),
            Self::AgentConnect(e) => write!(f, "agent connect failed: {e}"),
            Self::SftpConnect(e) => write!(f, "failed to connect to SFTP server: {e}"),
            Self::S3Connect(e) => write!(f, "failed to connect to S3 storage: {e}"),
            Self::UnsupportedProtocol => write!(f, "unsupported protocol"),
            Self::InvalidResource(e) => write!(f, "invalid resource: {e}"),
        }
    }
}

/// 将 [`ConnectError`] 映射为与历史 `connect` 相同的 HTTP 错误码 + 审计行为：
/// - SFTP/S3 连接失败：写 `FILE_CONNECT` failure 审计 + `connect_error_response`（自动分类根因码）。
/// - Agent/协议错误：不写审计，返回对应错误码。
fn connect_failure_response(
    state: &AppState,
    resource_id: &str,
    res: &ResourceConnInfo,
    e: ConnectError,
) -> axum::response::Response {
    match e {
        ConnectError::AgentUnavailable => {
            error_response("AGENT_UNAVAILABLE", "no online agent for environment").into_response()
        }
        ConnectError::AgentConnect(msg) => {
            error_response("AGENT_CONNECT_FAILED", &msg).into_response()
        }
        ConnectError::SftpConnect(msg) => {
            tracing::error!(action = "FILE_CONNECT", resource_id = %resource_id, resource_name = %res.name, protocol = %res.protocol, error = %msg, "SFTP connection failed");
            audit_log(
                &state.db,
                "FILE_CONNECT",
                "failure",
                Some(resource_id.to_string()),
            );
            crate::error::connect_error_response("failed to connect to SFTP server", msg)
                .into_response()
        }
        ConnectError::S3Connect(msg) => {
            audit_log(
                &state.db,
                "FILE_CONNECT",
                "failure",
                Some(resource_id.to_string()),
            );
            crate::error::connect_error_response("failed to connect to S3 storage", msg)
                .into_response()
        }
        ConnectError::UnsupportedProtocol => {
            error_response("UNSUPPORTED_PROTOCOL", "unsupported protocol").into_response()
        }
        ConnectError::InvalidResource(msg) => {
            error_response("INVALID_RESOURCE", &msg).into_response()
        }
    }
}

/// 从 `ResourceConnInfo` 构造 S3 连接请求（bucket/region/endpoint/keys 从 config 提取）。
fn s3_connect_request(res: &ResourceConnInfo) -> FileConnectRequest {
    FileConnectRequest {
        protocol: "s3".to_string(),
        host: res.host.clone(),
        port: res.port.unwrap_or(443),
        username: None,
        password: None,
        private_key: None,
        keepalive_interval: None,
        bucket: res
            .config
            .get("bucket")
            .and_then(|v| v.as_str())
            .map(String::from),
        region: res
            .config
            .get("region")
            .and_then(|v| v.as_str())
            .map(String::from),
        endpoint: res
            .config
            .get("endpoint")
            .and_then(|v| v.as_str())
            .map(String::from),
        access_key: res
            .config
            .get("access_key")
            .and_then(|v| v.as_str())
            .map(String::from),
        secret_key: res
            .config
            .get("secret_key")
            .and_then(|v| v.as_str())
            .map(String::from),
    }
}

/// 依据 `ResourceConnInfo` 打开对应的 `FileConnector`（agent / sftp / s3）。
///
/// `connect` 与 T2 的 [`connect_resource`] 共用此函数，保证两条入口的协议分发一致。
async fn build_connector(
    state: &AppState,
    res: &ResourceConnInfo,
    resource_id: &str,
) -> Result<Box<dyn FileConnector>, ConnectError> {
    if res.use_agent {
        let agent_id = res.agent_id.clone().ok_or(ConnectError::AgentUnavailable)?;
        let cfg = agent_file_config(res);
        let channel_id =
            crate::agent_ws::open_agent_session(state, &agent_id, resource_id, &res.protocol, cfg)
                .await
                .map_err(|e| ConnectError::AgentConnect(e.to_string()))?;
        return Ok(Box::new(crate::agent_proxy::AgentFileProxy::new(
            state.clone(),
            channel_id,
            res.protocol.clone(),
        )));
    }
    match res.protocol.as_str() {
        "sftp" | "ssh" => {
            let conn = rex_ssh::sftp::SftpConnector::connect_with_config(ssh_connect_config(res))
                .await
                .map_err(|e| ConnectError::SftpConnect(e.to_string()))?;
            Ok(Box::new(conn))
        }
        "s3" => {
            let conn = rex_s3::S3Connector::connect_from_request(&s3_connect_request(res))
                .await
                .map_err(|e| ConnectError::S3Connect(e.to_string()))?;
            Ok(Box::new(conn))
        }
        _ => Err(ConnectError::UnsupportedProtocol),
    }
}

/// 打开一个资源对应的文件连接器（agent + sftp/ssh + s3 分发）。
///
/// T2 传输协调器用它在服务端打开 source/target 两个连接器；与 `connect`
/// 共享 [`build_connector`]，保证协议分发与错误分类一致。
pub async fn connect_resource(
    state: &AppState,
    resource_id: &str,
) -> Result<Box<dyn FileConnector>, ConnectError> {
    let res = load_resource_config(state, resource_id).map_err(ConnectError::InvalidResource)?;
    build_connector(state, &res, resource_id).await
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

    if !res.use_agent {
        tracing::info!(action = "FILE_CONNECT", resource_id = %body.resource_id, resource_name = %res.name, protocol = %res.protocol, use_agent = res.use_agent, "file connect request");
        if !res.config.is_null() {
            tracing::debug!(action = "FILE_CONNECT", resource_id = %body.resource_id, has_password = res.config.get("password").is_some(), has_private_key = config_private_key(&res.config).is_some(), "resource config loaded");
        }
    }

    let conn = match build_connector(&state, &res, &body.resource_id).await {
        Ok(c) => c,
        Err(e) => {
            return connect_failure_response(&state, &body.resource_id, &res, e).into_response()
        }
    };

    let session_id = format!("file_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    state
        .file_pool
        .lock()
        .await
        .insert(session_id.clone(), conn);
    if res.use_agent {
        let agent_id = res.agent_id.clone().unwrap_or_default();
        tracing::info!(action = "FILE_CONNECT_AGENT", session_id = %session_id, resource_id = %body.resource_id, resource_name = %res.name, agent_id = %agent_id, protocol = %res.protocol, "file connected via agent");
    } else {
        tracing::info!(
            action = "FILE_CONNECT",
            session_id = %session_id,
            resource_id = %body.resource_id,
            protocol = %res.protocol,
            "file session connected"
        );
        let audit_db = state.db.clone();
        let audit_target = body.resource_id.clone();
        let _ = tokio::task::spawn_blocking(move || {
            audit_db.write_audit_log(&crate::models::NewAuditEntry {
                action: "FILE_CONNECT".into(),
                target: Some(audit_target),
                result: "success".into(),
                ..Default::default()
            })
        })
        .await;
    }
    (StatusCode::OK, Json(ConnectResponse { session_id })).into_response()
}

async fn disconnect(
    State(state): State<AppState>,
    Json(body): Json<DisconnectBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    if let Some(mut conn) = pool.remove(&body.session_id) {
        let _ = conn.close().await;
        tracing::info!(
            action = "FILE_DISCONNECT",
            session_id = %body.session_id,
            "file session disconnected"
        );
        let audit_db = state.db.clone();
        let session_id = body.session_id.clone();
        let _ = tokio::task::spawn_blocking(move || {
            audit_db.write_audit_log(&crate::models::NewAuditEntry {
                action: "FILE_DISCONNECT".into(),
                target: Some(session_id),
                result: "success".into(),
                ..Default::default()
            })
        })
        .await;
        (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
    } else {
        error_response("SESSION_NOT_FOUND", "session not found").into_response()
    }
}

async fn list(
    State(state): State<AppState>,
    Query(params): Query<PathQuery>,
) -> axum::response::Response {
    tracing::debug!(action = "FILE_LIST", session_id = %params.session_id, path = %params.path, "file list");
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.list(&params.path).await {
        Ok(entries) => (StatusCode::OK, Json(entries)).into_response(),
        Err(e) => error_response("LIST_FAILED", &e.to_string()).into_response(),
    }
}

async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> axum::response::Response {
    let mut session_id = String::new();
    let mut remote_path = String::new();
    let mut file_data: Option<Vec<u8>> = None;
    let mut offset: u64 = 0;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "session_id" => {
                session_id =
                    String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string();
            }
            "path" => {
                remote_path =
                    String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string();
            }
            "offset" => {
                if let Ok(v) = String::from_utf8_lossy(&field.bytes().await.unwrap_or_default())
                    .to_string()
                    .parse::<u64>()
                {
                    offset = v;
                }
            }
            "file" => {
                file_data = Some(field.bytes().await.unwrap_or_default().to_vec());
            }
            _ => {}
        }
    }

    let data = match file_data {
        Some(d) => d,
        None => return error_response("MISSING_FILE", "no file uploaded").into_response(),
    };

    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    audit_log(
        &state.db,
        "FILE_TRANSFER_START",
        "success",
        Some(remote_path.clone()),
    );
    tracing::info!(action = "TRANSFER_START", op = "upload", path = %remote_path, session_id = %session_id, "upload starting");
    match conn.upload(&remote_path, data, offset, None).await {
        Ok(result) => {
            tracing::info!(
                action = "FILE_OP",
                op = "upload",
                path = %remote_path,
                session_id = %session_id,
                "file uploaded"
            );
            audit_log(
                &state.db,
                "FILE_TRANSFER_COMPLETE",
                "success",
                Some(remote_path.clone()),
            );
            tracing::info!(action = "TRANSFER_COMPLETE", op = "upload", path = %remote_path, session_id = %session_id, "upload complete");
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "upload_id": result.upload_id
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(action = "TRANSFER_FAIL", op = "upload", path = %remote_path, session_id = %session_id, error = %e, "upload failed");
            audit_log(
                &state.db,
                "FILE_TRANSFER_FAILED",
                "failure",
                Some(remote_path.clone()),
            );
            error_response("UPLOAD_FAILED", &e.to_string()).into_response()
        }
    }
}

async fn download(
    State(state): State<AppState>,
    Query(params): Query<PathQuery>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    // If the path is a directory, return a directory listing instead of
    // triggering a download.  This prevents browsers from downloading
    // directories when a user navigates into a workspace resource path.
    if let Ok(entry) = conn.stat(&params.path).await {
        if entry.is_dir {
            match conn.list(&params.path).await {
                Ok(entries) => {
                    tracing::info!(
                        action = "FILE_OP",
                        op = "download_dir_listing",
                        session_id = %params.session_id,
                        path = %params.path,
                        "directory listing returned instead of download"
                    );
                    return (StatusCode::OK, Json(entries)).into_response();
                }
                Err(e) => return error_response("LIST_FAILED", &e.to_string()).into_response(),
            }
        }
    }

    audit_log(
        &state.db,
        "FILE_TRANSFER_START",
        "success",
        Some(params.path.clone()),
    );
    tracing::info!(action = "TRANSFER_START", op = "download", session_id = %params.session_id, path = %params.path, "download starting");

    // Check for Range header

    let range = headers.get("range").and_then(|v| v.to_str().ok());

    if let Some(range_str) = range {
        // Parse Range: bytes=offset-limit or bytes=offset-
        if let Some(range_val) = range_str.strip_prefix("bytes=") {
            let parts: Vec<&str> = range_val.splitn(2, '-').collect();
            if parts.len() == 2 {
                if let Ok(offset) = parts[0].parse::<u64>() {
                    let limit = if parts[1].is_empty() {
                        None // bytes=offset- → to end of file
                    } else {
                        parts[1].parse::<u64>().ok()
                    };
                    match conn.download_range(&params.path, offset, limit).await {
                        Ok(data) => {
                            let filename = params.path.rsplit('/').next().unwrap_or("file");
                            tracing::info!(
                                action = "FILE_OP",
                                op = "download",
                                session_id = %params.session_id,
                                path = %params.path,
                                "file downloaded"
                            );
                            let audit_db = state.db.clone();

                            let download_path = params.path.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                                    action: "FILE_OP".into(),
                                    target: Some(download_path),
                                    detail: Some("op=download".into()),
                                    result: "success".into(),
                                    ..Default::default()
                                })
                            })
                            .await;
                            audit_log(
                                &state.db,
                                "FILE_TRANSFER_COMPLETE",
                                "success",
                                Some(params.path.clone()),
                            );
                            tracing::info!(action = "TRANSFER_COMPLETE", op = "download", session_id = %params.session_id, path = %params.path, "download complete");
                            (
                                StatusCode::OK,
                                [
                                    ("Content-Type", "application/octet-stream"),
                                    (
                                        "Content-Disposition",
                                        &format!("attachment; filename=\"{filename}\""),
                                    ),
                                ],
                                data,
                            )
                                .into_response()
                        }
                        Err(e) => {
                            tracing::error!(action = "TRANSFER_FAIL", op = "download", session_id = %params.session_id, path = %params.path, error = %e, "download range failed");
                            audit_log(
                                &state.db,
                                "FILE_TRANSFER_FAILED",
                                "failure",
                                Some(params.path.clone()),
                            );
                            error_response("DOWNLOAD_FAILED", &e.to_string()).into_response()
                        }
                    }
                } else {
                    error_response("INVALID_RANGE", "invalid range header").into_response()
                }
            } else {
                error_response("INVALID_RANGE", "invalid range header").into_response()
            }
        } else {
            error_response("INVALID_RANGE", "invalid range header").into_response()
        }
    } else {
        match conn.download(&params.path).await {
            Ok(data) => {
                let filename = params.path.rsplit('/').next().unwrap_or("file");
                tracing::info!(
                    action = "FILE_OP",
                    op = "download",
                    session_id = %params.session_id,
                    path = %params.path,
                    "file downloaded"
                );
                let audit_db = state.db.clone();

                let download_path = params.path.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    audit_db.write_audit_log(&crate::models::NewAuditEntry {
                        action: "FILE_OP".into(),
                        target: Some(download_path),
                        detail: Some("op=download".into()),
                        result: "success".into(),
                        ..Default::default()
                    })
                })
                .await;
                audit_log(
                    &state.db,
                    "FILE_TRANSFER_COMPLETE",
                    "success",
                    Some(params.path.clone()),
                );
                tracing::info!(action = "TRANSFER_COMPLETE", op = "download", session_id = %params.session_id, path = %params.path, "download complete");
                (
                    StatusCode::OK,
                    [
                        ("Content-Type", "application/octet-stream"),
                        (
                            "Content-Disposition",
                            &format!("attachment; filename=\"{filename}\""),
                        ),
                    ],
                    data,
                )
                    .into_response()
            }
            Err(e) => {
                tracing::error!(action = "TRANSFER_FAIL", op = "download", session_id = %params.session_id, path = %params.path, error = %e, "download failed");
                audit_log(
                    &state.db,
                    "FILE_TRANSFER_FAILED",
                    "failure",
                    Some(params.path.clone()),
                );
                error_response("DOWNLOAD_FAILED", &e.to_string()).into_response()
            }
        }
    }
}

async fn read_for_edit(
    State(state): State<AppState>,
    Query(params): Query<PathQuery>,
) -> axum::response::Response {
    tracing::debug!(action = "FILE_READ_FOR_EDIT", session_id = %params.session_id, path = %params.path, "file read_for_edit");
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.read_for_edit(&params.path).await {
        Ok(data) => {
            let filename = params.path.rsplit('/').next().unwrap_or("file").to_string();
            let content = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "content": content,
                    "filename": filename,
                    "size": data.len(),
                })),
            )
                .into_response()
        }
        Err(e) => error_response("READ_FAILED", &e.to_string()).into_response(),
    }
}

async fn save_from_edit(
    State(state): State<AppState>,
    Json(body): Json<SaveFromEditBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match base64::engine::general_purpose::STANDARD.decode(&body.content) {
        Ok(data) => match conn.save_from_edit(&body.path, data).await {
            Ok(()) => {
                tracing::info!(
                    action = "FILE_OP",
                    op = "save_edit",
                    session_id = %body.session_id,
                    path = %body.path,
                    "file saved from edit"
                );
                let audit_db = state.db.clone();

                let save_path = body.path.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    audit_db.write_audit_log(&crate::models::NewAuditEntry {
                        action: "FILE_OP".into(),
                        target: Some(save_path),
                        detail: Some("op=save_edit".into()),
                        result: "success".into(),
                        ..Default::default()
                    })
                })
                .await;
                (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
            }
            Err(e) => error_response("SAVE_FAILED", &e.to_string()).into_response(),
        },
        Err(e) => {
            error_response("INVALID_CONTENT", &format!("invalid base64: {e}")).into_response()
        }
    }
}

async fn delete(
    State(state): State<AppState>,
    Json(body): Json<DeleteBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.delete(&body.path).await {
        Ok(()) => {
            tracing::info!(
                action = "FILE_OP",
                op = "delete",
                path = %body.path,
                session_id = %body.session_id,
                "file deleted"
            );
            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => error_response("DELETE_FAILED", &e.to_string()).into_response(),
    }
}

async fn rename(
    State(state): State<AppState>,
    Json(body): Json<RenameBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.rename(&body.from, &body.to).await {
        Ok(()) => {
            tracing::info!(
                action = "FILE_RENAME",
                session_id = %body.session_id,
                from = %body.from,
                to = %body.to,
                "file renamed"
            );
            let audit_db = state.db.clone();

            let rename_from = body.from.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "FILE_RENAME".into(),
                    target: Some(rename_from),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;
            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => error_response("RENAME_FAILED", &e.to_string()).into_response(),
    }
}

async fn mkdir(
    State(state): State<AppState>,
    Json(body): Json<MkdirBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.mkdir(&body.path).await {
        Ok(()) => {
            tracing::info!(
                action = "FILE_MKDIR",
                session_id = %body.session_id,
                path = %body.path,
                "directory created"
            );
            let audit_db = state.db.clone();

            let mkdir_path = body.path.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "FILE_MKDIR".into(),
                    target: Some(mkdir_path),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;
            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => error_response("MKDIR_FAILED", &e.to_string()).into_response(),
    }
}

async fn chmod(
    State(state): State<AppState>,
    Json(body): Json<ChmodBody>,
) -> axum::response::Response {
    tracing::debug!(
        action = "FILE_CHMOD",
        session_id = %body.session_id,
        path = %body.path,
        mode = %body.mode,
        "file chmod"
    );
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.chmod(&body.path, &body.mode).await {
        Ok(()) => {
            tracing::info!(
                action = "FILE_CHMOD",
                session_id = %body.session_id,
                path = %body.path,
                mode = %body.mode,
                "file chmod applied"
            );
            let audit_db = state.db.clone();
            let chmod_path = body.path.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "FILE_CHMOD".into(),
                    target: Some(chmod_path),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;
            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => connector_op_error("CHMOD_FAILED", &e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PresignedUrlBody {
    session_id: String,
    path: String,
    #[serde(default = "default_expires")]
    expires_in: u64,
}

fn default_expires() -> u64 {
    3600
}

#[derive(Debug, Serialize)]
struct PresignedUrlResponse {
    url: String,
}

async fn presigned_url(
    State(state): State<AppState>,
    Json(body): Json<PresignedUrlBody>,
) -> axum::response::Response {
    tracing::debug!(action = "FILE_PRESIGNED_URL", session_id = %body.session_id, path = %body.path, expires = %body.expires_in, "file presigned_url");
    let pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    // S3-only 操作直接走 trait 方法：非 S3 连接器由默认实现回
    // UnsupportedProtocolError（下同），错误码与历史 downcast 分支一致。
    match conn.presigned_url(&body.path, body.expires_in).await {
        Ok(url) => (StatusCode::OK, Json(PresignedUrlResponse { url })).into_response(),
        Err(e) => connector_op_error("PRESIGNED_URL_FAILED", &e).into_response(),
    }
}

// ---------------------------------------------------------------------------
// ACL handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct AclResponse {
    acl: String,
}

async fn get_acl(
    State(state): State<AppState>,
    Query(params): Query<PathQuery>,
) -> axum::response::Response {
    let pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    match conn.get_acl(&params.path).await {
        Ok(acl) => {
            tracing::info!(
                action = "FILE_ACL",
                session_id = %params.session_id,
                path = %params.path,
                "ACL retrieved"
            );
            let audit_db = state.db.clone();

            let acl_path = params.path.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "FILE_ACL".into(),
                    target: Some(acl_path),
                    detail: Some("op=get_acl".into()),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;
            (StatusCode::OK, Json(AclResponse { acl })).into_response()
        }
        Err(e) => connector_op_error("GET_ACL_FAILED", &e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PutAclBody {
    session_id: String,
    path: String,
    acl: String,
}

async fn put_acl(
    State(state): State<AppState>,
    Json(body): Json<PutAclBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    match conn.put_acl(&body.path, &body.acl).await {
        Ok(()) => {
            tracing::info!(
                action = "FILE_ACL",
                session_id = %body.session_id,
                path = %body.path,
                "ACL applied"
            );
            let audit_db = state.db.clone();

            let acl_path = body.path.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "FILE_ACL".into(),
                    target: Some(acl_path),
                    detail: Some("op=put_acl".into()),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;
            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => connector_op_error("PUT_ACL_FAILED", &e).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(username: &str, config: &str) -> ResourceConnInfo {
        ResourceConnInfo {
            resource_id: "r1".into(),
            name: "files".into(),
            protocol: "sftp".into(),
            host: "10.0.0.1".into(),
            port: Some(22),
            username: username.to_string(),
            config: serde_json::from_str(config).unwrap(),
            subtype: None,
            use_agent: true,
            agent_id: Some("agent-1".into()),
        }
    }

    #[test]
    fn agent_file_config_carries_username_and_merged_credentials() {
        let cfg = agent_file_config(&info("alice", r#"{"password":"pw","private_key":"PEM"}"#));
        assert_eq!(
            cfg.get("username").and_then(|v| v.as_str()),
            Some("alice"),
            "Agent authenticates with cfg.username — a missing field means empty-user auth"
        );
        assert_eq!(cfg.get("host").and_then(|v| v.as_str()), Some("10.0.0.1"));
        assert_eq!(cfg.get("port").and_then(|v| v.as_u64()), Some(22));
        assert_eq!(cfg.get("password").and_then(|v| v.as_str()), Some("pw"));
        assert_eq!(cfg.get("private_key").and_then(|v| v.as_str()), Some("PEM"));
    }

    #[test]
    fn agent_file_config_username_beats_stale_config_key() {
        let cfg = agent_file_config(&info("alice", r#"{"username":"stale"}"#));
        assert_eq!(cfg.get("username").and_then(|v| v.as_str()), Some("alice"));
    }

    /// CR1 回归：SSH/SFTP 入口必须把空 username 归一为 `root`，否则与
    /// `terminal_ws` 的归一（同源 `normalize_username`）分叉 → 连接池键
    /// `user@host:port` 不同 → SFTP 必然新建连接且空用户名认证失败。
    #[test]
    fn ssh_entry_normalizes_empty_username_for_pool_key_parity() {
        let cfg = ssh_connect_config(&info("", "{}"));
        assert_eq!(cfg.username, "root");
        assert_eq!(
            crate::resource_conn::normalize_username(""),
            cfg.username,
            "terminal_ws and file_api must share one normalization source"
        );
    }

    #[test]
    fn ssh_entry_keeps_explicit_username() {
        let cfg = ssh_connect_config(&info("alice", r#"{"password":"pw"}"#));
        assert_eq!(cfg.username, "alice");
        assert_eq!(cfg.password.as_deref(), Some("pw"));
    }

    /// CR12 regression: the agent file entry must normalize an empty username too,
    /// otherwise Hub forwards `""` and `agent_file.rs`'s `unwrap_or("")`
    /// authenticates as an empty user (SSH always rejects), diverging from the
    /// terminal entry which sends `root` for the same resource.
    #[test]
    fn agent_file_entry_normalizes_empty_username() {
        let cfg = agent_file_config(&info("", "{}"));
        assert_eq!(
            cfg.get("username").and_then(|v| v.as_str()),
            Some("root"),
            "empty top-level username must not be forwarded as an empty user"
        );
        assert_eq!(
            cfg.get("username").and_then(|v| v.as_str()),
            Some(crate::resource_conn::normalize_username("").as_str()),
            "agent file and terminal entries must share one normalization source"
        );
    }

    #[test]
    fn agent_file_entry_keeps_explicit_username() {
        let cfg = agent_file_config(&info("alice", r#"{"password":"pw"}"#));
        assert_eq!(cfg.get("username").and_then(|v| v.as_str()), Some("alice"));
    }

    fn create_s3_resource(state: &AppState) -> String {
        let env = state
            .db
            .create_environment(&crate::models::NewEnvironment {
                name: format!("env-{}", uuid::Uuid::new_v4()),
                description: None,
                connection_mode: Some("direct".into()),
            })
            .unwrap();
        state
            .db
            .create_resource(
                &env.id,
                &crate::models::NewResource {
                    name: "files-s3".into(),
                    protocol: "s3".into(),
                    host: "10.0.0.1".into(),
                    port: Some(4566),
                    username: Some("root".into()),
                    config_json: None,
                    subtype: None,
                    color: None,
                    sort_order: None,
                },
            )
            .unwrap()
            .id
    }

    // ---- 能力查询 API（v0.91.0 T3）----

    #[test]
    fn capabilities_for_protocol_mapping() {
        assert_eq!(
            capabilities_for_protocol("s3"),
            FileCapabilitySet {
                chmod: false,
                presigned_url: true,
                acl: true,
                multipart: true,
            }
        );
        // SFTP/SSH：chmod 已实现 (T4)，其余 S3 专属能力为 false。
        assert_eq!(
            capabilities_for_protocol("sftp"),
            FileCapabilitySet {
                chmod: true,
                presigned_url: false,
                acl: false,
                multipart: false,
            }
        );
        assert_eq!(
            capabilities_for_protocol("ssh"),
            FileCapabilitySet {
                chmod: true,
                presigned_url: false,
                acl: false,
                multipart: false,
            }
        );
        assert_eq!(
            capabilities_for_protocol("oss"),
            FileCapabilitySet::default()
        );
    }

    #[test]
    fn load_connector_capability_404_for_missing_resource() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());

        let err = load_connector_capability(&state, "no-such-resource").unwrap_err();
        assert_eq!(err.0, StatusCode::NOT_FOUND);
        assert_eq!(err.1, "RESOURCE_NOT_FOUND");
        assert!(err.2.contains("no-such-resource"));
    }

    #[test]
    fn load_connector_capability_reports_s3_operations() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());
        let id = create_s3_resource(&state);

        let resp = load_connector_capability(&state, &id).unwrap();
        assert_eq!(resp.protocol, "s3");
        assert!(resp.capabilities.presigned_url);
        assert!(resp.capabilities.acl);
        assert!(resp.capabilities.multipart);
        assert!(!resp.capabilities.chmod);
    }

    /// `connector_op_error` 把 `UnsupportedProtocolError` 映射为
    /// `UNSUPPORTED_PROTOCOL`（文案不变），其它错误保留原业务码。
    #[test]
    fn connector_op_error_maps_unsupported_protocol_code() {
        let err = anyhow::Error::from(UnsupportedProtocolError::new("only supported for S3"));
        let (status, body) = connector_op_error("PRESIGNED_URL_FAILED", &err);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.error.code, "UNSUPPORTED_PROTOCOL");
        assert_eq!(body.error.message, "only supported for S3");
    }

    #[test]
    fn connector_op_error_passes_other_errors_through() {
        let err = anyhow::anyhow!("s3 backend connection refused");
        let (status, body) = connector_op_error("PUT_ACL_FAILED", &err);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.error.code, "PUT_ACL_FAILED");
        assert_eq!(body.error.message, "s3 backend connection refused");
    }

    // ---- Sync task API（v0.92.0 子任务 1）----

    fn sync_body(path: &str) -> SyncRequestBody {
        SyncRequestBody {
            source: TransferEndpointRef {
                resource_id: "res-src".into(),
                path: path.into(),
            },
            target: TransferEndpointRef {
                resource_id: "res-dst".into(),
                path: "/dst/".into(),
            },
            options: rex_transfer::SyncOptions {
                direction: rex_transfer::SyncDirection::Upload,
                delete_orphans: true,
                ..Default::default()
            },
            conflict: None,
        }
    }

    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn sync_task_create_get_cancel_endpoints() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());

        let resp = create_sync_task(State(state.clone()), Json(sync_body("/src/"))).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = body_json(resp).await;
        let id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["status"], "pending");

        // 入库：kind=sync，选项以 JSON 落库，状态 pending
        let rec = state.db.get_transfer_task(&id).unwrap().unwrap();
        assert_eq!(rec.kind, "sync");
        assert_eq!(rec.status, "pending");
        assert!(rec.sync_options.contains("\"delete_orphans\":true"));

        // 查询
        let resp = get_sync_task(State(state.clone()), Path(id.clone())).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let got = body_json(resp).await;
        assert_eq!(got["id"], id.as_str());
        assert_eq!(got["kind"], "sync");

        // 取消
        let resp = cancel_sync_task(State(state.clone()), Path(id.clone())).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            state.db.get_transfer_task(&id).unwrap().unwrap().status,
            "canceled"
        );

        // 未知任务 404
        let resp = get_sync_task(State(state), Path("missing".into())).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn sync_create_rejects_invalid_conflict_and_missing_path() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());

        let mut bad = sync_body("/src/");
        bad.conflict = Some("nope".into());
        let resp = create_sync_task(State(state.clone()), Json(bad)).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            body_json(resp).await["error"]["code"],
            "INVALID_CONFLICT_POLICY"
        );

        let resp = create_sync_task(State(state.clone()), Json(sync_body(""))).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"]["code"], "SYNC_PATH_REQUIRED");
    }

    // ---- dry-run 预览 API（v0.92.0 子任务 3）----
    //
    // 端点层只校验参数 / 映射错误码；计划的生成语义（掩码、方向、孤儿、空计划）
    // 由 `sync_coordinator::preview_plan` 的单测覆盖——端点用真实 connector，
    // 离线不可构造。

    /// 预览与创建共用同一套校验：非法冲突策略、缺路径都回同一错误码。
    #[tokio::test]
    async fn preview_shares_validation_with_create() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());

        let mut bad = sync_body("/src/");
        bad.conflict = Some("nope".into());
        let resp = preview_sync(State(state.clone()), Json(bad.clone())).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            body_json(resp).await["error"]["code"],
            "INVALID_CONFLICT_POLICY"
        );
        let resp = create_sync_task(State(state.clone()), Json(bad)).await;
        assert_eq!(
            body_json(resp).await["error"]["code"],
            "INVALID_CONFLICT_POLICY"
        );

        let resp = preview_sync(State(state.clone()), Json(sync_body(""))).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(resp).await["error"]["code"], "SYNC_PATH_REQUIRED");
    }

    /// 资源连不上时预览回错，且不产生任何任务记录（dry-run 无副作用）。
    #[tokio::test]
    async fn preview_reports_connect_failure_without_persisting_task() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());

        let resp = preview_sync(State(state.clone()), Json(sync_body("/src/"))).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = body_json(resp).await;
        assert_eq!(body["error"]["code"], "SYNC_PREVIEW_CONNECT_FAILED");
        assert!(
            body["error"]["message"]
                .as_str()
                .unwrap()
                .contains("res-src"),
            "message must name the failing resource, got {body}"
        );
        assert!(
            state.db.list_transfer_tasks(10, 0).unwrap().is_empty(),
            "preview must not persist a task"
        );
    }

    /// `preview` 是静态路径，不被 `/sync/{id}` 吞掉：POST 命中预览处理器
    /// （空 body → JSON 提取失败 400），而非 `{id}` 路由的 405。
    #[tokio::test]
    async fn preview_route_is_not_shadowed_by_task_id_route() {
        use axum::body::Body;
        use tower::ServiceExt;

        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/sync/preview")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"source":{"resource_id":"res-src","path":""},
                    "target":{"resource_id":"res-dst","path":"/dst/"},
                    "options":{}}"#,
            ))
            .unwrap();

        let resp = file_routes().with_state(state).oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "POST /sync/preview must reach the preview handler (405 = shadowed by /sync/{{id}})"
        );
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body["error"]["code"], "SYNC_PATH_REQUIRED",
            "the response must come from the preview handler"
        );
    }
}
