# 错误码参考（v0.90.1）

REX Hub/Agent 对 REST 与 WS 统一错误响应体格式：

```json
{ "error": { "code": "<ErrorCode>", "message": "<human-readable>" } }
```

`code` 取值统一在 `crates/rex-hub/src/error.rs` 的 `ErrorCode` 枚举生成；`as_str()` 决定 JSON 字段字面量。
handler 通过 `crate::error::api_error`（别名 `err`）构造响应，HTTP 状态由 handler 显式传入。

### REST 错误码表

| `code`（字面量） | `ErrorCode` 变体 | 说明 | 典型 HTTP 状态 |
| --- | --- | --- | --- |
| `ERROR` | `Error` | 默认/通用错误（历史默认，行为不变） | 500 |
| `AUTH_INVALID` | `AuthInvalid` | token 校验失败（签名过期/非法） | 401 |
| `AUTH_REQUIRED` | `AuthRequired` | 缺少 `Authorization`/`token` | 401 |
| `AGENT_UNAVAILABLE` | `AgentUnavailable` | 目标 agent 离线/不可达 | 503 |
| `NOT_FOUND` | `NotFound` | 资源不存在 | 404 |
| `CONFLICT` | `Conflict` | 状态冲突（例如并发写/资源已占用） | 409 |
| `BAD_REQUEST` | `BadRequest` | 参数缺陷 / 请求体非法 | 400 |
| `INTERNAL` | `Internal` | 内部错误 | 500 |
| `UNIMPLEMENTED` | `Unimplemented` | 功能未实现 | 501 |

> `HTTP 状态` 为各 handler 显式传入，非自动派生；上表为约定中的典型值。

### 连接类错误（v0.90.1）

上游数据库/Redis/S3/SSH/SFTP 连接失败时，`connect_error_response(context, err)` 根据错误源链做字符串启活式分类，统一为以下 `ErrorCode`。
分类函数 `classify_connect_error` 兼容 sqlx / redis / s3 等不同错误类型，不引入类型耦合；结果码直接用于 `ErrorBody.error.code`，供前端区分 timeout / refused / dns / tls / auth。

| `code`（字面量） | `ErrorCode` 变体 | 分类依据（启发式） | 说明 |
| --- | --- | --- | --- |
| `CONNECTION_TIMEOUT` | `ConnectionTimeout` | 信息含 `timed out`/`timeout`/`deadline` | 连接超时 |
| `CONNECTION_REFUSED` | `ConnectionRefused` | 信息含 `connection refused` | 连接被拒 |
| `DNS_FAILURE` | `DnsFailure` | 信息含 `dns`/`no such host`/`lookup`/`failed to resolve` | 域名解析失败 |
| `TLS_FAILURE` | `TlsFailure` | 信息含 `tls`/`ssl`/`certificate`/`handshake` | TLS/SSL 失败 |
| `AUTH_FAILED` | `AuthFailed` | 信息含 `password`/`access denied`/`authentication`/`login failed`/`invalid credentials` | 上游资源鉴权失败（区别于 Hub 自身的 `AUTH_INVALID`/`AUTH_REQUIRED`） |
| `CONNECTION_FAILED` | `ConnectionFailed` | 兜底（无法归类） | 其它连接失败 |

### WS 信令错误（v0.90.1）

`send_ws_error` 复用共享 `send_ws_json` 序列化+发送助手（消除 terminal / sip / tunnel 间的重复序列化逻辑）。

- `terminal` 的 `ErrorPayload` 新增 `code` 字段（**加法，wire 兼容**）：`send_ws_error` 在序列化前用 `classify_connect_error` 填充，取值见上表连接类错误；供前端弹 toast，并可据此跳转至本文档。
- `sip` 用 `ReasonPayload { reason }`；`tunnel` 用 `TunnelMsg::Error { message }` — 形状不变。

> 后续可选灰度：在 `terminal.error` 之外的 WS 信令 payload 补充结构化 `code`，以完全打通 REST 与 WS 的错误码体系。
