//! REX Hub 入口 — supervisor + worker 进程模型。

use std::collections::HashMap;
#[cfg(unix)]
use std::os::unix::io::IntoRawFd;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use rex_hub::agent_api;
use rex_hub::agent_ws;
use rex_hub::audit_api;
use rex_hub::auth;
use rex_hub::cdr_api;
use rex_hub::crypto;
use rex_hub::dashboard_api;
use rex_hub::db::Database;
use rex_hub::env_api;
use rex_hub::file_api::{self, FileState};
use rex_hub::file_ws;
use rex_hub::middleware::{self, AuthUser};
use rex_hub::mongodb_api;
use rex_hub::redis_api::{self, RedisState};
use rex_hub::resource_api;
use rex_hub::settings_api;
use rex_hub::sip_capture::SipCaptureRegistry;
use rex_hub::sip_capture_api;
use rex_hub::sip_recording::SipRecordingRegistry;
use rex_hub::sip_recording_api;
use rex_hub::sip_ws;
use rex_hub::sql_api::{self, SqlState};
use rex_hub::terminal_ws;
use rex_hub::tunnel_ws;
use rex_hub::update_api;
use rex_hub::update_checker;
use rex_hub::AppState;

use rex_common::cli::{self, RunOpts, ServiceKind};

#[cfg(feature = "embedded-static")]
use axum::routing::get_service;
use axum::Router;
#[cfg(feature = "embedded-static")]
use rex_hub::static_embed::create_embedded_static;
#[cfg(not(feature = "embedded-static"))]
use rex_hub::static_embed::dev_static_dir;

