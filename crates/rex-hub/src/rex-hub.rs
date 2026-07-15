//! REX Hub 入口 — supervisor + worker 进程模型。
//!
//! 2.0 重设计骨架。worker 启动 axum HTTP server，托管前端静态资源（单端口代理）。

use std::path::PathBuf;
use std::sync::Arc;

use rex_hub::file_api::{self, FileState};
use rex_hub::redis_api::{self, RedisState};
use rex_hub::sql_api::{self, SqlState};
use rex_hub::terminal_ws;

use axum::routing::get_service;
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

fn main() {
    // supervisor 模式：开发阶段直接调用 worker（不 fork 子进程）
    // 后续实现：PID 1 = supervisor，fork worker 子进程，监控存活/替换/回滚
    if std::env::var("REX_WORKER").is_err() {
        std::env::set_var("REX_WORKER", "1");
        worker_main();
    } else {
        worker_main();
    }
}

fn worker_main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    tracing::info!(
        name = "REX Hub",
        version = env!("CARGO_PKG_VERSION"),
        status = "starting"
    );

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let port: u16 = std::env::var("REX_PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse()
            .expect("REX_PORT must be a valid u16");

        let static_dir = resolve_static_dir();

        tracing::info!("serving frontend from: {}", static_dir.display());
        tracing::info!("listening on 0.0.0.0:{port}");

        let app = build_router(static_dir);
        let addr = format!("0.0.0.0:{port}");
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("failed to bind");

        axum::serve(listener, app).await.expect("server error");
    });
}

/// 构建路由：WebSocket 终端 + SQL API + Redis API + Files API + 静态文件 + SPA fallback
fn build_router(static_dir: PathBuf) -> Router {
    let index_path = static_dir.join("index.html");

    // 静态文件服务（按文件实际路径响应，找不到文件时回退到 index.html）
    let serve_dir = ServeDir::new(&static_dir).not_found_service(ServeFile::new(index_path));

    // SQL 连接池状态
    let sql_state: SqlState = Arc::new(tokio::sync::Mutex::new(sql_api::SqlConnectionPool::new()));

    // Redis 连接池状态
    let redis_state: RedisState = Arc::new(tokio::sync::Mutex::new(
        redis_api::RedisConnectionPool::new(),
    ));

    // 文件管理连接池状态
    let file_state: FileState =
        Arc::new(tokio::sync::Mutex::new(file_api::FileConnectionPool::new()));

    Router::new()
        // WebSocket 终端桥接
        .route("/ws/terminal", axum::routing::get(terminal_ws::ws_handler))
        // SQL 控制台 API
        .nest("/api/sql", sql_api::sql_routes().with_state(sql_state))
        // Redis 控制台 API
        .nest(
            "/api/redis",
            redis_api::redis_routes().with_state(redis_state),
        )
        // 文件管理 API
        .nest("/api/files", file_api::file_routes().with_state(file_state))
        .fallback(get_service(serve_dir).handle_error(|err| async move {
            tracing::error!(error = %err, "static file serve error");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
            )
        }))
}

/// 确定前端 dist 目录路径（优先级：REX_STATIC_DIR > 内嵌路径 > 默认路径）
fn resolve_static_dir() -> PathBuf {
    // 1. 环境变量覆盖（开发/测试/容器场景）
    if let Ok(dir) = std::env::var("REX_STATIC_DIR") {
        return PathBuf::from(dir);
    }

    // 2. 与可执行文件同级的 static 目录（部署场景）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidate = exe_dir.join("static");
            if candidate.exists() {
                return candidate;
            }
        }
    }

    // 3. 默认：从工作目录推导（开发场景，从 crates/rex-hub 运行时）
    //    workspace root / packages/rex-console-web / dist
    let dev_dist = std::env::current_dir()
        .ok()
        .map(|cwd| {
            // 如果 cwd 是 crates/rex-hub，向上两级到 workspace root
            if cwd.ends_with("crates/rex-hub") {
                cwd.join("../../packages/rex-console-web/dist")
            } else {
                cwd.join("packages/rex-console-web/dist")
            }
        })
        .unwrap_or_else(|| PathBuf::from("packages/rex-console-web/dist"));

    if dev_dist.exists() {
        return dev_dist;
    }

    // 4. 兜底：dist（用户可能从 workspace root 启动）
    PathBuf::from("dist")
}
