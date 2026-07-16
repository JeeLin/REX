//! Dashboard 统计 REST API。

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })))
}

pub fn dashboard_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/stats", axum::routing::get(stats))
        .route("/recent", axum::routing::get(recent))
}

async fn stats(
    State(state): State<AppState>,
) -> ApiResult<serde_json::Value> {
    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let envs = db.list_environments().map_err(|e| e.to_string())?;
        let mut resource_count: i64 = 0;
        let mut online_agents: i64 = 0;
        for env in &envs {
            let resources = db.list_resources_by_env(&env.id).map_err(|e| e.to_string())?;
            resource_count += resources.len() as i64;
            let agents = db.list_agents_by_env(&env.id).map_err(|e| e.to_string())?;
            for agent in &agents {
                if agent.status == "online" {
                    online_agents += 1;
                }
            }
        }
        Ok(serde_json::json!({
            "environment_count": envs.len() as i64,
            "resource_count": resource_count,
            "online_agents": online_agents,
        }))
    })
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(result))
}

async fn recent(
    State(state): State<AppState>,
) -> ApiResult<serde_json::Value> {
    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let envs = db.list_environments().map_err(|e| e.to_string())?;
        let mut all_resources = Vec::new();
        for env in &envs {
            let resources = db.list_resources_by_env(&env.id).map_err(|e| e.to_string())?;
            all_resources.extend(resources);
        }
        all_resources.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        all_resources.truncate(10);
        Ok(serde_json::to_value(&all_resources).unwrap_or_default())
    })
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(result))
}
