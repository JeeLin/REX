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
use rex_hub::middleware::AuthUser;
use rex_hub::redis_api::{self, RedisState};
use rex_hub::resource_api;
use rex_hub::settings_api;
use rex_hub::sql_api::{self, SqlState};
use rex_hub::terminal_ws;
use rex_hub::tunnel_ws;
use rex_hub::AppState;

use axum::routing::get_service;
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

fn main() {
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

        let state = AppState {
            db,
            auth,
            crypto,
            sql_pool,
            redis_pool,
            file_pool,
            agent_tunnel,
        };

        tracing::info!("serving frontend from: {}", static_dir.display());
        tracing::info!("listening on 0.0.0.0:{port}");

        let app = build_router(state, static_dir);
        let addr = format!("0.0.0.0:{port}");
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("failed to bind");
        axum::serve(listener, app).await.expect("server error");
    });
}

fn default_data_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".rex"))
        .unwrap_or_else(|| PathBuf::from(".rex"))
}

fn build_router(state: AppState, static_dir: PathBuf) -> Router {
    let index_path = static_dir.join("index.html");
    let serve_dir = ServeDir::new(&static_dir).not_found_service(ServeFile::new(index_path));

    let public_routes = Router::new()
        .route("/api/auth/check", axum::routing::get(auth::check_auth))
        .route("/api/auth/login", axum::routing::post(auth::login))
        .route(
            "/api/auth/password",
            axum::routing::post(auth::set_password),
        );

    let protected_routes = Router::new()
        .nest(
            "/api/environments",
            env_api::env_routes().merge(agent_api::env_agent_routes()),
        )
        .nest("/api/environments", resource_api::resource_routes())
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
        .layer(axum::middleware::from_extractor_with_state::<AuthUser, AppState>(
            state.clone(),
        ));

    // Agent WebSocket — 使用 Agent 自己的 token 认证，不走 JWT 中间件
    let agent_ws_route = Router::new()
        .route("/ws/agent", axum::routing::get(agent_ws::ws_handler));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(agent_ws_route)
        .with_state(state)
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
