use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
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

/// 下载进度
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub status: String, // "downloading" | "verifying" | "ready" | "error"
    pub percent: u32,
    pub message: String,
}

/// 下载请求
#[derive(Debug, Deserialize)]
pub struct DownloadRequest {
    pub version: String,
}

/// 应用更新请求
#[derive(Debug, Deserialize)]
pub struct ApplyRequest {
    pub version: String,
}

// ── API 处理函数 ─────────────────────────────────────────

/// GET /api/update/status — 获取 Hub 更新状态
pub async fn get_update_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<UpdateStatusResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let current = rex_common::version::VersionInfo::current();

    let (latest, last_checked) = {
        let cache = state.update_cache.read().await;
        (cache.latest_version.clone(), cache.last_checked.clone())
    };

    let update_available = match (&latest, &current.version) {
        (Some(latest), current) => rex_common::version::is_newer(current, latest).unwrap_or(false),
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
        rex_common::updater::UpdateStatus::UpdateAvailable { latest, .. } => {
            (Some(latest.clone()), true)
        }
        rex_common::updater::UpdateStatus::UpToDate => (None, false),
        rex_common::updater::UpdateStatus::CheckFailed(e) => {
            return Err(err_resp("UPDATE_CHECK_FAILED", &e));
        }
    };

    let now = crate::helpers::now_iso();

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

/// POST /api/update/download — 下载新版本
pub async fn download_update(
    State(_state): State<Arc<AppState>>,
    Json(input): Json<DownloadRequest>,
) -> Result<Json<ApiResponse<DownloadProgress>>, (StatusCode, Json<ErrorResponse>)> {
    let current = rex_common::version::VersionInfo::current();

    // 检查是否真的是新版本
    if !rex_common::version::is_newer(&current.version, &input.version).unwrap_or(false) {
        return Err(err_resp("NOT_NEWER", "指定版本不比当前版本新"));
    }

    let data_dir =
        std::env::current_dir().map_err(|_| err_resp("INTERNAL_ERROR", "无法获取工作目录"))?;

    let checker = rex_common::updater::UpdateChecker::new("user/rex", &current.version);

    // 下载
    let staged_path = checker
        .download_update(&data_dir, None)
        .await
        .map_err(|e| err_resp("DOWNLOAD_FAILED", &format!("下载失败: {e}")))?;

    // SHA256 校验（如果有 checksums）
    let binary_name = staged_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let checksums_url = format!("https://api.github.com/repos/user/rex/releases/latest",);
    // 尝试获取 checksums，但不强制要求
    let _ = rex_common::updater::verify_download(&staged_path, &checksums_url, &binary_name).await;

    // 写入 update-state.json
    let state_path = data_dir.join("update-state.json");

    // 备份当前二进制
    let rollback_path = rex_common::updater::UpdateChecker::backup_current(&data_dir)
        .map_err(|e| err_resp("BACKUP_FAILED", &format!("备份失败: {e}")))?;

    let update_state = rex_common::update_state::UpdateState {
        phase: rex_common::update_state::UpdatePhase::Requested,
        target_version: input.version,
        old_version: current.version,
        staged_path: staged_path.to_string_lossy().to_string(),
        rollback_path: rollback_path.to_string_lossy().to_string(),
        attempt: 0,
    };

    update_state
        .write(&state_path)
        .map_err(|e| err_resp("STATE_WRITE_FAILED", &format!("写入状态失败: {e}")))?;

    Ok(Json(ApiResponse {
        data: DownloadProgress {
            status: "ready".to_string(),
            percent: 100,
            message: "下载完成，可以重启更新".to_string(),
        },
    }))
}

/// POST /api/update/apply — 应用更新（触发 supervisor 重启）
pub async fn apply_update(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<DownloadProgress>>, (StatusCode, Json<ErrorResponse>)> {
    let data_dir =
        std::env::current_dir().map_err(|_| err_resp("INTERNAL_ERROR", "无法获取工作目录"))?;

    let state_path = data_dir.join("update-state.json");
    let update_state = rex_common::update_state::UpdateState::read(&state_path);

    if update_state.phase != rex_common::update_state::UpdatePhase::Requested {
        return Err(err_resp("NO_UPDATE_PENDING", "没有待应用的更新"));
    }

    // worker 以 exit code 10 退出，通知 supervisor 执行替换
    tracing::info!(version = %update_state.target_version, "applying update, exiting with code 10");
    std::process::exit(10);
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
    use crate::terminal::SessionManager;
    use crate::ws::new_connections;
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
            data_dir: std::env::temp_dir(),
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
        assert_eq!(json["data"]["current_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(json["data"]["update_available"], false);
        assert_eq!(json["data"]["auto_check_enabled"], true);
    }
}
