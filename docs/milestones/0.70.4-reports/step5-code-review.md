# 代码审查：0.70.4 SIP 资源按名称管理 + 多账户切换（重跑）

## 变更概览

- **变更文件**：10（相对 `milestone-0.70.4-start`，该 tag 已于 tag 清理时删除，改用基线提交 `9b97f342`）
- **审查范围**：步骤5 首次打回修复（5 个 🟡）后重跑，覆盖全部里程碑期间变更。
- **审查维度**：`AGENTS.md` 无 `## 代码审查维度`，使用 devflow-review 内置默认集（正确性 / 安全性 / 健壮性 / 可维护性 / 性能 / 规范）。

## 问题列表

| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 1 | 🟡 | `WizardModal.vue` | addSipAccount | SIP 账户 id 用长度派生（`a${len+1}`），删除中间账户（如 a2）后再新增会复用旧 id，造成重复 id / 状态错乱 |
| 2 | 🟡 | `WizardModal.vue` | submit | 提交未校验至少 1 个有效账户（仅前端 username 非空），空提交到后端才报错，体验差 |
| 3 | 🟡 | `resource_conn.rs` | load_sip_conn | 取 `active.port` 后未校验 0；账户显式传 0 会绕过 serde default 5060，进入非法配置 |
| 4 | 🟢 | `resource_conn.rs` | load_sip_conn | 顶层 `host` 回退逃生舱在模型已声明 server 下沉账户后仍保留（仅 legacy/异常 payload 触发） |
| 5 | 🟢 | `WizardModal.vue` / `SipPage.vue` / `lib.rs` | — | 共享 `SipProfile` 形状与 "active 或 first" 解析规则在三处镜像，存在类型/逻辑漂移风险 |
| 6 | 🟢 | `SipPage.vue` | selectAccount | 切换账户先 `get` 再 `update`，多一次 GET 往返（后端 `update_resource` 要求全字段，非正确性缺陷） |

## 汇总

- 🔴 必须修复：0
- 🟡 应该修复：3（已修复，见下方处置）
- 🟢 可选改进：3（记入缺陷池，不阻断）
- **结论**：3 个 🟡 均已在打回修复阶段修入代码（提交 `40864894`、`fc1ca46f`），Bugs 表对应行标记 `[x]`；🟢 入 `docs/BUGS.md`。**无 🔴/🟡 待打回项 → 步骤5 通过。**

## 处置明细

| # | 走向 | 提交 / 位置 |
|---|------|------------|
| 1 | 修复 | `fc1ca46f` 改用单调递增 `sipAccountSeq` 派生 id |
| 2 | 修复 | `fc1ca46f` submit 增加 ≥1 有效账户校验 |
| 3 | 修复 | `40864894` 增加 `if port == 0` 拒绝 |
| 4 | 🟢 缺陷池 | 后端行为变更属范围外，留待后续 |
| 5 | 🟢 缺陷池 | 跨组件抽出共享 TS 类型属范围外重构 |
| 6 | 🟢 缺陷池 | 引入专用 `set_active_account` 端点后消除，留待后续 |
