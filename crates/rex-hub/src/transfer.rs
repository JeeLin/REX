use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::helpers::{bad_request, err_resp, not_found, ApiResponse, ErrorResponse};
use crate::routes::AppState;
use rex_transfer::task::{TransferEndpoint, TransferManager, TransferStatus};

/// 共享传输管理器（通过 AppState 传递）
pub struct TransferState {
    pub manager: Arc<TransferManager>,
}

/// 创建传输任务请求
#[derive(Debug, Deserialize)]
pub struct CreateTransferRequest {
    pub source: TransferEndpoint,
    pub target: TransferEndpoint,
}

/// 传输任务响应（JSON 可序列化版本）
#[derive(Debug, serde::Serialize)]
pub struct TransferTaskResponse {
    pub id: String,
    pub source: TransferEndpoint,
    pub target: TransferEndpoint,
    pub status: String,
    /// 当 status="failed" 时包含错误原因
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_detail: Option<String>,
    pub progress: rex_transfer::task::TransferProgress,
    pub created_at: String,
    pub updated_at: String,
}

impl From<rex_transfer::task::TransferTask> for TransferTaskResponse {
    fn from(t: rex_transfer::task::TransferTask) -> Self {
        let (status, status_detail) = match &t.status {
            TransferStatus::Failed(reason) => ("failed".to_string(), Some(reason.clone())),
            other => (other.as_str().to_string(), None),
        };
        Self {
            id: t.id,
            source: t.source,
            target: t.target,
            status,
            status_detail,
            progress: t.progress,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

/// POST /api/transfers — 创建传输任务
pub async fn create_transfer(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateTransferRequest>,
) -> Result<(StatusCode, Json<ApiResponse<TransferTaskResponse>>), (StatusCode, Json<ErrorResponse>)>
{
    // 验证源和目标类型
    validate_endpoint(&input.source, "source")?;
    validate_endpoint(&input.target, "target")?;

    let transfer_state = state
        .transfer
        .as_ref()
        .ok_or_else(|| err_resp("INTERNAL_ERROR", "传输管理器未初始化"))?;

    let id = transfer_state
        .manager
        .create_task(input.source, input.target)
        .await;

    let task = transfer_state
        .manager
        .get_task(&id)
        .await
        .ok_or_else(|| err_resp("INTERNAL_ERROR", "创建任务后无法读取"))?;

    tracing::info!(task_id = %id, "transfer task created via API");

    Ok((StatusCode::CREATED, Json(ApiResponse { data: task.into() })))
}

/// GET /api/transfers — 列出所有传输任务
pub async fn list_transfers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<TransferTaskResponse>>>, (StatusCode, Json<ErrorResponse>)> {
    let transfer_state = state
        .transfer
        .as_ref()
        .ok_or_else(|| err_resp("INTERNAL_ERROR", "传输管理器未初始化"))?;

    let tasks = transfer_state.manager.list_tasks().await;
    let responses: Vec<TransferTaskResponse> = tasks.into_iter().map(Into::into).collect();

    Ok(Json(ApiResponse { data: responses }))
}

/// GET /api/transfers/:id — 获取传输任务详情
pub async fn get_transfer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<TransferTaskResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let transfer_state = state
        .transfer
        .as_ref()
        .ok_or_else(|| err_resp("INTERNAL_ERROR", "传输管理器未初始化"))?;

    let task = transfer_state
        .manager
        .get_task(&id)
        .await
        .ok_or_else(|| not_found("TRANSFER_NOT_FOUND", "传输任务不存在"))?;

    Ok(Json(ApiResponse { data: task.into() }))
}

/// DELETE /api/transfers/:id — 取消传输任务
pub async fn cancel_transfer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let transfer_state = state
        .transfer
        .as_ref()
        .ok_or_else(|| err_resp("INTERNAL_ERROR", "传输管理器未初始化"))?;

    match transfer_state.manager.cancel_task(&id).await {
        Ok(()) => {
            tracing::info!(task_id = %id, "transfer task cancelled via API");
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("task not found") {
                Err(not_found("TRANSFER_NOT_FOUND", "传输任务不存在"))
            } else {
                Err(bad_request(&msg))
            }
        }
    }
}

/// DELETE /api/transfers/:id/remove — 从列表中移除任务（仅 completed/failed/cancelled）
pub async fn remove_transfer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let transfer_state = state
        .transfer
        .as_ref()
        .ok_or_else(|| err_resp("INTERNAL_ERROR", "传输管理器未初始化"))?;

    match transfer_state.manager.remove_task(&id).await {
        Ok(()) => {
            tracing::info!(task_id = %id, "transfer task removed via API");
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("task not found") {
                Err(not_found("TRANSFER_NOT_FOUND", "传输任务不存在"))
            } else {
                Err(bad_request(&msg))
            }
        }
    }
}

/// 验证传输端点参数
fn validate_endpoint(
    ep: &TransferEndpoint,
    prefix: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    const VALID_TYPES: &[&str] = &["local", "sftp"];
    if !VALID_TYPES.contains(&ep.connector_type.as_str()) {
        return Err(bad_request(&format!(
            "{prefix}.connector_type 必须是 {} 中的一个",
            VALID_TYPES.join(" | ")
        )));
    }
    if ep.connector_type == "sftp" && ep.sftp_host.is_none() {
        return Err(bad_request(&format!(
            "{prefix}.sftp_host 在 connector_type=sftp 时必填"
        )));
    }
    Ok(())
}

// ── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::routes::AppState;
    use rex_transfer::task::TransferManager;
    use std::sync::Arc;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            db: Arc::new(crate::db::Database::new_in_memory().unwrap()),
            secret_key: "test-secret".to_string(),
            connections: Arc::new(crate::ws::new_connections()),
            sessions: Arc::new(crate::terminal::SessionManager::new(900)),
            transfer: Some(Arc::new(TransferState {
                manager: Arc::new(TransferManager::new()),
            })),
            update_cache: tokio::sync::RwLock::new(crate::routes::UpdateCache::new()),
        })
    }

    fn test_app() -> axum::Router {
        let state = test_state();

        use axum::routing::{get, post};
        axum::Router::new()
            .route("/api/transfers", post(create_transfer).get(list_transfers))
            .route(
                "/api/transfers/:id",
                get(get_transfer).delete(cancel_transfer),
            )
            .with_state(state)
    }

    fn auth_header() -> axum::http::header::HeaderValue {
        use jsonwebtoken::{encode, EncodingKey, Header};
        let exp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
            + 3600;
        let claims = crate::auth::Claims {
            sub: "admin".to_string(),
            exp,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("test-secret".as_bytes()),
        )
        .unwrap();
        axum::http::header::HeaderValue::from_str(&format!("Bearer {token}")).unwrap()
    }

    fn make_request(body: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/transfers")
            .header("content-type", "application/json")
            .header("authorization", auth_header())
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    async fn create_task_via_api(app: &axum::Router) -> String {
        let body = serde_json::json!({
            "source": { "connector_type": "local", "path": "/a" },
            "target": { "connector_type": "local", "path": "/b" }
        });
        let resp = app
            .clone()
            .oneshot(make_request(&body.to_string()))
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["data"]["id"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn create_transfer_returns_201() {
        let app = test_app();
        let body = serde_json::json!({
            "source": {
                "connector_type": "local",
                "path": "/data/source.txt"
            },
            "target": {
                "connector_type": "sftp",
                "sftp_host": "192.168.1.1",
                "sftp_port": 22,
                "sftp_username": "root",
                "path": "/remote/dest.txt"
            }
        });
        let resp = app.oneshot(make_request(&body.to_string())).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["data"]["id"].as_str().unwrap().starts_with("xfer_"));
        assert_eq!(json["data"]["status"].as_str().unwrap(), "pending");
    }

    #[tokio::test]
    async fn create_transfer_invalid_connector_type() {
        let app = test_app();
        let body = serde_json::json!({
            "source": {
                "connector_type": "ftp",
                "path": "/data"
            },
            "target": {
                "connector_type": "local",
                "path": "/data"
            }
        });
        let resp = app.oneshot(make_request(&body.to_string())).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_transfer_sftp_without_host_fails() {
        let app = test_app();
        let body = serde_json::json!({
            "source": { "connector_type": "local", "path": "/a" },
            "target": {
                "connector_type": "sftp",
                "path": "/remote/b"
            }
        });
        let resp = app.oneshot(make_request(&body.to_string())).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_transfers_empty() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/transfers")
                    .header("authorization", auth_header())
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
        assert_eq!(json["data"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn get_transfer_not_found() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/transfers/nonexistent")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_then_get_transfer() {
        let app = test_app();
        let id = create_task_via_api(&app).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/transfers/{id}"))
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn cancel_pending_transfer() {
        let app = test_app();
        let id = create_task_via_api(&app).await;

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/transfers/{id}"))
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // 验证状态已变为 cancelled
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/transfers/{id}"))
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["status"].as_str().unwrap(), "cancelled");
    }

    #[tokio::test]
    async fn cancel_completed_transfer_fails() {
        // 先创建任务，再通过 API 将状态改为 completed，最后验证取消失败
        // 使用独立的 TransferManager 和 Arc 包装共享
        let manager = Arc::new(TransferManager::new());
        let transfer_state = Arc::new(TransferState {
            manager: Arc::clone(&manager),
        });

        let state = Arc::new(AppState {
            db: Arc::new(crate::db::Database::new_in_memory().unwrap()),
            secret_key: "test-secret".to_string(),
            connections: Arc::new(crate::ws::new_connections()),
            sessions: Arc::new(crate::terminal::SessionManager::new(900)),
            transfer: Some(transfer_state),
            update_cache: tokio::sync::RwLock::new(crate::routes::UpdateCache::new()),
        });

        use axum::routing::{get, post};
        let app = axum::Router::new()
            .route("/api/transfers", post(create_transfer).get(list_transfers))
            .route(
                "/api/transfers/:id",
                get(get_transfer).delete(cancel_transfer),
            )
            .with_state(state);

        let id = create_task_via_api(&app).await;

        // 通过共享 manager 直接设为 completed
        manager
            .set_status(&id, TransferStatus::Completed)
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/transfers/{id}"))
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
