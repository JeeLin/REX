//! 统一错误响应格式。

use axum::extract::ws::{Message, WebSocket};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    /// 连接失败发生在握手的哪一步（`dns` / `tcp` / `tls` / `auth` /
    /// `timeout` / `connect` / `config`），取 [`connect_stage`] 与各协议的
    /// 解析阶段。非连接类错误不产出该字段，wire 行为与此前一致。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
}

/// 返回 JSON 错误体（不含状态码），用于需要自行包装的场景。
pub fn error_response(code: &str, message: &str) -> Json<ErrorBody> {
    Json(ErrorBody {
        error: ErrorDetail {
            code: code.to_string(),
            message: message.to_string(),
            stage: None,
        },
    })
}

/// 返回 JSON 错误体并附带阶段码，用于连接/解析类失败的诊断。
pub fn error_response_with_stage(code: &str, message: &str, stage: &str) -> Json<ErrorBody> {
    Json(ErrorBody {
        error: ErrorDetail {
            code: code.to_string(),
            message: message.to_string(),
            stage: Some(stage.to_string()),
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

/// [`error_with_status`] 的带阶段变体：状态码与 `stage` 同时给出。
pub fn error_with_status_and_stage(
    status: StatusCode,
    code: &str,
    message: &str,
    stage: &str,
) -> (StatusCode, Json<ErrorBody>) {
    (status, error_response_with_stage(code, message, stage))
}

/// REST 错误码枚举：默认码与连接分类码定义在这里，`as_str()` → JSON `error.code`。
///
/// 历史业务码（`AUTH_INVALID` 等）仍由各 handler 以字面量产出（wire 行为不变），
/// 全量清单见 `docs/reference/error-codes.md`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorCode {
    /// 通用错误（历史默认，保持行为不变）。
    #[default]
    Error,
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

/// 根因码 → 连接阶段。回答「失败发生在握手的哪一步」，取值沿用 Agent 侧
/// `STAGE_*` 的风格（小写单词）：`dns` 解析、`tcp` 建链、`tls` 握手加密、
/// `auth` 鉴权、`timeout` 等待超时，`connect` 为无法再细分的兜底。
pub fn connect_stage(root: ErrorCode) -> &'static str {
    match root {
        ErrorCode::ConnectionTimeout => "timeout",
        ErrorCode::ConnectionRefused => "tcp",
        ErrorCode::DnsFailure => "dns",
        ErrorCode::TlsFailure => "tls",
        ErrorCode::AuthFailed => "auth",
        // 兜底：失败确实发生在建连阶段，但根因无法从文案里再细分。
        ErrorCode::Error | ErrorCode::ConnectionFailed => "connect",
    }
}

/// 连接类错误响应：自动分类根因码，附加上下文前缀并保留原始错误文案。
pub fn connect_error_response(context: &str, err: impl std::fmt::Display) -> Json<ErrorBody> {
    let raw = err.to_string();
    let code = classify_connect_error(&raw);
    error_response(code.as_str(), &format!("{context}: {raw}"))
}

/// 连接类错误响应的结构化变体：`code` 带协议前缀、`error.stage` 给出连接阶段、
/// `message` 保留完整错误链。
///
/// `raw` 是最外层 `Display`，分类输入与 [`connect_error_response`] 逐字一致，
/// 因此既有根因码不变（只加协议前缀）；`chain` 是 `error_chain` 或 `{err:#}`
/// 产出的完整错误链，只用于 message —— 这样既有错误信息不丢，
/// 又能让用户看到驱动回显的底层原因。
pub fn connect_error_response_with_stage(
    context: &str,
    raw: &str,
    chain: &str,
    proto: ProtoKind,
) -> Json<ErrorBody> {
    let root = classify_connect_error(raw);
    let code = wire_code(proto, root.as_str());
    let detail = if chain.is_empty() { raw } else { chain };
    error_response_with_stage(&code, &format!("{context}: {detail}"), connect_stage(root))
}

/// 把错误展开为完整错误链（逐层 `source()` 以 `: ` 连接）。
///
/// `Display` 只给最外层文案，驱动侧的根因（拒绝连接 / 证书 / 认证失败）通常在
/// `source()` 里；诊断要完整又不丢根因，故按 Agent 侧 `{err:#}` 的做法展开整条链。
pub fn error_chain(err: &dyn std::error::Error) -> String {
    let mut chain = err.to_string();
    let mut source = std::error::Error::source(err);
    while let Some(cause) = source {
        let text = cause.to_string();
        if !chain.contains(&text) {
            chain.push_str(": ");
            chain.push_str(&text);
        }
        source = std::error::Error::source(cause);
    }
    chain
}

/// 抹掉文案中的凭据片段（密码 / 连接串密钥），命中处替换为 `***`。
///
/// 完整错误链可能带上驱动回显的连接串；凭据只应留在内存里，不能进用户可见
/// 文案或日志。空串片段会被忽略 —— 空密码不该把整条文案抹成 `***`。
pub fn redact_secrets(message: &str, secrets: &[&str]) -> String {
    let mut out = message.to_string();
    for secret in secrets.iter().copied().filter(|s| !s.is_empty()) {
        if out.contains(secret) {
            out = out.replace(secret, "***");
        }
    }
    out
}

/// 建连失败的分类：决定是否值得让前端重连。
///
/// 只有「重试一次可能变成成功」的错误才算可重试。凭据解密失败、config_json
/// 不是合法 JSON、资源不存在、host 为空 —— 这些重连多少次都会在同一处失败，
/// 继续重连既刷 `/ws/terminal` 日志又让用户看到终端反复回到初始状态却看不到
/// 原因，因此按终止性错误上报。DB / spawn_blocking 这类基础设施抖动仍可重试。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnFailure {
    /// 终止性：重连无法改变结果。
    Fatal,
    /// 可重试：传输或数据库层的瞬时故障。
    Transient,
}

impl ConnFailure {
    pub fn retryable(self) -> bool {
        matches!(self, ConnFailure::Transient)
    }
}

/// WS 错误帧负载（`terminal.error` 的 `payload`）。集中定义，供所有 WS 信令共用。
///
/// `Deserialize` 用于断言线上 JSON 形状；`skip_serializing_if` 与历史行为
/// 完全一致 —— `retryable: true` 时不输出该字段，前端按缺省=可重试处理。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    /// 该错误重连是否有可能自愈。前端据此决定是否自动重连：
    /// `false` = 终止性（配置/凭据类，重试多少次都是同一结果）→ 停止重连并
    /// 把 `message` 呈现给用户；`true` / 缺省 = 可重试（传输中断等）。
    #[serde(
        rename = "retryable",
        default = "default_true",
        skip_serializing_if = "is_true"
    )]
    pub retryable: bool,
}

