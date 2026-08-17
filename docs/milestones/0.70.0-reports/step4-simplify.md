# 步骤4：代码精简检查 — 0.70.0 SIP 电话资源基础

> 重跑（打回步骤5 后，旧报告 `step4-simplify.md.rejected`）。对比基线：`git diff --name-only milestone-0.70.0-start`。

## 检查维度（CLAUDE.md simplify）

- 重复代码 / 过度设计 / 提前实现下一阶段能力
- 大文件拆分 / Rust workspace 结构 / `workspace = true` 依赖
- 与里程碑文档一致性

## 变更文件清单（本里程碑）

```
crates/rex-sip/        lib.rs, baresip_ua.rs, mock.rs, build.rs, Cargo.toml
crates/rex-hub/        sip_ws.rs(新增 884 行), resource_conn.rs, resource_api.rs, agent_ws.rs, rex-hub.rs, lib.rs
crates/rex-agent/      agent_ws.rs(隧道 + UA₂)
前端                   features/resource/(WizardModal/protocols), features/sip/(Dialpad/CallState/SipPage), api/sip.ts, i18n
```

## 逐项检查结论

| 文件 | 检查项 | 结论 |
|------|--------|------|
| `rex-sip/baresip_ua.rs` | 线程安全重写后（ensure_runtime 单例 / mqueue 主线程序列化 / 无递归锁 / calls map 关闭即移除）结构清晰，无重复控制路径 | 🟢 无问题 |
| `rex-sip/mock.rs` | Mock 实现确定性、职责单一，无冗余 | 🟢 无问题 |
| `rex-hub/sip_ws.rs` | 直连 UA₁（`handle_sip_session`）与链式 UA₂（`handle_agent_sip`）双路径，职责分离清晰；`map_control`/`map_event`/`dispatch_cmd` 单一职责；回归测试覆盖 #4/#7 | 🟢 无问题 |
| `rex-hub/resource_conn.rs` | `load_sip_conn` 解析分支合理（host/port 顶层回退 config），无过度设计 | 🟢 无问题 |
| `rex-agent/agent_ws.rs` | 统一数值 `AGENT_CHANNEL_SEQ` 生成所有 protocol channel_id（修复 #7 根因）；本轮已清掉 clippy `mut` warning | 🟢 无问题 |
| 前端 `features/sip/*` `WizardModal` `protocols` | 统一用设计 token（`--space-*`/`--text-*`/`--border`/`--danger` 等），无硬编码 hex（仅 `var()` fallback `#fff`/`#d29922` 符合组件库惯例）；组件拆分合理 | 🟢 无问题 |

## 发现汇总

- 🔴 必须修复：无
- 🟡 应该修复：无
- 🟢 可选改进：无（本里程碑代码已在步骤5 打回后两次精修，无遗留可精简点）

## 门禁判断

精简无 🔴/🟡 发现 → 不触发打回，勾选步骤4 继续。

（步骤4/5 审查·精简发现的 🟢 本应记入缺陷池 `docs/BUGS.md`，但本次无任何 🟢 发现，故缺陷池无新增条目。）
