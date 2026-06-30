use axum::extract::{Path, State};
use axum::http::{header::HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::audit::write_audit_log;
use crate::helpers::{
    bad_request, conflict, err_resp, gen_id, not_found, now_iso, ApiResponse, ErrorResponse,
};
use crate::routes::{extract_client_ip, AppState};

#[derive(Debug, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub connection_mode: String,
    pub agent_token_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_types: Option<std::collections::HashMap<String, i64>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEnvironment {
    pub name: String,
    pub description: Option<String>,
    pub connection_mode: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnvironment {
    pub name: Option<String>,
    pub description: Option<String>,
}

pub async fn list_envs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<Environment>>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let envs = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let mut envs: Vec<Environment> = conn
            .prepare(
                "SELECT id, name, description, connection_mode, agent_token_hash, created_at, updated_at FROM environments ORDER BY created_at DESC",
            )
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?
            .query_map([], |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    connection_mode: row.get(3)?,
                    agent_token_hash: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    resource_count: None,
                    agent_count: None,
                    resource_types: None,
                })
            })
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?
            .filter_map(|r| r.ok())
            .collect();

        // Populate resource stats for each environment
        for env in &mut envs {
            let resource_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM resources WHERE environment_id = ?1",
                rusqlite::params![env.id],
                |row| row.get(0),
            ).unwrap_or(0);

            let agent_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM agents WHERE environment_id = ?1",
                rusqlite::params![env.id],
                |row| row.get(0),
            ).unwrap_or(0);

            let mut resource_types = std::collections::HashMap::new();
            if let Ok(mut stmt) = conn.prepare(
                "SELECT protocol, COUNT(*) as cnt FROM resources WHERE environment_id = ?1 GROUP BY protocol",
            ) {
                if let Ok(rows) = stmt.query_map(rusqlite::params![env.id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                }) {
                    for row in rows.flatten() {
                        resource_types.insert(row.0, row.1);
                    }
                }
            }

            env.resource_count = Some(resource_count);
            env.agent_count = Some(agent_count);
            env.resource_types = Some(resource_types);
        }

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(envs)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: envs }))
}

pub async fn create_env(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateEnvironment>,
) -> Result<(StatusCode, Json<ApiResponse<Environment>>), (StatusCode, Json<ErrorResponse>)> {
    if input.name.trim().is_empty() {
        return Err(bad_request("环境名称不能为空"));
    }
    if input.connection_mode != "agent_proxy" && input.connection_mode != "direct" {
        return Err(bad_request("connection_mode 必须是 agent_proxy 或 direct"));
    }

    let ip = extract_client_ip(&headers);
    let id = gen_id("env");
    let now = now_iso();
    let db = state.db.clone();
    let id_clone = id.clone();
    let name = input.name.clone();
    let conn_mode = input.connection_mode.clone();
    let desc = input.description.clone();

    let env = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        conn.execute(
            "INSERT INTO environments (id, name, description, connection_mode, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id_clone, name, desc, conn_mode, now.clone(), now],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(Environment {
            id: id_clone, name, description: desc, connection_mode: conn_mode,
            agent_token_hash: None, created_at: now.clone(), updated_at: now,
            resource_count: Some(0), agent_count: Some(0), resource_types: Some(std::collections::HashMap::new()),
        })
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    write_audit_log(
        &state.db,
        "environment_create",
        "success",
        &format!("创建环境「{}」", input.name),
        Some(&id),
        None,
        None,
        None,
        Some(&ip),
    );

    Ok((StatusCode::CREATED, Json(ApiResponse { data: env })))
}

pub async fn get_env(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Environment>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let env = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let mut env = conn.query_row(
            "SELECT id, name, description, connection_mode, agent_token_hash, created_at, updated_at FROM environments WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok(Environment {
                id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                connection_mode: row.get(3)?, agent_token_hash: row.get(4)?,
                created_at: row.get(5)?, updated_at: row.get(6)?,
                resource_count: None, agent_count: None, resource_types: None,
            }),
        )
        .map_err(|_| not_found("ENVIRONMENT_NOT_FOUND", "环境不存在"))?;

        let resource_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM resources WHERE environment_id = ?1", rusqlite::params![id], |row| row.get(0),
        ).unwrap_or(0);
        let agent_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM agents WHERE environment_id = ?1", rusqlite::params![id], |row| row.get(0),
        ).unwrap_or(0);
        let mut resource_types = std::collections::HashMap::new();
        if let Ok(mut stmt) = conn.prepare("SELECT protocol, COUNT(*) FROM resources WHERE environment_id = ?1 GROUP BY protocol") {
            if let Ok(rows) = stmt.query_map(rusqlite::params![id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))) {
                for row in rows.flatten() { resource_types.insert(row.0, row.1); }
            }
        }

        env.resource_count = Some(resource_count);
        env.agent_count = Some(agent_count);
        env.resource_types = Some(resource_types);
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(env)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: env }))
}

