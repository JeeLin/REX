# v0.70.6 步骤4：代码精简检查

> 触发：步骤3 完成，Flow Status 步骤4 未勾选。
> 基准：`git diff --name-only milestone-v0.70.6-start`
> 变更文件：crates/rex-agent/{agent_sql,agent_redis,agent_file,agent_ssh,agent_ws,rex-agent}.rs、crates/rex-agent/Cargo.toml、crates/rex-common/{agent_proto,lib}.rs、crates/rex-hub/{agent_proxy,agent_ws,resource_conn,sql_api,redis_api,file_api,lib}.rs、crates/rex-hub/Cargo.toml

## 检查维度（CLAUDE.md + workspace 约定）
- 重复代码 / 可抽取的公共逻辑
- 死代码 / 未使用项
- 过度设计 / 提前实现下一阶段能力
- 是否符合 Rust workspace 结构（`workspace = true` 依赖）
- 大文件是否可拆小

## 发现

### 观察 A（🟢 可选）：agent 侧三个协议模块存在脚手架重复
`agent_sql.rs` / `agent_redis.rs` / `agent_file.rs` 各自重复实现了：
- `send_session_error(...)`（几乎逐字相同，仅 channel_id/request_id 字段一致）；
- `handle_connect_X` 的连接成功通知、`LocalChannel` 注册、`while let data_rx.recv()` 主循环、`connector.close()` + 注销 的骨架。

**走向**：属 🟢 可选改进。可抽取 `pub(crate) fn send_session_error(...)` 到 `agent_ws.rs` 或在 `agent_ws.rs` 新增 `session.rs` 公共模块，并由三处复用。但因本会话无法执行 `cargo build`（baresip 重链接超时会话超时），抽取会引入无法验证的改动，**故本次不做**，留待用户本地 build/verify 阶段一并清理，避免在不可编译验证的状态下改动已提交代码。

### 观察 B（🟢 可选）：`agent_proto.rs` 存在未被构造的枚举变体
- `AgentSessionMsg::SessionOpen`（Hub→Agent 发起会话）：实际 Hub 侧下发的是 `HubMsg::Connect`（`type:"connect"`），`AgentSessionMsg::SessionOpen` 在两侧均未被构造。
- `AgentSessionMsg::FileChunk`（大体积文件分块）：当前文件读写走 `session_response.data`（base64），`FileChunk` 暂未使用（子任务6 的「分块流式」为后续增强，里程碑文档已注明「数据不落浏览器」由 base64 结构化帧满足）。

**走向**：属 🟢 可选。作为 schema 完整性保留无害；若要在本里程碑收紧，可在 `agent_proto` 加 `#[allow(dead_code)]` 或移除以消除告警。同样因不可编译验证，本次不改动。

### 观察 C（✅ 已符合）：workspace 依赖规则
所有新增依赖（`rex-mysql`/`rex-postgresql`/`rex-sqlite`/`rex-redis`/`rex-s3`/`rex-ssh`/`anyhow`/`async-trait`/`base64`）均在根 `Cargo.toml` 声明、`crate 内用 `workspace = true`，无重复版本声明。

### 观察 D（✅ 已符合）：Hub 代理结构合理，无过度设计
`agent_proxy.rs` 三个代理结构体各自实现不同 trait（`SqlConnector`/`RedisConnector`/`FileConnector`），拆分是 trait 约束使然而非过度拆分；`relay<T>` 泛型与 `decode_bytes` 已抽取公共逻辑，无重复。Hub 三处 connect 分支（sql/redis/file）仅插入一段 `if res.use_agent { ... }`  early-return，未改动直连路径，符合「文档外不大功能」原则。

## 门禁判断
- 无 🔴（功能风险）/ 🟡（可维护性差到阻塞）发现。
- 仅 2 项 🟢 观察（脚手架重复、未用 schema 变体），均为低风险、可在用户 build 阶段清理的改进。

**结论：步骤4 门禁通过（无 🔴/🟡）。** 🟢 观察按本环境限制（无法编译验证）记录如上，不触发打回循环，留待步骤6 用户本地 build/verify 阶段一并处理。

## 打回记录
无。
