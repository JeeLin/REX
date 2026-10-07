//! SQL 控制台 REST + WebSocket 路由。
//!
//! 提供数据库连接、查询执行、元数据获取等 API。

use std::collections::HashMap;
use std::sync::Arc;

use crate::db::{audit_log_scoped, AuditScope};
use crate::error::{connect_error_response_with_stage, redact_secrets, ProtoKind};
use crate::models::SavedQuery;
use crate::resource_conn::{load_resource_config, ResourceConnInfo};
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use rex_common::sql::{ConnectRequest, DatabaseType, SqlConnector};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// 全局 SQL 连接池状态
pub type SqlState = Arc<Mutex<SqlConnectionPool>>;

/// 连接池 — sessionId → 连接器
///
/// `scopes` 是与 `connectors` 同键的审计归属表（`sql_api::SQL session_id` 唯一
/// 标识一个资源）。`disconnect` / `query` 只有 session id，得靠它还原归属；
/// 缺失时审计照写，只是维度留空（宁可缺维度，也不要编造）。
pub struct SqlConnectionPool {
    connectors: HashMap<String, Box<dyn SqlConnector>>,
    scopes: HashMap<String, AuditScope>,
}

impl Default for SqlConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlConnectionPool {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
            scopes: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: String, conn: Box<dyn SqlConnector>) {
        self.connectors.insert(id, conn);
    }

    /// 连同审计归属建连（连接时资源 id 与环境 id 都可得）。
    pub fn insert_with_scope(
        &mut self,
        id: String,
        conn: Box<dyn SqlConnector>,
        scope: AuditScope,
    ) {
        self.scopes.insert(id.clone(), scope);
        self.connectors.insert(id, conn);
    }

    pub fn remove(&mut self, id: &str) -> Option<Box<dyn SqlConnector>> {
        self.scopes.remove(id);
        self.connectors.remove(id)
    }

    /// 取会话的审计归属（session 缺失时给空归属，不让审计写入失败）。
    pub fn audit_scope(&self, id: &str) -> AuditScope {
        self.scopes.get(id).cloned().unwrap_or_default()
    }
}

/// 创建 SQL API 路由
pub fn sql_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/connect", axum::routing::post(connect))
        .route("/disconnect", axum::routing::post(disconnect))
        .route("/query", axum::routing::post(query))
        .route("/databases", axum::routing::get(databases))
        .route("/tables", axum::routing::get(tables))
        .route("/columns", axum::routing::get(columns))
        .route("/indexes", axum::routing::get(indexes))
        .route("/foreign_keys", axum::routing::get(foreign_keys))
        .route("/ddl", axum::routing::get(ddl))
        .route("/saved-queries", axum::routing::get(list_saved_queries))
        .route("/saved-queries", axum::routing::post(upsert_saved_query))
        .route(
            "/saved-queries/{id}",
            axum::routing::delete(delete_saved_query),
        )
        .route("/compare", axum::routing::post(compare))
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ConnectBody {
    #[serde(rename = "type", default)]
    subtype: Option<String>,
    resource_id: String,
}

