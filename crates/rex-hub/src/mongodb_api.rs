//! MongoDB REST API 路由。
//!
//! 提供 MongoDB 连接、数据库列表、集合列表、查询执行等 API。

use std::collections::HashMap;
use std::sync::Arc;
use url::form_urlencoded;

use crate::db::{audit_log_scoped, AuditScope};
use crate::resource_conn::{load_resource_config, ResourceConnInfo};
use crate::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use futures_util::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::options::ClientOptions;
use mongodb::{Client, Collection};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::error::{
    connect_error_response_with_stage, error_chain, error_with_status, error_with_status_and_stage,
    redact_secrets, ErrorBody, ProtoKind,
};

/// MongoDB 连接池：session_id → Client
///
/// 注意：池里只有 `Client`，不带资源信息，因此只持有 session_id 的
/// `disconnect` / `query` 无法还原归属（补齐需要改 `AppState` 字段或池的
/// 元素类型，超出本文件范围）。这两处审计维持无归属，见交付说明。
pub type MongoState = Arc<Mutex<HashMap<String, Client>>>;

/// MongoDB 建连事件的归属：资源与环境取自资源记录，agent 维度取
/// `load_resource_config` 解析出的在线 Agent（直连为 None）。
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

/// 创建 MongoDB API 路由
pub fn mongodb_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/connect", axum::routing::post(connect))
        .route("/disconnect", axum::routing::post(disconnect))
        .route("/databases", axum::routing::get(databases))
        .route("/collections", axum::routing::get(collections))
        .route("/query", axum::routing::post(query))
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ConnectBody {
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
struct SessionQuery {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct CollectionsQuery {
    session_id: String,
    database: String,
}

#[derive(Debug, Deserialize)]
struct QueryBody {
    session_id: String,
    database: String,
    collection: String,
    operation: String,
    #[serde(default)]
    filter: Option<Document>,
    #[serde(default)]
    projection: Option<Document>,
    #[serde(default)]
    sort: Option<Document>,
    #[serde(default)]
    limit: Option<i64>,
}

fn error_response(code: &str, message: &str) -> (StatusCode, Json<ErrorBody>) {
    error_with_status(StatusCode::BAD_REQUEST, code, message)
}

/// 连接串 / 客户端构造阶段失败：网络阶段尚未开始，`stage` 固定为 `config`。
fn config_error(code: &str, message: &str) -> (StatusCode, Json<ErrorBody>) {
    error_with_status_and_stage(StatusCode::BAD_REQUEST, code, message, "config")
}

/// MongoDB 建连失败的结构化响应：`code` 带 `MONGODB_` 前缀（既有
/// `INVALID_URI` 之类的裸码由调用方另行给出，保持 wire 兼容）、`stage` 给出
/// 连接阶段、`message` 保留完整错误链并抹掉密码。
fn connect_error(context: &str, raw: &str, chain: &str) -> Json<ErrorBody> {
    connect_error_response_with_stage(context, raw, chain, ProtoKind::MongoDb)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/mongodb/connect
async fn connect(
    State(state): State<AppState>,
    Json(body): Json<ConnectBody>,
) -> impl IntoResponse {
    let res = match load_resource_config(&state, &body.resource_id) {
        Ok(r) => r,
        Err(e) => return error_response("INVALID_RESOURCE", &e).into_response(),
    };

    // Build MongoDB connection string
    let host = &res.host;
    let port = res.port.unwrap_or(27017);
    let username = &res.username;
    let password = res
        .config
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let auth_db = res
        .config
        .get("auth_database")
        .and_then(|v| v.as_str())
        .unwrap_or("admin");

    // 审计归属在建连前固定：三条 MONGO_CONNECT failure 分支与 success 共用。
    let audit_scope = resource_scope(&state, &res, &body.resource_id);

    let enc_pass = form_urlencoded::byte_serialize(password.as_bytes()).collect::<String>();
    // 错误文案可能回显连接串，密码按「原文 / URL 编码」两种形态一起抹除
    let secrets = [password, enc_pass.as_str()];

    let conn_str = if !username.is_empty() {
        let enc_user = form_urlencoded::byte_serialize(username.as_bytes()).collect::<String>();
        format!(
            "mongodb://{}:{}@{}:{}/?authSource={}",
            enc_user, enc_pass, host, port, auth_db
        )
    } else {
        format!("mongodb://{}:{}", host, port)
    };

    let options = match ClientOptions::parse(&conn_str).await {
        Ok(o) => o,
        Err(e) => {
            audit_log_scoped(
                &state.db,
                "MONGO_CONNECT",
                "failure",
                Some(res.name.clone()),
                audit_scope.clone(),
            );
            return config_error(
                "INVALID_URI",
                &redact_secrets(
                    &format!("invalid MongoDB URI: {}", error_chain(&e)),
                    &secrets,
                ),
            )
            .into_response();
        }
    };

    let client = match Client::with_options(options) {
        Ok(c) => c,
        Err(e) => {
            audit_log_scoped(
                &state.db,
                "MONGO_CONNECT",
                "failure",
                Some(res.name.clone()),
                audit_scope.clone(),
            );
            return connect_error(
                "failed to create MongoDB client",
                &e.to_string(),
                &redact_secrets(&error_chain(&e), &secrets),
            )
            .into_response();
        }
    };

    // Verify connection with a ping
    match client
        .database("admin")
        .run_command(doc! { "ping": 1 })
        .await
    {
        Ok(_) => {}
        Err(e) => {
            audit_log_scoped(
                &state.db,
                "MONGO_CONNECT",
                "failure",
                Some(res.name.clone()),
                audit_scope.clone(),
            );
            return connect_error(
                &format!("MongoDB ping failed at {}:{}", host, port),
                &e.to_string(),
                &redact_secrets(&error_chain(&e), &secrets),
            )
            .into_response();
        }
    }

    let session_id = uuid::Uuid::new_v4().to_string();

    state
        .mongo_pool
        .lock()
        .await
        .insert(session_id.clone(), client);

    tracing::info!(
        action = "MONGO_CONNECT",
        session_id = %session_id,
        resource_id = %body.resource_id,
        host = %host,
        port = port,
        "MongoDB connected"
    );

    audit_log_scoped(
        &state.db,
        "MONGO_CONNECT",
        "success",
        Some(res.name),
        audit_scope,
    );
    (StatusCode::OK, Json(ConnectResponse { session_id })).into_response()
}

/// POST /api/mongodb/disconnect
async fn disconnect(
    State(state): State<AppState>,
    Json(body): Json<DisconnectBody>,
) -> impl IntoResponse {
    state.mongo_pool.lock().await.remove(&body.session_id);
    // 会话已销毁且池内无资源信息，归属留空（见 `MongoState` 说明）。
    audit_log_scoped(
        &state.db,
        "MONGO_DISCONNECT",
        "success",
        Some(body.session_id),
        AuditScope::default(),
    );
    StatusCode::NO_CONTENT.into_response()
}

/// GET /api/mongodb/databases?session_id=xxx
async fn databases(
    State(state): State<AppState>,
    Query(params): Query<SessionQuery>,
) -> impl IntoResponse {
    let pool = state.mongo_pool.lock().await;
    let client = match pool.get(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    match client.list_database_names().await {
        Ok(names) => {
            let result: Vec<String> = names.into_iter().collect();
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(e) => error_response("LIST_FAILED", &e.to_string()).into_response(),
    }
}

/// GET /api/mongodb/collections?session_id=xxx&database=xxx
async fn collections(
    State(state): State<AppState>,
    Query(params): Query<CollectionsQuery>,
) -> impl IntoResponse {
    let pool = state.mongo_pool.lock().await;
    let client = match pool.get(&params.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    let db = client.database(&params.database);
    match db.list_collection_names().await {
        Ok(names) => {
            let result: Vec<String> = names.into_iter().collect();
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(e) => error_response("LIST_FAILED", &e.to_string()).into_response(),
    }
}

/// POST /api/mongodb/query
async fn query(State(state): State<AppState>, Json(body): Json<QueryBody>) -> impl IntoResponse {
    let pool = state.mongo_pool.lock().await;
    let client = match pool.get(&body.session_id) {
        Some(c) => c,
        None => return error_response("SESSION_NOT_FOUND", "session not found").into_response(),
    };

    let db = client.database(&body.database);
    let coll: Collection<Document> = db.collection(&body.collection);

    let start = std::time::Instant::now();

    // 请求只有 session id，连接池不带资源信息 → 归属留空（见 `MongoState` 说明）。
    audit_log_scoped(
        &state.db,
        "MONGO_QUERY",
        "success",
        Some(format!("{}/{}", body.database, body.collection)),
        AuditScope::default(),
    );

    match body.operation.to_lowercase().as_str() {
        "find" => {
            let filter = body.filter.unwrap_or_default();
            let opts = mongodb::options::FindOptions::builder()
                .projection(body.projection)
                .sort(body.sort)
                .limit(body.limit.or(Some(1000)))
                .build();
            match coll.find(filter).with_options(opts).await {
                Ok(mut cursor) => {
                    let mut docs = Vec::new();
                    loop {
                        match cursor.try_next().await {
                            Ok(None) => break,
                            Ok(Some(doc)) => docs.push(doc),
                            Err(e) => {
                                return error_response("CURSOR_ERROR", &e.to_string())
                                    .into_response()
                            }
                        }
                    }
                    let elapsed = start.elapsed().as_millis() as u64;
                    let count = docs.len() as i64;
                    let response = QueryResponse {
                        documents: docs,
                        count,
                        elapsed_ms: elapsed,
                    };
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
            }
        }
        "count" => {
            let filter = body.filter.unwrap_or_default();
            match coll.count_documents(filter).await {
                Ok(count) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    let response = CountResponse {
                        count: count as i64,
                        elapsed_ms: elapsed,
                    };
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
            }
        }
        "aggregate" => {
            let pipeline = body.filter.unwrap_or_default();
            // Treat filter as pipeline stages document
            let mut stages: Vec<Document> = Vec::new();
            if let Ok(arr) = pipeline.get_array("pipeline") {
                for val in arr {
                    if let Some(doc) = val.as_document() {
                        stages.push(doc.clone());
                    }
                }
            } else {
                stages.push(doc! { "$match": pipeline });
            }

            match coll.aggregate(stages).await {
                Ok(mut cursor) => {
                    let mut docs = Vec::new();
                    loop {
                        match cursor.try_next().await {
                            Ok(None) => break,
                            Ok(Some(doc)) => docs.push(doc),
                            Err(e) => {
                                return error_response("CURSOR_ERROR", &e.to_string())
                                    .into_response()
                            }
                        }
                    }
                    let elapsed = start.elapsed().as_millis() as u64;
                    let count = docs.len() as i64;
                    let response = QueryResponse {
                        documents: docs,
                        count,
                        elapsed_ms: elapsed,
                    };
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => error_response("QUERY_FAILED", &e.to_string()).into_response(),
            }
        }
        _ => error_response(
            "INVALID_OPERATION",
            &format!("unsupported operation: {}", body.operation),
        )
        .into_response(),
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct QueryResponse {
    documents: Vec<Document>,
    count: i64,
    elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
struct CountResponse {
    count: i64,
    elapsed_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_uri_keeps_legacy_code_and_adds_config_stage() {
        let chain = redact_secrets(
            "invalid MongoDB URI: error parsing uri mongodb://alice:s3cr3t@10.0.0.1:27017",
            &["s3cr3t"],
        );
        let (status, resp) = config_error("INVALID_URI", &format!("invalid MongoDB URI: {chain}"));
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(resp.0.error.code, "INVALID_URI");
        assert_eq!(resp.0.error.stage.as_deref(), Some("config"));
        assert!(!resp.0.error.message.contains("s3cr3t"));
    }

    #[test]
    fn client_creation_failure_is_prefixed_and_does_not_leak_password() {
        let raw = "invalid username or password";
        let chain = redact_secrets(
            "error connecting to mongodb://alice:s3cr3t@10.0.0.1:27017: invalid username or password",
            &["s3cr3t"],
        );
        let resp = connect_error("failed to create MongoDB client", raw, &chain);
        assert_eq!(resp.0.error.code, "MONGODB_AUTH_FAILED");
        assert_eq!(resp.0.error.stage.as_deref(), Some("auth"));
        assert!(!resp.0.error.message.contains("s3cr3t"));
        assert!(resp
            .0
            .error
            .message
            .contains("invalid username or password"));
    }

    #[test]
    fn ping_failure_stages_by_root_cause() {
        let cases = [
            (
                "Kind: Command ... timed out",
                "MONGODB_CONNECTION_TIMEOUT",
                "timeout",
            ),
            ("connection refused", "MONGODB_CONNECTION_REFUSED", "tcp"),
            ("no such host", "MONGODB_DNS_FAILURE", "dns"),
            ("certificate verify failed", "MONGODB_TLS_FAILURE", "tls"),
            (
                "server selection error",
                "MONGODB_CONNECTION_FAILED",
                "connect",
            ),
        ];
        for (raw, code, stage) in cases {
            let resp = connect_error("MongoDB ping failed at 10.0.0.1:27017", raw, raw);
            assert_eq!(resp.0.error.code, code);
            assert_eq!(resp.0.error.stage.as_deref(), Some(stage));
        }
    }

    /// `MONGO_CONNECT` 建连成功事件的归属来自资源记录：resource_id 与环境 id
    /// 真实可得，agent 维度取自资源所属环境（直连环境为 None）。
    #[tokio::test]
    async fn mongo_connect_success_event_is_visible_when_filtering_by_resource_id() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());
        let env = state
            .db
            .create_environment(&crate::models::NewEnvironment {
                name: format!("env-{}", uuid::Uuid::new_v4()),
                description: None,
                connection_mode: Some("direct".into()),
            })
            .unwrap();
        let resource_id = state
            .db
            .create_resource(
                &env.id,
                &crate::models::NewResource {
                    name: "catalog".into(),
                    protocol: "mongodb".into(),
                    host: "10.0.0.1".into(),
                    port: Some(27017),
                    username: Some("root".into()),
                    config_json: None,
                    subtype: None,
                    color: None,
                    sort_order: None,
                },
            )
            .unwrap()
            .id;

        let res = load_resource_config(&state, &resource_id).unwrap();
        let scope = resource_scope(&state, &res, &resource_id);

        // 建连成功写审计（connect handler 走直连路径写这里）。
        crate::db::audit_log_scoped(
            &state.db,
            "MONGO_CONNECT",
            "success",
            Some(res.name.clone()),
            scope,
        );

        let mut seen = Vec::new();
        for _ in 0..50 {
            seen = state
                .db
                .query_audit_log(&crate::models::AuditFilter {
                    resource_id: Some(resource_id.clone()),
                    ..Default::default()
                })
                .unwrap();
            if !seen.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert_eq!(
            seen.len(),
            1,
            "MONGO_CONNECT must be indexed under its resource"
        );
        let entry = &seen[0];
        assert_eq!(entry.action, "MONGO_CONNECT");
        assert_eq!(entry.result, "success");
        assert_eq!(entry.resource_id.as_deref(), Some(&*resource_id));
        assert_eq!(entry.environment_id.as_deref(), Some(&*env.id));
        assert!(
            entry.agent_id.is_none(),
            "a direct environment carries no agent"
        );
        assert!(state
            .db
            .query_audit_log(&crate::models::AuditFilter {
                resource_id: Some("res-other".into()),
                ..Default::default()
            })
            .unwrap()
            .is_empty());
    }
}
