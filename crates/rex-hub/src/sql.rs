use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use rex_common::sql::SqlConnector;

use crate::helpers::{bad_request, err_resp, not_found, ApiResponse, ErrorResponse};
use crate::routes::AppState;

// ── 请求/响应类型 ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ExecuteRequest {
    pub sql: String,
}

#[derive(Debug, Deserialize)]
pub struct TablesQuery {
    pub database: String,
}

#[derive(Debug, Deserialize)]
pub struct ColumnsQuery {
    pub database: String,
    pub table: String,
}

// ── 路由处理函数 ──────────────────────────────────────────

/// POST /api/resources/:resource_id/sql/execute — 执行 SQL
pub async fn execute_sql(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
    Json(input): Json<ExecuteRequest>,
) -> Result<Json<ApiResponse<rex_common::sql::SqlResult>>, (StatusCode, Json<ErrorResponse>)> {
    if input.sql.trim().is_empty() {
        return Err(bad_request("SQL 不能为空"));
    }
    let mut connector = get_sql_connector(&state, &resource_id).await?;

    connector
        .connect()
        .await
        .map_err(|e| err_resp("SQL_CONNECT_FAILED", &format!("连接失败: {e}")))?;

    let result = connector
        .execute(&input.sql)
        .await
        .map_err(|e| err_resp("SQL_EXECUTE_FAILED", &format!("执行失败: {e}")))?;

    let _ = connector.close().await;

    Ok(Json(ApiResponse { data: result }))
}

/// GET /api/resources/:resource_id/sql/databases — 列出数据库
pub async fn list_databases(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<rex_common::sql::DatabaseInfo>>>, (StatusCode, Json<ErrorResponse>)>
{
    let mut connector = get_sql_connector(&state, &resource_id).await?;

    connector
        .connect()
        .await
        .map_err(|e| err_resp("SQL_CONNECT_FAILED", &format!("连接失败: {e}")))?;

    let databases = connector
        .list_databases()
        .await
        .map_err(|e| err_resp("SQL_LIST_FAILED", &format!("列出数据库失败: {e}")))?;

    let _ = connector.close().await;

    Ok(Json(ApiResponse { data: databases }))
}

/// GET /api/resources/:resource_id/sql/tables?database=x — 列出表
pub async fn list_tables(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
    Query(query): Query<TablesQuery>,
) -> Result<Json<ApiResponse<Vec<rex_common::sql::TableInfo>>>, (StatusCode, Json<ErrorResponse>)> {
    let mut connector = get_sql_connector(&state, &resource_id).await?;

    connector
        .connect()
        .await
        .map_err(|e| err_resp("SQL_CONNECT_FAILED", &format!("连接失败: {e}")))?;

    let tables = connector
        .list_tables(&query.database)
        .await
        .map_err(|e| err_resp("SQL_LIST_FAILED", &format!("列出表失败: {e}")))?;

    let _ = connector.close().await;

    Ok(Json(ApiResponse { data: tables }))
}

/// GET /api/resources/:resource_id/sql/columns?database=x&table=y — 列出列
pub async fn list_columns(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
    Query(query): Query<ColumnsQuery>,
) -> Result<Json<ApiResponse<Vec<rex_common::sql::ColumnInfo>>>, (StatusCode, Json<ErrorResponse>)>
{
    let mut connector = get_sql_connector(&state, &resource_id).await?;

    connector
        .connect()
        .await
        .map_err(|e| err_resp("SQL_CONNECT_FAILED", &format!("连接失败: {e}")))?;

    let columns = connector
        .list_columns(&query.database, &query.table)
        .await
        .map_err(|e| err_resp("SQL_LIST_FAILED", &format!("列出列失败: {e}")))?;

    let _ = connector.close().await;

    Ok(Json(ApiResponse { data: columns }))
}

#[derive(Debug, serde::Serialize)]
pub struct ResourceInfo {
    pub name: String,
    pub protocol: String,
}

/// GET /api/resources/:resource_id/info — 获取资源基本信息
pub async fn get_resource_info(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
) -> Result<Json<ApiResponse<ResourceInfo>>, (StatusCode, Json<ErrorResponse>)> {
    let resource = state
        .db
        .get_resource_by_id(&resource_id)
        .map_err(|e| err_resp("DB_ERROR", &format!("查询资源失败: {e}")))?
        .ok_or_else(|| not_found("RESOURCE_NOT_FOUND", "资源不存在"))?;

    Ok(Json(ApiResponse {
        data: ResourceInfo {
            name: resource.name,
            protocol: resource.protocol,
        },
    }))
}

// ── 内部函数 ──────────────────────────────────────────────

/// 根据资源 ID 获取 SqlConnector
async fn get_sql_connector(
    state: &Arc<AppState>,
    resource_id: &str,
) -> Result<Box<dyn SqlConnector>, (StatusCode, Json<ErrorResponse>)> {
    let resource = state
        .db
        .get_resource_by_id(resource_id)
        .map_err(|e| err_resp("DB_ERROR", &format!("查询资源失败: {e}")))?
        .ok_or_else(|| not_found("RESOURCE_NOT_FOUND", "资源不存在"))?;

    match resource.protocol.as_str() {
        "mysql" => {
            let connector = rex_mysql::MySqlConnector::from_json(&resource.config_json)
                .map_err(|e| bad_request(&format!("MySQL 配置解析错误: {e}")))?;
            Ok(Box::new(connector))
        }
        "postgresql" => {
            let connector = rex_postgresql::PostgresConnector::from_json(&resource.config_json)
                .map_err(|e| bad_request(&format!("PostgreSQL 配置解析错误: {e}")))?;
            Ok(Box::new(connector))
        }
        other => Err(bad_request(&format!("不支持的 SQL 协议: {other}"))),
    }
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, post};
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
        })
    }

    fn test_app() -> Router {
        let state = test_state();
        Router::new()
            .route("/api/resources/:resource_id/sql/execute", post(execute_sql))
            .route(
                "/api/resources/:resource_id/sql/databases",
                get(list_databases),
            )
            .route("/api/resources/:resource_id/sql/tables", get(list_tables))
            .route("/api/resources/:resource_id/sql/columns", get(list_columns))
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
    async fn execute_sql_returns_404_for_unknown_resource() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/resources/nonexistent/sql/execute")
                    .header("authorization", auth_header())
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"sql":"SELECT 1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "RESOURCE_NOT_FOUND");
    }

    #[tokio::test]
    async fn execute_sql_rejects_empty_sql() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/resources/nonexistent/sql/execute")
                    .header("authorization", auth_header())
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"sql":"  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["message"], "SQL 不能为空");
    }

    #[tokio::test]
    async fn list_databases_returns_404_for_unknown_resource() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/resources/nonexistent/sql/databases")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn list_tables_returns_404_for_unknown_resource() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/resources/nonexistent/sql/tables?database=test")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn list_columns_returns_404_for_unknown_resource() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/resources/nonexistent/sql/columns?database=test&table=users")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

}
