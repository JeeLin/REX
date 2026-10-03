# 错误码参考（v0.90.1）

REX Hub/Agent 对 REST 与 WS 统一错误响应体格式：

```json
{ "error": { "code": "<code>", "message": "<human-readable>" } }
```

`error.code` 有两个来源（wire 字面量保持历史行为不变）：

1. **`ErrorCode` 枚举**（`crates/rex-hub/src/error.rs`）：默认码与连接分类码由 `as_str()` 生成，经 `api_error` / `error_with_status` / `connect_error_response` 产出。
2. **各 handler 字面量**：业务错误码在各 `*_api.rs` 中以字符串字面量直接产出（历史行为，wire 不变）。

handler 通过 `crate::error::api_error`（别名 `err`）或 `error_with_status(status, code, message)` 构造响应；HTTP 状态由 handler 显式传入，非由 code 派生。

### 默认码（`ErrorCode` 枚举）

| `code`（字面量） | `ErrorCode` 变体 | 说明 |
| --- | --- | --- |
| `ERROR` | `Error` | 默认/通用错误（`api_error` 默认，历史默认，行为不变） |

### 连接类错误（v0.90.1）

上游数据库/Redis/S3/SSH/SFTP 连接失败时，`connect_error_response(context, err)` 根据错误源链做字符串启发式分类，统一为以下 `ErrorCode`。
分类函数 `classify_connect_error` 兼容 sqlx / redis / s3 等不同错误类型，不引入类型耦合；结果码直接用于 `ErrorBody.error.code`，供前端区分 timeout / refused / dns / tls / auth。

| `code`（字面量） | `ErrorCode` 变体 | 分类依据（启发式） | 说明 |
| --- | --- | --- | --- |
| `CONNECTION_TIMEOUT` | `ConnectionTimeout` | 信息含 `timed out`/`timeout`/`deadline` | 连接超时 |
| `CONNECTION_REFUSED` | `ConnectionRefused` | 信息含 `connection refused` | 连接被拒 |
| `DNS_FAILURE` | `DnsFailure` | 信息含 `dns`/`no such host`/`lookup`/`failed to resolve` | 域名解析失败 |
| `TLS_FAILURE` | `TlsFailure` | 信息含 `tls`/`ssl`/`certificate`/`handshake` | TLS/SSL 失败 |
| `AUTH_FAILED` | `AuthFailed` | 信息含 `password`/`access denied`/`authentication`/`login failed`/`invalid credentials` | 上游资源鉴权失败（区别于 Hub 自身的 `AUTH_INVALID`/`AUTH_REQUIRED`） |
| `CONNECTION_FAILED` | `ConnectionFailed` | 兜底（无法归类） | 其它连接失败 |

### 业务错误码（handler 字面量）

以下码由各 handler 直接以字面量产出；HTTP 状态由调用处显式传入（多为 4xx/5xx）。

