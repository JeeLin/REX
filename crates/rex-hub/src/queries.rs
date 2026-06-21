use axum::extract::{Path as AxPath, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tracing::info;

use crate::helpers::{bad_request, err_resp, not_found, ApiResponse, ErrorResponse};
use crate::routes::AppState;

// ── 数据模型 ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryFileMeta {
    pub id: String,
    pub name: String,
    pub database: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveQueryRequest {
    pub name: String,
    pub sql: String,
    pub database: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQueryRequest {
    pub name: Option<String>,
    pub sql: Option<String>,
    pub database: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RenameQueryRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct QueryDetail {
    #[serde(flatten)]
    pub meta: QueryFileMeta,
    pub sql: String,
}

// ── 路由处理函数 ──────────────────────────────────────────

/// GET /api/resources/:resource_id/queries — 列出该资源的所有查询文件
pub async fn list_queries(
    State(state): State<Arc<AppState>>,
    AxPath(resource_id): AxPath<String>,
) -> Result<Json<ApiResponse<Vec<QueryFileMeta>>>, (StatusCode, Json<ErrorResponse>)> {
    let queries_dir = state.data_dir.join("queries").join(&resource_id);
    let entries = list_query_files(&queries_dir)?;
    Ok(Json(ApiResponse { data: entries }))
}

/// POST /api/resources/:resource_id/queries — 保存查询文件
pub async fn save_query(
    State(state): State<Arc<AppState>>,
    AxPath(resource_id): AxPath<String>,
    Json(input): Json<SaveQueryRequest>,
) -> Result<Json<ApiResponse<QueryFileMeta>>, (StatusCode, Json<ErrorResponse>)> {
    if input.name.trim().is_empty() {
        return Err(bad_request("查询名称不能为空"));
    }
    if input.sql.trim().is_empty() {
        return Err(bad_request("SQL 不能为空"));
    }

    let queries_dir = state.data_dir.join("queries").join(&resource_id);
    std::fs::create_dir_all(&queries_dir)
        .map_err(|e| err_resp("IO_ERROR", &format!("创建查询目录失败: {e}")))?;

    let id = crate::helpers::gen_id("q");
    let now = crate::helpers::now_iso();

    let meta = QueryFileMeta {
        id: id.clone(),
        name: input.name,
        database: input.database,
        created_at: now.clone(),
        updated_at: now,
    };

    // 写入元数据
    let meta_path = queries_dir.join(format!("{id}.json"));
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| err_resp("SERIALIZE_ERROR", &format!("序列化失败: {e}")))?;
    std::fs::write(&meta_path, meta_json)
        .map_err(|e| err_resp("IO_ERROR", &format!("写入元数据失败: {e}")))?;

    // 写入 SQL 内容
    let sql_path = queries_dir.join(format!("{id}.sql"));
    std::fs::write(&sql_path, &input.sql)
        .map_err(|e| err_resp("IO_ERROR", &format!("写入 SQL 失败: {e}")))?;

    info!(query_id = %id, resource = %resource_id, name = %meta.name, "query saved");
    Ok(Json(ApiResponse { data: meta }))
}

/// GET /api/resources/:resource_id/queries/:id — 读取查询文件
pub async fn get_query(
    State(state): State<Arc<AppState>>,
    AxPath((resource_id, id)): AxPath<(String, String)>,
) -> Result<Json<ApiResponse<QueryDetail>>, (StatusCode, Json<ErrorResponse>)> {
    let queries_dir = state.data_dir.join("queries").join(&resource_id);

    let meta = read_meta(&queries_dir, &id)?;

    let sql_path = queries_dir.join(format!("{id}.sql"));
    let sql = std::fs::read_to_string(&sql_path)
        .map_err(|e| err_resp("IO_ERROR", &format!("读取 SQL 失败: {e}")))?;

    Ok(Json(ApiResponse {
        data: QueryDetail { meta, sql },
    }))
}

/// PUT /api/resources/:resource_id/queries/:id — 更新查询文件
pub async fn update_query(
    State(state): State<Arc<AppState>>,
    AxPath((resource_id, id)): AxPath<(String, String)>,
    Json(input): Json<UpdateQueryRequest>,
) -> Result<Json<ApiResponse<QueryFileMeta>>, (StatusCode, Json<ErrorResponse>)> {
    let queries_dir = state.data_dir.join("queries").join(&resource_id);

    let mut meta = read_meta(&queries_dir, &id)?;

    if let Some(name) = input.name {
        if name.trim().is_empty() {
            return Err(bad_request("查询名称不能为空"));
        }
        meta.name = name;
    }
    if let Some(database) = input.database {
        meta.database = database;
    }
    meta.updated_at = crate::helpers::now_iso();

    // 更新元数据
    let meta_path = queries_dir.join(format!("{id}.json"));
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| err_resp("SERIALIZE_ERROR", &format!("序列化失败: {e}")))?;
    std::fs::write(&meta_path, meta_json)
        .map_err(|e| err_resp("IO_ERROR", &format!("写入元数据失败: {e}")))?;

    // 更新 SQL 内容（如果提供）
    if let Some(sql) = input.sql {
        let sql_path = queries_dir.join(format!("{id}.sql"));
        std::fs::write(&sql_path, &sql)
            .map_err(|e| err_resp("IO_ERROR", &format!("写入 SQL 失败: {e}")))?;
    }

    info!(query_id = %id, resource = %resource_id, name = %meta.name, "query updated");
    Ok(Json(ApiResponse { data: meta }))
}

/// DELETE /api/resources/:resource_id/queries/:id — 删除查询文件
pub async fn delete_query(
    State(state): State<Arc<AppState>>,
    AxPath((resource_id, id)): AxPath<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let queries_dir = state.data_dir.join("queries").join(&resource_id);

    // 验证文件存在
    let _ = read_meta(&queries_dir, &id)?;

    let meta_path = queries_dir.join(format!("{id}.json"));
    let sql_path = queries_dir.join(format!("{id}.sql"));

    std::fs::remove_file(&meta_path)
        .map_err(|e| err_resp("IO_ERROR", &format!("删除元数据失败: {e}")))?;
    let _ = std::fs::remove_file(&sql_path); // SQL 文件可能不存在，忽略错误

    info!(query_id = %id, resource = %resource_id, "query deleted");
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/resources/:resource_id/queries/:id/rename — 重命名查询文件
pub async fn rename_query(
    State(state): State<Arc<AppState>>,
    AxPath((resource_id, id)): AxPath<(String, String)>,
    Json(input): Json<RenameQueryRequest>,
) -> Result<Json<ApiResponse<QueryFileMeta>>, (StatusCode, Json<ErrorResponse>)> {
    if input.name.trim().is_empty() {
        return Err(bad_request("查询名称不能为空"));
    }

    let queries_dir = state.data_dir.join("queries").join(&resource_id);

    let mut meta = read_meta(&queries_dir, &id)?;
    meta.name = input.name;
    meta.updated_at = crate::helpers::now_iso();

    let meta_path = queries_dir.join(format!("{id}.json"));
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| err_resp("SERIALIZE_ERROR", &format!("序列化失败: {e}")))?;
    std::fs::write(&meta_path, meta_json)
        .map_err(|e| err_resp("IO_ERROR", &format!("写入元数据失败: {e}")))?;

    info!(query_id = %id, resource = %resource_id, name = %meta.name, "query renamed");
    Ok(Json(ApiResponse { data: meta }))
}

// ── 内部函数 ──────────────────────────────────────────────

/// 读取查询文件元数据
fn read_meta(
    queries_dir: &Path,
    id: &str,
) -> Result<QueryFileMeta, (StatusCode, Json<ErrorResponse>)> {
    let meta_path = queries_dir.join(format!("{id}.json"));
    let content = std::fs::read_to_string(&meta_path)
        .map_err(|_| not_found("QUERY_NOT_FOUND", "查询文件不存在"))?;
    serde_json::from_str(&content)
        .map_err(|e| err_resp("PARSE_ERROR", &format!("元数据格式错误: {e}")))
}

/// 列出所有查询文件
fn list_query_files(
    queries_dir: &Path,
) -> Result<Vec<QueryFileMeta>, (StatusCode, Json<ErrorResponse>)> {
    if !queries_dir.exists() {
        return Ok(vec![]);
    }

    let mut entries = Vec::new();
    let dir_entries = std::fs::read_dir(queries_dir)
        .map_err(|e| err_resp("IO_ERROR", &format!("读取查询目录失败: {e}")))?;

    for entry in dir_entries {
        let entry = entry.map_err(|e| err_resp("IO_ERROR", &format!("读取目录项失败: {e}")))?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(meta) = serde_json::from_str::<QueryFileMeta>(&content) {
                    entries.push(meta);
                }
            }
        }
    }

    entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(entries)
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, put};
    use axum::Router;
    use tower::ServiceExt;

    use crate::routes::AppState;
    use crate::terminal::SessionManager;
    use crate::ws::new_connections;
    use rex_transfer::task::TransferManager;

    const RID: &str = "res_00000001";

    fn test_state() -> (Arc<AppState>, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState {
            db: Arc::new(crate::db::Database::new_in_memory().unwrap()),
            secret_key: "test-secret".to_string(),
            connections: Arc::new(new_connections()),
            sessions: Arc::new(SessionManager::new(900)),
            transfer: Some(Arc::new(crate::transfer::TransferState {
                manager: Arc::new(TransferManager::new()),
            })),
            update_cache: tokio::sync::RwLock::new(crate::routes::UpdateCache::new()),
            data_dir: tmp.path().to_path_buf(),
        });
        (state, tmp)
    }

    fn test_app() -> (Router, tempfile::TempDir) {
        let (state, tmp) = test_state();
        let app = Router::new()
            .route(
                "/api/resources/:resource_id/queries",
                get(list_queries).post(save_query),
            )
            .route(
                "/api/resources/:resource_id/queries/:id",
                get(get_query).put(update_query).delete(delete_query),
            )
            .route(
                "/api/resources/:resource_id/queries/:id/rename",
                put(rename_query),
            )
            .with_state(state);
        (app, tmp)
    }

    fn queries_uri(path: &str) -> String {
        format!("/api/resources/{RID}/queries{path}")
    }

    #[tokio::test]
    async fn list_queries_returns_empty_initially() {
        let (app, _tmp) = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(queries_uri(""))
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
    async fn save_query_creates_file() {
        let (app, _tmp) = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(queries_uri(""))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"test","sql":"SELECT 1","database":"mydb"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["name"], "test");
        assert_eq!(json["data"]["database"], "mydb");
        assert!(json["data"]["id"].as_str().unwrap().starts_with("q_"));
    }

    #[tokio::test]
    async fn save_query_rejects_empty_name() {
        let (app, _tmp) = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(queries_uri(""))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"","sql":"SELECT 1","database":"mydb"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn save_query_rejects_empty_sql() {
        let (app, _tmp) = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(queries_uri(""))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"test","sql":"  ","database":"mydb"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn save_and_get_query_roundtrip() {
        let (app, _tmp) = test_app();

        // Save
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(queries_uri(""))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"test","sql":"SELECT 1","database":"mydb"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = json["data"]["id"].as_str().unwrap().to_string();

        // Get
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(queries_uri(&format!("/{id}")))
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
        assert_eq!(json["data"]["name"], "test");
        assert_eq!(json["data"]["sql"], "SELECT 1");
        assert_eq!(json["data"]["database"], "mydb");
    }

    #[tokio::test]
    async fn get_query_returns_404_for_unknown() {
        let (app, _tmp) = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(queries_uri("/q_00000000"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_query_changes_name() {
        let (app, _tmp) = test_app();

        // Save
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(queries_uri(""))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"old","sql":"SELECT 1","database":"mydb"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = json["data"]["id"].as_str().unwrap().to_string();

        // Update
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(queries_uri(&format!("/{id}")))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"new"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["name"], "new");
    }

    #[tokio::test]
    async fn delete_query_removes_files() {
        let (app, _tmp) = test_app();

        // Save
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(queries_uri(""))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"test","sql":"SELECT 1","database":"mydb"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = json["data"]["id"].as_str().unwrap().to_string();

        // Delete
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(queries_uri(&format!("/{id}")))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Verify deleted
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(queries_uri(&format!("/{id}")))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn rename_query_changes_name() {
        let (app, _tmp) = test_app();

        // Save
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(queries_uri(""))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"old","sql":"SELECT 1","database":"mydb"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = json["data"]["id"].as_str().unwrap().to_string();

        // Rename
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(queries_uri(&format!("/{id}/rename")))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"renamed"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["name"], "renamed");
    }

    #[tokio::test]
    async fn rename_query_rejects_empty_name() {
        let (app, _tmp) = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(queries_uri("/q_00000000/rename"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":""}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_queries_returns_saved_files() {
        let (app, _tmp) = test_app();

        // Save two queries
        for name in &["first", "second"] {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(queries_uri(""))
                        .header("content-type", "application/json")
                        .body(Body::from(format!(
                            r#"{{"name":"{name}","sql":"SELECT 1","database":"mydb"}}"#
                        )))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let resp = app
            .oneshot(
                Request::builder()
                    .uri(queries_uri(""))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn queries_are_isolated_by_resource() {
        let tmp = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState {
            db: Arc::new(crate::db::Database::new_in_memory().unwrap()),
            secret_key: "test-secret".to_string(),
            connections: Arc::new(new_connections()),
            sessions: Arc::new(SessionManager::new(900)),
            transfer: Some(Arc::new(crate::transfer::TransferState {
                manager: Arc::new(TransferManager::new()),
            })),
            update_cache: tokio::sync::RwLock::new(crate::routes::UpdateCache::new()),
            data_dir: tmp.path().to_path_buf(),
        });

        let app = Router::new()
            .route(
                "/api/resources/:resource_id/queries",
                get(list_queries).post(save_query),
            )
            .with_state(state);

        // Save to res_a
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/resources/res_a/queries")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"q1","sql":"SELECT 1","database":"db1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // List res_a → 1 item
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/resources/res_a/queries")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"].as_array().unwrap().len(), 1);

        // List res_b → 0 items
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/resources/res_b/queries")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"].as_array().unwrap().len(), 0);
    }
}
