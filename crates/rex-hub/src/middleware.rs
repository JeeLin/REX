//! axum auth middleware — 从 Authorization header 或 query param 提取 JWT token。

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
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

/// CSRF 保护中间件：验证 POST/PUT/DELETE 请求的 Origin/Referer 头。
/// 仅对非 GET/HEAD 请求验证，本地开发地址跳过验证。
pub async fn csrf_protection(req: Request<axum::body::Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    // GET/HEAD/OPTIONS 不需要 CSRF 验证
    if method != axum::http::Method::POST
        && method != axum::http::Method::PUT
        && method != axum::http::Method::DELETE
    {
        return next.run(req).await;
    }

    // 静态资源和 WebSocket 路径跳过验证
    let path = uri.path();
    if path.starts_with("/static")
        || path.starts_with("/ws")
        || path.starts_with("/api/agent/ws")
    {
        return next.run(req).await;
    }

    // 提取 Origin 或 Referer
    let origin = req
        .headers()
        .get("Origin")
        .or_else(|| req.headers().get("Referer"));

    match origin {
        Some(val) => {
            let origin_str = val.to_str().unwrap_or("");

            // 本地开发地址跳过验证
            if origin_str.contains("localhost")
                || origin_str.contains("127.0.0.1")
                || origin_str.contains("[::1]")
            {
                return next.run(req).await;
            }

            // 提取 host
            let origin_host = origin_str
                .strip_prefix("http://")
                .or_else(|| origin_str.strip_prefix("https://"))
                .unwrap_or(origin_str)
                .split('/')
                .next()
                .unwrap_or("");

            // 服务器自身的 Host 头
            let server_host = req
                .headers()
                .get("Host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");

            // 允许无 Origin/Referer 的请求（同源直连）
            if origin_host == server_host {
                return next.run(req).await;
            }

            tracing::warn!(
                method = %method,
                path = %path,
                origin = %origin_str,
                server = %server_host,
                "CSRF protection: origin mismatch"
            );

            // Origin 不匹配，拒绝请求
            return (
                StatusCode::FORBIDDEN,
                "CSRF validation failed: origin mismatch",
            )
                .into_response();
        }
        None => {
            // 无 Origin/Referer：允许同源直连（如 curl/API 客户端）
            return next.run(req).await;
        }
    }
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
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' ws: wss:; font-src 'self' data:; frame-ancestors 'none'; base-uri 'self'; form-action 'self'".parse().unwrap(),
    );
    response
}