fn is_true(v: &bool) -> bool {
    *v
}

/// `retryable` 缺省为 `true`：历史/裸 `code` 帧无该字段时视为可重试，
/// 与 `skip_serializing_if = "is_true"` 释出时保持一致。
fn default_true() -> bool {
    true
}

/// 终止性错误的结构化码。用于 `retryable=false` 错误帧的 `code` 字段，
/// 让前端 / 用户能精确定位（解密失败 vs 认证失败 vs 资源缺失）。
///
/// 码值是既有线上契约，逐字不变。
pub fn fatal_error_code(msg: &str) -> &'static str {
    let lower = msg.to_lowercase();
    if lower.contains("decryption failed") || lower.contains("decrypt failed") {
        "SSH_CONFIG_DECRYPT_FAILED"
    } else if lower.contains("invalid config json") {
        "SSH_CONFIG_INVALID"
    } else if lower.contains("resource not found") {
        "RESOURCE_NOT_FOUND"
    } else if lower.contains("environment not found") {
        "ENVIRONMENT_NOT_FOUND"
    } else if lower.contains("host is empty") || lower.contains("host is required") {
        "HOST_REQUIRED"
    } else if lower.contains("authentication failed") || lower.contains("auth") {
        "AUTH_FAILED"
    } else {
        "ERROR"
    }
}

