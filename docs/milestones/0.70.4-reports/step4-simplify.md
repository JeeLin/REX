# 步骤4 代码精简报告 — 0.70.4 SIP 资源按名称管理 + 多账户切换

审查范围：`git diff --name-only milestone-0.70.4-start`（14 个文件）。
方法：4 个并行清理 agent（reuse / simplification / efficiency / altitude），仅记录发现，分级后按门禁判断。

## 发现与处置

| # | 维度 | 位置 | 发现 | 严重度 | 处置 |
|---|------|------|------|--------|------|
| 1 | reuse | `rex-sip/src/lib.rs` + `resource_conn.rs:102` | SIP 默认端口 `5060` 在两处重复（私有 `default_sip_port()` 与 `load_sip_conn` 的 `unwrap_or(5060)`） | 🟢 | **已修**：提升为 `pub const DEFAULT_SIP_PORT` 并在 `resource_conn.rs` 引用 |
| 2 | simplification | `SipPage.test.ts` fixture | 测试 fixture 带顶层 `server` 字段，与新 per-account schema 不符（误导数据） | 🟢 | **已修**：移除顶层 `server`，每个账户自带 `server` |
| 3 | simplification/altitude | `WizardModal.vue` `buildConfig` | 账户按 `username` 过滤后，`activeAccount` 仍无条件指向原始 id，可能指向被过滤掉的账户（与解析层 `accounts[0]` 回退不一致） | 🟡 | **已修**：过滤后若 `activeAccount` 不在保留账户中则回退首账户 |
| 4 | efficiency/simplification/altitude | `SipPage.vue` `selectAccount` | 每次切换账户前重新 `resourcesApi.get` 全量资源 | 🟢 | **跳过**：后端 `db.update_resource` 对 `name`/`protocol`/`host` 直接写入（无 `unwrap_or(existing)`），部分更新会覆盖其他字段，GET 是必需的。修此需新增 PATCH 端点，超出本 diff 范围，留待后续 |
| 5 | efficiency | `resource_api.rs` test-connection | 先 `from_str::<Value>` 再 `load_sip_conn` 内 `from_value(cfg.clone())` 二次解析+深拷贝 | 🟢 | **跳过**：非热路径，影响极小 |
| 6 | altitude | `resource_conn.rs` port 回退 | `active.port != 0` → `info.port.unwrap_or(...)` 被指为死代码（port 默认 5060 永不为 0） | 🟢 | **跳过**：`"port": 0` 是「使用 info.port/默认」的显式哨兵，语义与文档一致，非死代码 |
| 7 | reuse | `api/sip.ts` | 前端 `SipAccountForm` / `SipAccountView` 两套类型 + 手写 `parseSipProfile` 可抽共享 | 🟢 | **跳过**：跨组件重构，超出本 diff 范围，留待后续统一 |

## 门禁判断

无 🔴、无 🔴/🟡 待打回项。已落地修复 3 项（#1/#2/#3），其余 4 项判为 🟢 可选改进（#4 因后端约束须保留 GET，#5/#6/#7 为范围外/语义正确）。

**结论：步骤4 通过（无 🔴/🟡）。**
