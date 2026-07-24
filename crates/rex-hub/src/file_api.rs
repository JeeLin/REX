//! 文件管理 REST 路由。

use std::collections::HashMap;
use std::sync::Arc;

use crate::AppState;
use axum::extract::{Multipart, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use base64::Engine;
use rex_common::file_transfer::{FileConnectRequest, FileConnector};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pub type FileState = Arc<Mutex<FileConnectionPool>>;

pub struct FileConnectionPool {
    connectors: HashMap<String, Box<dyn FileConnector>>,
}

impl Default for FileConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl FileConnectionPool {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }
    pub fn insert(&mut self, id: String, conn: Box<dyn FileConnector>) {
        self.connectors.insert(id, conn);
    }
    pub fn remove(&mut self, id: &str) -> Option<Box<dyn FileConnector>> {
        self.connectors.remove(id)
    }
}

pub fn file_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/connect", axum::routing::post(connect))
        .route("/disconnect", axum::routing::post(disconnect))
        .route("/list", axum::routing::get(list))
        .route("/stat", axum::routing::get(stat))
        .route("/upload", axum::routing::post(upload))
        .route("/download", axum::routing::get(download))
        .route("/delete", axum::routing::post(delete))
        .route("/rename", axum::routing::post(rename))
        .route("/mkdir", axum::routing::post(mkdir))
        .route("/presigned-url", axum::routing::post(presigned_url))
        .route(
            "/s3/multipart-uploads",
            axum::routing::get(list_multipart_uploads),
        )
        .route(
            "/s3/resume-upload",
            axum::routing::post(resume_multipart_upload),
        )
        .route(
            "/s3/abort-upload",
            axum::routing::post(abort_multipart_upload),
        )
        .route("/acl", axum::routing::get(get_acl).put(put_acl))
        .route("/read-for-edit", axum::routing::get(read_for_edit))
        .route("/save-from-edit", axum::routing::post(save_from_edit))
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ConnectBody {
    #[serde(flatten)]
    req: FileConnectRequest,
}

