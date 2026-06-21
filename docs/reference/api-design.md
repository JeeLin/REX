# API 设计规范

## 认证

单用户登录：

```http
POST /api/auth/login
```

请求：

```json
{
  "username": "admin",
  "password": "password"
}
```

响应：

```json
{
  "token": "rex_session_xxx",
  "expiresAt": "2026-06-17T00:00:00Z"
}
```

认证方式：

- 登录成功后返回 bearer token。
- 前端请求携带 `Authorization: Bearer <token>` header。
- 所有管理 API 必须认证。
- `/healthz` 可以公开，只返回基础健康状态。

## 统一错误响应

```json
{
  "error": {
    "code": "RESOURCE_NOT_FOUND",
    "message": "资源不存在",
    "details": {}
  }
}
```

错误码：

```text
AUTH_REQUIRED
AUTH_INVALID
RESOURCE_NOT_FOUND
ENVIRONMENT_NOT_FOUND
AGENT_NOT_FOUND
AGENT_OFFLINE
CONNECTION_FAILED
TRANSFER_NOT_FOUND
UPDATE_NOT_AVAILABLE
UPDATE_FAILED
VALIDATION_ERROR
INTERNAL_ERROR
```

## 分页响应

```json
{
  "items": [],
  "page": 1,
  "pageSize": 50,
  "total": 128
}
```

## WebSocket 消息

统一消息结构：

```ts
interface REXMessage<T = unknown> {
  id?: string;
  type: string;
  payload: T;
}
```

示例：

```json
{
  "type": "terminal.data",
  "payload": {
    "data": "base64-or-utf8-stream"
  }
}
```
