use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

use crate::helpers::{err_resp, ApiResponse, ErrorResponse};
use crate::routes::AppState;

// ── 数据模型 ─────────────────────────────────────────────

/// Hub 更新状态
#[derive(Debug, Serialize)]
pub struct UpdateStatusResponse {
    pub current_version: String,
    pub git_commit: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub last_checked: Option<String>,
    pub auto_check_enabled: bool,
}

/// Agent 版本信息
#[derive(Debug, Serialize)]
pub struct AgentVersionInfo {
    pub agent_id: String,
    pub name: String,
    pub environment_id: String,
    pub version: String,
    pub sha256: String,
    pub needs_update: bool,
    pub status: String,
    pub last_seen_at: Option<String>,
}

// ── API 处理函数 ─────────────────────────────────────────

/// GET /api/update/status — 获取 Hub 更新状态
pub async fn get_update_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<UpdateStatusResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let current = rex_common::version::VersionInfo::current();

    // 从内存中读取上次检查结果（如果有）
    let (latest, last_checked) = {
        let cache = state.update_cache.read().await;
        (
            cache.latest_version.clone(),
            cache.last_checked.clone(),
        )
    };

    let update_available = match (&latest, &current.version) {
        (Some(latest), current) => {
            rex_common::version::is_newer(current, latest).unwrap_or(false)
        }
        _ => false,
    };

    Ok(Json(ApiResponse {
        data: UpdateStatusResponse {
            current_version: current.version,
            git_commit: current.git_commit,
            latest_version: latest,
            update_available,
            last_checked,
            auto_check_enabled: true,
        },
    }))
}

/// GET /api/update/check — 手动触发更新检查
pub async fn check_update(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<UpdateStatusResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let current = rex_common::version::VersionInfo::current();
    let checker = rex_common::updater::UpdateChecker::new("user/rex", &current.version);

    let result = checker.check_for_update().await;

    let (latest, update_available) = match result {
        rex_common::updater::UpdateStatus::UpdateAvailable {
            latest, ..
        } => (Some(latest.clone()), true),
        rex_common::updater::UpdateStatus::UpToDate => (None, false),
        rex_common::updater::UpdateStatus::CheckFailed(e) => {
            return Err(err_resp("UPDATE_CHECK_FAILED", &e));
        }
    };

    let now = crate::helpers::now_iso();

    // 缓存检查结果
    {
        let mut cache = state.update_cache.write().await;
        cache.latest_version = latest.clone();
        cache.last_checked = Some(now.clone());
    }

    Ok(Json(ApiResponse {
        data: UpdateStatusResponse {
            current_version: current.version,
            git_commit: current.git_commit,
            latest_version: latest,
            update_available,
            last_checked: Some(now),
            auto_check_enabled: true,
        },
    }))
}

/// GET /api/update/agents — 获取所有 Agent 版本信息
pub async fn list_agent_versions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<AgentVersionInfo>>>, (StatusCode, Json<ErrorResponse>)> {
    let hub_version = rex_common::version::VERSION;
    let db = state.db.clone();

    let agents = tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, environment_id, version, sha256, status, last_seen_at
                 FROM agents ORDER BY environment_id, name",
            )
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        let agents = stmt
            .query_map([], |row| {
                let version: String = row.get(3)?;
                Ok(AgentVersionInfo {
                    agent_id: row.get(0)?,
                    name: row.get(1)?,
                    environment_id: row.get(2)?,
                    version: version.clone(),
                    sha256: row.get(4)?,
                    needs_update: version != hub_version,
                    status: row.get(5)?,
                    last_seen_at: row.get(6)?,
                })
            })
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?
            .filter_map(|r| r.ok())
            .collect();

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(agents)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    Ok(Json(ApiResponse { data: agents }))
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    use crate::routes::AppState;
    use crate::ws::new_connections;
    use crate::terminal::SessionManager;
    use rex_transfer::task::TransferManager;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            db: Arc::new(crate::db::Database::new_in_memory().unwrap()),
            secret_key: "test-secret".to_string(),
            connections: Arc::new(new_connections()),
            sessions: Arc::new(SessionManager::new(900)),
            transfer: Some(Arc::new(crate::transfer::TransferState {
                manager: Arc::new(TransferManager::new()),
            })),
            update_cache: tokio::sync::RwLock::new(crate::routes::UpdateCache::new()),
        })
    }

    fn test_app() -> Router {
        let state = test_state();
        Router::new()
            .route("/api/update/status", get(get_update_status))
            .route("/api/update/check", get(check_update))
            .with_state(state)
    }

    #[tokio::test]
    async fn get_update_status_returns_version() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/update/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["current_version"], "0.1.0");
        assert_eq!(json["data"]["update_available"], false);
        assert_eq!(json["data"]["auto_check_enabled"], true);
    }
}
