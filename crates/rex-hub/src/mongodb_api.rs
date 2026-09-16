//! MongoDB REST API 路由。
//!
//! 提供 MongoDB 连接、数据库列表、集合列表、查询执行等 API。

use std::collections::HashMap;
use std::sync::Arc;

use crate::resource_conn::load_resource_config;
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

/// MongoDB 连接池：session_id → Client
pub type MongoState = Arc<Mutex<HashMap<String, Client>>>;

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

    let conn_str = if !username.is_empty() {
        format!(
            "mongodb://{}:{}@{}:{}/?authSource={}",
            username, password, host, port, auth_db
        )
    } else {
        format!("mongodb://{}:{}", host, port)
    };

    let options = match ClientOptions::parse(&conn_str).await {
        Ok(o) => o,
        Err(e) => {
            return error_response("INVALID_URI", &format!("invalid MongoDB URI: {}", e))
                .into_response()
        }
    };

    let client = match Client::with_options(options) {
        Ok(c) => c,
        Err(e) => {
            return error_response("CONNECT_FAILED", &format!("failed to create client: {}", e))
                .into_response()
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
            return error_response("PING_FAILED", &format!("ping failed: {}", e)).into_response()
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

    (StatusCode::OK, Json(ConnectResponse { session_id })).into_response()
}

/// POST /api/mongodb/disconnect
async fn disconnect(
    State(state): State<AppState>,
    Json(body): Json<DisconnectBody>,
) -> impl IntoResponse {
    state.mongo_pool.lock().await.remove(&body.session_id);
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
