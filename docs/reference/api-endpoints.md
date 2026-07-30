# REX Hub — API 端点参考

## 认证

所有 `/api/*` 端点（除 `/api/auth/*`）需要 `Authorization: Bearer <token>` 头。

### POST /api/auth/login
登录获取 JWT Token。

**请求体：**
```json
{ "password": "string" }
```

**响应：**
```json
{ "token": "string", "expiresAt": "2026-01-01T00:00:00Z" }
```

### GET /api/auth/check
检查认证状态。

**响应：**
```json
{ "requires_setup": false }
```

### POST /api/auth/password
首次设置密码。

### POST /api/auth/change-password
修改密码。

---

## 环境

### GET /api/environments
获取所有环境列表。

### POST /api/environments
创建环境。

### PUT /api/environments/:id
更新环境。

### DELETE /api/environments/:id
删除环境。

### GET /api/environments/export
导出所有环境配置（不含敏感字段）。

### POST /api/environments/import
导入环境配置。

---

## 资源

### GET /api/environments/:id/resources
获取环境下的资源列表。

### POST /api/environments/:id/resources
创建资源。

### PUT /api/resources/:id
更新资源。

### DELETE /api/resources/:id
删除资源。

---

## Agent

### GET /api/agents
获取所有 Agent 列表。

### GET /api/agents/:id
获取 Agent 详情。

### DELETE /api/agents/:id
删除 Agent。

### GET /api/agents/:id/download
下载 Agent 二进制文件。

---

## 审计日志

### GET /api/audit
查询审计日志（支持筛选）。

**查询参数：** `time_from`, `time_to`, `action`, `result`, `limit`, `offset`

### GET /api/audit/stats
获取审计日志统计。

### GET /api/audit/export
导出审计日志（`?format=csv` 或 `?format=json`）。

### GET /api/audit/security-report
安全审计报告（最近 24h 登录失败统计）。

---

## 备份

### POST /api/backup/create
创建数据库备份。

### GET /api/backup/list
列出所有备份。

### POST /api/backup/restore
恢复备份。

---

## WebSocket

### /ws/agent
Agent WebSocket 隧道。通过 query param `?token=<jwt>` 认证。

### /ws/terminal/:resource_id
终端 WebSocket 连接。

### /ws/sql/:resource_id
SQL 控制台 WebSocket 连接。

### /ws/redis/:resource_id
Redis 控制台 WebSocket 连接。

---

## 错误格式

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable message"
  }
}
```

常见错误码：
- `AUTH_INVALID` — 密码错误
- `PASSWORD_ALREADY_SET` — 密码已设置
- `INTERNAL_ERROR` — 服务器内部错误
- `NOT_FOUND` — 资源不存在