fn main() {
    rex_common::config::load_dotenv();
    let cli = cli::parse();
    let kind = ServiceKind::Hub;
    if let Err(e) = cli::dispatch(cli, kind, run_service) {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

/// 启动逻辑：`run` 子命令（及无子命令默认）。
///
/// 1. 读取可选配置文件（env 优先）；2. 单实例互斥（pid 文件）；
/// 3. 把命令行参数写入 env（worker / supervisor 子进程继承）；
/// 4. `--single` 直接跑 worker（无 supervisor，无法自动更新）；
/// 5. `--background` 脱离终端后台运行。
fn run_service(opts: &RunOpts) -> anyhow::Result<()> {
    // 配置文件（env 优先）— 必须在读任何 env 之前
    rex_common::config::apply_config_env(ServiceKind::Hub);

    // 解析 data_dir（可能被配置文件设置），供单实例/pid 逻辑与后续复用
    let data_dir = data_dir_or_default();

    // 单实例互斥：同一 data_dir 只允许一个 Hub
    rex_common::process::ensure_single_instance(ServiceKind::Hub, &data_dir)?;

    // 命令行参数 > env：把相关字段写回 env，供 worker / supervisor 子进程继承
    if let Some(port) = opts.port {
        std::env::set_var("REX_PORT", port.to_string());
    }
    if let Some(data_dir) = &opts.data_dir {
        std::env::set_var("REX_DATA_DIR", data_dir);
    }

    // 后台模式：脱离终端（daemonize），日志重定向到数据目录 rex-hub.log
    #[cfg(unix)]
    if opts.background {
        let log_path = data_dir.join("rex-hub.log");
        redirect_stdio(&log_path)?;
        rex_common::process::daemonize()?;
    }

    // 写 pid 文件（前台 / 后台主进程）
    rex_common::process::write_pid_file(ServiceKind::Hub, &data_dir)?;

    if opts.single {
        // 单进程：直接 worker，无 supervisor → 无法自动更新
        tracing::warn!(
            status = "single-process mode; auto-update is NOT available (no supervisor)"
        );
        worker_main();
    } else {
        if std::env::var("REX_WORKER").is_err() {
            supervisor_main();
        } else {
            worker_main();
        }
    }
    Ok(())
}

fn data_dir_or_default() -> PathBuf {
    std::env::var("REX_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| rex_common::config::default_data_dir())
}

fn supervisor_main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    tracing::info!(
        name = "REX Hub",
        version = env!("CARGO_PKG_VERSION"),
        status = "supervisor starting"
    );

    let data_dir = std::env::var("REX_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| rex_common::config::default_data_dir());
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

/// 启动期数据密钥错配的告警判定。
///
/// 两个条件缺一不可：`crypto.was_generated()` 表示本次启动新生成了 `.master-key`
/// （磁盘上原本没有），`db.has_encrypted_config()` 表示库里仍有密文配置 —— 只有
/// 同时成立才说明「加密凭据的那把 key 丢了」；全新部署是新 key + 空库，不该刷这条
/// 告警。判定结果只用于 `tracing::error!`，软失败，不影响启动。
///
/// 判定依据必须是 [`crypto::CredentialCrypto`] 自报的新生标记：`from_data_dir`
/// 在 miss 时已经把新 key 写盘，事后再查 `.master-key` 是否存在恒为真，守卫会变成
/// 永不触发的死代码。
///
/// 该函数为 async：`db.has_encrypted_config()` 是同步 rusqlite 查询，必须透过
/// `tokio::task::spawn_blocking` 移出 runtime worker 线程，否则会阻塞整个多线程
/// runtime 的 worker 池，把所有 WebSocket 隧道（terminal / file / agent）拖挂。
/// `JoinError`（spawn_blocking panic）按软失败语义处理：记 `tracing::error!`
/// 后返回 false，不 panic、不阻塞启动 —— 正是本函数「软失败，不影响启动」的含义。
async fn should_warn_data_key_mismatch(
    crypto: &crypto::CredentialCrypto,
    db: Arc<Database>,
) -> bool {
    // 保持原 `was_generated() && ...` 短路顺序：未生成新 key 时不发起
    // spawn_blocking，避免无谓线程切换。
    if !crypto.was_generated() {
        return false;
    }
    match tokio::task::spawn_blocking(move || db.has_encrypted_config()).await {
        Ok(has) => has,
        Err(e) => {
            tracing::error!(
                error = %e,
                "spawn_blocking for has_encrypted_config panicked; data-key mismatch guard skipped"
            );
            false
        }
    }
}

fn worker_main() {
    let timer = tracing_subscriber::fmt::time::ChronoLocal::rfc_3339();

    // 日志轮转：滚动写入 data/logs/<name>.log，自动按天切分 + 旧日志清理（REX_LOG_* 可配）。
    let (appender, log_dir, max_log_files) = rex_common::logging::rolling_appender("rex-hub.log");

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .with_writer(appender)
        .with_timer(timer)
        .init();

    tracing::info!(
        name = "REX Hub",
        version = env!("CARGO_PKG_VERSION"),
        log_dir = %log_dir.display(),
        max_log_files,
        status = "worker starting"
    );

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let port: u16 = std::env::var("REX_PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse()
            .expect("REX_PORT must be a valid u16");

        let data_dir = std::env::var("REX_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| rex_common::config::default_data_dir());
        let db_path = data_dir.join("rex.db");
        let db = Arc::new(Database::open(&db_path).expect("failed to open database"));
        let auth = Arc::new(auth::AuthConfig::new(db.clone()).expect("failed to init auth"));
        let crypto = crypto::CredentialCrypto::from_data_dir(&data_dir)
            .expect("failed to init credential crypto");

        // Startup soft-fail guard: a key generated on this boot means
        // `.master-key` was missing, and encrypted `config_json` rows mean the
        // data key that encrypted those credentials is gone. Fresh installs
        // generate a key with no ciphertext, hence both conditions.
        // `from_data_dir` writes the new key immediately, so the judgement is
        // made on `was_generated()` — a later `Path::exists()` check on
        // `.master-key` is already true by then and would never fire.
        // The Hub keeps running; we surface the mismatch loudly so it is not
        // mistaken for a healthy start.
        if should_warn_data_key_mismatch(&crypto, db.clone()).await {
            tracing::error!(
                data_key_mismatch = true,
                key_file_missing = true,
                "{}",
                rex_hub::error::CREDENTIAL_DECRYPT_MSG
            );
        }
        let crypto = Arc::new(crypto);

        let sql_pool: SqlState =
            Arc::new(tokio::sync::Mutex::new(sql_api::SqlConnectionPool::new()));
        let redis_pool: RedisState = Arc::new(tokio::sync::Mutex::new(
            redis_api::RedisConnectionPool::new(),
        ));
        let file_pool: FileState =
            Arc::new(tokio::sync::Mutex::new(file_api::FileConnectionPool::new()));
        let mongo_pool: mongodb_api::MongoState = Arc::new(Mutex::new(HashMap::new()));

        let agent_tunnel = Arc::new(agent_ws::AgentTunnelState::new());
        let agent_binaries = Arc::new(update_api::AgentBinaries::new());

        let state = AppState {
            db,
            auth,
            crypto,
            sql_pool,
            redis_pool,
            file_pool,
            mongo_pool,
            agent_tunnel,
            agent_binaries,
            sip_capture: Arc::new(SipCaptureRegistry::new()),
            sip_recording: Arc::new(SipRecordingRegistry::new(data_dir.clone())),
            data_dir: data_dir.clone(),
            coordinator: Arc::new(rex_hub::transfer_coordinator::TransferCoordinator::new()),
            sync_coordinator: Arc::new(rex_hub::sync_coordinator::SyncCoordinator::new()),
            transfer_bcast: tokio::sync::broadcast::channel(128).0,
        };

        #[cfg(feature = "embedded-static")]
        tracing::info!(name = "REX Hub", status = "serving embedded frontend");
        #[cfg(not(feature = "embedded-static"))]
        tracing::info!(
            name = "REX Hub",
            status = "serving frontend from directory (dev mode)"
        );

        let tls_config = rex_hub::tls::TlsConfig::from_env();
        let app = build_router(state);
        let addr = format!("0.0.0.0:{port}");
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("failed to bind");

        if tls_config.is_enabled() {
            tracing::info!("listening on HTTPS 0.0.0.0:{port}");
        } else {
            tracing::info!("listening on HTTP 0.0.0.0:{port}");
        }

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

/// 把 stdout / stderr 重定向到日志文件（后台模式用）。
#[cfg(unix)]
fn redirect_stdio(log_path: &std::path::Path) -> anyhow::Result<()> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| anyhow::anyhow!("open log file {}: {e}", log_path.display()))?;
    // into_raw_fd 把文件 fd 的所有权移出 File，关闭 File 不会关闭该 fd；
    // dup2 把其复制到 stdout/stderr，原 fd 随后必须关闭，避免泄漏。
    let fd = file.into_raw_fd();
    unsafe {
        libc::dup2(fd, libc::STDOUT_FILENO);
        libc::dup2(fd, libc::STDERR_FILENO);
        libc::close(fd);
    }
    Ok(())
}

/// GET /api/health — 健康检查端点（供 supervisor 验证 worker 存活）
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "mode": "hub",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
}

/// `/api` 前缀兜底：未知 API 路径一律返回 JSON 404，绝不落 SPA fallback。
///
/// 否则 `GET /api`、`GET /api/anything` 会拿到 200 text/html（index.html），
/// Agent 反代与浏览器直连都把 html 当业务响应解析（CR14②）。
async fn api_not_found() -> (
    axum::http::StatusCode,
    axum::Json<rex_hub::error::ErrorBody>,
) {
    rex_hub::error::error_with_status(axum::http::StatusCode::NOT_FOUND, "NOT_FOUND", "not found")
}

/// `CR14②`：`/api` 前缀兜底路由（精确 `/api`、尾斜杠 `/api/`、其余 `/api/{*path}`）。
fn api_fallback_routes() -> Router<AppState> {
    Router::new()
        .route("/api", axum::routing::any(api_not_found))
        .route("/api/", axum::routing::any(api_not_found))
        .route("/api/{*path}", axum::routing::any(api_not_found))
}

/// GET /api/system-info — 宿主机系统信息（os / arch / hostname）
async fn system_info() -> axum::Json<serde_json::Value> {
    let hostname = hostname::get()
        .ok()
        .map(|h| h.to_string_lossy().into_owned())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    axum::Json(serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "hostname": hostname,
    }))
}

fn build_router(state: AppState) -> Router {
    #[cfg(feature = "embedded-static")]
    let embedded = create_embedded_static("/");

    let public_routes = Router::new()
        .route("/api/health", axum::routing::get(health_check))
        .route("/api/auth/check", axum::routing::get(auth::check_auth))
        .route("/api/auth/login", axum::routing::post(auth::login))
        .route(
            "/api/auth/password",
            axum::routing::post(auth::set_password),
        )
        .route(
            "/api/agents/download",
            axum::routing::get(update_api::download_agent_binary),
        );

    let protected_routes = Router::new()
        .route("/api/system-info", axum::routing::get(system_info))
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
        .nest("/api/sip/cdr", cdr_api::cdr_routes())
        .nest("/api/sip/capture", sip_capture_api::sip_capture_routes())
        .nest(
            "/api/sip/recording",
            sip_recording_api::sip_recording_routes(),
        )
        .nest("/api/settings", settings_api::settings_routes())
        .route(
            "/api/resources/test-connection",
            axum::routing::post(resource_api::test_connection),
        )
        .nest("/api/sql", sql_api::sql_routes())
        .nest("/api/redis", redis_api::redis_routes())
        .nest("/api/mongodb", mongodb_api::mongodb_routes())
        .nest("/api/files", file_api::file_routes())
        .route("/ws/terminal", axum::routing::get(terminal_ws::ws_handler))
        .route("/ws/files", axum::routing::get(file_ws::ws_handler))
        .route("/ws/sip", axum::routing::get(sip_ws::ws_handler))
        .route("/ws/tunnel", axum::routing::get(tunnel_ws::ws_handler))
        .layer(axum::middleware::from_extractor_with_state::<
            AuthUser,
            AppState,
        >(state.clone()))
        .layer(axum::middleware::from_fn(middleware::request_logger));

    // Agent WebSocket — 使用 Agent 自己的 token 认证，不走 JWT 中间件
    let agent_ws_route = Router::new().route("/ws/agent", axum::routing::get(agent_ws::ws_handler));

    let router = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(agent_ws_route)
        // `api_fallback_routes` 挂在 auth 中间件之外：API 前缀未匹配路径直接
        // 返回 JSON 404，不需要（也不应）先要 JWT。
        .merge(api_fallback_routes())
        .with_state(state)
        .layer(axum::middleware::from_fn(middleware::security_headers))
        .layer(axum::middleware::from_fn(middleware::csrf_protection));

    #[cfg(feature = "embedded-static")]
    let router = router.fallback(get_service(embedded).handle_error(
        |err: std::convert::Infallible| async move {
            tracing::error!(error = %err, "static file serve error");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
            )
        },
    ));

    #[cfg(not(feature = "embedded-static"))]
    let router = {
        let dir = dev_static_dir();
        tracing::info!(path = %dir.display(), "serving static files from directory (dev mode)");
        if !dir.join("index.html").exists() {
            tracing::error!(
                path = %dir.display(),
                "frontend dist not found — UI will 404; build with --features embedded-static \
                 or set REX_STATIC_DIR to a directory containing the built frontend"
            );
        }
        // SPA fallback: unknown paths (e.g. /dashboard) serve index.html so
        // vue-router history mode works on direct open / refresh, matching
        // the embedded-static behavior.
        let index = dir.join("index.html");
        router.fallback_service(
            tower_http::services::ServeDir::new(&dir)
                .fallback(tower_http::services::ServeFile::new(index)),
        )
    };

    router
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt;

    /// Dev-mode SPA fallback: ServeDir + fallback ServeFile(index.html),
    /// mirroring the `not(feature = "embedded-static")` branch of `build_router`.
    fn spa_router(dir: &std::path::Path) -> Router {
        let index = dir.join("index.html");
        Router::new().fallback_service(
            tower_http::services::ServeDir::new(dir)
                .fallback(tower_http::services::ServeFile::new(index)),
        )
    }

    /// CR14②：`/api` 前缀（精确、尾斜杠、任意子路径）一律 JSON 404，
    /// 绝不落 SPA fallback 的 200 text/html —— Agent 反代与浏览器直连共用此约束。
    #[tokio::test]
    async fn api_prefix_answers_json_404_not_spa_html() {
        let dir = tempfile::tempdir().expect("create tempdir");
        // AppState 构造与 lib 侧单元测试共用 `resource_conn::build_test_state`。
        let app = build_router(rex_hub::resource_conn::build_test_state(dir.path()));

        for path in ["/api", "/api/", "/api/anything"] {
            let resp = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(
                resp.status(),
                StatusCode::NOT_FOUND,
                "{path} must be 404, not the SPA's 200"
            );
            let content_type = resp
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                content_type.starts_with("application/json"),
                "{path} must answer JSON, got content-type {content_type:?}"
            );

            let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
            let json: serde_json::Value = serde_json::from_slice(&body).expect("json body");
            assert_eq!(json["error"]["message"], "not found", "{path}");
        }

        // 静态/SPA 路径不受影响：仍走 fallback。
        let resp = app
            .oneshot(Request::get("/dashboard").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_ne!(
            resp.headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            "application/json",
            "non-API paths must keep falling back to the SPA"
        );
    }

    #[tokio::test]
    async fn dev_spa_fallback_serves_index_html() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let index_html = "<!DOCTYPE html><html><body>rex-spa</body></html>";
        std::fs::write(tmp.path().join("index.html"), index_html).expect("write index.html");

        let app = spa_router(tmp.path());

        // Root path serves index.html
        let resp = app
            .clone()
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body.as_ref(), index_html.as_bytes());

        // Unknown non-root path falls back to index.html (vue-router history mode)
        let resp = app
            .clone()
            .oneshot(Request::get("/dashboard").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body.as_ref(), index_html.as_bytes());

        // Existing static asset is still served from the directory
        std::fs::write(tmp.path().join("app.js"), "console.log(1);").expect("write asset");
        let resp = app
            .oneshot(Request::get("/app.js").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body.as_ref(), b"console.log(1);");
    }

    /// 启动守卫的触发条件：本次新生成了数据密钥 ∧ 库里仍有密文。
    ///
    /// 守卫此前在生成 key 之后再查 `.master-key` 存在性，该条件恒为 false，
    /// 告警永不触发。这里把两个判定输入都取自真实调用（`was_generated()` /
    /// `has_encrypted_config()`），因此任一半边失效（key 不再上报新生、或密文
    /// 检测漏判）断言都会红。
    #[tokio::test]
    async fn startup_data_key_guard_fires_only_on_generated_key_with_ciphertext() {
        use rex_hub::crypto::CredentialCrypto;
        use rex_hub::db::Database;
        use rex_hub::models::{NewEnvironment, NewResource};

        let dir = tempfile::tempdir().expect("create tempdir");
        let db = Arc::new(Database::open(&dir.path().join("rex.db")).expect("open sqlite"));

        // 全新部署：新 key + 空库 → 不告警。
        let first_boot = CredentialCrypto::from_data_dir(dir.path()).expect("crypto");
        assert!(first_boot.was_generated());
        assert!(!db.has_encrypted_config());
        assert!(!should_warn_data_key_mismatch(&first_boot, db.clone()).await);

        // 有密文 + key 文件已在盘上 → 沿用原密钥，不告警。
        let env = db
            .create_environment(&NewEnvironment {
                name: "env".into(),
                description: None,
                connection_mode: Some("direct".into()),
            })
            .expect("create env");
        db.create_resource(
            &env.id,
            &NewResource {
                name: "ssh-1".into(),
                protocol: "ssh".into(),
                host: "10.0.0.1".into(),
                port: Some(22),
                username: Some("root".into()),
                config_json: Some("Y2lwaGVydGV4dA==".into()),
                subtype: None,
                color: None,
                sort_order: None,
            },
        )
        .expect("create resource");
        assert!(db.has_encrypted_config());

        let restarted = CredentialCrypto::from_data_dir(dir.path()).expect("crypto");
        assert!(!restarted.was_generated());
        assert!(!should_warn_data_key_mismatch(&restarted, db.clone()).await);

        // 密文仍在、`.master-key` 丢失 → 新 key + 密文，必须告警。
        std::fs::remove_file(dir.path().join(crypto::MASTER_KEY_FILE)).expect("remove key");
        let key_lost = CredentialCrypto::from_data_dir(dir.path()).expect("crypto");
        assert!(key_lost.was_generated());
        assert!(db.has_encrypted_config());
        // 此处 `.master-key` 已重新落盘：改回按文件存在性判定会让这一条转红。
        assert!(dir.path().join(crypto::MASTER_KEY_FILE).exists());
        assert!(should_warn_data_key_mismatch(&key_lost, db.clone()).await);
    }
}