#[derive(Debug, Serialize)]
struct ConnectResponse {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct DisconnectBody {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct QueryBody {
    session_id: String,
    sql: String,
    #[serde(default)]
    #[allow(dead_code)]
    database: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionQuery {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct TablesQuery {
    session_id: String,
    db: String,
}

#[derive(Debug, Deserialize)]
struct ColumnsQuery {
    session_id: String,
    db: String,
    table: String,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

fn error_response(code: &str, message: &str) -> (StatusCode, Json<ErrorBody>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorBody {
            error: ErrorDetail {
                code: code.to_string(),
                message: message.to_string(),
            },
        }),
    )
}

/// SQL 建连失败的结构化响应：`code` 带 `SQL_` 前缀、`stage` 给出连接阶段、
/// `message` 保留完整错误链并抹掉密码。
fn connect_error(context: &str, raw: &str, chain: &str) -> Json<crate::error::ErrorBody> {
    connect_error_response_with_stage(context, raw, chain, ProtoKind::Sql)
}

#[derive(Debug, Deserialize)]
struct CompareBody {
    session_id: String,
    sql_left: String,
    sql_right: String,
    #[serde(default)]
    key_columns: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct CompareResult {
    left: rex_common::sql::QueryResult,
    right: rex_common::sql::QueryResult,
    diffs: Vec<DiffRow>,
    summary: CompareSummary,
}

#[derive(Debug, Serialize)]
struct DiffRow {
    row_index: usize,
    diff_type: String,
    column: String,
    left_value: serde_json::Value,
    right_value: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct CompareSummary {
    left_rows: usize,
    right_rows: usize,
    identical_rows: usize,
    modified_rows: usize,
    only_in_left: usize,
    only_in_right: usize,
}
// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// SQL 事件的审计归属：资源 id 与环境 id 都取自资源记录，agent 维度取
/// `load_resource_config` 解析出的在线 Agent（直连为 None）。
///
/// 环境 id 回查资源表：`ResourceConnInfo` 只在「环境为 agent 模式」时用到
/// `environment_id`，直连资源不携带它，所以这里按资源 id 补查一次。
fn resource_scope(state: &AppState, res: &ResourceConnInfo, resource_id: &str) -> AuditScope {
    AuditScope {
        environment_id: state
            .db
            .get_resource(resource_id)
            .ok()
            .flatten()
            .map(|r| r.environment_id),
        resource_id: Some(resource_id.to_string()),
        agent_id: res.agent_id.clone(),
    }
}

/// 组装发给 Agent 的 SQL 连接配置（host/port/subtype + config_json 合并）。
///
/// 口径与 `file_api::agent_file_config` 一致：先合并 `config_json`，再以资源顶层
/// `username` 强制覆盖——前端只写顶层字段，`config_json` 中的历史 `username` 键
/// 不是权威值。Agent 侧 `agent_sql.rs` 直接读 `cfg["username"]` 发起认证。
fn agent_sql_config(res: &ResourceConnInfo, subtype: &str) -> serde_json::Value {
    let mut cfg = serde_json::json!({
        "host": res.host,
        "port": res.port.unwrap_or(0),
        "subtype": subtype,
    });
    if let serde_json::Value::Object(m) = res.config.clone() {
        for (k, v) in m {
            cfg[k] = v;
        }
    }
    if let serde_json::Value::Object(m) = &mut cfg {
        // 资源顶层 username 为权威字段，不被 config_json 中的历史键覆盖
        m.insert("username".to_string(), res.username.clone().into());
    }
    cfg
}

/// POST /api/sql/connect
async fn connect(
    State(state): State<AppState>,
    Json(body): Json<ConnectBody>,
) -> axum::response::Response {
    // 从 DB 加载资源连接信息
    let res = match load_resource_config(&state, &body.resource_id) {
        Ok(r) => r,
        Err(e) => return error_response("INVALID_RESOURCE", &e).into_response(),
    };

    // v0.70.7：SQL 资源合并后，subtype 取自资源探测结果（res.subtype），
    // 缺省时进入探测分支（直连侧下方 detect_dialect，agent 侧由 Agent 私网内探测）。
    let db_type = body
        .subtype
        .clone()
        .or_else(|| res.subtype.clone())
        .unwrap_or_else(|| "auto".to_string());

    // 审计归属在协议分发之前固定：agent 模式与直连都要写同一份归属。
    let audit_scope = resource_scope(&state, &res, &body.resource_id);

    // v0.70.6 子任务 #7：agent 模式 —— 协议在 Agent 私网内终结，Hub 仅做隧道中转。
    if res.use_agent {
        let agent_id = match res.agent_id.clone() {
            Some(id) => id,
            None => {
                return error_response("AGENT_UNAVAILABLE", "no online agent for environment")
                    .into_response()
            }
        };
        let agent_db_type = if db_type == "auto" {
            "auto"
        } else {
            db_type.as_str()
        };
        let cfg = agent_sql_config(&res, agent_db_type);
        tracing::debug!(action = "SQL_AGENT_CONFIG", resource_id = %body.resource_id, host = %res.host, port = %res.port.unwrap_or(0), subtype = %agent_db_type, has_password = res.config.get("password").and_then(|v| v.as_str()).is_some(), has_database = res.config.get("database").and_then(|v| v.as_str()).is_some(), "SQL agent config forwarded to agent");
        let channel_id = match crate::agent_ws::open_agent_session(
            &state,
            &agent_id,
            &body.resource_id,
            agent_db_type,
            cfg,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                return error_response("AGENT_CONNECT_FAILED", &e.to_string()).into_response()
            }
        };
        // 探测模式下 Agent 会回传 detected dialect（SessionOpened.subtype），在此持久化。
        let resolved_subtype: Option<String> = if db_type == "auto" {
            match crate::agent_ws::take_session_subtype(&state, &channel_id).await {
                Some(detected) => {
                    let _ = state.db.set_resource_subtype(&body.resource_id, &detected);
                    Some(detected)
                }
                None => None,
            }
        } else {
            None
        };
        let session_id = format!("sql_{}", &uuid::Uuid::new_v4().to_string()[..8]);
        state.sql_pool.lock().await.insert_with_scope(
            session_id.clone(),
            Box::new(crate::agent_proxy::AgentSqlProxy::new(
                state.clone(),
                channel_id,
                resolved_subtype.or_else(|| {
                    if db_type == "auto" {
                        None
                    } else {
                        Some(db_type.clone())
                    }
                }),
            )),
            audit_scope.clone(),
        );
        tracing::info!(action = "SQL_CONNECT_AGENT", session_id = %session_id, resource_id = %body.resource_id, resource_name = %res.name, agent_id = %agent_id, db_type = %db_type, "SQL connected via agent");
        return (StatusCode::OK, Json(ConnectResponse { session_id })).into_response();
    }

