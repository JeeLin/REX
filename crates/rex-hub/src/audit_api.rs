//! 审计日志 REST API。

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::models::{AuditEntry, AuditFilter};
use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })))
}

pub fn audit_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(query_audit_log))
}

#[derive(serde::Deserialize, Default)]
pub struct AuditQuery {
    pub time_from: Option<String>,
    pub time_to: Option<String>,
    pub action: Option<String>,
    pub result: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

async fn query_audit_log(
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> ApiResult<Vec<AuditEntry>> {
    let filter = AuditFilter {
        time_from: q.time_from,
        time_to: q.time_to,
        action: q.action,
        environment_id: None,
        result: q.result,
        limit: q.limit.or(Some(100)),
        offset: q.offset,
    };
    let db = state.db.clone();
    let entries = tokio::task::spawn_blocking(move || db.query_audit_log(&filter))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    Ok(Json(entries))
}
