//! 审计日志 REST API。

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::models::{AuditEntry, AuditFilter, AuditStats};
use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })),
    )
}

pub fn audit_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(query_audit_log))
        .route("/stats", axum::routing::get(query_audit_stats))
        .route("/security-report", axum::routing::get(security_report))
}

#[derive(serde::Deserialize, Default)]
pub struct AuditQuery {
    pub time_from: Option<String>,
    pub time_to: Option<String>,
    pub action: Option<String>,
    pub environment_id: Option<String>,
    pub agent_id: Option<String>,
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
        environment_id: q.environment_id,
        agent_id: q.agent_id,
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

#[derive(serde::Deserialize, Default)]
pub struct AuditStatsQuery {
    pub action: Option<String>,
    pub environment_id: Option<String>,
    pub result: Option<String>,
    pub time_from: Option<String>,
    pub time_to: Option<String>,
}

async fn query_audit_stats(
    State(state): State<AppState>,
    Query(q): Query<AuditStatsQuery>,
) -> ApiResult<AuditStats> {
    let filter = AuditFilter {
        time_from: q.time_from,
        time_to: q.time_to,
        action: q.action,
        environment_id: q.environment_id,
        agent_id: None,
        result: q.result,
        limit: None,
        offset: None,
    };
    let db = state.db.clone();
    let stats = tokio::task::spawn_blocking(move || db.query_audit_stats(&filter))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    Ok(Json(stats))
}

/// 安全审计报告：最近 24h 登录失败次数、异常 IP 列表。
async fn security_report(State(state): State<AppState>) -> ApiResult<serde_json::Value> {
    let db = state.db.clone();
    let entries = tokio::task::spawn_blocking(move || {
        let filter = AuditFilter {
            time_from: Some((chrono::Utc::now() - chrono::Duration::hours(24)).to_rfc3339()),
            action: Some("AUTH_LOGIN".into()),
            result: Some("failure".into()),
            ..Default::default()
        };
        db.query_audit_log(&filter)
    })
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    // 统计失败次数和异常 IP
    let total_failures = entries.len();
    let unique_ips: Vec<String> = entries
        .iter()
        .filter_map(|e| e.ip.clone())
        .filter(|ip| !ip.is_empty())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    Ok(Json(serde_json::json!({
        "period": "24h",
        "total_failures": total_failures,
        "unique_ips": unique_ips,
        "events": entries.into_iter().map(|e| serde_json::json!({
            "time": e.time,
            "ip": e.ip,
            "detail": e.detail,
        })).collect::<Vec<_>>(),
    })))
}
