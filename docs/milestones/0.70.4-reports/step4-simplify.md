# 步骤4 代码精简报告 — 0.70.4 SIP 资源按名称管理 + 多账户切换（重跑）

审查范围：`git diff --name-only 9b97f342`（10 个文件；`milestone-0.70.4-start` tag 已在 tag 清理时删除，改用目标提交 `9b97f342`）。
方法：4 个并行清理 agent（reuse / simplification / efficiency / altitude），仅记录发现，分级后按门禁判断。本轮为步骤5 打回修复后的重跑。

## 发现与处置

| # | 维度 | 位置 | 发现 | 严重度 | 处置 |
|---|------|------|------|--------|------|
| 1 | simplification/altitude | `WizardModal.vue` `sipHost` ref + `buildConfig` 调用顺序 | `sipHost` 是冗余的可派生状态，仅在 `buildConfig()` 副作用中刷新；`submit()` 必须先调 `buildConfig` 再读 `resourceHost()`，否则 host 过期；`reset()` 不清空导致取消后残留上一生效 server | 🟡 | **已修**：移除 `sipHost` ref，`resourceHost()` 直接由账户列表派生生效账户 server（提交 `c3214c4a`） |
| 2 | efficiency | `resource_conn.rs:77` `serde_json::from_value(cfg.clone())` | 每次连接/测试克隆整个 JsonValue | 🟢 | **跳过**：非热路径，改为 by-value 需改 `load_sip_conn` 签名并波及调用方，超出精简范围 |
| 3 | efficiency | `resource_api.rs` test_connection 先 `from_str::<Value>` 再 `load_sip_conn` 内 `from_value` 二次解析 | 同一负载两次反序列化 | 🟢 | **跳过**：非热路径，与 #2 同源 |
| 4 | efficiency | `SipPage.vue` `selectAccount` 每次切换先 `resourcesApi.get` 再 `update` | 切换账户多一次 GET 往返（mount 已拉过全量） | 🟢 | **跳过**：后端 `update_resource` 要求全字段，GET 用于避免覆盖其他字段；非正确性缺陷，留待引入专用 `set_active_account` 端点 |
| 5 | simplification | `lib.rs` `DEFAULT_SIP_PORT` const + `default_sip_port()` 包装 | 双重间接（const 仅被该 fn 消费） | 🟢 | **跳过**：serde `#[serde(default = "fn")]` 强制 fn 路径，属惯用法 |
| 6 | simplification | `resource_api.rs` test_connection 构造 `ResourceConnInfo` 时填充 `username`/`port`，SIP 解析层已忽略 | 死字段填充 | 🟢 | **跳过**：字段跨协议共享，影响极小 |
| 7 | reuse | 前端 `SipAccountForm` / `SipAccountView` / 内联 `{accounts?, activeAccount?}` 三处镜像 `SipProfile`；`transport` 联合类型 `'udp'|'tcp'|'tls'` 两处手敲 | 类型漂移风险 | 🟢 | **跳过**：跨组件抽出共享 TS 类型属范围外重构 |
| 8 | reuse | "active 或 first" 解析规则在 `load_sip_conn` / `SipPage.parseSipProfile` / `WizardModal.buildConfig` 三处重复 | 规则三份实现 | 🟢 | **跳过**：横跨后端 + 两个前端文件，范围外 |
| 9 | altitude | 后端 `load_sip_conn` 仍保留顶层 `host` 回退（"直连缺省写法"逃生舱），而模型已声明 server 完全下沉账户 | 特殊用例叠加于共享 `ResourceConnInfo` | 🟢 | **跳过**：仅 legacy/异常 payload 触发，移除属行为变更，超出精简范围 |

## 门禁判断

无 🔴、无 🔴/🟡 待打回项。已落地修复 1 项（#1 冗余 `sipHost` ref）。其余 8 项判为 🟢 可选改进（非热路径克隆/二次解析、跨组件类型重构、后端行为变更均超出本 diff 范围，留待后续）。

**结论：步骤4 通过（无 🔴/🟡）。**
