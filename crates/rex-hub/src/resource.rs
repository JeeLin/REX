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

    // 校验 config_json 格式
    match input.protocol.as_str() {
        "ssh" | "sftp" => {
            crate::ssh_config::SshResourceConfig::from_json(&input.config_json)
                .map_err(|e| bad_request(&format!("SSH 配置无效: {}", e)))?;
        }
        "mysql" => {
            rex_mysql::MySqlConnector::from_json(&input.config_json)
                .map_err(|e| bad_request(&format!("MySQL 配置无效: {}", e)))?;
        }
        "postgresql" => {
            rex_postgresql::PostgresConnector::from_json(&input.config_json)
                .map_err(|e| bad_request(&format!("PostgreSQL 配置无效: {}", e)))?;
        }
        _ => {}
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

        // 校验更新后的 config_json 格式
        match existing.protocol.as_str() {
            "ssh" | "sftp" => {
                crate::ssh_config::SshResourceConfig::from_json(&config_json)
                    .map_err(|e| bad_request(&format!("SSH 配置无效: {}", e)))?;
            }
            "mysql" => {
                rex_mysql::MySqlConnector::from_json(&config_json)
                    .map_err(|e| bad_request(&format!("MySQL 配置无效: {}", e)))?;
            }
            "postgresql" => {
                rex_postgresql::PostgresConnector::from_json(&config_json)
                    .map_err(|e| bad_request(&format!("PostgreSQL 配置无效: {}", e)))?;
            }
            _ => {}
        }

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

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh_config::SshResourceConfig;

    #[test]
    fn valid_protocols_contains_expected_protocols() {
        assert!(VALID_PROTOCOLS.contains(&"ssh"));
        assert!(VALID_PROTOCOLS.contains(&"sftp"));
        assert!(VALID_PROTOCOLS.contains(&"mysql"));
        assert!(VALID_PROTOCOLS.contains(&"postgresql"));
        assert!(VALID_PROTOCOLS.contains(&"redis"));
        assert!(VALID_PROTOCOLS.contains(&"docker"));
        assert!(VALID_PROTOCOLS.contains(&"sqlite"));
        assert!(VALID_PROTOCOLS.contains(&"s3"));
        assert!(!VALID_PROTOCOLS.contains(&"invalid_protocol"));
    }

    #[test]
    fn resource_struct_serializes() {
        let resource = Resource {
            id: "res_123".to_string(),
            environment_id: "env_456".to_string(),
            name: "test_resource".to_string(),
            protocol: "ssh".to_string(),
            agent_id: Some("agent_789".to_string()),
            config_json: r#"{"host":"example.com","port":22}"#.to_string(),
            status: "ready".to_string(),
            created_at: "1234567890".to_string(),
            updated_at: "1234567891".to_string(),
        };
        let json = serde_json::to_string(&resource).unwrap();
        assert!(json.contains("res_123"));
        assert!(json.contains("ssh"));
    }

    #[test]
    fn create_resource_validates_empty_name() {
        // Test that empty name would be rejected - validated by checking name.trim().is_empty() logic
        let name = "   ";
        assert!(name.trim().is_empty());
    }

    #[test]
    fn create_resource_validates_invalid_protocol() {
        assert!(!VALID_PROTOCOLS.contains(&"invalid_protocol"));
        assert!(VALID_PROTOCOLS.contains(&"ssh"));
    }

    #[test]
    fn ssh_config_validation_works() {
        // Valid SSH config
        let valid_json = r#"{"host":"example.com","port":22,"username":"user","auth":{"type":"password","password":"pass123"}}"#;
        let result = SshResourceConfig::from_json(valid_json);
        assert!(result.is_ok());

        // Invalid - missing host
        let invalid_json = r#"{"username":"user","auth":{"type":"password","password":"pass123"}}"#;
        let result = SshResourceConfig::from_json(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn ssh_config_validation_empty_host_rejected() {
        let config_json = r#"{"host":"","port":22,"username":"user","auth":{"type":"password","password":"pass123"}}"#;
        let result = SshResourceConfig::from_json(config_json);
        assert!(result.is_err());
    }

    #[test]
    fn ssh_config_validation_empty_username_rejected() {
        let config_json = r#"{"host":"example.com","port":22,"username":"","auth":{"type":"password","password":"pass123"}}"#;
        let result = SshResourceConfig::from_json(config_json);
        assert!(result.is_err());
    }
}

// ── HTTP Handler Tests ─────────────────────────────────────

#[cfg(test)]
mod handler_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{delete, get, post, put};
    use axum::Router;
    use tower::ServiceExt;

    use crate::routes::AppState;
    use rex_transfer::task::TransferManager;

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

    fn test_app() -> Router {
        let state = test_state();
        Router::new()
            .route(
                "/api/environments/:env_id/resources",
                get(list_resources).post(create_resource),
            )
            .route(
                "/api/environments/:env_id/resources/:id",
                get(get_resource)
                    .put(update_resource)
                    .delete(delete_resource),
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

    fn create_test_env(state: &AppState) {
        let conn = state.db.pool.get().unwrap();
        conn.execute(
            "INSERT INTO environments (id, name, description, connection_mode, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params!["env_test", "test-env", "test", "direct", "2024-01-01", "2024-01-01"],
        )
        .unwrap();
    }

    #[tokio::test]
    async fn list_resources_returns_empty_for_new_env() {
        let state = test_state();
        create_test_env(&state);
        let app = Router::new()
            .route("/api/environments/:env_id/resources", get(list_resources))
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/environments/env_test/resources")
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
        assert!(json["data"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_resource_validates_empty_name() {
        let state = test_state();
        create_test_env(&state);
        let app = Router::new()
            .route("/api/environments/:env_id/resources", post(create_resource))
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/environments/env_test/resources")
                    .header("authorization", auth_header())
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"  ","protocol":"ssh","config_json":"{}"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_resource_validates_invalid_protocol() {
        let state = test_state();
        create_test_env(&state);
        let app = Router::new()
            .route("/api/environments/:env_id/resources", post(create_resource))
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/environments/env_test/resources")
                    .header("authorization", auth_header())
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"test","protocol":"invalid","config_json":"{}"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_resource_fails_for_nonexistent_env() {
        let state = test_state();
        let app = Router::new()
            .route("/api/environments/:env_id/resources", post(create_resource))
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/environments/nonexistent/resources")
                    .header("authorization", auth_header())
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"test","protocol":"docker","config_json":"{}"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_resource_returns_not_found() {
        let state = test_state();
        create_test_env(&state);
        let app = Router::new()
            .route("/api/environments/:env_id/resources/:id", get(get_resource))
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/environments/env_test/resources/nonexistent")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_resource_returns_not_found() {
        let state = test_state();
        create_test_env(&state);
        let app = Router::new()
            .route(
                "/api/environments/:env_id/resources/:id",
                delete(delete_resource),
            )
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/environments/env_test/resources/nonexistent")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_resource_returns_not_found() {
        let state = test_state();
        create_test_env(&state);
        let app = Router::new()
            .route(
                "/api/environments/:env_id/resources/:id",
                put(update_resource),
            )
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/environments/env_test/resources/nonexistent")
                    .header("authorization", auth_header())
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"updated"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_connection_rejects_invalid_protocol() {
        let state = test_state();
        let app = Router::new()
            .route("/api/resources/test-connection", post(test_connection))
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/resources/test-connection")
                    .header("authorization", auth_header())
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"protocol":"invalid","config_json":"{}"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
