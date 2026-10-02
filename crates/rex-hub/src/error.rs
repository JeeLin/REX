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

/// REST 错误码枚举：驼峰字面量统一在这里定义，`as_str()` → JSON `error.code`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// 通用错误（历史默认，保持行为不变）。
    Error,
    AuthInvalid,
    AuthRequired,
    AgentUnavailable,
    NotFound,
    Conflict,
    BadRequest,
    Internal,
    Unimplemented,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::Error => "ERROR",
            ErrorCode::AuthInvalid => "AUTH_INVALID",
            ErrorCode::AuthRequired => "AUTH_REQUIRED",
            ErrorCode::AgentUnavailable => "AGENT_UNAVAILABLE",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::Conflict => "CONFLICT",
            ErrorCode::BadRequest => "BAD_REQUEST",
            ErrorCode::Internal => "INTERNAL",
            ErrorCode::Unimplemented => "UNIMPLEMENTED",
        }
    }
}

impl Default for ErrorCode {
    fn default() -> Self {
        ErrorCode::Error
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 统一的 handler 级错误构造：默认 code = `ErrorCode::Error`（"ERROR"），
/// 供各 api handler `use crate::error::api_error as err`，消除逐文件复制的 `fn err`。
pub fn api_error(status: StatusCode, message: &str) -> (StatusCode, Json<ErrorBody>) {
    error_with_status(status, ErrorCode::default().as_str(), message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_format() {
        let resp = error_response("TEST_CODE", "test message");
        let body = resp.0;
        assert_eq!(body.error.code, "TEST_CODE");
        assert_eq!(body.error.message, "test message");
    }

    #[test]
    fn test_error_with_status() {
        let (status, resp) = error_with_status(StatusCode::NOT_FOUND, "NOT_FOUND", "not found");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(resp.0.error.code, "NOT_FOUND");
    }

    #[test]
    fn test_error_body_serialization() {
        let body = ErrorBody {
            error: ErrorDetail {
                code: "AUTH_REQUIRED".into(),
                message: "missing token".into(),
            },
        };
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["error"]["code"], "AUTH_REQUIRED");
        assert_eq!(json["error"]["message"], "missing token");
    }
}
