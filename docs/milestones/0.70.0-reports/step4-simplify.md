# 步骤4：代码精简（0.70.0 SIP 电话资源基础）

## 范围

本次精简针对里程碑 0.70.0 的源码变更（`git diff milestone-0.70.0-start`，非测试、非 lockfile）。
变更覆盖：`crates/rex-sip/`、`crates/rex-agent/src/agent_ws.rs`、`crates/rex-hub/src/{agent_ws.rs,sip_ws.rs,resource_conn.rs,resource_api.rs,lib.rs,rex-hub.rs}`、前端 `packages/rex-console-web/src/**`。

## 四路审查结论（Reuse / Simplification / Efficiency / Altitude）

派发 4 个并行清理 agent，逐文件审查本次 diff。采纳「行为不变、仅改组织方式」的精简；架构级重构（超出 diff、需改共享隧道编解码/路由键方案）标记 SKIP，留待后续里程碑。

## 已应用的精简

### 1. `crates/rex-hub/src/sip_ws.rs` — `CallStatePayload.state` 类型收敛
- **前**：`state: String`，经 `call_state_str(state)` 手动转 snake_case 字符串（`CallState::Active` → `"active"`）。
- **后**：`state: CallState`（直接复用 `rex_sip::CallState`，其已派生 `#[serde(rename_all = "snake_case")]`）。
- **收益**：删除 `call_state_str` 函数及其调用，去掉一份与 serde 属性重复的字符串映射，单一事实来源。

### 2. `crates/rex-hub/src/sip_ws.rs` — `handle_socket` 去重 `find_online_agent` 双查
- **前**：`is_agent_resource(&state, &resource_id)` 判环境 `connection_mode=="agent"`；为真再调 `handle_agent_sip`，后者内部**再次** `find_online_agent` 查同一条 resource/env 数据取 `agent_id`。
- **后**：`handle_socket` 一次 `find_online_agent` 拿到 `agent_id`，直接传给 `handle_agent_sip(agent_id)`；删除已死函数 `is_agent_resource`。
- **收益**：消除同一份 resource→env→agent 数据的重复 DB 查询，删 ~16 行死代码。

### 3. `crates/rex-hub/src/sip_ws.rs` — 清理冗余 import
- 删除顶层冗余 `use uuid::Uuid;`（调用处均用全限定 `uuid::Uuid::new_v4()`）。
- 测试中 `use rex_sip::{MockAction, MockSipUa};` 改为 `use rex_sip::MockSipUa;`（实际使用为全限定 `rex_sip::MockAction::...`）。

### 4. `crates/rex-agent/src/agent_ws.rs` — 数值 channel_id（修复性精简，见 Bugs 表#2）
- 原 `format!("ch_{uuid}")` 因隧道二进制帧前缀要求数值 `u32` 而无法路由，导致所有非 SIP 协议回传帧被丢。将全协议改用 `AGENT_CHANNEL_SEQ` 数值 ID，删除 `uuid` 依赖与 `SIP_CHANNEL_SEQ`，新增回归测试（见步骤6）。

## 校验（行为不变证明）

```bash
cargo fmt -p rex-hub -p rex-agent -- --check      # ✅ 通过
cargo clippy -p rex-agent -p rex-hub --all-targets # ✅ 无 sip_ws/agent_ws 警告（仅有与本次无关的预存 warning）
cargo test  -p rex-sip -p rex-agent -p rex-hub    # ✅ 全部测试通过（rex-sip 4 + rex-agent + rex-hub 64 + integration 5，0 失败）
```

`ClientMsg`/`ServerMsg` 的 JSON 契约（`sip.call_state.payload.state` 仍输出 `"ringing"|"active"|"held"|"ended"`）经 `CallState` 的 `snake_case` serde 保持不变，前端 `decodeEvent` 契约未受影响。

## SKIP（架构级，超出本 diff，留待后续里程碑）

以下经架构审查认为应做，但改动波及共享隧道编解码/路由键，超出 0.70.0 范围，记录供后续：
- 共享隧道帧 codec（编解码目前 Hub/Agent 各自 inline）。
- `find_online_agent_for_resource` 抽为公共 helper（已在 agent_ws/terminal_ws/sip_ws 多处重复）。
- `dispatch_sip_control` 通用化、`bridge_tunnel` 泛型化、`u32` 键方案统一改造。
- `SipEvent::Message` 实现（属 M82b 媒体层）。

## 结论

精简未改变任何用户可见行为（信令层契约、隧道路由、前端契约均不变）。步骤4 门禁（功能不变）✅ 通过。
