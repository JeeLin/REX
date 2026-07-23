//! 资源管理 REST API。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::models::{NewResource, Resource};
use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })),
    )
}

/// 资源路由（嵌套在 /api/environments 下）
pub fn resource_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/{id}/resources", axum::routing::get(list_resources))
        .route("/{id}/resources", axum::routing::post(create_resource))
        .route("/{id}/resources/{rid}", axum::routing::get(get_resource))
        .route(
            "/{id}/resources/{rid}",
            axum::routing::put(update_resource),
        )
        .route(
            "/{id}/resources/{rid}",
            axum::routing::delete(delete_resource),
        )
}

// --- API handlers ---

async fn list_resources(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Vec<Resource>> {
    let db = state.db.clone();
    let mut resources = tokio::task::spawn_blocking(move || db.list_resources_by_env(&id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    // 解密每条资源的 config_json
    for r in &mut resources {
        if !r.config_json.is_empty() && r.config_json != "{}" {
            if let Ok(dec) = state.crypto.decrypt(&r.config_json) {
                r.config_json = dec;
            }
        }
    }
    Ok(Json(resources))
}

async fn get_resource(
    State(state): State<AppState>,
    Path((_id, rid)): Path<(String, String)>,
) -> ApiResult<Resource> {
    let db = state.db.clone();
    let mut resource = tokio::task::spawn_blocking(move || db.get_resource(&rid))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "resource not found"))?;
    // 解密 config_json
    if !resource.config_json.is_empty() && resource.config_json != "{}" {
        if let Ok(dec) = state.crypto.decrypt(&resource.config_json) {
            resource.config_json = dec;
        }
    }
    Ok(Json(resource))
}

async fn create_resource(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut body): Json<NewResource>,
) -> ApiResult<Resource> {
    if body.name.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "name is required"));
    }
    if body.host.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "host is required"));
    }
    // 加密 config_json 中的凭据
    if let Some(ref cfg) = body.config_json {
        match state.crypto.encrypt(cfg) {
            Ok(enc) => body.config_json = Some(enc),
            Err(e) => return Err(err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())),
        }
    }
    // 验证环境存在
    let db = state.db.clone();
    let env_id_check = id.clone();
    let env_exists = tokio::task::spawn_blocking(move || db.get_environment(&env_id_check))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .is_some();
    if !env_exists {
        return Err(err(StatusCode::NOT_FOUND, "environment not found"));
    }
    let db = state.db.clone();
    let resource = tokio::task::spawn_blocking(move || db.create_resource(&id, &body))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    // 审计日志
    let audit_db = state.db.clone();
    let res_name = resource.name.clone();
    let res_env_id = resource.environment_id.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_CREATE".into(),
            target: Some(res_name),
            environment_id: Some(res_env_id),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;
    Ok(Json(resource))
}

async fn update_resource(
    State(state): State<AppState>,
    Path((id, rid)): Path<(String, String)>,
    Json(mut body): Json<NewResource>,
) -> ApiResult<Resource> {
    // 加密 config_json 中的凭据
    if let Some(ref cfg) = body.config_json {
        match state.crypto.encrypt(cfg) {
            Ok(enc) => body.config_json = Some(enc),
            Err(e) => return Err(err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())),
        }
    }
    let db = state.db.clone();
    let resource = tokio::task::spawn_blocking(move || db.update_resource(&id, &rid, &body))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") {
                err(StatusCode::NOT_FOUND, &msg)
            } else {
                err(StatusCode::INTERNAL_SERVER_ERROR, &msg)
            }
        })?;
    Ok(Json(resource))
}

