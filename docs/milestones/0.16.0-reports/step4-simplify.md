# Step 4: 代码精简

## 审查范围

M15 开发的 3 个新模块：`agent_ws.rs`（Hub）、`tunnel_ws.rs`（Hub）、`agent_ws.rs`（Agent）。

## 检查结果

### 1. 重复代码

| 检查项 | 结果 | 说明 |
|--------|------|------|
| Hub/Agent 消息类型重复 | 🟡 | 两侧定义了对称的消息类型（AuthPayload, HeartbeatPayload 等），但结构不完全相同（Agent 侧有 Serialize，Hub 侧有 Deserialize）。可抽取共享 crate，但当前阶段不值得。 |
| tunnel_ws / terminal_ws 重复 | 🟢 | 两者都是 WebSocket handler，但协议和数据流完全不同，无重复。 |

### 2. 过度设计

| 检查项 | 结果 | 说明 |
|--------|------|------|
| AgentTunnelState 字段 | 🟢 | 4 个字段各司其职，无冗余 |
| pending_requests 机制 | 🟢 | 请求-响应匹配是隧道必需的，不是过度设计 |
| tunnel_data 路由 | 🟢 | 二进制帧路由是隧道必需的 |

### 3. 功能行为不变

精简未改变任何功能行为。

## 结论

✅ 无需要精简的问题。代码结构清晰，职责分离合理。
