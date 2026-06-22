# Step 2: 设计核对报告 — 0.9.0 Docker Management

## 审查维度

### 产品定位

| 检查项 | 结果 | 说明 |
|--------|------|------|
| 单用户、自托管 | ✅ | 无多用户/RBAC 概念引入 |
| 深色优先 | ✅ | 前端组件遵循深色主题 |
| 文件数据不经过浏览器 | ✅ | Docker API 通过后端直连，前端仅发送命令 |

### 架构一致性

| 检查项 | 结果 | 说明 |
|--------|------|------|
| 单二进制 + supervisor + worker | ✅ | Docker 功能集成到 rex-hub，不改变进程模型 |
| Docker crate 遵循 stub 模式 | ✅ | 与 rex-redis/rex-mysql 一致，实际连接通过 Agent 代理隧道 |
| DockerConnector trait object safety | ✅ | `#[async_trait]` + `Send + Sync` |
| workspace 依赖规则 | ✅ | `reqwest` 在根 Cargo.toml 已声明，子 crate 用 `workspace = true` |

### 产品边界

| 检查项 | 结果 | 说明 |
|--------|------|------|
| Docker 协议覆盖 | ✅ | PRODUCT.md 定义 Docker 为"容器管理"，本里程碑实现容器列表/启动/停止/日志 |
| 不引入 Compose/Swarm | ✅ | 明确标注"不做什么" |
| 不引入镜像管理 | ✅ | 仅容器操作 |
| 保持单实例模式 | ✅ | 不支持 Swarm 集群 |

### Docker Engine API 正确性

| 端点 | 里程碑设计 | 正确性 |
|------|-----------|--------|
| `GET /containers/json` | `list_containers()` | ✅ 正确 |
| `GET /containers/{id}/json` | `inspect_container()` | ✅ 正确 |
| `POST /containers/{id}/start` | `start_container()` | ✅ 正确 |
| `POST /containers/{id}/stop` | `stop_container()` | ✅ 正确 |
| `POST /containers/{id}/restart` | `restart_container()` | ✅ 正确 |
| `DELETE /containers/{id}` | `remove_container()` | ✅ 正确 |
| `GET /containers/{id}/logs` | `logs()` | ✅ 正确 |
| `GET /info` | `info()` | ✅ 正确 |

### 与现有协议一致性

| 检查项 | 结果 | 说明 |
|--------|------|------|
| connector.rs 结构 | ✅ | DockerConfig + DockerConnector trait + stub |
| ws_xxx.rs 结构 | ✅ | ClientMsg/ServerMsg 枚举 + WebSocket handler |
| 前端组件结构 | ✅ | features/docker/ 目录，与 features/redis/ 一致 |
| useSession composable | ✅ | useDockerSession 返回 connected/connect/disconnect 等 |
| 资源创建向导 | ✅ | ResourceNew.vue 已有 Docker 协议选项（line 196），需添加表单 |

### WebSocket 消息协议

| 检查项 | 结果 | 说明 |
|--------|------|------|
| 消息类型清晰 | ✅ | command/response/error/connected/disconnected/pong |
| 请求-响应模式 | ✅ | 前端发 command（带 id），Hub 返回 response（带 id） |
| 与 Redis 协议风格一致 | ✅ | 相同的 type/data/message 模式 |

### 前端交互设计

| 检查项 | 结果 | 说明 |
|--------|------|------|
| 容器列表展示 | ✅ | 表格形式，显示名称/镜像/状态/操作 |
| 状态颜色区分 | ✅ | 🟢 Running / 🟡 Paused / ⚫ Stopped |
| 搜索和过滤 | ✅ | 搜索框 + 包含已停止复选框 |
| 右键菜单 | ✅ | 启动/停止/重启/删除/日志/inspect |
| 日志查看 | ✅ | 终端风格，tail 行数限制 |

## 设计核对点

| 检查项 | 结果 | 说明 |
|--------|------|------|
| Docker crate 遵循 stub 模式 | ✅ | 与 rex-redis 一致 |
| Docker Engine API 覆盖满足需求 | ✅ | 8 个端点覆盖容器基本操作 |
| WebSocket 消息协议清晰 | ✅ | 6 种消息类型，请求-响应模式 |
| 前端交互直观 | ✅ | 容器列表+操作+日志 |
| Docker 连接表单支持 Unix/TCP | ✅ | 两种模式切换 |
| 保持单用户、自托管定位 | ✅ | 无多用户/RBAC |

## 小问题（可直接修正）

1. **DockerConnector trait 缺少 `from_json()` 和 `into_config()`**：rex-redis 的 `RedisConnectorImpl` 有这两个方法用于从 JSON 加载配置。DockerConnector 也应提供。

2. **`info()` 返回 `serde_json::Value`**：建议改为返回 `HashMap<String, String>`，与 `RedisConnector::info()` 一致，避免前端处理嵌套 JSON。

## 结论

设计与 PRODUCT.md 一致，架构遵循现有 stub 模式，Docker Engine API 端点正确，前端交互设计合理。2 个小问题可直接修正。

**结论：✅ 通过**
