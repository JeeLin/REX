//! 环境管理 REST API。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::models::{EnvironmentDetail, NewEnvironment, UpdateEnvironment};
use crate::AppState;
use serde::{Deserialize, Serialize};

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

pub fn env_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/",
            axum::routing::get(list_environments).post(create_environment),
        )
        .route("/export", axum::routing::get(export_environments))
        .route("/import", axum::routing::post(import_environments))
        .route(
            "/{id}",
            axum::routing::get(get_environment)
                .put(update_environment)
                .delete(delete_environment),
        )
}

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })),
    )
}

async fn list_environments(State(state): State<AppState>) -> ApiResult<Vec<EnvironmentDetail>> {
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
    tracing::info!(
        action = "ENV_CREATE",
        name = %body.name,
        connection_mode = body.connection_mode.as_deref().unwrap_or("none"),
        "creating environment"
    );

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
    tracing::info!(
        action = "ENV_UPDATE",
        env_id = %id,
        "updating environment"
    );

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
    tracing::info!(
        action = "ENV_DELETE",
        env_id = %id,
        "deleting environment"
    );

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

// --- Export / Import ---

#[derive(Debug, Serialize)]
struct ExportResource {
    name: String,
    protocol: String,
    host: String,
    port: Option<u16>,
    username: String,
    config_json: String,
    color: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExportEnvironment {
    name: String,
    description: String,
    connection_mode: String,
    resources: Vec<ExportResource>,
}

#[derive(Debug, Serialize)]
struct ExportData {
    version: String,
    environments: Vec<ExportEnvironment>,
}

async fn export_environments(State(state): State<AppState>) -> ApiResult<ExportData> {
    let db = state.db.clone();
    let envs = tokio::task::spawn_blocking(move || db.list_environments())
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    let mut export_envs = Vec::new();
    for env in &envs {
        let db = state.db.clone();
        let env_id = env.id.clone();
        let resources = tokio::task::spawn_blocking(move || db.list_resources_by_env(&env_id))
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

        export_envs.push(ExportEnvironment {
            name: env.name.clone(),
            description: env.description.clone(),
            connection_mode: env.connection_mode.clone(),
            resources: resources
                .into_iter()
                .map(|r| ExportResource {
                    name: r.name,
                    protocol: r.protocol,
                    host: r.host,
                    port: r.port,
                    username: r.username,
                    config_json: r.config_json,
                    color: r.color,
                })
                .collect(),
        });
    }

    Ok(Json(ExportData {
        version: "1.0".into(),
        environments: export_envs,
    }))
}

#[derive(Debug, Deserialize)]
struct ImportBody {
    environments: Vec<ImportEnvironment>,
}

#[derive(Debug, Deserialize)]
struct ImportEnvironment {
    name: String,
    description: Option<String>,
    connection_mode: Option<String>,
    resources: Vec<ImportResource>,
}

#[derive(Debug, Deserialize)]
struct ImportResource {
    name: String,
    protocol: String,
    host: String,
    port: Option<u16>,
    username: Option<String>,
    config_json: Option<String>,
    color: Option<String>,
}

async fn import_environments(
    State(state): State<AppState>,
    Json(body): Json<ImportBody>,
) -> ApiResult<serde_json::Value> {
    let mut imported = 0u64;
    let mut skipped = 0u64;

    for imp_env in &body.environments {
        // Check if environment with same name exists
        let db = state.db.clone();
        let name = imp_env.name.clone();
        let existing = tokio::task::spawn_blocking(move || db.get_environment_by_name(&name))
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

        if existing.is_some() {
            skipped += 1;
            continue;
        }

        // Create environment
        let db = state.db.clone();
        let new_env = crate::models::NewEnvironment {
            name: imp_env.name.clone(),
            description: imp_env.description.clone(),
            connection_mode: imp_env.connection_mode.clone(),
        };
        let env = tokio::task::spawn_blocking(move || db.create_environment(&new_env))
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

        // Create resources
        for imp_res in &imp_env.resources {
            let db = state.db.clone();
            let env_id = env.id.clone();
            let new_res = crate::models::NewResource {
                name: imp_res.name.clone(),
                protocol: imp_res.protocol.clone(),
                host: imp_res.host.clone(),
                port: imp_res.port,
                username: imp_res.username.clone(),
                config_json: imp_res.config_json.clone(),
                color: imp_res.color.clone(),
                sort_order: None,
            };
            let _ =
                tokio::task::spawn_blocking(move || db.create_resource(&env_id, &new_res)).await;
        }

        imported += 1;
    }

    tracing::info!(
        action = "ENV_IMPORT",
        total = body.environments.len(),
        imported = imported,
        skipped = skipped,
        "environments imported"
    );

    // 审计日志
    let audit_db = state.db.clone();
    let count = imported;
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "ENV_IMPORT".into(),
            detail: Some(format!("imported={}, skipped={}", count, skipped)),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;

    Ok(Json(
        serde_json::json!({ "imported": imported, "skipped": skipped }),
    ))
}
