//! 统一错误响应格式。

use axum::extract::ws::{Message, WebSocket};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorCode {
    /// 通用错误（历史默认，保持行为不变）。
    #[default]
    Error,
    AuthInvalid,
    AuthRequired,
    AgentUnavailable,
    NotFound,
    Conflict,
    BadRequest,
    Internal,
    Unimplemented,
    /// 上游数据库/Redis/文件系统连接失败（分类后的根因码）。
    ConnectionFailed,
    ConnectionTimeout,
    ConnectionRefused,
    DnsFailure,
    TlsFailure,
    /// 上游资源鉴权失败（区别于 Hub 自身的 AUTH_*）。
    AuthFailed,
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
            ErrorCode::ConnectionFailed => "CONNECTION_FAILED",
            ErrorCode::ConnectionTimeout => "CONNECTION_TIMEOUT",
            ErrorCode::ConnectionRefused => "CONNECTION_REFUSED",
            ErrorCode::DnsFailure => "DNS_FAILURE",
            ErrorCode::TlsFailure => "TLS_FAILURE",
            ErrorCode::AuthFailed => "AUTH_FAILED",
        }
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

/// 将连接类错误的文案归类为结构化根因码。
///
/// 基于错误源链的字符串启发式，兼容 sqlx / redis / s3 等不同错误类型，不引入类型耦合。
/// 返回的码可直接用于 `error.code`，供前端区分 timeout/refused/dns/tls/auth。
pub fn classify_connect_error(err_msg: &str) -> ErrorCode {
    let lower = err_msg.to_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") || lower.contains("deadline") {
        ErrorCode::ConnectionTimeout
    } else if lower.contains("connection refused") {
        ErrorCode::ConnectionRefused
    } else if lower.contains("dns")
        || lower.contains("no such host")
        || lower.contains("lookup")
        || lower.contains("failed to resolve")
    {
        ErrorCode::DnsFailure
    } else if lower.contains("tls")
        || lower.contains("ssl")
        || lower.contains("certificate")
        || lower.contains("handshake")
    {
        ErrorCode::TlsFailure
    } else if lower.contains("password")
        || lower.contains("access denied")
        || lower.contains("authentication")
        || lower.contains("login failed")
        || lower.contains("invalid credentials")
    {
        ErrorCode::AuthFailed
    } else {
        ErrorCode::ConnectionFailed
    }
}

/// 连接类错误响应：自动分类根因码，附加上下文前缀并保留原始错误文案。
pub fn connect_error_response(context: &str, err: impl std::fmt::Display) -> Json<ErrorBody> {
    let raw = err.to_string();
    let code = classify_connect_error(&raw);
    error_response(code.as_str(), &format!("{context}: {raw}"))
}

/// 统一的 WS 错误信令「序列化 + 发送」底座。
///
/// 各 ws handler（terminal / sip / tunnel）保留自己原有的负载形状
/// （`ErrorPayload{message}` / `ReasonPayload{reason}` / `TunnelMsg::Error{message}`），
/// 仅复用本函数的序列化-发送逻辑，**不改变线上信令格式**。
pub async fn send_ws_json<T: Serialize>(ws: &mut WebSocket, msg: &T) -> Result<(), axum::Error> {
    ws.send(Message::Text(
        serde_json::to_string(msg).unwrap_or_default().into(),
    ))
    .await
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

    #[test]
    fn test_classify_connect_error() {
        assert_eq!(
            classify_connect_error("connection timed out"),
            ErrorCode::ConnectionTimeout
        );
        assert_eq!(
            classify_connect_error("connection refused"),
            ErrorCode::ConnectionRefused
        );
        assert_eq!(
            classify_connect_error("failed to resolve host"),
            ErrorCode::DnsFailure
        );
        assert_eq!(
            classify_connect_error("TLS handshake failed"),
            ErrorCode::TlsFailure
        );
        assert_eq!(
            classify_connect_error("password authentication failed"),
            ErrorCode::AuthFailed
        );
        assert_eq!(
            classify_connect_error("something odd"),
            ErrorCode::ConnectionFailed
        );
    }
}