#[derive(Debug, Serialize)]
struct ConnectResponse {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct DisconnectBody {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct PathQuery {
    session_id: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct DeleteBody {
    session_id: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct RenameBody {
    session_id: String,
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct MkdirBody {
    session_id: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct SaveFromEditBody {
    session_id: String,
    path: String,
    content: String, // base64 encoded
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

fn error_response(code: &str, message: &str) -> (StatusCode, Json<ErrorBody>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorBody {
            error: ErrorDetail {
                code: code.to_string(),
                message: message.to_string(),
            },
        }),
    )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn connect(
    State(state): State<AppState>,
    Json(body): Json<ConnectBody>,
) -> axum::response::Response {
    let req = body.req;
    let conn: Box<dyn FileConnector> = match req.protocol.as_str() {
        "sftp" => {
            let conn = rex_ssh::sftp::SftpConnector::connect_with_config(rex_ssh::SshConfig {
                host: req.host,
                port: req.port,
                username: req.username.unwrap_or_default(),
                password: req.password,
                private_key: req.private_key,
                keepalive_interval: req.keepalive_interval,
            })
            .await;
            match conn {
                Ok(c) => Box::new(c),
                Err(e) => {
                    return error_response("CONNECTION_FAILED", &e.to_string()).into_response()
                }
            }
        }
        "s3" => {
            let conn = rex_s3::S3Connector::connect_from_request(&req).await;
            match conn {
                Ok(c) => Box::new(c),
                Err(e) => {
                    return error_response("CONNECTION_FAILED", &e.to_string()).into_response()
                }
            }
        }
        _ => return error_response("UNSUPPORTED_PROTOCOL", "unsupported protocol").into_response(),
    };

    let session_id = format!("file_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    state
        .file_pool
        .lock()
        .await
        .insert(session_id.clone(), conn);
    tracing::info!(
        action = "FILE_CONNECT",
        session_id = %session_id,
        protocol = %req.protocol,
        "file session connected"
    );
    let audit_db = state.db.clone();
    let audit_session_id = session_id.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "FILE_CONNECT".into(),
            target: Some(audit_session_id),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;
    (StatusCode::OK, Json(ConnectResponse { session_id })).into_response()
}

async fn disconnect(
    State(state): State<AppState>,
    Json(body): Json<DisconnectBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    if let Some(mut conn) = pool.remove(&body.session_id) {
        let _ = conn.close().await;
        tracing::info!(
            action = "FILE_DISCONNECT",
            session_id = %body.session_id,
            "file session disconnected"
        );
        let audit_db = state.db.clone();
        let session_id = body.session_id.clone();
        let _ = tokio::task::spawn_blocking(move || {
            audit_db.write_audit_log(&crate::models::NewAuditEntry {
                action: "FILE_DISCONNECT".into(),
                target: Some(session_id),
                result: "success".into(),
                ..Default::default()
            })
        })
        .await;
        (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
    } else {
        error_response("SESSION_NOT_FOUND", "session not found").into_response()
    }
}

async fn list(
    State(state): State<AppState>,
    Query(params): Query<PathQuery>,
) -> axum::response::Response {
    tracing::debug!(action = "FILE_LIST", session_id = %params.session_id, path = %params.path, "file list");
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.list(&params.path).await {
        Ok(entries) => (StatusCode::OK, Json(entries)).into_response(),
        Err(e) => error_response("LIST_FAILED", &e.to_string()).into_response(),
    }
}

async fn stat(
    State(state): State<AppState>,
    Query(params): Query<PathQuery>,
) -> axum::response::Response {
    tracing::debug!(action = "FILE_STAT", session_id = %params.session_id, path = %params.path, "file stat");
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.stat(&params.path).await {
        Ok(entry) => (StatusCode::OK, Json(entry)).into_response(),
        Err(e) => error_response("STAT_FAILED", &e.to_string()).into_response(),
    }
}

async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> axum::response::Response {
    let mut session_id = String::new();
    let mut remote_path = String::new();
    let mut file_data: Option<Vec<u8>> = None;
    let mut offset: u64 = 0;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "session_id" => {
                session_id =
                    String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string();
            }
            "path" => {
                remote_path =
                    String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string();
            }
            "offset" => {
                if let Ok(v) = String::from_utf8_lossy(&field.bytes().await.unwrap_or_default())
                    .to_string()
                    .parse::<u64>()
                {
                    offset = v;
                }
            }
            "file" => {
                file_data = Some(field.bytes().await.unwrap_or_default().to_vec());
            }
            _ => {}
        }
    }

    let data = match file_data {
        Some(d) => d,
        None => return error_response("MISSING_FILE", "no file uploaded").into_response(),
    };

    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.upload(&remote_path, data, offset, None).await {
        Ok(result) => {
            tracing::info!(
                action = "FILE_OP",
                op = "upload",
                path = %remote_path,
                session_id = %session_id,
                "file uploaded"
            );
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "upload_id": result.upload_id
                })),
            )
                .into_response()
        }
        Err(e) => error_response("UPLOAD_FAILED", &e.to_string()).into_response(),
    }
}

async fn download(
    State(state): State<AppState>,
    Query(params): Query<PathQuery>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    // Check for Range header
    let range = headers.get("range").and_then(|v| v.to_str().ok());

    if let Some(range_str) = range {
        // Parse Range: bytes=offset-limit or bytes=offset-
        if let Some(range_val) = range_str.strip_prefix("bytes=") {
            let parts: Vec<&str> = range_val.splitn(2, '-').collect();
            if parts.len() == 2 {
                if let Ok(offset) = parts[0].parse::<u64>() {
                    let limit = if parts[1].is_empty() {
                        None // bytes=offset- → to end of file
                    } else {
                        parts[1].parse::<u64>().ok()
                    };
                    match conn.download_range(&params.path, offset, limit).await {
                        Ok(data) => {
                            let filename = params.path.rsplit('/').next().unwrap_or("file");
                            tracing::info!(
                                action = "FILE_OP",
                                op = "download",
                                session_id = %params.session_id,
                                path = %params.path,
                                "file downloaded"
                            );
                            let audit_db = state.db.clone();
                            
                            let download_path = params.path.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                                    action: "FILE_OP".into(),
                                    target: Some(download_path),
                                    detail: Some("op=download".into()),
                                    result: "success".into(),
                                    ..Default::default()
                                })
                            })
                            .await;
                            (
                                StatusCode::OK,
                                [
                                    ("Content-Type", "application/octet-stream"),
                                    (
                                        "Content-Disposition",
                                        &format!("attachment; filename=\"{filename}\""),
                                    ),
                                ],
                                data,
                            )
                                .into_response()
                        }
                        Err(e) => error_response("DOWNLOAD_FAILED", &e.to_string()).into_response(),
                    }
                } else {
                    error_response("INVALID_RANGE", "invalid range header").into_response()
                }
            } else {
                error_response("INVALID_RANGE", "invalid range header").into_response()
            }
        } else {
            error_response("INVALID_RANGE", "invalid range header").into_response()
        }
    } else {
        match conn.download(&params.path).await {
            Ok(data) => {
                let filename = params.path.rsplit('/').next().unwrap_or("file");
                tracing::info!(
                    action = "FILE_OP",
                    op = "download",
                    session_id = %params.session_id,
                    path = %params.path,
                    "file downloaded"
                );
                let audit_db = state.db.clone();
                
                let download_path = params.path.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    audit_db.write_audit_log(&crate::models::NewAuditEntry {
                        action: "FILE_OP".into(),
                        target: Some(download_path),
                        detail: Some("op=download".into()),
                        result: "success".into(),
                        ..Default::default()
                    })
                })
                .await;
                (
                    StatusCode::OK,
                    [
                        ("Content-Type", "application/octet-stream"),
                        (
                            "Content-Disposition",
                            &format!("attachment; filename=\"{filename}\""),
                        ),
                    ],
                    data,
                )
                    .into_response()
            }
            Err(e) => error_response("DOWNLOAD_FAILED", &e.to_string()).into_response(),
        }
    }
}