    let req = match db_type.to_lowercase().as_str() {
        "mysql" | "postgresql" | "postgres" | "clickhouse" | "sqlserver" | "mssql" | "oracle"
        | "mariadb" => ConnectRequest {
            host: res.host,
            port: res.port.unwrap_or(0),
            username: res.username,
            password: res
                .config
                .get("password")
                .and_then(|v| v.as_str())
                .map(String::from),
            database: res
                .config
                .get("database_name")
                .and_then(|v| v.as_str())
                .map(String::from),
        },
        "sqlite" => {
            let file_path = res
                .config
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            ConnectRequest {
                host: file_path.to_string(),
                port: 0,
                username: String::new(),
                password: None,
                database: None,
            }
        }
        "auto" => ConnectRequest {
            host: res.host,
            port: res.port.unwrap_or(0),
            username: res.username,
            password: res
                .config
                .get("password")
                .and_then(|v| v.as_str())
                .map(String::from),
            database: res
                .config
                .get("database_name")
                .and_then(|v| v.as_str())
                .map(String::from),
        },
        _ => {
            return error_response(
                "INVALID_DB_TYPE",
                &format!("unsupported database type: {}", db_type),
            )
            .into_response();
        }
    };

    // 建连失败时用于抹掉错误文案里的密码（连接请求随即被移动）
    let secret_password = req.password.clone();

    // v0.70.7：db_type 缺省（auto）或显式给出都走探测/连接；探测成功回写资源 db_type。
    let conn_result = match db_type.to_lowercase().as_str() {
        "mysql" | "mariadb" => rex_mysql::MySqlConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "postgresql" | "postgres" => rex_postgresql::PostgresConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "sqlite" => rex_sqlite::SqliteConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "clickhouse" => rex_clickhouse::ClickHouseConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "sqlserver" | "mssql" => rex_mssql::SqlServerConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "oracle" => rex_oracle::OracleConnector::connect(req)
            .await
            .map(|c| Box::new(c) as Box<dyn SqlConnector>),
        "auto" => detect_dialect(req).await,
        _ => unreachable!(),
    };

    match conn_result {
        Ok(conn) => {
            // 探测模式下回写 dialect 到资源（后续连接读缓存，无额外往返）。
            if db_type == "auto" {
                if let Some(detected) =
                    rex_common::sql::DetectedDialect::from_connector(conn.as_ref())
                {
                    let _ = state
                        .db
                        .set_resource_subtype(&body.resource_id, detected.as_str());
                }
            }
            let session_id = format!("sql_{}", &uuid::Uuid::new_v4().to_string()[..8]);
            let final_db_type = conn.database_type();
            tracing::info!(
                action = "SQL_CONNECT",
                db_type = ?final_db_type,
                resource_id = %body.resource_id,
                resource_name = %res.name,
                session_id = %session_id,
                "SQL connection established"
            );
            state.sql_pool.lock().await.insert_with_scope(
                session_id.clone(),
                conn,
                audit_scope.clone(),
            );
            audit_log_scoped(
                &state.db,
                "SQL_CONNECT",
                "success",
                Some(res.name.clone()),
                audit_scope.clone(),
            );
            (StatusCode::OK, Json(ConnectResponse { session_id })).into_response()
        }
        Err(e) => {
            tracing::warn!(
                action = "SQL_CONNECT",
                db_type = %db_type,
                resource_id = %body.resource_id,
                resource_name = %res.name,
                error = %e,
                "SQL connection failed"
            );
            // 带 detail 的失败事件：`audit_log_with_detail` 不接受归属维度，
            // `audit_log_scoped` 又没有 detail 位，只能在此直写补齐三个归属字段
            // （action/result/target/detail 逐字沿用既有语义）。
            let audit_db = state.db.clone();
            let scope = audit_scope.clone();
            let error_text = e.to_string();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "SQL_CONNECT".into(),
                    target: Some(res.name),
                    result: "failure".into(),
                    detail: Some(error_text),
                    environment_id: scope.environment_id,
                    resource_id: scope.resource_id,
                    agent_id: scope.agent_id,
                    ..Default::default()
                })
            })
            .await;
            connect_error(
                &format!("failed to connect to {} SQL database", db_type),
                &e.to_string(),
                &redact_secrets(
                    &format!("{e:#}"),
                    &[secret_password.as_deref().unwrap_or("")],
                ),
            )
            .into_response()
        }
    }
}

