//! axum auth middleware — 从 Authorization header 或 query param 提取 JWT token。

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;

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
impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = (StatusCode, axum::Json<ErrorBody>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let app_state = parts
            .extensions
            .get::<AppState>()
            .ok_or_else(|| {
                error_with_status(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "missing app state")
            })?;

        // 方式 1: Authorization header
        if let Some(header) = parts.headers.get("Authorization") {
            if let Ok(val) = header.to_str() {
                if let Some(token) = val.strip_prefix("Bearer ") {
                    match app_state.auth.verify_token(token) {
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
                        match app_state.auth.verify_token(value) {
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