/// 协议种类 —— 错误码的第二个正交维度（第一个是 `ErrorCode` 根因码）。
///
/// 与 `ErrorCode` 组合时给出 wire `code` 的前缀；不参与组合时（`Ssh`）保持
/// 既有裸码输出，避免改动前端判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtoKind {
    /// SSH 终端：终止性码与根因码均无前缀，保持既有裸码（向后兼容约束）。
    #[default]
    Ssh,
    Sql,
    Redis,
    MongoDb,
    Tunnel,
    /// 资源测试连接（`resource_api::test_connection`）的 TCP / Agent 探测。
    ///
    /// 该路径的响应不是 `ErrorBody` 而是 `TestConnectionResponse{ok,error}`，
    /// 没有 `code` 字段，故只共用 `classify_connect_error` 与本枚举的通用根因码，
    /// 不参与前缀组合；保留变体是为后续把该路径也改成结构化 code 留出位置。
    ResourceTest,
}

impl ProtoKind {
    /// wire `code` 的协议前缀；`None` = 不加前缀（输出既有裸码）。
    pub fn prefix(self) -> Option<&'static str> {
        match self {
            ProtoKind::Ssh | ProtoKind::ResourceTest => None,
            ProtoKind::Sql => Some("SQL"),
            ProtoKind::Redis => Some("REDIS"),
            ProtoKind::MongoDb => Some("MONGODB"),
            ProtoKind::Tunnel => Some("TUNNEL"),
        }
    }
}

/// wire `code` 的唯一拼串入口：`<prefix>_<root>`，`prefix` 为 `None` 时输出
/// 裸 `root`。`root` 已由调用方给出（从文案分类得到，或协议自身的失败点常量）。
pub fn wire_code(proto: ProtoKind, root: &str) -> String {
    match proto.prefix() {
        Some(prefix) => format!("{prefix}_{root}"),
        None => root.to_string(),
    }
}

/// 错误码的唯一合成入口。
///
/// 规则（新增错误码一律走这里，禁止再各自拼串）：
///
/// ```text
/// code     = <prefix>_<root>   prefix = ProtoKind::prefix()，None 时不拼前缀
/// root     = ErrorCode::as_str()              （可重试）
/// root     = fatal_error_code(msg)             （终止性）
/// retryable = !fatal
/// ```
///
/// 协议前缀的选用：`SQL` / `REDIS` / `MONGODB` 与既有 audit action
/// （`MONGO_CONNECT` 等）及 API 路径（`/api/mongodb/*`）一致；`TUNNEL` 用于
/// 传输隧道。SSH 终端不拼前缀 —— 其终止性码字面量已含 `SSH_`/`RESOURCE_`
/// 等领域信息，强行再拼会得到 `SSH_SSH_CONFIG_DECRYPT_FAILED`，既冗余又会
/// 破坏前端既有的 `retryable=false` 判定。
///
/// 根因码的分类函数与既有实现共用（`classify_connect_error` / `fatal_error_code`），
/// 不引入第三套码表；`ProtoKind::Ssh` + 无前缀的组合逐字复刻迁移前的输出。
pub fn connect_error_with_stage(err_msg: &str, proto: ProtoKind, fatal: bool) -> ErrorPayload {
    let root = if fatal {
        fatal_error_code(err_msg)
    } else {
        classify_connect_error(err_msg).as_str()
    };
    ErrorPayload {
        code: wire_code(proto, root),
        message: err_msg.to_string(),
        retryable: !fatal,
    }
}

