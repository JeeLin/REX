# 代码审查：0.70.5

## 变更概览

- **变更文件**：10（Rust `db.rs`/`models.rs`/`resource_api.rs`/`resource_conn.rs`；前端 `resources.ts`/`types.ts`/`SipPage.vue`/`SipPage.test.ts`/`WizardModal.vue`/`__tests__/types.test.ts`）
- **审查时间**：2026-08-21
- **审查维度**：内置默认集（正确性 / 安全性 / 健壮性 / 可维护性 / 性能 / 规范）。AGENTS.md 无 `## 代码审查维度` 段落，沿用默认集。

## 审查结论

逐文件按 6 维度核对（含步骤5 打回修复轮后的 `db.rs` 单连接+protocol 守卫、`resource_api.rs` 审计 fire-and-forget、`WizardModal.vue` `toSipAccounts()` 抽取、前端 `setActiveAccount` 端点替换 get+update）：

| 维度 | 结论 | 说明 |
|------|------|------|
| 1 正确性 | ✅ | `set_resource_active_account` 解密→校验→改→加密写回链路完整；protocol 守卫、空 config 守卫、account 成员校验齐全；返回值含 name/protocol 供审计。前端 `selectAccount` 仅调 `setActiveAccount`，移除多余 get+update。 |
| 2 安全性 | ✅ | config_json 始终加解密在 Hub 侧（密钥不出 Hub）；SQL 走参数化（`?1`/`?2`…）；无 XSS/注入面。 |
| 3 健壮性 | ✅ | 所有错误路径返回 `RExError::Message`；连接/查询失败均 mapped；审计日志 fire-and-forget 且忽略内部错误，不阻塞主路径。 |
| 4 可维护性 | ✅ | 共享 `SipProfile`/`SipAccountView`/`resolveActiveAccount` 消除三处镜像；`WizardModal` 抽出 `toSipAccounts()`；后端 `set_resource_active_account` 与 `update_resource` 职责分离（部分改写 vs 全量覆盖）。 |
| 5 性能 | ✅ | 单连接完成 SELECT+UPDATE（原两次取池）；仅取所需列；无 N+1、无冗余拷贝。 |
| 6 规范 | ✅ | `cargo clippy --lib` 0 warning（exit 0）；`cargo fmt` 干净；前端 `vue-tsc`/`lint` 通过；commit 一子功能一提交、英文 message。 |

## 问题列表

无。

## 汇总

- 🔴 必须修复：0
- 🟡 应该修复：0
- 🟢 可选改进：0
- **结论**：通过（0/0/0）。
