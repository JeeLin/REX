# REX Hub API 文档

## 认证

所有 API 请求需要在 `Authorization` header 中携带 JWT token：

```
Authorization: Bearer <token>
```

获取 token：`POST /api/auth/login`

## 公开端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 健康检查 |
| GET | `/api/auth/check` | 检查是否需要设置密码 |
| POST | `/api/auth/login` | 登录 |
| POST | `/api/auth/password` | 首次设置密码 |

## 环境管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/environments` | 列出所有环境 |
| POST | `/api/environments` | 创建环境 |
| GET | `/api/environments/:id` | 获取环境详情 |
| PUT | `/api/environments/:id` | 更新环境 |
| DELETE | `/api/environments/:id` | 删除环境 |

## 资源管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/environments/:id/resources` | 列出环境下的资源 |
| POST | `/api/environments/:id/resources` | 创建资源 |
| PUT | `/api/environments/:id/resources/:rid` | 更新资源 |
| DELETE | `/api/environments/:id/resources/:rid` | 删除资源 |
| POST | `/api/resources/test-connection` | 测试连接 |

## Agent 管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/agents` | 列出所有 Agent |
| GET | `/api/agents/:id` | 获取 Agent 详情 |
| POST | `/api/agents/:id/reset-token` | 重置 Agent 令牌 |

## 数据库操作

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/sql/connect` | 连接数据库 |
| POST | `/api/sql/query` | 执行查询 |
| POST | `/api/sql/disconnect` | 断开连接 |

## Redis 操作

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/redis/connect` | 连接 Redis |
| POST | `/api/redis/command` | 执行命令 |
| POST | `/api/redis/disconnect` | 断开连接 |

## 文件操作

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/files/connect` | 连接 SFTP |
| POST | `/api/files/list` | 列出文件 |
| POST | `/api/files/upload` | 上传文件 |
| POST | `/api/files/download` | 下载文件 |

## 审计日志

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/audit-log` | 查询审计日志 |

## 设置

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/settings` | 获取设置 |
| PUT | `/api/settings` | 更新设置 |

## WebSocket

| 路径 | 说明 |
|------|------|
| `/ws/terminal` | SSH 终端 WebSocket |
| `/ws/tunnel` | Agent 隧道 WebSocket |
| `/ws/agent` | Agent 注册 WebSocket |

## 错误格式

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable message"
  }
}
```
