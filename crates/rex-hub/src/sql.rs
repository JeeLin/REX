use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use rex_common::sql::{SqlConnector, SqlResult};

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

// 全局查询请求和响应类型
#[derive(Debug, Deserialize)]
pub struct GlobalQueryRequest {
    pub resource_ids: Vec<String>,
    pub sql: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum GlobalQueryEvent {
    Start {
        total_connections: usize,
    },
    Result {
        connection_id: String,
        data: Vec<serde_json::Value>,
        columns: Vec<String>,
        row_count: usize,
    },
    Progress {
        completed: usize,
        total: usize,
    },
    Done {
        total_rows: usize,
    },
    Error {
        connection_id: String,
        message: String,
    },
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

#[derive(Debug, serde::Serialize)]
pub struct SqlResourceInfo {
    pub id: String,
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

/// GET /api/resources/:resource_id/sql/peers — 获取同环境下的 SQL 资源列表
pub async fn list_peer_sql_resources(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<SqlResourceInfo>>>, (StatusCode, Json<ErrorResponse>)> {
    let resource = state
        .db
        .get_resource_by_id(&resource_id)
        .map_err(|e| err_resp("DB_ERROR", &format!("查询资源失败: {e}")))?
        .ok_or_else(|| not_found("RESOURCE_NOT_FOUND", "资源不存在"))?;

    let peers = state
        .db
        .list_sql_resources(&resource.environment_id)
        .map_err(|e| err_resp("DB_ERROR", &format!("查询同环境资源失败: {e}")))?;

    let result: Vec<SqlResourceInfo> = peers
        .into_iter()
        .map(|r| SqlResourceInfo {
            id: r.id,
            name: r.name,
            protocol: r.protocol,
        })
        .collect();

    Ok(Json(ApiResponse { data: result }))
}

/// POST /api/sql/global-query — 全局查询（并行执行相同SQL于多个资源）
pub async fn global_query(
    State(state): State<Arc<AppState>>,
    Json(input): Json<GlobalQueryRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<ErrorResponse>)> {
    if input.sql.trim().is_empty() {
        return Err(bad_request("SQL 不能为空"));
    }

    if input.resource_ids.is_empty() {
        return Err(bad_request("至少需要选择一个资源"));
    }

    // 从数据库加载资源配置，验证方言一致性
    let mut first_dialect = None;
    let mut connections_info = Vec::new();

    for rid in &input.resource_ids {
        let resource = state
            .db
            .get_resource_by_id(rid)
            .map_err(|e| err_resp("DB_ERROR", &format!("查询资源失败: {e}")))?
            .ok_or_else(|| not_found("RESOURCE_NOT_FOUND", &format!("资源 {rid} 不存在")))?;

        let dialect = match resource.protocol.as_str() {
            "mysql" | "postgresql" => resource.protocol.clone(),
            other => return Err(bad_request(&format!("不支持的协议: {other}"))),
        };

        if first_dialect.is_none() {
            first_dialect = Some(dialect.clone());
        } else if first_dialect.as_deref() != Some(dialect.as_str()) {
            return Err(bad_request(
                "所有资源必须使用相同方言（MySQL 或 PostgreSQL）",
            ));
        }

        // 从 config_json 解析连接参数
        let config: serde_json::Value = serde_json::from_str(&resource.config_json)
            .map_err(|e| bad_request(&format!("资源配置解析错误: {e}")))?;

        let host = config
            .get("host")
            .and_then(|v| v.as_str())
            .ok_or_else(|| bad_request(&format!("资源 {} 缺少 host", rid)))?
            .to_string();
        let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(3306) as u16;
        let username = config
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let password = config
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let database = config
            .get("database")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        connections_info.push((
            rid.clone(),
            resource.name.clone(),
            resource.protocol.clone(),
            host,
            port,
            username,
            password,
            database,
        ));
    }

    let (tx, rx) = mpsc::channel(100);
    let sql = input.sql.clone();
    let limit = input.limit;
    let offset = input.offset;
    let total = connections_info.len();

    tokio::spawn(async move {
        send_event(
            &tx,
            GlobalQueryEvent::Start {
                total_connections: total,
            },
        )
        .await;

        let results: Arc<Mutex<Vec<Option<QueryResult>>>> = Arc::new(Mutex::new(vec![None; total]));
        let completed = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for (idx, (conn_id, name, db_type, host, port, username, password, database)) in
            connections_info.into_iter().enumerate()
        {
            let tx = tx.clone();
            let results = results.clone();
            let completed = completed.clone();
            let sql = sql.clone();

            let handle = tokio::spawn(async move {
                let query_result = execute_single_db(
                    &name, &db_type, &host, port, &username, &password, &database, &sql, &tx,
                )
                .await;

                let completed_count = completed.fetch_add(1, Ordering::SeqCst) + 1;
                send_event(
                    &tx,
                    GlobalQueryEvent::Progress {
                        completed: completed_count,
                        total,
                    },
                )
                .await;

                // 用 name 而不是 conn_id 作为结果标识
                let mut qr = query_result;
                qr.connection_id = name;
                let mut results_lock = results.lock().unwrap();
                (*results_lock)[idx] = Some(qr);
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let ordered_results: Vec<QueryResult> = {
            let results_lock = results.lock().unwrap();
            results_lock.iter().filter_map(|opt| opt.clone()).collect()
        };
        for qr in &ordered_results {
            send_query_result(qr, limit, offset, &tx).await;
        }

        send_event(&tx, GlobalQueryEvent::Done { total_rows: 0 }).await;
    });

    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Some(Ok(event)) => Some((Ok(event), rx)),
            None => None,
        }
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

#[derive(Clone)]
struct QueryResult {
    connection_id: String,
    result: Result<SqlResult, String>,
}

/// 在单个数据库上执行查询（带超时和错误处理）
async fn execute_single_db(
    conn_id: &str,
    db_type: &str,
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    database: &str,
    sql: &str,
    tx: &mpsc::Sender<Result<Event, Infallible>>,
) -> QueryResult {
    let mut connector = match create_connector(db_type, host, port, username, password, database) {
        Ok(c) => c,
        Err(e) => {
            send_error(tx, conn_id, &e).await;
            return QueryResult {
                connection_id: conn_id.to_string(),
                result: Err(e),
            };
        }
    };

    if let Err(e) = timeout(Duration::from_secs(30), connector.connect())
        .await
        .map_err(|_| "连接超时".to_string())
        .and_then(|r| r.map_err(|e| format!("连接失败: {e}")))
    {
        send_error(tx, conn_id, &e).await;
        return QueryResult {
            connection_id: conn_id.to_string(),
            result: Err(e),
        };
    }

    let exec_result = timeout(Duration::from_secs(30), connector.execute(sql))
        .await
        .map_err(|_| "查询超时".to_string())
        .and_then(|r| r.map_err(|e| format!("查询执行失败: {e}")));

    let _ = connector.close().await;

    match exec_result {
        Ok(result) => QueryResult {
            connection_id: conn_id.to_string(),
            result: Ok(result),
        },
        Err(e) => QueryResult {
            connection_id: conn_id.to_string(),
            result: Err(e),
        },
    }
}

/// 根据数据库类型创建连接器
fn create_connector(
    db_type: &str,
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    database: &str,
) -> Result<Box<dyn SqlConnector>, String> {
    let config = serde_json::json!({
        "host": host,
        "port": port,
        "username": username,
        "password": password,
        "database": database
    });

    match db_type {
        "mysql" => {
            let connector = rex_mysql::MySqlConnector::from_json(&config.to_string())
                .map_err(|e| format!("MySQL 配置解析错误: {e}"))?;
            Ok(Box::new(connector))
        }
        "postgresql" => {
            let connector = rex_postgresql::PostgresConnector::from_json(&config.to_string())
                .map_err(|e| format!("PostgreSQL 配置解析错误: {e}"))?;
            Ok(Box::new(connector))
        }
        _ => Err(format!("不支持的数据库类型: {db_type}")),
    }
}

async fn send_error(tx: &mpsc::Sender<Result<Event, Infallible>>, conn_id: &str, message: &str) {
    let event = Event::default()
        .json_data(GlobalQueryEvent::Error {
            connection_id: conn_id.to_string(),
            message: message.to_string(),
        })
        .unwrap();
    let _ = tx.send(Ok(event)).await;
}

async fn send_event(tx: &mpsc::Sender<Result<Event, Infallible>>, data: GlobalQueryEvent) {
    let event = Event::default().json_data(data).unwrap();
    let _ = tx.send(Ok(event)).await;
}

/// 将查询结果通过 SSE 发送
async fn send_query_result(
    qr: &QueryResult,
    limit: Option<usize>,
    offset: Option<usize>,
    tx: &mpsc::Sender<Result<Event, Infallible>>,
) {
    match &qr.result {
        Ok(result) => {
            let limited: Vec<_> = result
                .rows
                .iter()
                .skip(offset.unwrap_or(0))
                .take(limit.unwrap_or(usize::MAX))
                .collect();
            let row_count = limited.len();
            let columns: Vec<String> = result.columns.iter().map(|c| c.name.clone()).collect();
            send_event(
                tx,
                GlobalQueryEvent::Result {
                    connection_id: qr.connection_id.clone(),
                    data: limited
                        .into_iter()
                        .map(|row| serde_json::to_value(row).unwrap_or(serde_json::json!({})))
                        .collect(),
                    columns,
                    row_count,
                },
            )
            .await;
        }
        Err(e) => {
            send_error(tx, &qr.connection_id, e).await;
        }
    }
}

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
            data_dir: std::env::temp_dir(),
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
