use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

use crate::agent;
use crate::audit;
use crate::auth;
use crate::db::Database;
use crate::env;
use crate::helpers::{ErrorBody, ErrorResponse};
use crate::resource;
use crate::terminal::SessionManager;
use crate::ws::AgentConnections;

/// 更新检查缓存
pub struct UpdateCache {
    pub latest_version: Option<String>,
    pub last_checked: Option<String>,
}

impl UpdateCache {
    pub fn new() -> Self {
        Self {
            latest_version: None,
            last_checked: None,
        }
    }
}

/// 共享应用状态
pub struct AppState {
    pub db: Arc<Database>,
    pub secret_key: String,
    pub connections: Arc<AgentConnections>,
    pub sessions: Arc<SessionManager>,
    pub transfer: Option<Arc<crate::transfer::TransferState>>,
    pub update_cache: tokio::sync::RwLock<UpdateCache>,
}

pub fn app(db: Arc<Database>, secret_key: String) -> axum::Router {
    app_with_static(db, secret_key, None)
}

pub fn app_with_static(
    db: Arc<Database>,
    secret_key: String,
    static_dir: Option<std::path::PathBuf>,
) -> axum::Router {
    let connections = Arc::new(crate::ws::new_connections());
    let sessions = Arc::new(SessionManager::new(900));
    let transfer_state = Arc::new(crate::transfer::TransferState {
        manager: Arc::new(rex_transfer::task::TransferManager::new()),
    });
    let state = Arc::new(AppState {
        db,
        secret_key,
        connections,
        sessions,
        transfer: Some(transfer_state),
        update_cache: tokio::sync::RwLock::new(UpdateCache::new()),
    });

    let public_routes = Router::new()
        .route("/healthz", get(healthz))
        .route("/api/auth/login", post(auth::login))
        .route("/api/agents/register", post(agent::register))
        .route("/ws/agent", get(crate::ws::agent_ws_handler))
        .route(
            "/ws/terminal/:session_id",
            get(crate::ws_terminal::terminal_ws_handler),
        );

    let protected_routes = Router::new()
        .route(
            "/api/environments",
            get(env::list_envs).post(env::create_env),
        )
        .route(
            "/api/environments/:id",
            get(env::get_env)
                .put(env::update_env)
                .delete(env::delete_env),
        )
        .route(
            "/api/environments/:env_id/resources",
            get(resource::list_resources).post(resource::create_resource),
        )
        .route(
            "/api/environments/:env_id/resources/:id",
            get(resource::get_resource)
                .put(resource::update_resource)
                .delete(resource::delete_resource),
        )
        .route("/api/environments/:env_id/agents", get(agent::list_agents))
        .route("/api/audit-log", get(audit::list_audit_log))
        .route(
            "/api/ssh/sessions",
            post(crate::ws_terminal::create_session_handler),
        )
        .route(
            "/api/ssh/sessions/:session_id",
            delete(crate::ws_terminal::delete_session_handler),
        )
        .route(
            "/api/transfers",
            get(crate::transfer::list_transfers).post(crate::transfer::create_transfer),
        )
        .route(
            "/api/transfers/:id",
            get(crate::transfer::get_transfer).delete(crate::transfer::cancel_transfer),
        )
        .route(
            "/api/transfers/:id/remove",
            delete(crate::transfer::remove_transfer),
        )
        .route(
            "/api/resources/:resource_id/files",
            get(crate::files::list_files).delete(crate::files::delete_file),
        )
        .route(
            "/api/resources/:resource_id/files/mkdir",
            post(crate::files::mkdir),
        )
        .route(
            "/api/resources/:resource_id/files/touch",
            post(crate::files::touch),
        )
        .route(
            "/api/resources/:resource_id/files/rename",
            put(crate::files::rename_file),
        )
        .route(
            "/api/resources/:resource_id/sql/info",
            get(crate::sql::get_resource_info),
        )
        .route(
            "/api/resources/:resource_id/sql/execute",
            post(crate::sql::execute_sql),
        )
        .route(
            "/api/resources/:resource_id/sql/databases",
            get(crate::sql::list_databases),
        )
        .route(
            "/api/resources/:resource_id/sql/tables",
            get(crate::sql::list_tables),
        )
        .route(
            "/api/resources/:resource_id/sql/columns",
            get(crate::sql::list_columns),
        )
        .route("/api/update/status", get(crate::update::get_update_status))
        .route("/api/update/check", get(crate::update::check_update))
        .route(
            "/api/update/agents",
            get(crate::update::list_agent_versions),
        )
        .route(
            "/api/update/download",
            post(crate::update::download_update),
        )
        .route(
            "/api/update/apply",
            post(crate::update::apply_update),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let mut router = public_routes.merge(protected_routes).with_state(state);

    // 前端静态文件服务：不经过鉴权
    if let Some(dir) = static_dir {
        let index_path = dir.join("index.html");
        router = router.fallback_service(
            ServeDir::new(&dir)
                .append_index_html_on_directories(true)
                .not_found_service(ServeFile::new(index_path)),
        );
    }

    router
}

async fn healthz() -> &'static str {
    "ok"
}

