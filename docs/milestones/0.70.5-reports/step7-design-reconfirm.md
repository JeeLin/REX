# 设计再确认：0.70.5

## 审查对象

- 已实现代码（变更文件，自 `milestone-0.70.5-start` 起）：`db.rs` / `models.rs` / `resource_api.rs` / `resource_conn.rs`；前端 `resources.ts` / `types.ts` / `SipPage.vue` / `SipPage.test.ts` / `WizardModal.vue` / `__tests__/types.test.ts`
- 里程碑文档：`docs/milestones/0.70.5-sip-config-cleanup.md`

## 审查维度

`AGENTS.md` 无 `## 审查维度` 段落，按 dev-flow 约定以「产品定位硬约束 + 架构一致性 + 产品边界 + 里程碑设计核对点」为设计再确认维度。

| # | 维度 | 结论 | 说明 |
|---|------|------|------|
| 1 | 单用户/自托管定位不被破坏 | ✅ | 仅新增「资源内 SIP 多账户切换」端点，无任何多用户/RBAC/企业协作概念；切换发生在单用户已持有资源的账户集合内。 |
| 2 | 文件传输数据不经浏览器 | ✅ | 本里程碑不涉及文件传输；`set_active_account` 仅改写 `config_json.activeAccount`，无文件数据出入浏览器。 |
| 3 | Hub/Agent 版本一致模型未被改动 | ✅ | 仅 SIP 配置层重构；`load_sip_conn` 输出 `SipConfig` 形状（server/port/username/password/display_name/transport）与 Agent `parse_sip_config` 完全不变，隧道帧未动。 |
| 4 | `SipConfig` 线格式不变 | ✅ | `resource_conn.rs` L77–96：`active.server` 取自身值、空即报 `sip: missing server`，移除的仅是顶层 host 回退；对外线格式零变化。 |
| 5 | `SipProfile` 数据形状不变（仅抽类型/解析函数） | ✅ | 前端 `types.ts` 抽出 `SipProfile`/`SipAccountView`/`SipAccountForm` 与 `resolveActiveAccount`（active 或 first），与后端 `load_sip_conn` 语义一致；无 schema 迁移、无字段增删。 |
| 6 | 依赖声明 `workspace = true`，无新增依赖 | ✅ | `git diff milestone-0.70.5-start -- Cargo.toml crates/*/Cargo.toml` 为空，无任何依赖增删。 |
| 7 | 实现与里程碑详细设计一致 | ✅ | 三项子任务均落地且经步骤4/5/6 验证：① `load_sip_conn` 去顶层 host 回退（L77–96，注释同步收敛）；② 共享 `SipProfile` 类型 + `resolveActiveAccount`，`SipPage`/`WizardModal` 共用，`WizardModal` 抽 `toSipAccounts()` 消除两处重复变换；③ `set_active_account` 端点（`resource_api.rs:198`、路由 `:33`）经 `db.set_resource_active_account`（单连接、protocol 守卫、空 config 守卫、account 成员校验、审计 fire-and-forget），前端 `selectAccount` 仅调 `setActiveAccount`、移除多余 get+update。 |
| 8 | 用户可见行为不变 | ✅ | 切换生效账户的交互（下拉选择）与最终效果（activeAccount 变更、后续连接使用该账户）与 0.70.4 一致，仅减少一次 GET 往返（无可见差异）。 |
| 9 | 提交粒度 / commit message / fmt | ✅ | 步骤6 验证 `cargo fmt --check` 干净、`cargo clippy --workspace --all-targets` 0 warning、前端 type-check/lint/build 通过；commit 一子功能点一提交、英文 message（见步骤3-5 提交记录）。 |

## 汇总

- **通过维度**：9/9
- **结论**：✅ 通过

## 发现的问题

无。
