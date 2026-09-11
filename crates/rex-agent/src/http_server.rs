//! Agent HTTP server — 提供本地 Web 访问能力。
//!
//! Agent 启动时创建 HTTP server，提供静态文件服务和 API 端点。
//! 用户可以通过浏览器直连 Agent 进行管理。

use axum::routing::get;
use axum::Router;
use include_dir::{include_dir, Dir};
use rex_common::embedded_static::EmbeddedStatic;

/// 嵌入的前端 dist 目录
static DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../packages/rex-console-web/dist");

/// 健康检查端点
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "mode": "agent",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// 启动 Agent HTTP server
pub async fn start_http_server(port: u16) -> anyhow::Result<()> {
    let embedded = EmbeddedStatic::new("/", &DIST);

    let app = Router::new()
        .route("/api/health", get(health_check))
        .fallback_service(axum::routing::any_service(embedded));

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!(addr = %addr, "starting agent HTTP server");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
