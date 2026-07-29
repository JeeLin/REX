//! REX Hub 入口 — supervisor + worker 进程模型。

use std::path::PathBuf;
use std::sync::Arc;

use rex_hub::agent_api;
use rex_hub::agent_ws;
use rex_hub::audit_api;
use rex_hub::auth;
use rex_hub::crypto;
use rex_hub::dashboard_api;
use rex_hub::db::Database;
use rex_hub::env_api;
use rex_hub::file_api::{self, FileState};
use rex_hub::middleware::{self, AuthUser};
use rex_hub::redis_api::{self, RedisState};
use rex_hub::resource_api;
use rex_hub::settings_api;
use rex_hub::sql_api::{self, SqlState};
use rex_hub::terminal_ws;
use rex_hub::tunnel_ws;
use rex_hub::update_api;
use rex_hub::update_checker;
use rex_hub::AppState;

use axum::routing::get_service;
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

fn main() {
    // 初始化日志（supervisor 和 worker 都需要）
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    if std::env::var("REX_WORKER").is_err() {
        // Supervisor 模式：监控 worker 子进程
        supervisor_main();
    } else {
        // Worker 模式：运行业务逻辑
        worker_main();
    }
}

fn supervisor_main() {
    tracing::info!(
        name = "REX Hub",
        version = env!("CARGO_PKG_VERSION"),
        status = "supervisor starting"
    );

    let data_dir = std::env::var("REX_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_data_dir());
    let _ = std::fs::create_dir_all(&data_dir);

    let port: u16 = std::env::var("REX_PORT")
        .unwrap_or_else(|_| "3000".into())
        .parse()
        .unwrap_or(3000);

    let config = rex_common::supervisor::SupervisorConfig {
        data_dir,
        health_url: format!("http://127.0.0.1:{port}/api/health"),
        max_restart_attempts: 3,
    };

    // 传递除程序名外的所有参数给 worker
    let args: Vec<String> = std::env::args().skip(1).collect();
    rex_common::supervisor::run_supervisor(config, &args);
}

fn worker_main() {
    tracing::info!(
        name = "REX Hub",
        version = env!("CARGO_PKG_VERSION"),
        status = "worker starting"
    );

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let port: u16 = std::env::var("REX_PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse()
            .expect("REX_PORT must be a valid u16");

        let static_dir = resolve_static_dir();

        let data_dir = std::env::var("REX_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_data_dir());
        let db_path = data_dir.join("rex.db");
        let db = Arc::new(Database::open(&db_path).expect("failed to open database"));
        let auth = Arc::new(auth::AuthConfig::new(db.clone()).expect("failed to init auth"));
        let crypto = Arc::new(
            crypto::CredentialCrypto::from_data_dir(&data_dir)
                .expect("failed to init credential crypto"),
        );

        let sql_pool: SqlState =
            Arc::new(tokio::sync::Mutex::new(sql_api::SqlConnectionPool::new()));
        let redis_pool: RedisState = Arc::new(tokio::sync::Mutex::new(
            redis_api::RedisConnectionPool::new(),
        ));
        let file_pool: FileState =
            Arc::new(tokio::sync::Mutex::new(file_api::FileConnectionPool::new()));

        let agent_tunnel = Arc::new(agent_ws::AgentTunnelState::new());
        let agent_binaries = Arc::new(update_api::AgentBinaries::new());

        let state = AppState {
            db,
            auth,
            crypto,
            sql_pool,
            redis_pool,
            file_pool,
            agent_tunnel,
            agent_binaries,
            data_dir: data_dir.clone(),
        };

        tracing::info!("serving frontend from: {}", static_dir.display());

        let tls_config = rex_hub::tls::TlsConfig::from_env();
        let app = build_router(state, static_dir);
        let addr = format!("0.0.0.0:{port}");
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("failed to bind");

        if tls_config.is_enabled() {
            tracing::info!("listening on HTTPS 0.0.0.0:{port}");
        } else {
            tracing::info!("listening on HTTP 0.0.0.0:{port}");
        }

        // 初始化指标收集
        rex_hub::metrics::init();

        // 启动后台更新检查任务（每 6 小时检查 GitHub Release）
        let update_data_dir = data_dir.clone();
        tokio::spawn(async move {
            update_checker::background_update_task(update_data_dir).await;
        });

        // 优雅关闭：监听 SIGTERM/SIGINT
        let shutdown_signal = async {
            let ctrl_c = tokio::signal::ctrl_c();
            #[cfg(unix)]
            {
                let mut sigterm =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("failed to install SIGTERM handler");
                tokio::select! {
                    _ = ctrl_c => {},
                    _ = sigterm.recv() => {},
                }
            }
            #[cfg(not(unix))]
            {
                ctrl_c.await.ok();
            }
            tracing::info!("shutdown signal received, starting graceful shutdown");
        };

        let server = rex_hub::tls::serve(app, listener, tls_config);
        tokio::select! {
            _ = server => {},
            _ = shutdown_signal => {},
        }
        tracing::info!("server stopped");
    });
}

fn default_data_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".rex"))
        .unwrap_or_else(|| PathBuf::from(".rex"))
}

