use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use std::sync::Arc;

use crate::audit;
use crate::auth;
use crate::db::Database;
use crate::env;
use crate::helpers::{ErrorBody, ErrorResponse};
use crate::resource;

/// 共享应用状态
pub struct AppState {
    pub db: Arc<Database>,
    pub secret_key: String,
}

pub fn app(db: Arc<Database>, secret_key: String) -> axum::Router {
    let state = Arc::new(AppState { db, secret_key });

    let public_routes = Router::new()
        .route("/healthz", get(healthz))
        .route("/api/auth/login", post(auth::login));

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
        .route("/api/audit-log", get(audit::list_audit_log))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    public_routes.merge(protected_routes).with_state(state)
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
