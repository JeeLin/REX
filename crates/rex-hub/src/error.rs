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
