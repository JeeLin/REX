//! Agent 管理 REST API。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use rex_common::RExError;

use crate::models::{Agent, NewAuditEntry};
use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })),
    )
}

pub fn agent_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/register", axum::routing::post(register_agent))
        .route("/{id}/heartbeat", axum::routing::post(heartbeat))
        .route("/{id}", axum::routing::get(get_agent))
        .route("/{id}/reset-token", axum::routing::post(reset_token))
}

pub fn env_agent_routes() -> axum::Router<AppState> {
    axum::Router::new().route("/{env_id}/agents", axum::routing::get(list_agents))
}

// --- Handlers ---

#[derive(serde::Deserialize)]
struct RegisterRequest {
    token: String,
    name: String,
    environment_id: String,
    version: Option<String>,
    os: Option<String>,
    arch: Option<String>,
    hostname: Option<String>,
}

async fn register_agent(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> ApiResult<serde_json::Value> {
    let env_id = body.environment_id.clone();
    let name = body.name.clone();
    let token = body.token.clone();
    let version = body.version.clone().unwrap_or_default();
    let os = body.os.clone().unwrap_or_default();
    let arch = body.arch.clone().unwrap_or_default();
    let hostname = body.hostname.clone().unwrap_or_default();

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

    // 创建 agent
    let db = state.db.clone();
    let agent = tokio::task::spawn_blocking(move || {
        db.create_agent(&env_id, &name, &token, &version, &os, &arch, &hostname)
    })
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    // 审计日志
    let audit_db = state.db.clone();
    let agent_name = agent.name.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&NewAuditEntry {
            action: "AGENT_REGISTER".into(),
            target: Some(agent_name),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;

    Ok(Json(serde_json::json!({ "agent_id": agent.id })))
}

async fn heartbeat(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<serde_json::Value> {
    let version = body
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let ip = body
        .get("ip")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.update_agent_heartbeat(&id, &version, &ip))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn get_agent(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Agent> {
    let db = state.db.clone();
    let agent = tokio::task::spawn_blocking(move || db.get_agent(&id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "agent not found"))?;
    Ok(Json(agent))
}

async fn list_agents(
    State(state): State<AppState>,
    Path(env_id): Path<String>,
) -> ApiResult<Vec<Agent>> {
    let db = state.db.clone();
    let agents = tokio::task::spawn_blocking(move || db.list_agents_by_env(&env_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    Ok(Json(agents))
}

async fn reset_token(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<serde_json::Value> {
    let new_token = uuid::Uuid::new_v4().to_string();
    let token_for_db = new_token.clone();
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.reset_agent_token(&id, &token_for_db))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    Ok(Json(serde_json::json!({ "token": new_token })))
}
