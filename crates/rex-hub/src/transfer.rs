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

/// POST /api/transfers — 创建传输任务并启动执行
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

    // 解析源和目标 connector
    let source_connector = resolve_connector(&input.source, &state).await?;
    let target_connector = resolve_connector(&input.target, &state).await?;

    let id = transfer_state
        .manager
        .create_task(input.source.clone(), input.target.clone())
        .await;

    let task = transfer_state
        .manager
        .get_task(&id)
        .await
        .ok_or_else(|| err_resp("INTERNAL_ERROR", "创建任务后无法读取"))?;

    // 启动异步传输执行
    let manager = Arc::clone(&transfer_state.manager);
    let task_id = id.clone();
    let source_path = input.source.path.clone();
    let target_path = input.target.path.clone();

    tokio::spawn(async move {
        rex_transfer::executor::execute_transfer(
            manager,
            task_id,
            source_connector,
            target_connector,
            source_path,
            target_path,
        )
        .await;
    });

    tracing::info!(task_id = %id, "transfer task created and execution started");

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
    if ep.connector_type == "sftp" && ep.resource_id.is_none() {
        return Err(bad_request(&format!(
            "{prefix}.resource_id 在 connector_type=sftp 时必填"
        )));
    }
    Ok(())
}

/// 根据端点信息解析为 FileConnector 实例
async fn resolve_connector(
    endpoint: &TransferEndpoint,
    state: &Arc<AppState>,
) -> Result<Arc<dyn rex_transfer::FileConnector + Send + Sync>, (StatusCode, Json<ErrorResponse>)> {
    match endpoint.connector_type.as_str() {
        "local" => {
            // 本地文件系统：使用根目录作为 base_path，允许访问任意路径
            let connector = rex_transfer::local::LocalConnector::new(std::path::PathBuf::from("/"))
                .map_err(|e| err_resp("CONNECTOR_ERROR", &format!("本地连接器创建失败: {e}")))?;
            Ok(Arc::new(connector))
        }
        "sftp" => {
            // SFTP：从数据库查找资源凭据，建立 SSH 连接
            let resource_id = endpoint.resource_id.as_ref()
                .ok_or_else(|| bad_request("SFTP 端点缺少 resource_id"))?;

            let db = state.db.clone();
            let rid = resource_id.clone();
            let resource = tokio::task::spawn_blocking(move || {
                let conn = db.pool.get().map_err(|e| err_resp("DB_ERROR", &e.to_string()))?;
                conn.query_row(
                    "SELECT id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at \
                     FROM resources WHERE id = ?1",
                    rusqlite::params![rid],
                    |row| Ok(crate::resource::Resource {
                        id: row.get(0)?, environment_id: row.get(1)?, name: row.get(2)?,
                        protocol: row.get(3)?, agent_id: row.get(4)?,
                        config_json: row.get(5)?, status: row.get(6)?,
                        created_at: row.get(7)?, updated_at: row.get(8)?,
                    }),
                )
                .map_err(|e| not_found("RESOURCE_NOT_FOUND", &format!("资源不存在: {e}")))
            })
            .await
            .map_err(|e| err_resp("INTERNAL_ERROR", &e.to_string()))??;

            if resource.protocol != "ssh" && resource.protocol != "sftp" {
                return Err(bad_request("资源不是 SSH/SFTP 协议"));
            }

            let ssh_config = crate::ssh_config::SshResourceConfig::from_encrypted_json(
                &resource.config_json,
                &state.secret_key,
            )
            .map_err(|e| bad_request(&format!("资源配置解析失败: {e}")))?;

            let auth_method = ssh_config
                .to_auth_method(&state.secret_key)
                .map_err(|e| bad_request(&format!("认证配置错误: {e}")))?;

            let connector = rex_transfer::sftp::SftpConnector::connect(
                &ssh_config.host,
                ssh_config.port,
                &ssh_config.username,
                auth_method,
            )
            .await
            .map_err(|e| err_resp("SFTP_CONNECT_FAILED", &format!("SFTP 连接失败: {e}")))?;

            Ok(Arc::new(connector))
        }
        other => Err(bad_request(&format!("不支持的连接类型: {other}"))),
    }
}

// ── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tempfile::TempDir;
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
            data_dir: std::env::temp_dir(),
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

    /// 创建一个带真实 temp 目录的 state，用于 local→local 传输测试
    fn test_state_with_dirs() -> (Arc<AppState>, TempDir, TempDir) {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();
        let state = Arc::new(AppState {
            db: Arc::new(crate::db::Database::new_in_memory().unwrap()),
            secret_key: "test-secret".to_string(),
            connections: Arc::new(crate::ws::new_connections()),
            sessions: Arc::new(crate::terminal::SessionManager::new(900)),
            transfer: Some(Arc::new(TransferState {
                manager: Arc::new(TransferManager::new()),
            })),
            update_cache: tokio::sync::RwLock::new(crate::routes::UpdateCache::new()),
            data_dir: std::env::temp_dir(),
        });
        (state, src_dir, dst_dir)
    }

    fn test_app_with_dirs(src: &TempDir, dst: &TempDir) -> axum::Router {
        let src_file = src.path().join("test.txt");
        std::fs::write(&src_file, b"hello").unwrap();

        let state = Arc::new(AppState {
            db: Arc::new(crate::db::Database::new_in_memory().unwrap()),
            secret_key: "test-secret".to_string(),
            connections: Arc::new(crate::ws::new_connections()),
            sessions: Arc::new(crate::terminal::SessionManager::new(900)),
            transfer: Some(Arc::new(TransferState {
                manager: Arc::new(TransferManager::new()),
            })),
            update_cache: tokio::sync::RwLock::new(crate::routes::UpdateCache::new()),
            data_dir: std::env::temp_dir(),
        });

        use axum::routing::{get, post};
        axum::Router::new()
            .route("/api/transfers", post(create_transfer).get(list_transfers))
            .route(
                "/api/transfers/:id",
                get(get_transfer).delete(cancel_transfer),
            )
            .with_state(state)
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
    async fn create_transfer_sftp_without_resource_id_fails() {
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
    async fn create_local_to_local_transfer() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();
        let src_file = src_dir.path().join("test.txt");
        std::fs::write(&src_file, b"hello world").unwrap();

        let state = Arc::new(AppState {
            db: Arc::new(crate::db::Database::new_in_memory().unwrap()),
            secret_key: "test-secret".to_string(),
            connections: Arc::new(crate::ws::new_connections()),
            sessions: Arc::new(crate::terminal::SessionManager::new(900)),
            transfer: Some(Arc::new(TransferState {
                manager: Arc::new(TransferManager::new()),
            })),
            update_cache: tokio::sync::RwLock::new(crate::routes::UpdateCache::new()),
            data_dir: std::env::temp_dir(),
        });

        use axum::routing::{get, post};
        let app = axum::Router::new()
            .route("/api/transfers", post(create_transfer).get(list_transfers))
            .route(
                "/api/transfers/:id",
                get(get_transfer).delete(cancel_transfer),
            )
            .with_state(state);

        let src_path = src_dir.path().join("test.txt");
        let dst_path = dst_dir.path().join("test.txt");
        let body = serde_json::json!({
            "source": { "connector_type": "local", "path": src_path },
            "target": { "connector_type": "local", "path": dst_path }
        });

        let resp = app.oneshot(make_request(&body.to_string())).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let task_id = json["data"]["id"].as_str().unwrap().to_string();
        assert!(task_id.starts_with("xfer_"));

        // 等待异步传输完成
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 验证目标文件存在且内容正确
        assert!(dst_path.exists());
        assert_eq!(std::fs::read(&dst_path).unwrap(), b"hello world");
    }

    #[tokio::test]
    async fn cancel_completed_transfer_fails() {
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
            data_dir: std::env::temp_dir(),
        });

        use axum::routing::{get, post};
        let app = axum::Router::new()
            .route("/api/transfers", post(create_transfer).get(list_transfers))
            .route(
                "/api/transfers/:id",
                get(get_transfer).delete(cancel_transfer),
            )
            .with_state(state);

        // 直接通过 manager 创建任务（绕过 API 的 connector 解析）
        let id = manager
            .create_task(
                TransferEndpoint { connector_type: "local".into(), resource_id: None, sftp_host: None, sftp_port: None, sftp_username: None, path: "/a".into() },
                TransferEndpoint { connector_type: "local".into(), resource_id: None, sftp_host: None, sftp_port: None, sftp_username: None, path: "/b".into() },
            )
            .await;

        manager.set_status(&id, TransferStatus::Completed).await.unwrap();

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
