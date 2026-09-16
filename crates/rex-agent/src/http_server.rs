//! Agent HTTP server — 提供本地 Web 访问能力。
//!
//! Agent 启动时创建 HTTP server，提供静态文件服务和 API 端点。
//! 用户可以通过浏览器直连 Agent 进行管理。
//!
//! API 代理：所有 /api/* 请求通过 HTTP 转发到 Hub，实现 Agent 本地访问。

use axum::extract::{Request, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use include_dir::{include_dir, Dir};
use rex_common::embedded_static::EmbeddedStatic;
use std::sync::Arc;

/// 嵌入的前端 dist 目录
static DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../packages/rex-console-web/dist");

/// Agent HTTP server 状态
struct AgentState {
    hub_url: String,
    http_client: reqwest::Client,
}

/// 健康检查端点
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "mode": "agent",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// API 代理：转发 /api/* 请求到 Hub
async fn proxy_api(
    State(state): State<Arc<AgentState>>,
    req: Request,
) -> Result<Response, axum::http::StatusCode> {
    // 提取请求信息（在 move req 之前）
    let path = req.uri().path().to_string();
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();
    let hub_target = format!("{}{}{}", state.hub_url, path, query);
    let method = req.method().clone();
    let headers = req.headers().clone();

    // 读取请求体
    let body = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    // 构建转发请求
    let mut builder = state.http_client.request(method, &hub_target);

    // 复制关键头
    for (key, value) in headers.iter() {
        if key == "content-type" || key == "authorization" || key == "cookie" {
            builder = builder.header(key.clone(), value.clone());
        }
    }

    if !body.is_empty() {
        builder = builder.body(body);
    }

    // 发送请求
    let hub_response = builder.send().await.map_err(|e| {
        tracing::warn!(error = %e, path = %path, "proxy to hub failed");
        axum::http::StatusCode::BAD_GATEWAY
    })?;

    // 构建响应
    let status = hub_response.status();
    let resp_headers = hub_response.headers().clone();
    let resp_body = hub_response
        .bytes()
        .await
        .map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;

    let mut response = Response::new(axum::body::Body::from(resp_body));
    *response.status_mut() = axum::http::StatusCode::from_u16(status.as_u16())
        .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);

    // 复制响应头
    for (key, value) in resp_headers.iter() {
        if key != "transfer-encoding" && key != "content-length" {
            response.headers_mut().insert(key.clone(), value.clone());
        }
    }

    Ok(response)
}

/// 启动 Agent HTTP server
pub async fn start_http_server(port: u16, hub_url: String) -> anyhow::Result<()> {
    let embedded = EmbeddedStatic::new("/", &DIST);

    let state = Arc::new(AgentState {
        hub_url,
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?,
    });

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/*path", axum::routing::any(proxy_api))
        .fallback_service(axum::routing::any_service(embedded))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!(addr = %addr, "starting agent HTTP server");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
