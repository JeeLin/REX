//! 文件管理 REST 路由。

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Multipart, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use rex_common::file_transfer::{FileConnectRequest, FileConnector, FileEntry};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pub type FileState = Arc<Mutex<FileConnectionPool>>;

pub struct FileConnectionPool {
    connectors: HashMap<String, Box<dyn FileConnector>>,
}

impl FileConnectionPool {
    pub fn new() -> Self {
        Self { connectors: HashMap::new() }
    }
    pub fn insert(&mut self, id: String, conn: Box<dyn FileConnector>) {
        self.connectors.insert(id, conn);
    }
    pub fn remove(&mut self, id: &str) -> Option<Box<dyn FileConnector>> {
        self.connectors.remove(id)
    }
}

pub fn file_routes() -> axum::Router<FileState> {
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
struct ConnectResponse { session_id: String }

#[derive(Debug, Deserialize)]
struct DisconnectBody { session_id: String }

#[derive(Debug, Deserialize)]
struct SessionQuery { session_id: String }

#[derive(Debug, Deserialize)]
struct PathQuery { session_id: String, path: String }

#[derive(Debug, Deserialize)]
struct DeleteBody { session_id: String, path: String }

#[derive(Debug, Deserialize)]
struct RenameBody { session_id: String, from: String, to: String }

#[derive(Debug, Deserialize)]
struct MkdirBody { session_id: String, path: String }

#[derive(Debug, Serialize)]
struct ErrorBody { error: ErrorDetail }

#[derive(Debug, Serialize)]
struct ErrorDetail { code: String, message: String }

fn error_response(code: &str, message: &str) -> (StatusCode, Json<ErrorBody>) {
    (StatusCode::BAD_REQUEST, Json(ErrorBody {
        error: ErrorDetail { code: code.to_string(), message: message.to_string() },
    }))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn connect(
    State(state): State<FileState>,
    Json(body): Json<ConnectBody>,
) -> axum::response::Response {
    let req = body.req;
    let conn: Box<dyn FileConnector> = match req.protocol.as_str() {
        "sftp" => {
            let conn = rex_ssh::sftp::SftpConnector::connect_with_config(
                rex_ssh::SshConfig {
                    host: req.host,
                    port: req.port,
                    username: req.username.unwrap_or_default(),
                    password: req.password,
                    private_key: req.private_key,
                }
            ).await;
            match conn { Ok(c) => Box::new(c), Err(e) => return error_response("CONNECTION_FAILED", &e.to_string()).into_response() }
        }
        "s3" => {
            let conn = rex_s3::S3Connector::connect_from_request(&req).await;
            match conn { Ok(c) => Box::new(c), Err(e) => return error_response("CONNECTION_FAILED", &e.to_string()).into_response() }
        }
        _ => return error_response("UNSUPPORTED_PROTOCOL", "unsupported protocol").into_response(),
    };

    let session_id = format!("file_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    state.lock().await.insert(session_id.clone(), conn);
    (StatusCode::OK, Json(ConnectResponse { session_id })).into_response()
}

async fn disconnect(
    State(state): State<FileState>,
    Json(body): Json<DisconnectBody>,
) -> axum::response::Response {
    let mut pool = state.lock().await;
    if let Some(mut conn) = pool.remove(&body.session_id) {
        let _ = conn.close().await;
        (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
    } else {
        error_response("SESSION_NOT_FOUND", "session not found").into_response()
    }
}

async fn list(
    State(state): State<FileState>,
    Query(params): Query<PathQuery>,
) -> axum::response::Response {
    let mut pool = state.lock().await;
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
    State(state): State<FileState>,
    Query(params): Query<PathQuery>,
) -> axum::response::Response {
    let mut pool = state.lock().await;
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
    State(state): State<FileState>,
    mut multipart: Multipart,
) -> axum::response::Response {
    let mut session_id = String::new();
    let mut remote_path = String::new();
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "session_id" => {
                session_id = String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string();
            }
            "path" => {
                remote_path = String::from_utf8_lossy(&field.bytes().await.unwrap_or_default()).to_string();
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

    let mut pool = state.lock().await;
    let conn = match pool.connectors.get_mut(&session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.upload(&remote_path, data, None).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => error_response("UPLOAD_FAILED", &e.to_string()).into_response(),
    }
}

async fn download(
    State(state): State<FileState>,
    Query(params): Query<PathQuery>,
) -> axum::response::Response {
    let mut pool = state.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.download(&params.path).await {
        Ok(data) => {
            let filename = params.path.rsplit('/').next().unwrap_or("file");
            (
                StatusCode::OK,
                [("Content-Type", "application/octet-stream"),
                 ("Content-Disposition", &format!("attachment; filename=\"{filename}\""))],
                data,
            ).into_response()
        }
        Err(e) => error_response("DOWNLOAD_FAILED", &e.to_string()).into_response(),
    }
}

async fn delete(
    State(state): State<FileState>,
    Json(body): Json<DeleteBody>,
) -> axum::response::Response {
    let mut pool = state.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.delete(&body.path).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => error_response("DELETE_FAILED", &e.to_string()).into_response(),
    }
}

async fn rename(
    State(state): State<FileState>,
    Json(body): Json<RenameBody>,
) -> axum::response::Response {
    let mut pool = state.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.rename(&body.from, &body.to).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => error_response("RENAME_FAILED", &e.to_string()).into_response(),
    }
}

async fn mkdir(
    State(state): State<FileState>,
    Json(body): Json<MkdirBody>,
) -> axum::response::Response {
    let mut pool = state.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };
    match conn.mkdir(&body.path).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => error_response("MKDIR_FAILED", &e.to_string()).into_response(),
    }
}
