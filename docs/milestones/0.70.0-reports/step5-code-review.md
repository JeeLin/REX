# 代码审查：0.70.0 SIP 电话资源基础

> 重跑（打回后，旧报告已移除）。审查对象：`git diff --name-only milestone-0.70.0-start`。
> AGENTS.md 无 `## 代码审查维度`，采用 devflow-review 内置默认维度集（正确性 / 安全性 / 健壮性 / 可维护性 / 性能 / 规范）。

## 变更概览

- **变更文件**：42（含新 crate `rex-sip`、Hub/Agent 隧道、前端 sip 模块；排除 docs/Cargo.lock）
- **审查时间**：2026-08-17

## 问题列表

| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| — | — | — | — | 无 🔴/🟡 发现 |

### 逐项维度结论

| # | 维度 | 结论 | 说明 |
|---|------|------|------|
| 1 | 正确性 | ✅ | 5 个 🔴（#1 password 未传 / #2 递归死锁 / #3 重复 init / #4 UAF / #5 跨线程竞争）已在步骤3 修复并验证；信令帧隧道前缀剥除（#7）与数值 channel_id（#7 根因）逻辑正确；前后端消息模型对齐（ClientMsg/ServerMsg ↔ SipClientMsg/SipServerEvent） |
| 2 | 安全性 | ✅ | SIP password 经 crypto 加密存储 + 前端仅在 `buildConfig` 序列化（不再丢弃）；`/ws/sip` 复用 terminal_ws 的 JWT query token 鉴权；token 在 URL 中 `encodeURIComponent`（api/sip.ts）；无 SQL 注入（全参数化/结构体）；无敏感信息泄露 |
| 3 | 健壮性 | ✅ | baresip 主循环单例 + mqueue 主线程序列化消除跨线程竞争；calls map 在 CALL_CLOSED 立即移除避免 UAF；隧道读写均带超时/关闭分支；前端 WebSocket onclose/onerror 清理心跳 |
| 4 | 可维护性 | ✅ | 直连/链式双路径职责分离（`handle_sip_session` / `handle_agent_sip`）；`map_control`/`map_event`/`dispatch_cmd` 单一职责；Mock `SipUa` 便于测试；前端组件按功能域拆分 |
| 5 | 性能 | ✅ | 无 N+1 / 无不必要的循环或查询；keepalive 25s 合理；mpsc channel 容量适中 |
| 6 | 规范 | ✅ | 遵循 workspace `workspace = true` 依赖；提交信息 `#1`-`#7` 粒度清晰；前端用设计 token（`--space-*`/`--text-*`/`--border`/`--danger`/`--accent`），无硬编码 hex（仅 `var()` fallback `#fff`/`#d29922` 符合组件库惯例）；协议色 `#2DD4BF` 青色与既有 7 色区分，符合设计核对点 |

## 汇总

- 🔴 必须修复：0
- 🟡 应该修复：0
- 🟢 可选改进：0
- **结论**：通过（🔴 0 + 🟡 0，无 🔴/🟡 发现，不触发打回）

（步骤5 审查发现的 🟢 本应记入缺陷池 `docs/BUGS.md`，但本次无 🟢 发现，缺陷池无新增条目。）