/// 从请求头提取客户端 IP
pub fn extract_client_ip(headers: &axum::http::header::HeaderMap) -> String {
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(val) = forwarded.to_str() {
            if let Some(first) = val.split(',').next() {
                let ip = first.trim().to_string();
                if !ip.is_empty() {
                    return ip;
                }
            }
        }
    }
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(val) = real_ip.to_str() {
            let ip = val.trim().to_string();
            if !ip.is_empty() {
                return ip;
            }
        }
    }
    "unknown".to_string()
}

/// 未认证错误响应
fn unauthorized(code: &str, msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: ErrorBody {
                code: code.to_string(),
                message: msg.to_string(),
            },
        }),
    )
}

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    headers: axum::http::header::HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match token {
        Some(token) => {
            if auth::verify_token(&state.secret_key, token) {
                Ok(next.run(request).await)
            } else {
                Err(unauthorized("AUTH_INVALID", "token 无效"))
            }
        }
        None => Err(unauthorized("AUTH_REQUIRED", "缺少 Authorization header")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    const TEST_SECRET: &str = "test-secret";

    fn test_db() -> Arc<Database> {
        Arc::new(Database::new_in_memory().unwrap())
    }

    #[tokio::test]
    async fn healthz_returns_ok() {
        let app = app(test_db(), TEST_SECRET.to_string());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body, "ok");
    }

    #[tokio::test]
    async fn protected_route_without_token_returns_401() {
        let app = app(test_db(), TEST_SECRET.to_string());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/environments")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"]["code"].as_str().is_some());
    }

    #[tokio::test]
    async fn login_route_is_public() {
        let app = app(test_db(), TEST_SECRET.to_string());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"wrong"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn login_with_default_password_succeeds() {
        let app = app(test_db(), TEST_SECRET.to_string());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"admin"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(!json["data"]["token"].as_str().unwrap().is_empty());
    }

    #[test]
    fn extract_ip_from_forwarded_for() {
        let mut headers = axum::http::header::HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            axum::http::header::HeaderValue::from_static("192.168.1.100, 10.0.0.1"),
        );
        assert_eq!(extract_client_ip(&headers), "192.168.1.100");
    }

    #[test]
    fn extract_ip_from_real_ip() {
        let mut headers = axum::http::header::HeaderMap::new();
        headers.insert(
            "x-real-ip",
            axum::http::header::HeaderValue::from_static("10.0.0.2"),
        );
        assert_eq!(extract_client_ip(&headers), "10.0.0.2");
    }

    #[test]
    fn extract_ip_unknown_when_no_headers() {
        let headers = axum::http::header::HeaderMap::new();
        assert_eq!(extract_client_ip(&headers), "unknown");
    }
}
