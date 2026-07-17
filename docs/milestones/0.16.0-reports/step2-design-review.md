# Step 2: 设计核对

## 审查范围

M15 Agent WebSocket 隧道里程碑文档 vs PRODUCT.md + 架构文档。

## 审查维度

### 1. 产品定位一致性

| 检查项 | 结果 | 说明 |
|--------|------|------|
| 单用户 | ✅ | 无多用户/RBAC 概念 |
| 自托管 | ✅ | Agent 部署在用户自己的内网 |
| 内网穿透 | ✅ | 核心功能：Agent 代理内网资源连接 |

### 2. 功能边界一致性

| PRODUCT.md 描述 | M15 覆盖 | 说明 |
|-----------------|----------|------|
| Agent 注册令牌 | ✅ | 通过 WebSocket auth 消息携带 token |
| Agent 状态 online/offline | ✅ | WebSocket 连接 = online，断开 = offline |
| Agent 元信息（版本/OS/架构/主机名） | ✅ | heartbeat 消息携带 |
| Agent 代理内网资源 | ✅ | connect + channel 二进制帧转发 |
| WebSocket 加密隧道 | ✅ | 协议设计支持 TLS（wss://） |
| Agent 管理页面 | ✅ | 复用现有 agents 页面，从 DB 读取状态 |

### 3. 架构一致性

| 检查项 | 结果 | 说明 |
|--------|------|------|
| 连接通道架构 | ✅ | 符合 `docs/architecture/connection-channels.md` 设计 |
| Hub/Agent 进程模型 | ✅ | 单二进制 + supervisor + worker |
| 共享 crate | ✅ | Agent 复用 rex-ssh/rex-mysql 等协议 crate |
| 版本兼容 | ✅ | Hub 和 Agent 版本一致 |

### 4. 子任务拆分

| 检查项 | 结果 | 说明 |
|--------|------|------|
| 粒度合理 | ✅ | 5 个子任务，每个 1-2 commit |
| 前后端同步 | ✅ | 子任务 1-2 覆盖后端，子任务 3 覆盖前端 |
| 依赖关系清晰 | ✅ | 1→2→3 顺序执行，4-5 可并行 |

### 5. 协议设计

| 检查项 | 结果 | 说明 |
|--------|------|------|
| 文本帧/二进制帧分离 | ✅ | 控制消息 JSON，数据转发二进制（避免 base64 开销） |
| channel 多路复用 | ✅ | 单连接支持多个并发资源连接 |
| 心跳机制 | ✅ | 30 秒间隔，含设备信息 |
| 断线处理 | ✅ | offline 标记 + channel 关闭 + 前端通知 |

### 6. 无 REST API 设计

| 检查项 | 结果 | 说明 |
|--------|------|------|
| 注册走 WebSocket | ✅ | auth 消息替代 POST /register |
| 心跳走 WebSocket | ✅ | heartbeat 消息替代 POST /heartbeat |
| 前端查询 Agent 状态 | ✅ | 从 DB 读取（WebSocket 事件更新），不需要 Agent 主动 REST 调用 |

## 发现

| # | 严重度 | 说明 | 建议 |
|---|--------|------|------|
| 1 | 🟡 | `agent_api.rs` 保留只读查询但子任务1说"移除 REST 端点"，描述不一致 | 明确：保留 GET 端点供前端查询，移除 POST 端点（register/heartbeat/reset-token） |
| 2 | 🟢 | tunnel 前端路由设计中 `/ws/tunnel/:tunnelId` 需要 Hub 侧实现 tunnel 代理 | 子任务1已包含，无需额外子任务 |

## 结论

✅ 设计合理，与 PRODUCT.md 和架构文档一致。1 个小问题（发现 #1）属措辞不一致，已直接修正。