/// 统一的 WS 错误信令「序列化 + 发送」底座。
///
/// 各 ws handler（terminal / sip / tunnel）复用本函数的序列化-发送逻辑；
/// `terminal` / `tunnel` 的错误负载统一为 [`ErrorPayload`]，
/// `sip` 仍保留自己的 `ReasonPayload { reason }` 形状，**不改变线上信令格式**。
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
                stage: None,
            },
        };
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["error"]["code"], "AUTH_REQUIRED");
        assert_eq!(json["error"]["message"], "missing token");
        assert!(
            json["error"].get("stage").is_none(),
            "non-connect errors must keep the wire shape unchanged"
        );
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

    #[test]
    fn test_fatal_error_code_values_unchanged() {
        assert_eq!(
            fatal_error_code("SSH credential decryption failed: decrypt failed: aead::Error"),
            "SSH_CONFIG_DECRYPT_FAILED"
        );
        assert_eq!(
            fatal_error_code("invalid config json: bad token"),
            "SSH_CONFIG_INVALID"
        );
        assert_eq!(
            fatal_error_code("SSH authentication failed (password: ...)"),
            "AUTH_FAILED"
        );
        assert_eq!(
            fatal_error_code("resource not found: r1"),
            "RESOURCE_NOT_FOUND"
        );
        assert_eq!(
            fatal_error_code("environment not found: e1"),
            "ENVIRONMENT_NOT_FOUND"
        );
        assert_eq!(
            fatal_error_code("resource r1: host is empty"),
            "HOST_REQUIRED"
        );
        assert_eq!(
            fatal_error_code("resource r1: host is required"),
            "HOST_REQUIRED"
        );
        assert_eq!(fatal_error_code("something odd"), "ERROR");
    }

    #[test]
    fn test_connect_error_with_stage_no_prefix_keeps_bare_code() {
        for proto in [ProtoKind::Ssh, ProtoKind::ResourceTest] {
            assert_eq!(proto.prefix(), None);
            let p = connect_error_with_stage("connection refused", proto, false);
            assert_eq!(p.code, "CONNECTION_REFUSED");
            assert_eq!(p.message, "connection refused");
            assert!(p.retryable);
        }
    }

    #[test]
    fn test_connect_error_with_stage_proto_prefixes() {
        let cases = [
            (
                ProtoKind::Sql,
                "connection refused",
                "SQL_CONNECTION_REFUSED",
            ),
            (
                ProtoKind::Redis,
                "connection timed out",
                "REDIS_CONNECTION_TIMEOUT",
            ),
            (
                ProtoKind::MongoDb,
                "password authentication failed",
                "MONGODB_AUTH_FAILED",
            ),
            (
                ProtoKind::Tunnel,
                "failed to resolve host",
                "TUNNEL_DNS_FAILURE",
            ),
            (
                ProtoKind::Tunnel,
                "something odd",
                "TUNNEL_CONNECTION_FAILED",
            ),
        ];
        for (proto, msg, expected) in cases {
            let p = connect_error_with_stage(msg, proto, false);
            assert_eq!(p.code, expected);
            assert_eq!(p.message, msg);
            assert!(p.retryable);
        }
    }

    #[test]
    fn test_connect_error_with_stage_fatal_uses_fatal_code() {
        let p = connect_error_with_stage("resource not found: r1", ProtoKind::Ssh, true);
        assert_eq!(p.code, "RESOURCE_NOT_FOUND");
        assert!(!p.retryable);

        let p = connect_error_with_stage("resource not found: r1", ProtoKind::Tunnel, true);
        assert_eq!(p.code, "TUNNEL_RESOURCE_NOT_FOUND");
        assert!(!p.retryable);
    }

    #[test]
    fn test_error_payload_serializes_with_fatal_flag() {
        let p = connect_error_with_stage("authentication failed", ProtoKind::Sql, true);
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["code"], "SQL_AUTH_FAILED");
        assert_eq!(json["message"], "authentication failed");
        assert_eq!(json["retryable"], false);
    }

    #[test]
    fn test_error_payload_retryable_flag_serialization() {
        let retryable = serde_json::to_value(connect_error_with_stage(
            "connection refused",
            ProtoKind::Sql,
            false,
        ))
        .unwrap();
        assert_eq!(retryable["code"], "SQL_CONNECTION_REFUSED");
        assert!(retryable.get("retryable").is_none());

        let fatal = serde_json::to_value(connect_error_with_stage(
            "authentication failed",
            ProtoKind::Sql,
            true,
        ))
        .unwrap();
        assert_eq!(fatal["retryable"], false);
    }

    #[test]
    fn test_error_payload_round_trip() {
        let p: ErrorPayload =
            serde_json::from_str(r#"{"code":"SQL_CONN","message":"boom","retryable":false}"#)
                .unwrap();
        assert_eq!(p.code, "SQL_CONN");
        assert_eq!(p.message, "boom");
        assert!(!p.retryable);

        let omitted: ErrorPayload =
            serde_json::from_str(r#"{"code":"SQL_CONN","message":"boom"}"#).unwrap();
        assert!(omitted.retryable);
    }

    #[test]
    fn test_conn_failure_retryable() {
        assert!(ConnFailure::Transient.retryable());
        assert!(!ConnFailure::Fatal.retryable());
    }

    #[test]
    fn test_connect_stage_covers_every_root_code() {
        assert_eq!(connect_stage(ErrorCode::ConnectionTimeout), "timeout");
        assert_eq!(connect_stage(ErrorCode::ConnectionRefused), "tcp");
        assert_eq!(connect_stage(ErrorCode::DnsFailure), "dns");
        assert_eq!(connect_stage(ErrorCode::TlsFailure), "tls");
        assert_eq!(connect_stage(ErrorCode::AuthFailed), "auth");
        assert_eq!(connect_stage(ErrorCode::ConnectionFailed), "connect");
        assert_eq!(connect_stage(ErrorCode::Error), "connect");
    }

    #[test]
    fn test_wire_code_composition() {
        assert_eq!(wire_code(ProtoKind::Sql, "AUTH_FAILED"), "SQL_AUTH_FAILED");
        assert_eq!(
            wire_code(ProtoKind::MongoDb, "AUTH_FAILED"),
            "MONGODB_AUTH_FAILED"
        );
        assert_eq!(wire_code(ProtoKind::Ssh, "AUTH_FAILED"), "AUTH_FAILED");
        assert_eq!(
            wire_code(ProtoKind::ResourceTest, "AUTH_FAILED"),
            "AUTH_FAILED"
        );
    }

    #[test]
    fn test_connect_error_response_with_stage_prefixed_code_and_stage() {
        let resp = connect_error_response_with_stage(
            "failed to connect to mysql SQL database",
            "error connecting: Connection refused (os error 111)",
            "error connecting: Connection refused (os error 111)",
            ProtoKind::Sql,
        );
        let body = resp.0;
        assert_eq!(body.error.code, "SQL_CONNECTION_REFUSED");
        assert_eq!(body.error.stage.as_deref(), Some("tcp"));
        assert!(body.error.message.contains("Connection refused"));
    }

    #[test]
    fn test_connect_error_response_with_stage_keeps_full_chain() {
        let resp = connect_error_response_with_stage(
            "MongoDB ping failed at 10.0.0.1:27017",
            "Kind: timed out",
            "Kind: timed out: connection timed out after 30s",
            ProtoKind::MongoDb,
        );
        let body = resp.0;
        assert_eq!(body.error.code, "MONGODB_CONNECTION_TIMEOUT");
        assert_eq!(body.error.stage.as_deref(), Some("timeout"));
        assert_eq!(
            body.error.message,
            "MongoDB ping failed at 10.0.0.1:27017: Kind: timed out: connection timed out after 30s"
        );
    }

    #[test]
    fn test_connect_error_response_with_stage_root_code_matches_legacy_classifier() {
        // 根因码必须与既有 `classify_connect_error` 逐字一致（只加协议前缀），
        // 否则既有按 `CONNECTION_*` 判定的调用方会被静默改码。
        for msg in [
            "connection refused",
            "connection timed out",
            "no such host",
            "certificate verify failed",
            "password authentication failed",
            "something odd",
        ] {
            let legacy = connect_error_response("ctx", msg).0.error.code;
            let staged = connect_error_response_with_stage("ctx", msg, msg, ProtoKind::Sql).0;
            assert_eq!(staged.error.code, format!("SQL_{legacy}"));
        }
    }

    #[test]
    fn test_connect_error_response_with_stage_empty_chain_falls_back_to_raw() {
        let resp =
            connect_error_response_with_stage("ctx", "connection refused", "", ProtoKind::Redis);
        assert_eq!(resp.0.error.message, "ctx: connection refused");
    }

    #[test]
    fn test_error_response_with_status_and_stage() {
        let (status, resp) = error_with_status_and_stage(
            StatusCode::BAD_REQUEST,
            "INVALID_URI",
            "invalid MongoDB URI",
            "config",
        );
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(resp.0.error.code, "INVALID_URI");
        assert_eq!(resp.0.error.stage.as_deref(), Some("config"));
    }

    #[test]
    fn test_error_chain_walks_source_chain() {
        #[derive(Debug)]
        struct Outer(&'static str, Inner);
        #[derive(Debug)]
        struct Inner(&'static str, Leaf);
        #[derive(Debug)]
        struct Leaf(&'static str);
        impl std::fmt::Display for Leaf {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.0)
            }
        }
        impl std::error::Error for Leaf {}
        impl std::fmt::Display for Inner {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.0)
            }
        }
        impl std::error::Error for Inner {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.1)
            }
        }
        impl std::fmt::Display for Outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.0)
            }
        }
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.1)
            }
        }

        let err = Outer("connect failed", Inner("handshaking", Leaf("refused")));
        let chain = error_chain(&err);
        assert_eq!(chain, "connect failed: handshaking: refused");
    }

    #[test]
    fn test_error_chain_does_not_duplicate_repeated_text() {
        #[derive(Debug)]
        struct Leaf(&'static str);
        impl std::fmt::Display for Leaf {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.0)
            }
        }
        impl std::error::Error for Leaf {}

        #[derive(Debug)]
        struct Mid(&'static str, Leaf);
        impl std::fmt::Display for Mid {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.0)
            }
        }
        impl std::error::Error for Mid {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.1)
            }
        }

        // Display 与 cause 文案重复时不再追加，链不会变成 "boom: boom"
        assert_eq!(error_chain(&Mid("boom", Leaf("boom"))), "boom");
    }

    #[test]
    fn test_redact_secrets_masks_password_and_keeps_reason() {
        let raw =
            "error connecting to mongodb://alice:s3cr3t-pw@10.0.0.1:27017: connection refused";
        let masked = redact_secrets(raw, &["s3cr3t-pw", "s3cr3t%2Dpw", ""]);
        assert!(!masked.contains("s3cr3t-pw"));
        assert_eq!(
            masked,
            "error connecting to mongodb://alice:***@10.0.0.1:27017: connection refused"
        );
    }

    #[test]
    fn test_redact_secrets_ignores_empty_secret() {
        let raw = "connection refused";
        assert_eq!(redact_secrets(raw, &[""]), raw);
        assert_eq!(redact_secrets(raw, &[]), raw);
    }

    #[test]
    fn test_connect_error_response_with_stage_redacts_credentials() {
        let chain = redact_secrets(
            "connection to mongodb://alice:s3cr3t@10.0.0.1:27017 failed: authentication failed",
            &["s3cr3t"],
        );
        let resp = connect_error_response_with_stage(
            "failed to create MongoDB client",
            "authentication failed",
            &chain,
            ProtoKind::MongoDb,
        );
        let body = resp.0;
        assert_eq!(body.error.code, "MONGODB_AUTH_FAILED");
        assert_eq!(body.error.stage.as_deref(), Some("auth"));
        assert!(!body.error.message.contains("s3cr3t"));
        assert!(body.error.message.contains("mongodb://alice:***@"));
    }
}