async fn read_for_edit(
    State(state): State<AppState>,
    Query(params): Query<PathQuery>,
) -> axum::response::Response {
    tracing::debug!(action = "FILE_READ_FOR_EDIT", session_id = %params.session_id, path = %params.path, "file read_for_edit");
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.read_for_edit(&params.path).await {
        Ok(data) => {
            let filename = params.path.rsplit('/').next().unwrap_or("file").to_string();
            let content = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "content": content,
                    "filename": filename,
                    "size": data.len(),
                })),
            )
                .into_response()
        }
        Err(e) => error_response("READ_FAILED", &e.to_string()).into_response(),
    }
}

async fn save_from_edit(
    State(state): State<AppState>,
    Json(body): Json<SaveFromEditBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match base64::engine::general_purpose::STANDARD.decode(&body.content) {
        Ok(data) => match conn.save_from_edit(&body.path, data).await {
            Ok(()) => {
                tracing::info!(
                    action = "FILE_OP",
                    op = "save_edit",
                    session_id = %body.session_id,
                    path = %body.path,
                    "file saved from edit"
                );
                let audit_db = state.db.clone();
                
                let save_path = body.path.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    audit_db.write_audit_log(&crate::models::NewAuditEntry {
                        action: "FILE_OP".into(),
                        target: Some(save_path),
                        detail: Some("op=save_edit".into()),
                        result: "success".into(),
                        ..Default::default()
                    })
                })
                .await;
                (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
            }
            Err(e) => error_response("SAVE_FAILED", &e.to_string()).into_response(),
        },
        Err(e) => {
            error_response("INVALID_CONTENT", &format!("invalid base64: {e}")).into_response()
        }
    }
}