/// 通过共享方言探测函数连接，返回已连接的 [`SqlConnector`]。
async fn detect_dialect(req: ConnectRequest) -> anyhow::Result<Box<dyn SqlConnector>> {
    let result = rex_common::sql::detect_dialect(req, connect_by_dialect).await?;
    Ok(result.conn)
}

async fn connect_by_dialect(
    db_type: DatabaseType,
    req: ConnectRequest,
) -> anyhow::Result<Box<dyn SqlConnector>> {
    match db_type {
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            Ok(Box::new(rex_mysql::MySqlConnector::connect(req).await?))
        }
        DatabaseType::PostgreSQL => Ok(Box::new(
            rex_postgresql::PostgresConnector::connect(req).await?,
        )),
        DatabaseType::SQLite => Ok(Box::new(rex_sqlite::SqliteConnector::connect(req).await?)),
        DatabaseType::ClickHouse => Ok(Box::new(
            rex_clickhouse::ClickHouseConnector::connect(req).await?,
        )),
        DatabaseType::SqlServer => Ok(Box::new(rex_mssql::SqlServerConnector::connect(req).await?)),
        DatabaseType::Oracle => Ok(Box::new(rex_oracle::OracleConnector::connect(req).await?)),
    }
}

/// POST /api/sql/disconnect
async fn disconnect(
    State(state): State<AppState>,
    Json(body): Json<DisconnectBody>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let session_id = body.session_id.clone();
    // 归属随连接器一并移除，先取出再 remove。
    let scope = pool.audit_scope(&body.session_id);
    if let Some(mut conn) = pool.remove(&body.session_id) {
        let _ = conn.close().await;
        tracing::info!(action = "SQL_DISCONNECT", session_id = %session_id, "SQL session disconnected");
        audit_log_scoped(
            &state.db,
            "SQL_DISCONNECT",
            "success",
            Some(session_id),
            scope,
        );
        (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
    } else {
        error_response("SESSION_NOT_FOUND", "session not found").into_response()
    }
}

/// POST /api/sql/query
async fn query(State(state): State<AppState>, Json(body): Json<QueryBody>) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let scope = pool.audit_scope(&body.session_id);
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    // Apply query timeout (30 seconds)
    let timeout = std::time::Duration::from_secs(30);
    let execute_future = conn.execute(&body.sql);
    let query_len = body.sql.len();

    let start = std::time::Instant::now();
    match tokio::time::timeout(timeout, execute_future).await {
        Ok(Ok(mut result)) => {
            let elapsed = start.elapsed().as_millis() as u64;
            tracing::info!(
                action = "SQL_QUERY",
                session_id = %body.session_id,
                query_length = query_len,
                row_count = result.rows.len(),
                duration_ms = elapsed,
                "SQL query executed"
            );
            audit_log_scoped(
                &state.db,
                "SQL_QUERY",
                "success",
                Some(body.session_id),
                scope.clone(),
            );
            // Apply row limit (10000 rows)
            if result.rows.len() > 10000 {
                result.rows.truncate(10000);
            }
            (StatusCode::OK, Json(result)).into_response()
        }
        Ok(Err(e)) => {
            tracing::warn!(
                action = "SQL_QUERY",
                session_id = %body.session_id,
                query_length = query_len,
                error = %e,
                "SQL query failed"
            );
            // 同 `SQL_CONNECT` failure：detail 与归属无法共存于一个既有 helper，
            // 直写一次以保住「单条事件 + 归属 + 错误详情」三件事。
            let audit_db = state.db.clone();
            let error_text = e.to_string();
            let _ = tokio::task::spawn_blocking(move || {
                audit_db.write_audit_log(&crate::models::NewAuditEntry {
                    action: "SQL_QUERY".into(),
                    target: Some(body.session_id),
                    result: "failure".into(),
                    detail: Some(error_text),
                    environment_id: scope.environment_id,
                    resource_id: scope.resource_id,
                    agent_id: scope.agent_id,
                    ..Default::default()
                })
            })
            .await;
            error_response("QUERY_FAILED", &e.to_string()).into_response()
        }
        Err(_) => {
            tracing::warn!(
                action = "SQL_QUERY",
                session_id = %body.session_id,
                query_length = query_len,
                "SQL query timed out"
            );
            audit_log_scoped(
                &state.db,
                "SQL_QUERY",
                "timeout",
                Some(body.session_id),
                scope.clone(),
            );
            error_response("QUERY_TIMEOUT", "query timed out after 30 seconds").into_response()
        }
    }
}