| `code`（字面量） | 出处 | 说明 |
| --- | --- | --- |
| `AUTH_INVALID` | `auth.rs` / `middleware.rs` | token 校验失败（签名过期/非法），401 |
| `AUTH_REQUIRED` | `middleware.rs` | 缺少 `Authorization`/`token`，401 |
| `INTERNAL_ERROR` | `auth.rs` | 内部错误，500 |
| `PASSWORD_ALREADY_SET` | `auth.rs` | 密码已设置，409 |
| `PASSWORD_CHANGE_FAILED` | `auth.rs` | 修改密码失败 |
| `NOT_FOUND` | `rex-hub.rs`（`/api` 兜底） | 未知 API 路径，404 |
| `SESSION_NOT_FOUND` | `file_api` / `sql_api` / `redis_api` / `mongodb_api` | 会话不存在或已失效 |
| `INVALID_RESOURCE` | `file_api` / `sql_api` / `redis_api` / `mongodb_api` | 资源无效或不存在 |
| `AGENT_UNAVAILABLE` | `file_api` / `sql_api` / `redis_api` | 目标 Agent 离线/不可达 |
| `AGENT_CONNECT_FAILED` | `file_api` / `sql_api` / `redis_api` | Agent 隧道会话建立失败 |
| `UNSUPPORTED_PROTOCOL` | `file_api` / `sql_api` | 不支持的资源协议 |
| `INVALID_DB_TYPE` | `sql_api` | 不支持的数据库类型 |
| `INVALID_NAME` | `sql_api` | 库/表名非法 |
| `QUERY_FAILED` | `sql_api` / `redis_api` / `mongodb_api` | 查询执行失败 |
| `QUERY_FAILED_LEFT` / `QUERY_FAILED_RIGHT` | `sql_api` | 多语句执行时左/右语句失败 |
| `QUERY_TIMEOUT` | `sql_api` | 查询超时 |
| `DB_ERROR` | `sql_api` | 数据库执行错误 |
| `CLIENT_ERROR` | `redis_api` | Redis 客户端错误 |
| `CONNECT_ERROR` | `redis_api` | Redis 连接错误 |
| `SUBSCRIBE_ERROR` | `redis_api` | Redis 订阅错误 |
| `CURSOR_ERROR` | `mongodb_api` | 游标迭代错误 |
| `INVALID_OPERATION` | `mongodb_api` | 非法操作 |
| `INVALID_URI` | `mongodb_api` | 非法连接 URI |
| `LIST_FAILED` | `file_api` / `mongodb_api` | 列表读取失败 |
| `MISSING_FILE` | `file_api` | 文件不存在 |
| `READ_FAILED` / `STAT_FAILED` | `file_api` | 读取/元数据失败 |
| `MKDIR_FAILED` / `RENAME_FAILED` | `file_api` | 建目录/重命名失败 |
| `DELETE_FAILED` | `file_api` | 删除失败 |
| `SAVE_FAILED` | `file_api` | 保存失败 |
| `INVALID_CONTENT` | `file_api` | 内容非法 |
| `INVALID_RANGE` | `file_api` | HTTP Range 请求非法 |
| `UPLOAD_FAILED` | `file_api`；前端 `files.ts` XHR 亦产生同码 | 文件上传失败 |
| `DOWNLOAD_FAILED` | `file_api` | 文件下载失败 |
| `ABORT_UPLOAD_FAILED` / `RESUME_UPLOAD_FAILED` | `file_api` | 中止/续传上传失败 |
| `PRESIGNED_URL_FAILED` | `file_api` | 预签名 URL 生成失败 |
| `LIST_UPLOADS_FAILED` / `GET_ACL_FAILED` / `PUT_ACL_FAILED` | `file_api` | 分片列表/ACL 操作失败 |
| `NO_ROLLBACK` / `NO_UPDATE_STATE` | `update_api` | 无回滚二进制/无更新状态 |
| `UPDATE_CHECK_FAILED` / `SERIALIZE_FAILED` | `update_api` | 检查更新/序列化失败 |
| `WRITE_FAILED` / `RENAME_FAILED` | `update_api` | 更新包写盘/替换失败 |

> 测试专用码（如 `TEST_CODE`）不出现在生产响应中，本文不列。

### WS 信令错误（v0.90.1）

`send_ws_error` 复用共享 `send_ws_json` 序列化+发送助手（消除 terminal / sip / tunnel 间的重复序列化逻辑）。

- `terminal` 的 `ErrorPayload` 新增 `code` 字段（**加法，wire 兼容**）：`send_ws_error` 在序列化前用 `classify_connect_error` 填充，取值见上表连接类错误；供前端弹 toast，并可据此跳转至本文档。
- `sip` 用 `ReasonPayload { reason }`；`tunnel` 用 `TunnelMsg::Error { message }` — 形状不变。

> 后续可选灰度：在 `terminal.error` 之外的 WS 信令 payload 补充结构化 `code`，以完全打通 REST 与 WS 的错误码体系。
