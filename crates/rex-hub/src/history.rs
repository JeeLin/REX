use axum::extract::{Path as AxPath, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use crate::helpers::{bad_request, err_resp, ApiResponse, ErrorResponse};
use crate::routes::AppState;

// ── 数据模型 ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryRecord {
    pub id: String,
    pub sql: String,
    pub database: String,
    pub executed_at: String,
    pub elapsed_ms: u64,
    pub row_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct RecordHistoryRequest {
    pub sql: String,
    pub database: String,
    pub elapsed_ms: u64,
    pub row_count: usize,
}

// ── 路径安全 ──────────────────────────────────────────────

const MAX_HISTORY: usize = 100;

fn sql_history_path(
    state: &AppState,
    resource_id: &str,
) -> Result<PathBuf, (StatusCode, Json<ErrorResponse>)> {
    if resource_id.contains('/') || resource_id.contains('\\') || resource_id.contains("..") {
        return Err(bad_request("resource_id 包含非法字符"));
    }
    Ok(state
        .data_dir
        .join("sql-history")
        .join(format!("{resource_id}.json")))
}

// ── 路由处理函数 ──────────────────────────────────────────

/// GET /api/resources/:resource_id/sql/history — 列出历史记录（最近 100 条）
pub async fn list_history(
    State(state): State<Arc<AppState>>,
    AxPath(resource_id): AxPath<String>,
) -> Result<Json<ApiResponse<Vec<HistoryRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    let path = sql_history_path(&state, &resource_id)?;
    let records = read_history(&path)?;
    Ok(Json(ApiResponse { data: records }))
}

/// POST /api/resources/:resource_id/sql/history — 记录一条执行历史
pub async fn record_history(
    State(state): State<Arc<AppState>>,
    AxPath(resource_id): AxPath<String>,
    Json(input): Json<RecordHistoryRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if input.sql.trim().is_empty() {
        return Err(bad_request("SQL 不能为空"));
    }

    let path = sql_history_path(&state, &resource_id)?;

    // 确保父目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| err_resp("IO_ERROR", &format!("创建历史目录失败: {e}")))?;
    }

    let mut records = read_history(&path)?;

    let record = HistoryRecord {
        id: crate::helpers::gen_id("h"),
        sql: input.sql,
        database: input.database,
        executed_at: crate::helpers::now_iso(),
        elapsed_ms: input.elapsed_ms,
        row_count: input.row_count,
    };

    // 插入到最前面（最新的在前）
    records.insert(0, record);

    // 超过 MAX_HISTORY 时删除最早的
    if records.len() > MAX_HISTORY {
        records.truncate(MAX_HISTORY);
    }

    // 写入文件
    let json = serde_json::to_string_pretty(&records)
        .map_err(|e| err_resp("SERIALIZE_ERROR", &format!("序列化失败: {e}")))?;
    std::fs::write(&path, json)
        .map_err(|e| err_resp("IO_ERROR", &format!("写入历史记录失败: {e}")))?;

    info!(resource = %resource_id, "SQL history recorded");
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/resources/:resource_id/sql/history — 清空历史记录
pub async fn clear_history(
    State(state): State<Arc<AppState>>,
    AxPath(resource_id): AxPath<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let path = sql_history_path(&state, &resource_id)?;

    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| err_resp("IO_ERROR", &format!("删除历史记录失败: {e}")))?;
    }

    info!(resource = %resource_id, "SQL history cleared");
    Ok(StatusCode::NO_CONTENT)
}

// ── 内部函数 ──────────────────────────────────────────────

/// 读取历史记录文件
fn read_history(path: &PathBuf) -> Result<Vec<HistoryRecord>, (StatusCode, Json<ErrorResponse>)> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| err_resp("IO_ERROR", &format!("读取历史记录失败: {e}")))?;

    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    serde_json::from_str(&content)
        .map_err(|e| err_resp("PARSE_ERROR", &format!("历史记录格式错误: {e}")))
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
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
                "/api/resources/:resource_id/sql/history",
                get(list_history).post(record_history).delete(clear_history),
            )
            .with_state(state);
        (app, tmp)
    }

    fn history_uri() -> String {
        format!("/api/resources/{RID}/sql/history")
    }

    #[tokio::test]
    async fn list_history_returns_empty_initially() {
        let (app, _tmp) = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(history_uri())
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
    async fn record_history_creates_entry() {
        let (app, _tmp) = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(history_uri())
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"sql":"SELECT 1","database":"mydb","elapsed_ms":12,"row_count":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn record_and_list_roundtrip() {
        let (app, _tmp) = test_app();

        // Record
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(history_uri())
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"sql":"SELECT * FROM users","database":"mydb","elapsed_ms":50,"row_count":100}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // List
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(history_uri())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let data = json["data"].as_array().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["sql"], "SELECT * FROM users");
        assert_eq!(data[0]["database"], "mydb");
        assert_eq!(data[0]["elapsed_ms"], 50);
        assert_eq!(data[0]["row_count"], 100);
    }

    #[tokio::test]
    async fn record_history_rejects_empty_sql() {
        let (app, _tmp) = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(history_uri())
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"sql":"  ","database":"mydb","elapsed_ms":12,"row_count":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn clear_history_removes_all() {
        let (app, _tmp) = test_app();

        // Record
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(history_uri())
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"sql":"SELECT 1","database":"mydb","elapsed_ms":12,"row_count":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Clear
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(history_uri())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // List → empty
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(history_uri())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn history_is_limited_to_100() {
        let (app, _tmp) = test_app();

        // Record 110 entries
        for i in 0..110 {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(history_uri())
                        .header("content-type", "application/json")
                        .body(Body::from(format!(
                            r#"{{"sql":"SELECT {i}","database":"mydb","elapsed_ms":12,"row_count":1}}"#
                        )))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        // List → should be 100
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(history_uri())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let data = json["data"].as_array().unwrap();
        assert_eq!(data.len(), 100);
        // 最新的应该在最前面
        assert_eq!(data[0]["sql"], "SELECT 109");
    }

    #[tokio::test]
    async fn history_is_isolated_by_resource() {
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
                "/api/resources/:resource_id/sql/history",
                get(list_history).post(record_history),
            )
            .with_state(state);

        // Record to res_a
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/resources/res_a/sql/history")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"sql":"SELECT 1","database":"db1","elapsed_ms":12,"row_count":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // List res_a → 1 item
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/resources/res_a/sql/history")
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
                    .uri("/api/resources/res_b/sql/history")
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
