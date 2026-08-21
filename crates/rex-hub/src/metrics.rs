//! Prometheus 指标端点 — 暴露系统运行指标。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use axum::response::IntoResponse;

static START_TIME: OnceLock<Instant> = OnceLock::new();
static REQUEST_COUNT: AtomicU64 = AtomicU64::new(0);
static AGENT_CONNECTIONS: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    START_TIME.set(Instant::now()).ok();
}

pub fn record_request() {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn set_agent_connections(count: u64) {
    AGENT_CONNECTIONS.store(count, Ordering::Relaxed);
}

/// GET /metrics — Prometheus 格式指标
pub async fn metrics_endpoint() -> impl IntoResponse {
    let uptime = START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0);
    let requests = REQUEST_COUNT.load(Ordering::Relaxed);
    let agents = AGENT_CONNECTIONS.load(Ordering::Relaxed);

    let body = format!(
        "# HELP rex_hub_uptime_seconds Hub uptime in seconds\n\
         # TYPE rex_hub_uptime_seconds gauge\n\
         rex_hub_uptime_seconds {uptime}\n\
         \n\
         # HELP rex_hub_requests_total Total number of requests\n\
         # TYPE rex_hub_requests_total counter\n\
         rex_hub_requests_total {requests}\n\
         \n\
         # HELP rex_hub_agent_connections Current number of agent connections\n\
         # TYPE rex_hub_agent_connections gauge\n\
         rex_hub_agent_connections {agents}\n"
    );

    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}
