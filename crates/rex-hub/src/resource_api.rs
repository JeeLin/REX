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
        .route(
            "/{env_id}/resources",
            axum::routing::get(list_resources).post(create_resource),
        )
        .route(
            "/{env_id}/resources/{resource_id}",
            axum::routing::get(get_resource)
                .put(update_resource)
                .delete(delete_resource),
        )
        .route(
            "/{env_id}/resources/{resource_id}/active-account",
            axum::routing::post(set_active_account),
        )
}

// --- API handlers ---

async fn list_resources(
    State(state): State<AppState>,
    Path(env_id): Path<String>,
) -> ApiResult<Vec<Resource>> {
    let db = state.db.clone();
    let mut resources = tokio::task::spawn_blocking(move || db.list_resources_by_env(&env_id))
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
    Path((_env_id, resource_id)): Path<(String, String)>,
) -> ApiResult<Resource> {
    let db = state.db.clone();
    let mut resource = tokio::task::spawn_blocking(move || db.get_resource(&resource_id))
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
    Path(env_id): Path<String>,
    Json(mut body): Json<NewResource>,
) -> ApiResult<Resource> {
    tracing::info!(
        action = "RESOURCE_CREATE",
        env_id = %env_id,
        protocol = %body.protocol,
        name = %body.name,
        host = %body.host,
        "creating resource"
    );

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
    let env_id_check = env_id.clone();
    let env_exists = tokio::task::spawn_blocking(move || db.get_environment(&env_id_check))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .is_some();
    if !env_exists {
        return Err(err(StatusCode::NOT_FOUND, "environment not found"));
    }
    let db = state.db.clone();
    let resource = tokio::task::spawn_blocking(move || db.create_resource(&env_id, &body))
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
    Path((env_id, resource_id)): Path<(String, String)>,
    Json(mut body): Json<NewResource>,
) -> ApiResult<Resource> {
    tracing::info!(
        action = "RESOURCE_UPDATE",
        env_id = %env_id,
        resource_id = %resource_id,
        protocol = %body.protocol,
        name = %body.name,
        "updating resource"
    );

    // 加密 config_json 中的凭据
    if let Some(ref cfg) = body.config_json {
        match state.crypto.encrypt(cfg) {
            Ok(enc) => body.config_json = Some(enc),
            Err(e) => return Err(err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())),
        }
    }
    let db = state.db.clone();
    let resource =
        tokio::task::spawn_blocking(move || db.update_resource(&env_id, &resource_id, &body))
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

    // 审计日志
    let audit_db = state.db.clone();
    let res_name = resource.name.clone();
    let res_env_id = resource.environment_id.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_UPDATE".into(),
            target: Some(res_name),
            environment_id: Some(res_env_id),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;

    Ok(Json(resource))
}

#[derive(serde::Deserialize)]
struct SetActiveAccountBody {
    account_id: String,
}

// 专用端点：仅切换 SIP 资源的生效账户，前端无需先 get 全量再 update。
async fn set_active_account(
    State(state): State<AppState>,
    Path((env_id, resource_id)): Path<(String, String)>,
    Json(body): Json<SetActiveAccountBody>,
) -> ApiResult<Resource> {
    tracing::info!(
        action = "RESOURCE_SET_ACTIVE_ACCOUNT",
        env_id = %env_id,
        resource_id = %resource_id,
        account_id = %body.account_id,
        "switching active sip account"
    );

    let db = state.db.clone();
    let crypto = state.crypto.clone();
    let account_id = body.account_id.clone();
    let resource = tokio::task::spawn_blocking(move || {
        db.set_resource_active_account(&crypto, &env_id, &resource_id, &account_id)
    })
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("not found") {
            err(StatusCode::NOT_FOUND, &msg)
        } else {
            err(StatusCode::BAD_REQUEST, &msg)
        }
    })?;

    // 审计日志：后台异步写入，不阻塞响应返回（fire-and-forget）。
    let audit_db = state.db.clone();
    let res_name = resource.name.clone();
    let res_env_id = resource.environment_id.clone();
    tokio::task::spawn_blocking(move || {
        let _ = audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_SET_ACTIVE_ACCOUNT".into(),
            target: Some(res_name),
            environment_id: Some(res_env_id),
            result: "success".into(),
            ..Default::default()
        });
    });

    Ok(Json(resource))
}