/// GET /api/sql/databases?session_id=xxx
async fn databases(
    State(state): State<AppState>,
    Query(params): Query<SessionQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.databases().await {
        Ok(dbs) => (StatusCode::OK, Json(dbs)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

/// GET /api/sql/tables?session_id=xxx&db=xxx
async fn tables(
    State(state): State<AppState>,
    Query(params): Query<TablesQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.tables(&params.db).await {
        Ok(tables) => (StatusCode::OK, Json(tables)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

/// GET /api/sql/columns?session_id=xxx&db=xxx&table=xxx
async fn columns(
    State(state): State<AppState>,
    Query(params): Query<ColumnsQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.columns(&params.db, &params.table).await {
        Ok(cols) => (StatusCode::OK, Json(cols)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

/// GET /api/sql/indexes?session_id=xxx&db=xxx&table=xxx
async fn indexes(
    State(state): State<AppState>,
    Query(params): Query<ColumnsQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.indexes(&params.db, &params.table).await {
        Ok(idx) => (StatusCode::OK, Json(idx)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

/// GET /api/sql/foreign_keys?session_id=xxx&db=xxx&table=xxx
async fn foreign_keys(
    State(state): State<AppState>,
    Query(params): Query<ColumnsQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.foreign_keys(&params.db, &params.table).await {
        Ok(fks) => (StatusCode::OK, Json(fks)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

/// GET /api/sql/ddl?session_id=xxx&db=xxx&table=xxx
async fn ddl(
    State(state): State<AppState>,
    Query(params): Query<ColumnsQuery>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&params.session_id) {
        Some(c) => c,
        None => {
            return error_response("SESSION_NOT_FOUND", "session not found").into_response();
        }
    };

    match conn.ddl(&params.db, &params.table).await {
        Ok(d) => (StatusCode::OK, Json(d)).into_response(),
        Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Saved SQL Queries (命名查询，持久化于 settings 表)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SavedQueryBody {
    #[serde(default)]
    id: String,
    name: String,
    #[serde(default)]
    sql: String,
    #[serde(default)]
    db_type: Option<String>,
}

/// GET /api/sql/saved-queries
async fn list_saved_queries(State(state): State<AppState>) -> impl IntoResponse {
    let db = state.db.clone();
    match db.list_saved_queries() {
        Ok(list) => (StatusCode::OK, Json(list)).into_response(),
        Err(e) => error_response("DB_ERROR", &e.to_string()).into_response(),
    }
}

/// POST /api/sql/saved-queries
async fn upsert_saved_query(
    State(state): State<AppState>,
    Json(body): Json<SavedQueryBody>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() {
        return error_response("INVALID_NAME", "query name must not be empty").into_response();
    }
    let db = state.db.clone();
    let q = SavedQuery {
        id: body.id,
        name: body.name,
        sql: body.sql,
        db_type: body.db_type,
        updated_at: None,
    };
    match db.upsert_saved_query(&q) {
        Ok(stored) => (StatusCode::OK, Json(stored)).into_response(),
        Err(e) => error_response("DB_ERROR", &e.to_string()).into_response(),
    }
}

/// DELETE /api/sql/saved-queries/{id}
async fn delete_saved_query(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.db.clone();
    match db.delete_saved_query(&id) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error_response("DB_ERROR", &e.to_string()).into_response(),
    }
}

/// POST /api/sql/compare
/// Execute two SQL queries and compare results, highlighting differences.
async fn compare(
    State(state): State<AppState>,
    Json(body): Json<CompareBody>,
) -> impl IntoResponse {
    let mut pool = state.sql_pool.lock().await;
    let conn = match pool.connectors.get_mut(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    let timeout = std::time::Duration::from_secs(30);

    // Execute left query
    let left_result = match tokio::time::timeout(timeout, conn.execute(&body.sql_left)).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return error_response("QUERY_FAILED_LEFT", &e.to_string()).into_response(),
        Err(_) => return error_response("QUERY_TIMEOUT", "left query timed out").into_response(),
    };

    // Execute right query
    let right_result = match tokio::time::timeout(timeout, conn.execute(&body.sql_right)).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return error_response("QUERY_FAILED_RIGHT", &e.to_string()).into_response(),
        Err(_) => return error_response("QUERY_TIMEOUT", "right query timed out").into_response(),
    };

    // Compare results
    let diffs = compare_results(&left_result, &right_result, &body.key_columns);
    let summary = build_summary(&left_result, &right_result, &diffs);

    tracing::info!(
        action = "SQL_COMPARE",
        session_id = %body.session_id,
        left_rows = left_result.rows.len(),
        right_rows = right_result.rows.len(),
        diff_count = diffs.len(),
        "SQL compare executed"
    );

    (
        StatusCode::OK,
        Json(CompareResult {
            left: left_result,
            right: right_result,
            diffs,
            summary,
        }),
    )
        .into_response()
}

/// Compare two query results row by row.
fn compare_results(
    left: &rex_common::sql::QueryResult,
    right: &rex_common::sql::QueryResult,
    key_columns: &Option<Vec<String>>,
) -> Vec<DiffRow> {
    let mut diffs = Vec::new();

    // Build column index maps
    let left_cols: std::collections::HashMap<&str, usize> = left
        .columns
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.as_str(), i))
        .collect();
    let right_cols: std::collections::HashMap<&str, usize> = right
        .columns
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.as_str(), i))
        .collect();

    // Common columns
    let common_cols: Vec<&str> = left_cols
        .keys()
        .filter(|k| right_cols.contains_key(*k))
        .copied()
        .collect();

    // Determine key column indices
    let key_indices: Vec<(usize, usize)> = if let Some(keys) = key_columns {
        keys.iter()
            .filter_map(|k| {
                let li = left_cols.get(k.as_str())?;
                let ri = right_cols.get(k.as_str())?;
                Some((*li, *ri))
            })
            .collect()
    } else {
        Vec::new()
    };

    if key_indices.is_empty() {
        // Row-by-row comparison (by index)
        let max_rows = left.rows.len().max(right.rows.len());
        for i in 0..max_rows {
            let left_row = left.rows.get(i);
            let right_row = right.rows.get(i);

            match (left_row, right_row) {
                (Some(lr), Some(rr)) => {
                    for &col_name in &common_cols {
                        let li = left_cols[col_name];
                        let ri = right_cols[col_name];
                        if lr[li] != rr[ri] {
                            diffs.push(DiffRow {
                                row_index: i,
                                diff_type: "modified".to_string(),
                                column: col_name.to_string(),
                                left_value: lr[li].clone(),
                                right_value: rr[ri].clone(),
                            });
                        }
                    }
                }
                (Some(_), None) => {
                    diffs.push(DiffRow {
                        row_index: i,
                        diff_type: "only_in_left".to_string(),
                        column: "*".to_string(),
                        left_value: serde_json::Value::Bool(true),
                        right_value: serde_json::Value::Null,
                    });
                }
                (None, Some(_)) => {
                    diffs.push(DiffRow {
                        row_index: i,
                        diff_type: "only_in_right".to_string(),
                        column: "*".to_string(),
                        left_value: serde_json::Value::Null,
                        right_value: serde_json::Value::Bool(true),
                    });
                }
                (None, None) => {}
            }
        }
    } else {
        // Key-based comparison
        let mut right_matched: std::collections::BTreeSet<usize> =
            std::collections::BTreeSet::new();

        for (li, left_row) in left.rows.iter().enumerate() {
            let mut found = None;
            for (ri, right_row) in right.rows.iter().enumerate() {
                if right_matched.contains(&ri) {
                    continue;
                }
                let mut key_match = true;
                for &(lki, rki) in &key_indices {
                    if left_row[lki] != right_row[rki] {
                        key_match = false;
                        break;
                    }
                }
                if key_match {
                    found = Some(ri);
                    break;
                }
            }

            match found {
                Some(ri) => {
                    right_matched.insert(ri);
                    let right_row = &right.rows[ri];
                    for &col_name in &common_cols {
                        let li_idx = left_cols[col_name];
                        let ri_idx = right_cols[col_name];
                        if left_row[li_idx] != right_row[ri_idx] {
                            diffs.push(DiffRow {
                                row_index: li,
                                diff_type: "modified".to_string(),
                                column: col_name.to_string(),
                                left_value: left_row[li_idx].clone(),
                                right_value: right_row[ri_idx].clone(),
                            });
                        }
                    }
                }
                None => {
                    diffs.push(DiffRow {
                        row_index: li,
                        diff_type: "only_in_left".to_string(),
                        column: "*".to_string(),
                        left_value: serde_json::Value::Bool(true),
                        right_value: serde_json::Value::Null,
                    });
                }
            }
        }

        for ri in 0..right.rows.len() {
            if !right_matched.contains(&ri) {
                diffs.push(DiffRow {
                    row_index: ri,
                    diff_type: "only_in_right".to_string(),
                    column: "*".to_string(),
                    left_value: serde_json::Value::Null,
                    right_value: serde_json::Value::Bool(true),
                });
            }
        }
    }

    diffs
}

fn build_summary(
    left: &rex_common::sql::QueryResult,
    right: &rex_common::sql::QueryResult,
    diffs: &[DiffRow],
) -> CompareSummary {
    use std::collections::HashSet;

    let only_left = diffs
        .iter()
        .filter(|d| d.diff_type == "only_in_left")
        .count();
    let only_right = diffs
        .iter()
        .filter(|d| d.diff_type == "only_in_right")
        .count();
    // Count unique row indices that have at least one "modified" diff
    let modified_row_indices: HashSet<usize> = diffs
        .iter()
        .filter(|d| d.diff_type == "modified")
        .map(|d| d.row_index)
        .collect();
    let modified = modified_row_indices.len();
    let total_rows = left.rows.len().max(right.rows.len());
    let identical = total_rows.saturating_sub(only_left + only_right + modified);

    CompareSummary {
        left_rows: left.rows.len(),
        right_rows: right.rows.len(),
        identical_rows: identical,
        modified_rows: modified,
        only_in_left: only_left,
        only_in_right: only_right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_common::sql::{ColumnInfo, QueryResult, TableInfo};

    fn info(username: &str, config: &str) -> ResourceConnInfo {
        ResourceConnInfo {
            resource_id: "r1".into(),
            name: "orders".into(),
            protocol: "mysql".into(),
            host: "10.0.0.1".into(),
            port: Some(3306),
            username: username.to_string(),
            config: serde_json::from_str(config).unwrap(),
            subtype: None,
            use_agent: true,
            agent_id: Some("agent-1".into()),
        }
    }

    #[test]
    fn agent_sql_config_carries_username_and_merged_credentials() {
        let cfg = agent_sql_config(
            &info("alice", r#"{"password":"pw","database_name":"app"}"#),
            "mysql",
        );
        assert_eq!(
            cfg.get("username").and_then(|v| v.as_str()),
            Some("alice"),
            "Agent authenticates with cfg.username — a missing field means empty-user auth"
        );
        assert_eq!(cfg.get("host").and_then(|v| v.as_str()), Some("10.0.0.1"));
        assert_eq!(cfg.get("port").and_then(|v| v.as_u64()), Some(3306));
        assert_eq!(cfg.get("subtype").and_then(|v| v.as_str()), Some("mysql"));
        assert_eq!(cfg.get("password").and_then(|v| v.as_str()), Some("pw"));
        assert_eq!(
            cfg.get("database_name").and_then(|v| v.as_str()),
            Some("app")
        );
    }

    #[test]
    fn agent_sql_config_username_beats_stale_config_key() {
        let cfg = agent_sql_config(&info("alice", r#"{"username":"stale"}"#), "auto");
        assert_eq!(
            cfg.get("username").and_then(|v| v.as_str()),
            Some("alice"),
            "top-level username must stay authoritative after config_json merge"
        );
        assert_eq!(cfg.get("subtype").and_then(|v| v.as_str()), Some("auto"));
    }

    #[test]
    fn sql_connect_error_codes_are_prefixed_by_root_cause() {
        let cases = [
            (
                "Connection refused (os error 111)",
                "SQL_CONNECTION_REFUSED",
                "tcp",
            ),
            ("connection timed out", "SQL_CONNECTION_TIMEOUT", "timeout"),
            ("no such host", "SQL_DNS_FAILURE", "dns"),
            ("certificate verify failed", "SQL_TLS_FAILURE", "tls"),
            ("password authentication failed", "SQL_AUTH_FAILED", "auth"),
            ("something odd", "SQL_CONNECTION_FAILED", "connect"),
        ];
        for (raw, code, stage) in cases {
            let resp = connect_error("failed to connect to mysql SQL database", raw, raw);
            assert_eq!(resp.0.error.code, code);
            assert_eq!(resp.0.error.stage.as_deref(), Some(stage));
            assert!(resp.0.error.message.contains(raw));
        }
    }

    #[test]
    fn sql_connect_error_keeps_full_chain_and_masks_password() {
        let raw = "failed to connect";
        let chain = redact_secrets(
            "failed to connect: error connecting to mysql://alice:s3cr3t@10.0.0.1:3306/app: \
             Access denied for user 'alice'",
            &["s3cr3t", ""],
        );
        let resp = connect_error("failed to connect to mysql SQL database", raw, &chain);
        assert_eq!(resp.0.error.code, "SQL_CONNECTION_FAILED");
        assert_eq!(
            resp.0.error.message,
            "failed to connect to mysql SQL database: failed to connect: error connecting to \
             mysql://alice:***@10.0.0.1:3306/app: Access denied for user 'alice'"
        );
        assert!(!resp.0.error.message.contains("s3cr3t"));
    }

    /// 归属随会话存活：`SQL_DISCONNECT` / `SQL_QUERY` 只有 session id，
    /// 靠连接池里的 scope 表还原资源与环境；移除连接器后归属一并清掉。
    #[test]
    fn pool_scope_survives_session_and_is_dropped_with_it() {
        let mut pool = SqlConnectionPool::new();
        let scope = AuditScope {
            environment_id: Some("env-1".into()),
            resource_id: Some("res-1".into()),
            agent_id: None,
        };
        pool.insert_with_scope("sql_abc".into(), Box::new(DummyConnector), scope);
        assert_eq!(
            pool.audit_scope("sql_abc").resource_id.as_deref(),
            Some("res-1")
        );
        assert_eq!(
            pool.audit_scope("sql_abc").environment_id.as_deref(),
            Some("env-1")
        );

        assert!(pool.remove("sql_abc").is_some());
        assert!(
            pool.audit_scope("sql_abc").resource_id.is_none(),
            "a removed session must not leave a stale scope behind"
        );
        assert!(
            pool.audit_scope("never-opened").resource_id.is_none(),
            "an unknown session yields an empty scope rather than a guess"
        );
    }

    /// 按 resource_id 过滤能看到 SQL 会话事件（`SQL_CONNECT` 与 `SQL_QUERY`
    /// 共用连接池里登记的同一份归属）。
    #[tokio::test]
    async fn sql_audit_events_are_visible_when_filtering_by_resource_id() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());
        let resource_id = seed_resource(&state, "mysql");

        let scope = AuditScope {
            environment_id: Some("env-sql".into()),
            resource_id: Some(resource_id.clone()),
            agent_id: Some("agent-9".into()),
        };
        state.sql_pool.lock().await.insert_with_scope(
            "sql_live".into(),
            Box::new(DummyConnector),
            scope.clone(),
        );

        crate::db::audit_log_scoped(
            &state.db,
            "SQL_CONNECT",
            "success",
            Some("orders".into()),
            scope,
        );
        let session_scope = state.sql_pool.lock().await.audit_scope("sql_live");
        crate::db::audit_log_scoped(
            &state.db,
            "SQL_QUERY",
            "success",
            Some("sql_live".into()),
            session_scope,
        );

        let found = wait_for_audit_by_resource(&state, &resource_id, 2).await;
        assert!(
            found
                .iter()
                .all(|e| e.resource_id.as_deref() == Some(&*resource_id)),
            "every SQL event must carry the resource it was opened against"
        );
        let actions: Vec<&str> = found.iter().map(|e| e.action.as_str()).collect();
        assert!(actions.contains(&"SQL_CONNECT"), "got {actions:?}");
        assert!(actions.contains(&"SQL_QUERY"), "got {actions:?}");

        // 按 agent 维度过滤同样能看到（归属里的 agent_id 来自资源所属环境）。
        let by_agent = state
            .db
            .query_audit_log(&crate::models::AuditFilter {
                agent_id: Some("agent-9".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(by_agent.len(), 2, "agent filter must see both SQL events");

        assert!(
            state
                .db
                .query_audit_log(&crate::models::AuditFilter {
                    resource_id: Some("res-other".into()),
                    ..Default::default()
                })
                .unwrap()
                .is_empty(),
            "an unrelated resource id must not match"
        );
    }

    /// 归属表需要真实环境 id，故资源必须落在真实环境里。
    fn seed_resource(state: &AppState, protocol: &str) -> String {
        let env = state
            .db
            .create_environment(&crate::models::NewEnvironment {
                name: format!("env-{}", uuid::Uuid::new_v4()),
                description: None,
                connection_mode: Some("direct".into()),
            })
            .unwrap();
        state
            .db
            .create_resource(
                &env.id,
                &crate::models::NewResource {
                    name: "orders".into(),
                    protocol: protocol.into(),
                    host: "10.0.0.1".into(),
                    port: Some(3306),
                    username: Some("root".into()),
                    config_json: None,
                    subtype: None,
                    color: None,
                    sort_order: None,
                },
            )
            .unwrap()
            .id
    }

    /// `audit_log_scoped` 是 fire-and-forget 的 blocking 任务，查询前等它落库。
    async fn wait_for_audit_by_resource(
        state: &AppState,
        resource_id: &str,
        want: usize,
    ) -> Vec<crate::models::AuditEntry> {
        for _ in 0..50 {
            let entries = state
                .db
                .query_audit_log(&crate::models::AuditFilter {
                    resource_id: Some(resource_id.to_string()),
                    ..Default::default()
                })
                .unwrap();
            if entries.len() >= want {
                return entries;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("audit rows for {resource_id} never reached {want}");
    }

    /// 归属表只关心键，不关心连接行为，故用空实现连接器占位。
    struct DummyConnector;

    #[async_trait::async_trait]
    impl SqlConnector for DummyConnector {
        fn database_type(&self) -> DatabaseType {
            DatabaseType::MySQL
        }

        async fn execute(&mut self, _sql: &str) -> anyhow::Result<QueryResult> {
            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: 0,
                elapsed_ms: 0,
            })
        }

        async fn databases(&mut self) -> anyhow::Result<Vec<String>> {
            Ok(Vec::new())
        }

        async fn tables(&mut self, _db: &str) -> anyhow::Result<Vec<TableInfo>> {
            Ok(Vec::new())
        }

        async fn columns(&mut self, _db: &str, _table: &str) -> anyhow::Result<Vec<ColumnInfo>> {
            Ok(Vec::new())
        }

        async fn close(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
    }
}
