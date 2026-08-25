# v0.70.6 步骤5：代码审查

> 触发：步骤4 完成，Flow Status 步骤5 未勾选。
> 框架：devflow-review（🔴 必须修复 / 🟡 应该修复 / 🟢 可选改进）。
> 对象：`git diff --name-only milestone-v0.70.6-start` 的变更文件。
> 约束：本会话无法运行 `cargo build`/`clippy`（baresip 重链接超时会话超时），以下为**静态**审查；编译验证为步骤6 用户本地门禁。

## 维度与结论

### 1. 正确性
- **Hub↔Agent 帧契约对齐（✅）**：Hub `agent_session_request` 下发**内层 `SessionRequest`**（`serde_json::to_vec`），Agent 侧 `handle_connect_*` 以 `serde_json::from_slice::<SessionRequest>` 解析——两侧一致。二进制帧 `[4B u32 channelId BE][json]` 与 Agent `agent_ws.rs` 读循环（strip 4B → `data_tx`）对齐。
- **响应路由键一致（✅）**：Hub 以 `format!("{channel_id}:{seq}")` 登记 `session_responses`，Agent 在 `SessionResponse` 回显同一 `seq`；`session_seq` 为 `AtomicU64`，并发安全。
- **connect 握手对齐（✅）**：Hub 下发既有 `HubMsg::Connect`（`type:"connect"`），Agent `handle_connect` 按 `req.protocol` 分支；Agent 回 `AgentSessionMsg::SessionOpened{request_id, channel_id}`，Hub `handle_session_msg` 据此完成 `pending_requests` 握手并登记 `channels[channel_id]=agent_id`。三套协议（sql/redis/file）握手路径统一。

### 2. 安全性 / 凭据信任边界
- **明文凭据经加密 WS（✅）**：config（含 `password`/`private_key`）随既有 `HubMsg::Connect` 经 TLS WebSocket 下发 Agent；Agent 在私网内使用，Hub 侧不再持有已终结会话明文。符合里程碑「凭据信任边界收敛」目标与 AGENTS.md。
- **文档漂移（🟢）**：里程碑子任务7 详细设计写「`connect` 消息增加 `trust_boundary` 标注」，实现未新增该字段（依赖既有 TLS WS 即满足语义）。属文档描述比实现多一步，无功能缺失；建议后续在 schema 补注释或更新文档，而非改代码。

### 3. 架构一致性
- **直连路径零改动（✅）**：Hub 三处 connect 仅插入 `if res.use_agent { early-return 代理 }`，直连分支完全不变；`load_resource_config` 仅在 `ResourceConnInfo` 上额外解析 `use_agent`/`agent_id`，对直连资源恒为 `(false, None)`。
- **前端透明（✅）**：连接池里存的是实现了同一 trait 的代理（`AgentSqlProxy` 等），前端 REST 接口与直连模式字节一致。
- **SIP 不受影响（✅）**：`resolve_agent_for_resource` 对 sip 资源也会解析 `use_agent`，但 `sip_*` API 不读取该字段，无行为变化（sip 沿用 M82 自有隧道逻辑）。

### 4. 测试覆盖 / 错误处理
- **结构化错误回传（✅）**：Agent 侧 `dispatch_*` 失败以 `SessionResponse{error:Some}` 或 `SessionError` 回传；Hub 代理 `relay` 将 `error` 转为 `anyhow::bail`，前端可见。
- **S3 特定端点降级（🟢 已知限制）**：`file_api` 的 `presigned_url`/`list_multipart_uploads` 通过 `as_any().downcast_ref::<rex_s3::S3Connector>()` 取 S3 专用方法；Agent 文件代理 `as_any` 返回自身，降级为返回「only supported for S3」。agent 模式下这两个 S3 高级端点暂不可用，基础 list/stat/upload/download 经隧道可用。属已知边界，已在里程碑文档「文件数据不经浏览器」范围内。

### 5. 编译验证（待用户本地步骤6）
- **最高风险项（🟡 待验证）**：子任务7 hub 网关为盲写（无法在本会话编译）。潜在编译点：`relay<T: DeserializeOwned>` 约束、`decode_bytes` 签名、`ProgressCallback` 路径导入、`state.0.clone()` 取 `AppState`、各 trait 方法签名与 `rex_common` 对齐。需在用户环境 `cargo build -p rex-hub -p rex-agent` 逐一确认。
- 子任务3/4/5/6 的 agent 模块此前亦未在本会话编译（仅子任务3 曾出过 13:47 二进制，早于 4-7）。

## 门禁判断
- 无 🔴 逻辑缺陷。
- 🟡 仅「hub 网关编译验证」一项，且根因是本环境构建超时（非代码逻辑缺陷），归属步骤6 用户本地门禁。
- 🟢 两项文档/边界观察，低风险。

**结论：步骤5 静态审查门禁通过（无 🔴 逻辑缺陷；🟡 编译验证与 🟢 观察均归用户本地 build/verify 阶段）。** 不触发打回循环（本环境无法执行编译门禁的 4→7 重跑）。

## 打回记录
无。