/// GET /api/health — 健康检查端点（供 supervisor 验证 worker 存活）
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
}

fn build_router(state: AppState, static_dir: PathBuf) -> Router {
    let index_path = static_dir.join("index.html");
    let serve_dir = ServeDir::new(&static_dir).not_found_service(ServeFile::new(index_path));

    let public_routes = Router::new()
        .route("/api/health", axum::routing::get(health_check))
        .route(
            "/metrics",
            axum::routing::get(rex_hub::metrics::metrics_endpoint),
        )
        .route("/api/auth/check", axum::routing::get(auth::check_auth))
        .route("/api/auth/login", axum::routing::post(auth::login))
        .route(
            "/api/auth/password",
            axum::routing::post(auth::set_password),
        );

    let protected_routes = Router::new()
        .route(
            "/api/agents/download",
            axum::routing::get(update_api::download_agent_binary),
        )
        .route(
            "/api/auth/change-password",
            axum::routing::post(auth::change_password),
        )
        .route(
            "/api/update/check",
            axum::routing::get(update_api::check_update),
        )
        .route(
            "/api/update/trigger",
            axum::routing::post(update_api::trigger_update),
        )
        .route(
            "/api/update/status",
            axum::routing::get(update_api::update_status),
        )
        .route(
            "/api/update/rollback",
            axum::routing::post(update_api::rollback_update),
        )
        .nest(
            "/api/environments",
            resource_api::resource_routes()
                .merge(agent_api::env_agent_routes())
                .merge(env_api::env_routes()),
        )
        .nest("/api/agents", agent_api::agent_routes())
        .nest("/api/dashboard", dashboard_api::dashboard_routes())
        .nest("/api/audit-log", audit_api::audit_routes())
        .nest("/api/settings", settings_api::settings_routes())
        .route(
            "/api/resources/test-connection",
            axum::routing::post(resource_api::test_connection),
        )
        .nest("/api/sql", sql_api::sql_routes())
        .nest("/api/redis", redis_api::redis_routes())
        .nest("/api/files", file_api::file_routes())
        .route("/ws/terminal", axum::routing::get(terminal_ws::ws_handler))
        .route("/ws/tunnel", axum::routing::get(tunnel_ws::ws_handler))
        .layer(axum::middleware::from_extractor_with_state::<
            AuthUser,
            AppState,
        >(state.clone()))
        .layer(axum::middleware::from_fn(middleware::request_logger));

    // Agent WebSocket — 使用 Agent 自己的 token 认证，不走 JWT 中间件
    let agent_ws_route = Router::new().route("/ws/agent", axum::routing::get(agent_ws::ws_handler));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(agent_ws_route)
        .with_state(state)
        .layer(axum::middleware::from_fn(middleware::security_headers))
        .fallback(get_service(serve_dir).handle_error(|err| async move {
            tracing::error!(error = %err, "static file serve error");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
            )
        }))
}

fn resolve_static_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("REX_STATIC_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidate = exe_dir.join("static");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    let dev_dist = std::env::current_dir()
        .ok()
        .map(|cwd| {
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
    PathBuf::from("dist")
}