async fn delete(
    State(state): State<AppState>,
    Json(body): Json<DeleteBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.delete(&body.path).await {
        Ok(()) => {
            tracing::info!(
                action = "FILE_OP",
                op = "delete",
                path = %body.path,
                session_id = %body.session_id,
                "file deleted"
            );
            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => error_response("DELETE_FAILED", &e.to_string()).into_response(),
    }
}

async fn rename(
    State(state): State<AppState>,
    Json(body): Json<RenameBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.rename(&body.from, &body.to).await {
        Ok(()) => {
            tracing::info!(
                action = "FILE_RENAME",
                session_id = %body.session_id,
                from = %body.from,
                to = %body.to,
                "file renamed"
            );
            let audit_db = state.db.clone();
            
            let rename_from = body.from.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "FILE_RENAME".into(),
                    target: Some(rename_from),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;
            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => error_response("RENAME_FAILED", &e.to_string()).into_response(),
    }
}

async fn mkdir(
    State(state): State<AppState>,
    Json(body): Json<MkdirBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.mkdir(&body.path).await {
        Ok(()) => {
            tracing::info!(
                action = "FILE_MKDIR",
                session_id = %body.session_id,
                path = %body.path,
                "directory created"
            );
            let audit_db = state.db.clone();
            
            let mkdir_path = body.path.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "FILE_MKDIR".into(),
                    target: Some(mkdir_path),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;
            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => error_response("MKDIR_FAILED", &e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PresignedUrlBody {
    session_id: String,
    path: String,
    #[serde(default = "default_expires")]
    expires_in: u64,
}

fn default_expires() -> u64 {
    3600
}

#[derive(Debug, Serialize)]
struct PresignedUrlResponse {
    url: String,
}

async fn presigned_url(
    State(state): State<AppState>,
    Json(body): Json<PresignedUrlBody>,
) -> axum::response::Response {
    tracing::debug!(action = "FILE_PRESIGNED_URL", session_id = %body.session_id, path = %body.path, expires = %body.expires_in, "file presigned_url");
    let pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    // Downcast to S3Connector to call presigned_url
    let s3_conn = match conn.as_any().downcast_ref::<rex_s3::S3Connector>() {
        Some(c) => c,
        None => {
            return error_response(
                "UNSUPPORTED_PROTOCOL",
                "presigned URL only supported for S3",
            )
            .into_response()
        }
    };

    match s3_conn.presigned_url(&body.path, body.expires_in).await {
        Ok(url) => (StatusCode::OK, Json(PresignedUrlResponse { url })).into_response(),
        Err(e) => error_response("PRESIGNED_URL_FAILED", &e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ListMultipartUploadsQuery {
    session_id: String,
    prefix: String,
}

#[derive(Debug, Serialize)]
struct MultipartUploadInfo {
    key: String,
    upload_id: String,
}

#[derive(Debug, Serialize)]
struct ListMultipartUploadsResponse {
    uploads: Vec<MultipartUploadInfo>,
}

async fn list_multipart_uploads(
    State(state): State<AppState>,
    Query(params): Query<ListMultipartUploadsQuery>,
) -> axum::response::Response {
    tracing::debug!(action = "FILE_LIST_MULTIPART", session_id = %params.session_id, "file list_multipart_uploads");
    let pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    let s3_conn = match conn.as_any().downcast_ref::<rex_s3::S3Connector>() {
        Some(c) => c,
        None => {
            return error_response("UNSUPPORTED_PROTOCOL", "only supported for S3").into_response()
        }
    };

    match s3_conn.list_multipart_uploads(&params.prefix).await {
        Ok(uploads) => {
            let resp = ListMultipartUploadsResponse {
                uploads: uploads
                    .into_iter()
                    .map(|(key, upload_id)| MultipartUploadInfo { key, upload_id })
                    .collect(),
            };
            (StatusCode::OK, Json(resp)).into_response()
        }
        Err(e) => error_response("LIST_UPLOADS_FAILED", &e.to_string()).into_response(),
    }
}

async fn resume_multipart_upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> axum::response::Response {
    let mut session_id = String::new();
    let mut remote_path = String::new();
    let mut upload_id = String::new();
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "session_id" => {
                session_id =
                    String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string();
            }
            "path" => {
                remote_path =
                    String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string();
            }
            "upload_id" => {
                upload_id =
                    String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string();
            }
            "file" => {
                file_data = Some(field.bytes().await.unwrap_or_default().to_vec());
            }
            _ => {}
        }
    }

    let data = match file_data {
        Some(d) => d,
        None => return error_response("MISSING_FILE", "no file uploaded").into_response(),
    };

    tracing::info!(action = "FILE_RESUME_MULTIPART", session_id = %session_id, "file resume_multipart_upload");
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    let s3_conn = match conn.as_any_mut().downcast_mut::<rex_s3::S3Connector>() {
        Some(c) => c,
        None => {
            return error_response("UNSUPPORTED_PROTOCOL", "only supported for S3").into_response()
        }
    };

    match s3_conn
        .resume_multipart_upload(&remote_path, &upload_id, data, None)
        .await
    {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => error_response("RESUME_UPLOAD_FAILED", &e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct AbortMultipartUploadBody {
    session_id: String,
    path: String,
    upload_id: String,
}

async fn abort_multipart_upload(
    State(state): State<AppState>,
    Json(body): Json<AbortMultipartUploadBody>,
) -> axum::response::Response {
    tracing::info!(action = "FILE_ABORT_MULTIPART", session_id = %body.session_id, "file abort_multipart_upload");
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    let s3_conn = match conn.as_any_mut().downcast_mut::<rex_s3::S3Connector>() {
        Some(c) => c,
        None => {
            return error_response("UNSUPPORTED_PROTOCOL", "only supported for S3").into_response()
        }
    };

    match s3_conn
        .abort_multipart_upload(&body.path, &body.upload_id)
        .await
    {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => error_response("ABORT_UPLOAD_FAILED", &e.to_string()).into_response(),
    }
}

// ---------------------------------------------------------------------------
// ACL handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct AclResponse {
    acl: String,
}

async fn get_acl(
    State(state): State<AppState>,
    Query(params): Query<PathQuery>,
) -> axum::response::Response {
    let pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    let s3_conn = match conn.as_any().downcast_ref::<rex_s3::S3Connector>() {
        Some(c) => c,
        None => {
            return error_response("UNSUPPORTED_PROTOCOL", "only supported for S3").into_response()
        }
    };

    match s3_conn.get_acl(&params.path).await {
        Ok(acl) => {
            tracing::info!(
                action = "FILE_ACL",
                session_id = %params.session_id,
                path = %params.path,
                "ACL retrieved"
            );
            let audit_db = state.db.clone();
            
            let acl_path = params.path.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "FILE_ACL".into(),
                    target: Some(acl_path),
                    detail: Some("op=get_acl".into()),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;
            (StatusCode::OK, Json(AclResponse { acl })).into_response()
        }
        Err(e) => error_response("GET_ACL_FAILED", &e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PutAclBody {
    session_id: String,
    path: String,
    acl: String,
}

async fn put_acl(
    State(state): State<AppState>,
    Json(body): Json<PutAclBody>,
) -> axum::response::Response {
    let mut pool = state.file_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    let s3_conn = match conn.as_any_mut().downcast_mut::<rex_s3::S3Connector>() {
        Some(c) => c,
        None => {
            return error_response("UNSUPPORTED_PROTOCOL", "only supported for S3").into_response()
        }
    };

    match s3_conn.put_acl(&body.path, &body.acl).await {
        Ok(()) => {
            tracing::info!(
                action = "FILE_ACL",
                session_id = %body.session_id,
                path = %body.path,
                "ACL applied"
            );
            let audit_db = state.db.clone();
            
            let acl_path = body.path.clone();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "FILE_ACL".into(),
                    target: Some(acl_path),
                    detail: Some("op=put_acl".into()),
                    result: "success".into(),
                    ..Default::default()
                })
            })
            .await;
            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => error_response("PUT_ACL_FAILED", &e.to_string()).into_response(),
    }
}
