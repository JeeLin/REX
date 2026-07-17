//! Agent 管理 REST API — 只读查询端点。
//!
//! Agent 的注册、心跳、状态更新全部通过 WebSocket（agent_ws）处理。
//! 这里只保留前端管理页面需要的只读查询接口。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::models::Agent;
use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })),
    )
}

/// 只读查询路由（前端管理页面使用）
pub fn agent_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/{id}", axum::routing::get(get_agent))
        .route("/{id}/reset-token", axum::routing::post(reset_token))
}

pub fn env_agent_routes() -> axum::Router<AppState> {
    axum::Router::new().route("/{env_id}/agents", axum::routing::get(list_agents))
}

// --- Handlers ---

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
