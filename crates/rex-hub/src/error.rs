//! 统一错误响应格式。

use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

/// 返回 JSON 错误体（不含状态码），用于需要自行包装的场景。
pub fn error_response(code: &str, message: &str) -> Json<ErrorBody> {
    Json(ErrorBody {
        error: ErrorDetail {
            code: code.to_string(),
            message: message.to_string(),
        },
    })
}

/// 返回 (StatusCode, Json<ErrorBody>) 元组，用于 handler 直接返回。
pub fn error_with_status(
    status: StatusCode,
    code: &str,
    message: &str,
) -> (StatusCode, Json<ErrorBody>) {
    (status, error_response(code, message))
}
