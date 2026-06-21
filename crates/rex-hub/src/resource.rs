use axum::extract::{Path, State};
use axum::http::{header::HeaderMap, StatusCode};
use axum::Json;
use rex_common::sql::SqlConnector;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::audit::write_audit_log;
use crate::helpers::{
    bad_request, err_resp, gen_id, not_found, now_iso, ApiResponse, ErrorResponse,
};
use crate::routes::{extract_client_ip, AppState};

#[derive(Debug, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub protocol: String,
    pub agent_id: Option<String>,
    pub config_json: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateResource {
    pub name: String,
    pub protocol: String,
    pub agent_id: Option<String>,
    pub config_json: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateResource {
    pub name: Option<String>,
    pub config_json: Option<String>,
}

const VALID_PROTOCOLS: &[&str] = &[
    "ssh",
    "sftp",
    "mysql",
    "postgresql",
    "redis",
    "docker",
    "sqlite",
    "s3",
];

pub async fn list_resources(
    State(state): State<Arc<AppState>>,
    Path(env_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Resource>>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let resources = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let mut stmt = conn.prepare(
            "SELECT id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at FROM resources WHERE environment_id = ?1 ORDER BY created_at DESC",
        ).map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let rows = stmt.query_map(rusqlite::params![env_id], |row| {
            Ok(Resource {
                id: row.get(0)?, environment_id: row.get(1)?, name: row.get(2)?,
                protocol: row.get(3)?, agent_id: row.get(4)?,
                config_json: row.get(5)?, status: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
            })
        }).map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let resources: Vec<Resource> = rows.filter_map(|r| r.ok()).collect();
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(resources)
    }).await.map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: resources }))
}

pub async fn create_resource(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(env_id): Path<String>,
    Json(input): Json<CreateResource>,
) -> Result<(StatusCode, Json<ApiResponse<Resource>>), (StatusCode, Json<ErrorResponse>)> {
    if input.name.trim().is_empty() {
        return Err(bad_request("资源名称不能为空"));
    }
    if !VALID_PROTOCOLS.contains(&input.protocol.as_str()) {
        return Err(bad_request(&format!("不支持的协议: {}", input.protocol)));
    }

    let ip = extract_client_ip(&headers);
    let db = state.db.clone();
    let env_id_clone = env_id.clone();

    let resource = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let env_exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM environments WHERE id = ?1",
            rusqlite::params![env_id_clone], |row| row.get::<_, i64>(0),
        ).map(|count| count > 0).map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        if !env_exists { return Err(not_found("ENVIRONMENT_NOT_FOUND", "环境不存在")); }

        let id = gen_id("res");
        let now = now_iso();
        conn.execute(
            "INSERT INTO resources (id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![id, env_id_clone, input.name, input.protocol, input.agent_id, input.config_json, "ready", now.clone(), now],
        ).map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(Resource {
            id, environment_id: env_id_clone, name: input.name, protocol: input.protocol,
            agent_id: input.agent_id, config_json: input.config_json,
            status: "ready".to_string(), created_at: now.clone(), updated_at: now,
        })
    }).await.map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    write_audit_log(
        &state.db,
        "resource_create",
        "success",
        &format!("在环境 {} 中创建资源「{}」", env_id, resource.name),
        Some(&env_id),
        Some(&resource.id),
        resource.agent_id.as_deref(),
        None,
        Some(&ip),
    );

    Ok((StatusCode::CREATED, Json(ApiResponse { data: resource })))
}

pub async fn get_resource(
    State(state): State<Arc<AppState>>,
    Path((env_id, id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Resource>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let resource = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        conn.query_row(
            "SELECT id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at FROM resources WHERE id = ?1 AND environment_id = ?2",
            rusqlite::params![id, env_id],
            |row| Ok(Resource {
                id: row.get(0)?, environment_id: row.get(1)?, name: row.get(2)?,
                protocol: row.get(3)?, agent_id: row.get(4)?,
                config_json: row.get(5)?, status: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
            }),
        ).map_err(|_| not_found("RESOURCE_NOT_FOUND", "资源不存在"))
    }).await.map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: resource }))
}

