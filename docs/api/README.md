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
| GET | `/api/agents/download` | 下载 Agent 二进制 |

## 认证（需登录）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/auth/change-password` | 修改密码 |
| GET | `/api/system-info` | 获取系统信息（os / arch / hostname） |

## 自更新

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/update/check` | 检查更新 |
| POST | `/api/update/trigger` | 触发更新 |
| GET | `/api/update/status` | 查询更新状态 |
| POST | `/api/update/rollback` | 回滚更新 |

## 环境管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/environments` | 列出所有环境 |
| POST | `/api/environments` | 创建环境 |
| GET | `/api/environments/export` | 导出环境 |
| POST | `/api/environments/import` | 导入环境 |
| GET | `/api/environments/topology` | 获取拓扑 |
| GET | `/api/environments/:id` | 获取环境详情 |
| PUT | `/api/environments/:id` | 更新环境 |
| DELETE | `/api/environments/:id` | 删除环境 |

## 资源管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/environments/:id/resources` | 列出环境下的资源 |
| POST | `/api/environments/:id/resources` | 创建资源 |
| GET | `/api/environments/:id/resources/:rid` | 获取资源详情 |
| PUT | `/api/environments/:id/resources/:rid` | 更新资源 |
| DELETE | `/api/environments/:id/resources/:rid` | 删除资源 |
| POST | `/api/environments/:id/resources/:rid/active-account` | 切换活跃账号 |
| POST | `/api/resources/test-connection` | 测试连接 |

## Agent 管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/environments/:id/agents` | 列出环境下的 Agent |
| GET | `/api/agents/:id` | 获取 Agent 详情 |
| POST | `/api/agents/:id/reset-token` | 重置 Agent 令牌 |

Agent 的注册、心跳、状态更新通过 `/ws/agent` WebSocket 处理。

## 数据库操作（SQL）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/sql/connect` | 连接数据库 |
| POST | `/api/sql/disconnect` | 断开连接 |
| POST | `/api/sql/query` | 执行查询 |
| GET | `/api/sql/databases` | 列出数据库 |
| GET | `/api/sql/tables` | 列出表 |
| GET | `/api/sql/columns` | 列出列 |
| GET | `/api/sql/indexes` | 列出索引 |
| GET | `/api/sql/foreign_keys` | 列出外键 |
| GET | `/api/sql/ddl` | 获取 DDL |
| GET | `/api/sql/saved-queries` | 查询保存的 SQL |
| POST | `/api/sql/saved-queries` | 新增/更新保存的 SQL |
| DELETE | `/api/sql/saved-queries/:id` | 删除保存的 SQL |
| POST | `/api/sql/compare` | 数据对比 |

## MongoDB 操作

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/mongodb/connect` | 连接 MongoDB |
| POST | `/api/mongodb/disconnect` | 断开连接 |
| GET | `/api/mongodb/databases` | 列出数据库 |
| GET | `/api/mongodb/collections` | 列出集合 |
| POST | `/api/mongodb/query` | 执行查询 |

## Redis 操作

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/redis/connect` | 连接 Redis |
| POST | `/api/redis/disconnect` | 断开连接 |
| GET | `/api/redis/databases` | 列出数据库 |
| POST | `/api/redis/select` | 切换数据库 |
| GET | `/api/redis/scan` | 扫描键 |
| GET | `/api/redis/key` | 获取键值 |
| POST | `/api/redis/set` | 设置键值 |
| POST | `/api/redis/del` | 删除键 |
| GET | `/api/redis/ttl` | 获取 TTL |
| POST | `/api/redis/set-ttl` | 设置 TTL |
| GET | `/api/redis/info` | 获取 INFO |
| POST | `/api/redis/command` | 执行命令 |
| POST | `/api/redis/pubsub/poll` | 轮询 Pub/Sub |

## 文件操作

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/files/connect` | 连接 SFTP/对象存储 |
| POST | `/api/files/disconnect` | 断开连接 |
| GET | `/api/files/list` | 列出文件 |
| GET | `/api/files/stat` | 获取文件信息 |
| POST | `/api/files/upload` | 上传文件 |
| GET | `/api/files/download` | 下载文件 |
| POST | `/api/files/delete` | 删除文件 |
| POST | `/api/files/rename` | 重命名/移动 |
| POST | `/api/files/mkdir` | 创建目录 |
| POST | `/api/files/presigned-url` | 获取预签名 URL |
| GET | `/api/files/s3/multipart-uploads` | 列出 S3 分片上传 |
| POST | `/api/files/s3/resume-upload` | 恢复分片上传 |
| POST | `/api/files/s3/abort-upload` | 取消分片上传 |
| GET | `/api/files/acl` | 获取 ACL |
| PUT | `/api/files/acl` | 设置 ACL |
| GET | `/api/files/read-for-edit` | 读取文件内容（编辑） |
| POST | `/api/files/save-from-edit` | 保存编辑内容 |

## 仪表盘

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/dashboard/stats` | 汇总统计 |
| GET | `/api/dashboard/recent` | 最近活动 |

## SIP

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/sip/cdr` | 查询 CDR |
| GET | `/api/sip/cdr/:id` | 获取 CDR 详情 |
| POST | `/api/sip/capture/:id/start` | 开始抓包 |
| POST | `/api/sip/capture/:id/stop` | 停止抓包 |
| GET | `/api/sip/capture/:id/packets` | 查询抓包包列表 |
| GET | `/api/sip/capture/:id/pcap` | 导出 pcap |
| POST | `/api/sip/recording/:id/start` | 开始录音 |
| POST | `/api/sip/recording/:id/stop` | 停止录音 |
| GET | `/api/sip/recording/:id` | 获取录音详情 |

## 审计日志

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/audit-log` | 查询审计日志 |
| GET | `/api/audit-log/stats` | 审计统计 |
| GET | `/api/audit-log/security-report` | 安全报告 |

## 设置

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/settings` | 获取设置 |
| PUT | `/api/settings` | 更新设置 |

## WebSocket

| 路径 | 说明 |
|------|------|
| `/ws/terminal` | SSH 终端 WebSocket |
| `/ws/sip` | SIP 事件 WebSocket |
| `/ws/tunnel` | Agent 隧道 WebSocket |
| `/ws/agent` | Agent 注册 WebSocket（Agent token 认证） |

## 错误格式

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable message"
  }
}
```
