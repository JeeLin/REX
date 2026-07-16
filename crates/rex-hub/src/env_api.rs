//! 环境管理 REST API。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::models::{EnvironmentDetail, NewEnvironment, UpdateEnvironment};
use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

pub fn env_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list_environments))
        .route("/", axum::routing::post(create_environment))
        .route("/{id}", axum::routing::get(get_environment))
        .route("/{id}", axum::routing::put(update_environment))
        .route("/{id}", axum::routing::delete(delete_environment))
}

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })))
}

async fn list_environments(
    State(state): State<AppState>,
) -> ApiResult<Vec<EnvironmentDetail>> {
    let envs = tokio::task::spawn_blocking(move || state.db.list_environments_with_stats())
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    Ok(Json(envs))
}

async fn get_environment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<EnvironmentDetail> {
    let env = tokio::task::spawn_blocking(move || state.db.get_environment_with_stats(&id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "environment not found"))?;
    Ok(Json(env))
}

async fn create_environment(
    State(state): State<AppState>,
    Json(body): Json<NewEnvironment>,
) -> ApiResult<crate::models::Environment> {
    if body.name.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "name is required"));
    }
    let db = state.db.clone();
    let env = tokio::task::spawn_blocking(move || db.create_environment(&body))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("UNIQUE") {
                err(StatusCode::CONFLICT, "environment name already exists")
            } else {
                err(StatusCode::INTERNAL_SERVER_ERROR, &msg)
            }
        })?;
    // 审计日志
    let audit_db = state.db.clone();
    let env_name = env.name.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "ENV_CREATE".into(),
            target: Some(env_name),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;
    Ok(Json(env))
}

async fn update_environment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateEnvironment>,
) -> ApiResult<crate::models::Environment> {
    let db = state.db.clone();
    let env_id = id.clone();
    let env = tokio::task::spawn_blocking(move || db.update_environment(&env_id, &body))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") {
                err(StatusCode::NOT_FOUND, &msg)
            } else if msg.contains("UNIQUE") {
                err(StatusCode::CONFLICT, "environment name already exists")
            } else {
                err(StatusCode::INTERNAL_SERVER_ERROR, &msg)
            }
        })?;
    // 审计日志
    let audit_db = state.db.clone();
    let env_name = env.name.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "ENV_UPDATE".into(),
            target: Some(env_name),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;
    Ok(Json(env))
}

async fn delete_environment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<serde_json::Value> {
    // 先获取环境名用于审计日志
    let db = state.db.clone();
    let env_id = id.clone();
    let env_name = tokio::task::spawn_blocking(move || db.get_environment(&env_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map(|e| e.name)
        .unwrap_or_default();

    let db = state.db.clone();
    let env_id = id.clone();
    tokio::task::spawn_blocking(move || db.delete_environment(&env_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    // 审计日志
    let audit_db = state.db.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "ENV_DELETE".into(),
            target: Some(env_name),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;

    Ok(Json(serde_json::json!({ "ok": true })))
}