pub async fn update_env(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateEnvironment>,
) -> Result<Json<ApiResponse<Environment>>, (StatusCode, Json<ErrorResponse>)> {
    let ip = extract_client_ip(&headers);
    let db = state.db.clone();
    let id_clone = id.clone();

    let env = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let existing: Environment = conn.query_row(
            "SELECT id, name, description, connection_mode, agent_token_hash, created_at, updated_at FROM environments WHERE id = ?1",
            rusqlite::params![id_clone],
            |row| Ok(Environment {
                id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                connection_mode: row.get(3)?, agent_token_hash: row.get(4)?,
                created_at: row.get(5)?, updated_at: row.get(6)?,
                resource_count: None, agent_count: None, resource_types: None,
            }),
        )
        .map_err(|_| not_found("ENVIRONMENT_NOT_FOUND", "环境不存在"))?;

        let name = input.name.unwrap_or(existing.name);
        let description = input.description.or(existing.description);
        let now = now_iso();
        conn.execute(
            "UPDATE environments SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![name, description, now, id_clone],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(Environment {
            id: existing.id, name, description, connection_mode: existing.connection_mode,
            agent_token_hash: existing.agent_token_hash, created_at: existing.created_at, updated_at: now,
            resource_count: existing.resource_count, agent_count: existing.agent_count, resource_types: existing.resource_types,
        })
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    write_audit_log(
        &state.db,
        "environment_update",
        "success",
        &format!("更新环境「{}」", env.name),
        Some(&id),
        None,
        None,
        None,
        Some(&ip),
    );

    Ok(Json(ApiResponse { data: env }))
}

pub async fn delete_env(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ip = extract_client_ip(&headers);
    let db = state.db.clone();
    let id_clone = id.clone();

    let _result = tokio::task::spawn_blocking(
        move || -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
            let conn = db
                .pool
                .get()
                .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
            let res_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM resources WHERE environment_id = ?1",
                    rusqlite::params![id_clone],
                    |row| row.get(0),
                )
                .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
            if res_count > 0 {
                return Err(conflict(
                    "ENVIRONMENT_HAS_RESOURCES",
                    "环境下仍有资源，无法删除",
                ));
            }
            let agent_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM agents WHERE environment_id = ?1",
                    rusqlite::params![id_clone],
                    |row| row.get(0),
                )
                .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
            if agent_count > 0 {
                return Err(conflict(
                    "ENVIRONMENT_HAS_AGENTS",
                    "环境下仍有 Agent，无法删除",
                ));
            }
            let affected = conn
                .execute(
                    "DELETE FROM environments WHERE id = ?1",
                    rusqlite::params![id_clone],
                )
                .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
            if affected == 0 {
                return Err(not_found("ENVIRONMENT_NOT_FOUND", "环境不存在"));
            }
            Ok(StatusCode::NO_CONTENT)
        },
    )
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    write_audit_log(
        &state.db,
        "environment_delete",
        "success",
        &format!("删除环境「{}」", id),
        Some(&id),
        None,
        None,
        None,
        Some(&ip),
    );

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt;

    fn test_app() -> axum::Router {
        let db = Arc::new(crate::db::Database::new_in_memory().unwrap());
        crate::routes::app(db, "test-secret".to_string())
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
    async fn create_and_get_env() {
        let app = test_app();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/environments")
                    .header("content-type", "application/json")
                    .header("authorization", auth_header())
                    .body(Body::from(
                        serde_json::to_string(
                            &serde_json::json!({"name": "测试环境", "connection_mode": "direct"}),
                        )
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let env_id = json["data"]["id"].as_str().unwrap().to_string();
        assert!(env_id.starts_with("env_"));

        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/environments/{env_id}"))
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_env_empty_name_returns_400() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/environments")
                    .header("content-type", "application/json")
                    .header("authorization", auth_header())
                    .body(Body::from(
                        serde_json::to_string(
                            &serde_json::json!({"name": "", "connection_mode": "direct"}),
                        )
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_nonexistent_env_returns_404() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/environments/env_nonexistent")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn list_envs_empty() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/environments")
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
    async fn delete_env_with_no_resources_succeeds() {
        let app = test_app();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/environments")
                    .header("content-type", "application/json")
                    .header("authorization", auth_header())
                    .body(Body::from(
                        serde_json::to_string(
                            &serde_json::json!({"name": "test", "connection_mode": "direct"}),
                        )
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let env_id = json["data"]["id"].as_str().unwrap().to_string();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/environments/{env_id}"))
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}
