# REX Hub — API 端点参考

## 认证

所有 `/api/*` 端点（除 `/api/auth/*`）需要 `Authorization: Bearer <token>` 头。

### POST /api/auth/login
登录获取 JWT Token。

**请求体：**
```json
{ "password": "string" }
```

> 单用户模型：无用户名，仅密码。首次访问需经 `POST /api/auth/password` 设置密码（`requires_setup=true` 时）。

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

## 资源连接（控制台）

### GET /api/environments/:id/resources/:resource_id/files/**
文件管理（SFTP/S3），见 `file_api`：`/connect` `/list` `/stat` `/mkdir` `/rename` `/delete` `/upload` `/download` `/acl`。

### POST /api/sql/connect
连接 SQL 数据库（mysql/postgresql/sqlite），返回会话 id；后续 `/query` `/tables` `/columns` `/indexes` `/foreign_keys` `/ddl` `/databases` `/disconnect` 均带该 id。

### POST /api/redis/connect
连接 Redis，返回会话 id；后续 `/scan` `/key` `/get` `/set` `/del` `/ttl` `/info` `/databases` `/command` `/select` `/disconnect` 均带该 id。

### POST /api/dashboard/**
仪表盘统计（环境数 / 资源数 / Agent 在线数 / 今日操作数）。

### /api/settings/**
系统设置读写（主题、语言、终端、安全、更新开关等；保存的 SQL 查询列表也存于此）。

---

## SIP 电话

### /api/sip/cdr/**
通话记录（CDR）：`GET /` 列表、`GET /:id` 详情。

### /api/sip/recording/:id
通话录音：`/start` `/stop` `/:id`（回放/下载）。

### /api/sip/capture/:id
信令抓包（pcap）：`/start` `/stop` `/packets` `/pcap` 导出。

---

## WebSocket

### /ws/agent
Agent WebSocket 隧道。通过 query param `?token=<jwt>` 认证。

### /ws/terminal/:resource_id
终端 WebSocket 连接。

### /ws/sip
SIP 控制/媒体 WebSocket 连接。

### /ws/tunnel
Agent 链式隧道帧转发（媒体/信令经 `kind` 字节多路复用）。

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