async fn delete_resource(
    State(state): State<AppState>,
    Path((env_id, rid)): Path<(String, String)>,
) -> ApiResult<serde_json::Value> {
    let db = state.db.clone();
    let check_id = rid.clone();
    let resource = tokio::task::spawn_blocking(move || db.get_resource(&check_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    if resource.is_none() {
        return Err(err(StatusCode::NOT_FOUND, "resource not found"));
    }
    let res_name = resource.map(|r| r.name).unwrap_or_default();

    let db = state.db.clone();
    let del_env_id = env_id.clone();
    let del_id = rid.clone();
    tokio::task::spawn_blocking(move || db.delete_resource(&del_env_id, &del_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    // 审计日志
    let audit_db = state.db.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_DELETE".into(),
            target: Some(res_name),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// --- Test connection ---

#[derive(serde::Deserialize)]
pub struct TestConnectionRequest {
    pub protocol: String,
    pub host: String,
    pub port: Option<u16>,
    #[allow(dead_code)]
    pub username: Option<String>,
    pub config_json: Option<String>,
}

#[derive(serde::Serialize)]
pub struct TestConnectionResult {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

pub async fn test_connection(
    Json(body): Json<TestConnectionRequest>,
) -> ApiResult<TestConnectionResult> {
    let start = std::time::Instant::now();
    let result = match body.protocol.as_str() {
        "ssh" | "sftp" => {
            let host = body.host.clone();
            let port = body.port.unwrap_or(22);
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                tokio::net::TcpStream::connect(format!("{host}:{port}")),
            )
            .await
            {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(e)) => Err(format!("TCP connect failed: {e}")),
                Err(_) => Err("connection timed out".into()),
            }
        }
        "redis" => {
            let addr = format!("redis://{}:{}/", body.host, body.port.unwrap_or(6379));
            match tokio::time::timeout(std::time::Duration::from_secs(5), async {
                let client =
                    redis::Client::open(addr.as_str()).map_err(|e| format!("redis error: {e}"))?;
                let mut conn = client
                    .get_multiplexed_async_connection()
                    .await
                    .map_err(|e| format!("redis connect error: {e}"))?;
                redis::Cmd::new()
                    .arg("PING")
                    .query_async::<String>(&mut conn)
                    .await
                    .map_err(|e| format!("redis PING failed: {e}"))?;
                Ok::<(), String>(())
            })
            .await
            {
                Ok(r) => r,
                Err(_) => Err("connection timed out".into()),
            }
        }
        "sqlite" => {
            let path = body
                .config_json
                .as_ref()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                .and_then(|v| v.get("file_path")?.as_str().map(String::from))
                .unwrap_or_else(|| ":memory:".into());
            match rusqlite::Connection::open(&path) {
                Ok(conn) => {
                    if conn.execute_batch("SELECT 1").is_ok() {
                        Ok(())
                    } else {
                        Err("SQLite query failed".into())
                    }
                }
                Err(e) => Err(format!("SQLite open failed: {e}")),
            }
        }
        "mysql" | "postgresql" => {
            let host = body.host.clone();
            let port = body
                .port
                .unwrap_or(if body.protocol == "mysql" { 3306 } else { 5432 });
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                tokio::net::TcpStream::connect(format!("{host}:{port}")),
            )
            .await
            {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(e)) => Err(format!("TCP connect failed: {e}")),
                Err(_) => Err("connection timed out".into()),
            }
        }
        "s3" => match body.config_json {
            Some(ref cfg) => {
                let v: serde_json::Value =
                    serde_json::from_str(cfg).unwrap_or(serde_json::Value::Null);
                let endpoint = v.get("endpoint").and_then(|e| e.as_str()).unwrap_or("");
                let access_key = v.get("access_key").and_then(|e| e.as_str()).unwrap_or("");
                let secret_key = v.get("secret_key").and_then(|e| e.as_str()).unwrap_or("");
                let region = v
                    .get("region")
                    .and_then(|e| e.as_str())
                    .unwrap_or("us-east-1");
                if endpoint.is_empty() || access_key.is_empty() || secret_key.is_empty() {
                    Err("missing endpoint, access_key, or secret_key".into())
                } else {
                    let config = aws_sdk_s3::Config::builder()
                        .endpoint_url(endpoint)
                        .region(aws_sdk_s3::config::Region::new(region.to_string()))
                        .credentials_provider(aws_sdk_s3::config::Credentials::new(
                            access_key.to_string(),
                            secret_key.to_string(),
                            None,
                            None,
                            "rex-hub-test",
                        ))
                        .behavior_version_latest()
                        .build();
                    let client = aws_sdk_s3::Client::from_conf(config);
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        client.list_buckets().send(),
                    )
                    .await
                    {
                        Ok(Ok(_)) => Ok(()),
                        Ok(Err(e)) => Err(format!("S3 ListBuckets failed: {e}")),
                        Err(_) => Err("S3 request timed out".into()),
                    }
                }
            }
            None => Err("missing config_json for S3".into()),
        },
        _ => Err(format!("unsupported protocol: {}", body.protocol)),
    };
    let latency = start.elapsed().as_millis() as u64;
    match result {
        Ok(()) => Ok(Json(TestConnectionResult {
            ok: true,
            latency_ms: Some(latency),
            error: None,
        })),
        Err(e) => Ok(Json(TestConnectionResult {
            ok: false,
            latency_ms: Some(latency),
            error: Some(e),
        })),
    }
}
