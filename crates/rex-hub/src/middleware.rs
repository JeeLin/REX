//! axum auth middleware — 从 Authorization header 或 query param 提取 JWT token。

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;

use crate::auth::Claims;
use crate::error::{error_with_status, ErrorBody};
use crate::AppState;

/// 从请求中提取已认证的用户信息。
///
/// 支持两种方式：
/// 1. `Authorization: Bearer <token>` header（REST API）
/// 2. `?token=<token>` query parameter（WebSocket）
pub struct AuthUser(pub Claims);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, axum::Json<ErrorBody>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 方式 1: Authorization header
        if let Some(header) = parts.headers.get("Authorization") {
            if let Ok(val) = header.to_str() {
                if let Some(token) = val.strip_prefix("Bearer ") {
                    match state.auth.verify_token(token) {
                        Ok(claims) => return Ok(AuthUser(claims)),
                        Err(_) => {
                            return Err(error_with_status(
                                StatusCode::UNAUTHORIZED,
                                "AUTH_INVALID",
                                "token expired or invalid",
                            ))
                        }
                    }
                }
            }
        }

        // 方式 2: query parameter（WebSocket 升级时浏览器无法设置自定义 header）
        if let Some(query) = parts.uri.query() {
            for pair in query.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    if key == "token" {
                        match state.auth.verify_token(value) {
                            Ok(claims) => return Ok(AuthUser(claims)),
                            Err(_) => {
                                return Err(error_with_status(
                                    StatusCode::UNAUTHORIZED,
                                    "AUTH_INVALID",
                                    "token expired or invalid",
                                ))
                            }
                        }
                    }
                }
            }
        }

        Err(error_with_status(
            StatusCode::UNAUTHORIZED,
            "AUTH_REQUIRED",
            "missing authentication token",
        ))
    }
}

/// 请求日志中间件 — 记录 method、path、status、latency。
pub async fn request_logger(req: Request<axum::body::Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path().to_owned();
    // 跳过静态文件请求
    let is_static = path.starts_with("/assets/")
        || path.ends_with(".js")
        || path.ends_with(".css")
        || path.ends_with(".png")
        || path.ends_with(".ico");
    let start = Instant::now();
    let response = next.run(req).await;
    let latency = start.elapsed();
    if !is_static {
        tracing::info!(
            method = %method,
            path = %path,
            status = response.status().as_u16(),
            latency_ms = latency.as_millis() as u64,
            "request"
        );
    }
    response
}

/// 安全 HTTP 响应头中间件。
pub async fn security_headers(req: Request<axum::body::Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    headers.insert(
        "Permissions-Policy",
        "camera=(), microphone=(), geolocation=()".parse().unwrap(),
    );
    response
}