pub async fn update_resource(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((env_id, id)): Path<(String, String)>,
    Json(input): Json<UpdateResource>,
) -> Result<Json<ApiResponse<Resource>>, (StatusCode, Json<ErrorResponse>)> {
    let ip = extract_client_ip(&headers);
    let db = state.db.clone();
    let id_clone = id.clone();
    let env_id_clone = env_id.clone();

    let resource = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let existing: Resource = conn.query_row(
            "SELECT id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at FROM resources WHERE id = ?1 AND environment_id = ?2",
            rusqlite::params![id_clone, env_id_clone],
            |row| Ok(Resource {
                id: row.get(0)?, environment_id: row.get(1)?, name: row.get(2)?,
                protocol: row.get(3)?, agent_id: row.get(4)?,
                config_json: row.get(5)?, status: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
            }),
        ).map_err(|_| not_found("RESOURCE_NOT_FOUND", "资源不存在"))?;
        let name = input.name.unwrap_or(existing.name);
        let config_json = input.config_json.unwrap_or(existing.config_json);
        let now = now_iso();
        conn.execute(
            "UPDATE resources SET name = ?1, config_json = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![name, config_json, now, id_clone],
        ).map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(Resource {
            id: existing.id, environment_id: existing.environment_id, name,
            protocol: existing.protocol,
            agent_id: existing.agent_id, config_json, status: existing.status,
            created_at: existing.created_at, updated_at: now,
        })
    }).await.map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    write_audit_log(
        &state.db,
        "resource_update",
        "success",
        &format!("更新资源「{}」", resource.name),
        Some(&env_id),
        Some(&id),
        resource.agent_id.as_deref(),
        None,
        Some(&ip),
    );

    Ok(Json(ApiResponse { data: resource }))
}

pub async fn delete_resource(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((env_id, id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ip = extract_client_ip(&headers);
    let db = state.db.clone();
    let env_id_clone = env_id.clone();
    let id_clone = id.clone();

    let _result = tokio::task::spawn_blocking(
        move || -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
            let conn = db
                .pool
                .get()
                .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
            let affected = conn
                .execute(
                    "DELETE FROM resources WHERE id = ?1 AND environment_id = ?2",
                    rusqlite::params![id_clone, env_id_clone],
                )
                .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
            if affected == 0 {
                return Err(not_found("RESOURCE_NOT_FOUND", "资源不存在"));
            }
            Ok(StatusCode::NO_CONTENT)
        },
    )
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    write_audit_log(
        &state.db,
        "resource_delete",
        "success",
        &format!("删除资源「{}」", id),
        Some(&env_id),
        Some(&id),
        None,
        None,
        Some(&ip),
    );

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    pub protocol: String,
    pub config_json: String,
}

#[derive(Debug, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
}

pub async fn test_connection(
    State(state): State<Arc<AppState>>,
    Json(input): Json<TestConnectionRequest>,
) -> Result<Json<ApiResponse<TestConnectionResponse>>, (StatusCode, Json<ErrorResponse>)> {
    if !VALID_PROTOCOLS.contains(&input.protocol.as_str()) {
        return Err(bad_request(&format!("不支持的协议: {}", input.protocol)));
    }

    let start = std::time::Instant::now();
    let result = match input.protocol.as_str() {
        "ssh" | "sftp" => {
            let config = crate::ssh_config::SshResourceConfig::from_json(&input.config_json)
                .map_err(|e| bad_request(&format!("配置解析失败: {}", e)))?;
            let auth = config
                .to_auth_method(&state.secret_key)
                .map_err(|e| bad_request(&format!("认证配置错误: {}", e)))?;
            match rex_ssh::client::SshClient::connect(
                &config.host,
                config.port,
                &config.username,
                auth,
            )
            .await
            {
                Ok(mut client) => {
                    let _ = client.disconnect().await;
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            }
        }
        "mysql" | "postgresql" => {
            let connector: Box<dyn SqlConnector> = match input.protocol.as_str() {
                "mysql" => Box::new(
                    rex_mysql::MySqlConnector::from_json(&input.config_json)
                        .map_err(|e| bad_request(&format!("配置解析失败: {}", e)))?,
                ),
                _ => Box::new(
                    rex_postgresql::PostgresConnector::from_json(&input.config_json)
                        .map_err(|e| bad_request(&format!("配置解析失败: {}", e)))?,
                ),
            };
            test_sql_connector(connector).await
        }
        _ => Err(format!("{} 协议暂不支持测试连接", input.protocol)),
    };
    let latency = start.elapsed().as_millis() as u64;

    let resp = match result {
        Ok(()) => TestConnectionResponse {
            success: true,
            message: "连接成功".to_string(),
            latency_ms: Some(latency),
        },
        Err(e) => TestConnectionResponse {
            success: false,
            message: e,
            latency_ms: None,
        },
    };

    Ok(Json(ApiResponse { data: resp }))
}

async fn test_sql_connector(mut connector: Box<dyn SqlConnector>) -> Result<(), String> {
    connector.connect().await.map_err(|e| e.to_string())?;
    let _ = connector.close().await;
    Ok(())
}