async fn delete_resource(
    State(state): State<AppState>,
    Path((env_id, resource_id)): Path<(String, String)>,
) -> ApiResult<serde_json::Value> {
    let db = state.db.clone();
    let check_id = resource_id.clone();
    let resource = tokio::task::spawn_blocking(move || db.get_resource(&check_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    if resource.is_none() {
        return Err(err(StatusCode::NOT_FOUND, "resource not found"));
    }
    let res_name = resource.map(|r| r.name).unwrap_or_default();

    let db = state.db.clone();
    tracing::info!(
        action = "RESOURCE_DELETE",
        env_id = %env_id,
        resource_id = %resource_id,
        resource_name = %res_name,
        "deleting resource"
    );

    let del_env_id = env_id.clone();
    let del_id = resource_id.clone();
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
    pub environment_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct TestConnectionResult {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

pub async fn test_connection(
    State(state): State<crate::AppState>,
    Json(body): Json<TestConnectionRequest>,
) -> ApiResult<TestConnectionResult> {
    // For S3, log endpoint from config_json instead of empty host
    let log_host = if body.protocol == "s3" {
        body.config_json
            .as_ref()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
            .and_then(|v| v.get("endpoint")?.as_str().map(String::from))
            .unwrap_or_default()
    } else {
        body.host.clone()
    };
    tracing::info!(
        action = "TEST_CONNECTION",
        protocol = %body.protocol,
        host = %log_host,
        port = body.port.unwrap_or(0),
        "testing connection"
    );

    let start = std::time::Instant::now();
    let result = match body.protocol.as_str() {
        "ssh" | "sftp" => {
            let host = body.host.clone();
            let port = body.port.unwrap_or(22);

            // Check if this environment uses agent proxy
            let use_agent = if let Some(ref env_id) = body.environment_id {
                let db = state.db.clone();
                let eid = env_id.clone();
                tokio::task::spawn_blocking(move || db.get_environment(&eid))
                    .await
                    .ok()
                    .and_then(|r| r.ok())
                    .flatten()
                    .map(|env| env.connection_mode == "agent")
                    .unwrap_or(false)
            } else {
                false
            };

            if use_agent {
                // Route through agent tunnel
                let env_id = body.environment_id.as_deref().unwrap_or("");
                let db = state.db.clone();
                let eid = env_id.to_string();
                let agent_result = tokio::task::spawn_blocking(move || {
                    let agents = db.list_agents_by_env(&eid).unwrap_or_default();
                    agents
                        .into_iter()
                        .find(|a| a.status == "online")
                        .map(|a| a.id)
                })
                .await
                .ok()
                .flatten();

                match agent_result {
                    Some(agent_id) => {
                        // Get agent WebSocket connection
                        let agent_conn = {
                            let conns = state.agent_tunnel.connections.read().await;
                            conns.get(&agent_id).cloned()
                        };
                        match agent_conn {
                            Some(conn) => {
                                // Send connect request via agent tunnel
                                let request_id =
                                    format!("req_{}", &uuid::Uuid::new_v4().to_string()[..8]);
                                let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                                {
                                    let mut pending =
                                        state.agent_tunnel.pending_requests.write().await;
                                    pending.insert(request_id.clone(), resp_tx);
                                }
                                let connect_msg = serde_json::json!({
                                    "type": "connect",
                                    "payload": {
                                        "request_id": request_id,
                                        "resource_id": "test",
                                        "protocol": body.protocol,
                                        "config": {
                                            "host": host,
                                            "port": port,
                                        }
                                    }
                                });
                                if conn
                                    .sender
                                    .send(crate::agent_ws::AgentEvent::Text(
                                        connect_msg.to_string(),
                                    ))
                                    .await
                                    .is_err()
                                {
                                    Err("failed to send connect request to agent".into())
                                } else {
                                    // Wait for agent response (5s timeout)
                                    match tokio::time::timeout(
                                        std::time::Duration::from_secs(5),
                                        resp_rx,
                                    )
                                    .await
                                    {
                                        Ok(Ok(resp)) => {
                                            if resp.error.is_some() {
                                                Err(resp
                                                    .error
                                                    .unwrap_or("agent connect failed".into()))
                                            } else {
                                                // Connection successful, notify agent to close the channel
                                                if let Some(channel_id) = &resp.channel_id {
                                                    // Send close message to agent
                                                    let close_msg = serde_json::json!({
                                                        "type": "close",
                                                        "payload": {
                                                            "channel_id": channel_id
                                                        }
                                                    });
                                                    let _ = conn
                                                        .sender
                                                        .send(crate::agent_ws::AgentEvent::Text(
                                                            close_msg.to_string(),
                                                        ))
                                                        .await;
                                                    // Remove from local channel map
                                                    let mut channels =
                                                        state.agent_tunnel.channels.write().await;
                                                    channels.remove(channel_id);
                                                }
                                                Ok(())
                                            }
                                        }
                                        Ok(Err(_)) => Err("agent response channel closed".into()),
                                        Err(_) => Err("agent connection timed out".into()),
                                    }
                                }
                            }
                            None => Err("agent not connected".into()),
                        }
                    }
                    None => Err("no online agent available".into()),
                }
            } else {
                // Direct TCP connection
                let addr = if host.contains(':') {
                    format!("[{host}]:{port}")
                } else {
                    format!("{host}:{port}")
                };
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    tokio::net::TcpStream::connect(&addr),
                )
                .await
                {
                    Ok(Ok(_)) => Ok(()),
                    Ok(Err(e)) => Err(format!("TCP connect failed: {e}")),
                    Err(_) => Err("connection timed out".into()),
                }
            }
        }
        "redis" => {
            let redis_host = if body.host.contains(':') {
                format!("[{}]", body.host)
            } else {
                body.host.clone()
            };
            let addr = format!("redis://{}:{}/", redis_host, body.port.unwrap_or(6379));
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
            let addr = if host.contains(':') {
                format!("[{host}]:{port}")
            } else {
                format!("{host}:{port}")
            };
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                tokio::net::TcpStream::connect(&addr),
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
        "sip" => {
            // SIP 测试连接：校验 config_json（SipProfile）解析后能选出生效账户
            // （账户自带 server + 生效账户 username）。复用 load_sip_conn，校验逻辑与信令注册一致。
            // 匿名注册（无 password）也是合法的，故 password 非必填。
            // 真正的 REGISTER 拨测在 /ws/sip 信令联调中完成。
            match &body.config_json {
                Some(cfg) => match serde_json::from_str::<serde_json::Value>(cfg) {
                    Ok(value) => {
                        // SIP 的 server/port 完全下沉到账户层，load_sip_conn 不读取顶层
                        // host/port/username（子任务 #1 已移除回退），故此处仅传 config。
                        let info = crate::resource_conn::ResourceConnInfo {
                            resource_id: String::new(),
                            name: String::new(),
                            protocol: "sip".into(),
                            host: String::new(),
                            port: None,
                            username: String::new(),
                            config: value,
                        };
                        match crate::resource_conn::load_sip_conn(&info) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(format!("invalid SIP config: {e}")),
                        }
                    }
                    Err(e) => Err(format!("invalid config_json: {e}")),
                },
                None => Err("missing config_json for SIP".into()),
            }
        }
        _ => Err(format!("unsupported protocol: {}", body.protocol)),
    };
    let latency = start.elapsed().as_millis() as u64;
    let ok = result.is_ok();
    let err_msg = match &result {
        Ok(()) => None,
        Err(e) => Some(e.clone()),
    };

    tracing::info!(
        action = "TEST_CONNECTION",
        protocol = %body.protocol,
        host = %body.host,
        ok = ok,
        latency_ms = latency,
        error = err_msg.as_deref().unwrap_or(""),
        "connection test completed"
    );

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
