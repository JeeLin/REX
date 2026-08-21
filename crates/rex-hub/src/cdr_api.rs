//! SIP 通话记录 (CDR) REST API。

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::models::{CdrFilter, CdrRecord};
use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })),
    )
}

pub fn cdr_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list_cdr))
        .route("/:id", axum::routing::get(get_cdr))
}

#[derive(serde::Deserialize, Default)]
pub struct CdrQuery {
    pub resource_id: Option<String>,
    pub direction: Option<String>,
    pub state: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub sort: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

async fn list_cdr(
    State(state): State<AppState>,
    Query(q): Query<CdrQuery>,
) -> ApiResult<serde_json::Value> {
    let filter = CdrFilter {
        resource_id: q.resource_id,
        direction: q.direction,
        state: q.state,
        from: q.from,
        to: q.to,
        sort: q.sort,
        limit: q.limit.or(Some(100)),
        offset: q.offset,
    };
    let db = state.db.clone();
    let (records, total) = tokio::task::spawn_blocking(move || {
        let records = db.query_cdr(&filter)?;
        let total = db.count_cdr(&filter)?;
        Ok::<_, rex_common::RExError>((records, total))
    })
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    Ok(Json(serde_json::json!({
        "records": records,
        "total": total,
    })))
}

async fn get_cdr(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<CdrRecord> {
    let db = state.db.clone();
    let rec = tokio::task::spawn_blocking(move || db.get_cdr(&id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    match rec {
        Some(r) => Ok(Json(r)),
        None => Err(err(StatusCode::NOT_FOUND, "CDR record not found")),
    }
}
