use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::helpers::{bad_request, err_resp, not_found, ApiResponse, ErrorResponse};
use crate::routes::AppState;
use rex_transfer::FileEntry;

/// 文件列表响应
#[derive(Debug, Serialize)]
pub struct FileListResponse {
    pub path: String,
    pub entries: Vec<FileEntry>,
}

/// 重命名请求
#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub old_path: String,
    pub new_path: String,
}

/// 创建目录请求
#[derive(Debug, Deserialize)]
pub struct MkdirRequest {
    pub path: String,
}

/// 创建文件请求
#[derive(Debug, Deserialize)]
pub struct TouchRequest {
    pub path: String,
}

/// GET /api/resources/:resource_id/files — 列出目录内容
pub async fn list_files(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<FileListResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let path = params.get("path").map(|s| s.as_str()).unwrap_or("/");
    let connector = get_connector(&state, &resource_id).await?;

    let entries = connector
        .list(path.as_ref())
        .await
        .map_err(|e| err_resp("FILE_LIST_FAILED", &format!("列出目录失败: {e}")))?;

    Ok(Json(ApiResponse {
        data: FileListResponse {
            path: path.to_string(),
            entries,
        },
    }))
}

/// POST /api/resources/:resource_id/files/mkdir — 创建目录
pub async fn mkdir(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
    Json(input): Json<MkdirRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let connector = get_connector(&state, &resource_id).await?;

    connector
        .mkdir(PathBuf::from(input.path).as_path())
        .await
        .map_err(|e| err_resp("MKDIR_FAILED", &format!("创建目录失败: {e}")))?;

    Ok(StatusCode::CREATED)
}

/// POST /api/resources/:resource_id/files/touch — 创建文件
pub async fn touch(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
    Json(input): Json<TouchRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let connector = get_connector(&state, &resource_id).await?;

    connector
        .write(PathBuf::from(input.path).as_path(), b"")
        .await
        .map_err(|e| err_resp("TOUCH_FAILED", &format!("创建文件失败: {e}")))?;

    Ok(StatusCode::CREATED)
}

/// DELETE /api/resources/:resource_id/files — 删除文件/目录
pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let path = params
        .get("path")
        .ok_or_else(|| bad_request("缺少 path 参数"))?;
    let connector = get_connector(&state, &resource_id).await?;

    connector
        .delete(PathBuf::from(path).as_path())
        .await
        .map_err(|e| err_resp("DELETE_FAILED", &format!("删除失败: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/resources/:resource_id/files/rename — 重命名
pub async fn rename_file(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
    Json(input): Json<RenameRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let connector = get_connector(&state, &resource_id).await?;

    connector
        .rename(
            PathBuf::from(input.old_path).as_path(),
            PathBuf::from(input.new_path).as_path(),
        )
        .await
        .map_err(|e| err_resp("RENAME_FAILED", &format!("重命名失败: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/resources/:resource_id/files/download — 下载文件
pub async fn download_file(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let path = params
        .get("path")
        .ok_or_else(|| bad_request("缺少 path 参数"))?;
    let connector = get_connector(&state, &resource_id).await?;

    let file_read = connector
        .read(path.as_ref())
        .await
        .map_err(|e| err_resp("FILE_READ_FAILED", &format!("读取文件失败: {e}")))?;

    let filename = file_read
        .entry
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "download".to_string());

    let body = Body::from(file_read.bytes);
    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(body)
        .unwrap();

    Ok(resp)
}

/// POST /api/resources/:resource_id/files/upload — 上传文件
pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    mut multipart: Multipart,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let dir_path = params.get("path").map(|s| s.as_str()).unwrap_or("/");
    let connector = get_connector(&state, &resource_id).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| bad_request(&format!("读取上传数据失败: {e}")))?
    {
        let filename = field
            .file_name()
            .ok_or_else(|| bad_request("缺少文件名"))?
            .to_string();

        let data = field
            .bytes()
            .await
            .map_err(|e| bad_request(&format!("读取文件内容失败: {e}")))?;

        let file_path = if dir_path.ends_with('/') {
            format!("{dir_path}{filename}")
        } else {
            format!("{dir_path}/{filename}")
        };

        connector
            .write(PathBuf::from(&file_path).as_path(), &data)
            .await
            .map_err(|e| err_resp("FILE_WRITE_FAILED", &format!("写入文件失败: {e}")))?;

        info!(resource_id = %resource_id, path = %file_path, size = data.len(), "file uploaded");
    }

    Ok(StatusCode::CREATED)
}

/// 根据资源 ID 获取 FileConnector
async fn get_connector(
    state: &Arc<AppState>,
    resource_id: &str,
) -> Result<Box<dyn rex_transfer::FileConnector + Send + Sync>, (StatusCode, Json<ErrorResponse>)> {
    let resource = state
        .db
        .get_resource_by_id(resource_id)
        .map_err(|e| err_resp("DB_ERROR", &format!("查询资源失败: {e}")))?
        .ok_or_else(|| not_found("RESOURCE_NOT_FOUND", "资源不存在"))?;

    // config_json 是 JSON 字符串，解析后提取连接参数
    let config: serde_json::Value = serde_json::from_str(&resource.config_json)
        .map_err(|e| bad_request(&format!("资源配置格式错误: {e}")))?;

    match resource.protocol.as_str() {
        "sftp" | "ssh" => {
            // SSH 终端内置 SFTP 面板和独立 SFTP 资源共用同一个 connector 创建逻辑
            // 使用 SshResourceConfig 解析配置并提取认证信息
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

            Ok(Box::new(connector))
        }
        "local" => {
            let path = config
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| bad_request("Local 资源缺少 path 配置"))?;

            let connector = rex_transfer::local::LocalConnector::new(PathBuf::from(path))
                .map_err(|e| err_resp("LOCAL_CONNECT_FAILED", &format!("本地连接失败: {e}")))?;

            Ok(Box::new(connector))
        }
        other => Err(bad_request(&format!("不支持的资源协议: {other}"))),
    }
}

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
            transfer: Some(Arc::new(crate::transfer::TransferState {
                manager: Arc::new(TransferManager::new()),
            })),
            update_cache: tokio::sync::RwLock::new(crate::routes::UpdateCache::new()),
            data_dir: std::env::temp_dir(),
        })
    }

    fn test_app() -> axum::Router {
        let state = test_state();

        use axum::routing::{get, post, put};
        axum::Router::new()
            .route(
                "/api/resources/:resource_id/files",
                get(list_files).delete(delete_file),
            )
            .route("/api/resources/:resource_id/files/mkdir", post(mkdir))
            .route("/api/resources/:resource_id/files/touch", post(touch))
            .route("/api/resources/:resource_id/files/rename", put(rename_file))
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

    #[tokio::test]
    async fn list_files_returns_empty_for_unknown_resource() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/resources/nonexistent/files?path=/")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn mkdir_returns_error_for_unknown_resource() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/resources/nonexistent/files/mkdir")
                    .header("authorization", auth_header())
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"path": "/test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn touch_returns_error_for_unknown_resource() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/resources/nonexistent/files/touch")
                    .header("authorization", auth_header())
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"path": "/test.txt"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_file_returns_error_for_missing_path() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/resources/nonexistent/files")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // 缺少 path 参数，应该是 BAD_REQUEST
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rename_file_returns_error_for_unknown_resource() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/resources/nonexistent/files/rename")
                    .header("authorization", auth_header())
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"old_path": "/a", "new_path": "/b"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
