# 步骤7 设计再确认：0.70.4 SIP 资源按名称管理 + 多账户切换

## 审查维度（对照 AGENTS.md 硬性约束 + 里程碑设计核对点）

| # | 维度 | 结论 | 说明 |
|---|------|------|------|
| 1 | 单用户/自托管定位不被破坏 | ✅ | 全 diff 无多用户/RBAC/企业协作概念；SIP 资源仍为单用户资源模型 |
| 2 | 文件传输数据不经浏览器 | ✅ | 本次未涉及文件传输；重申不越界 |
| 3 | Hub/Agent 版本一致模型未被改动 | ✅ | 多账户切换在配置解析层（`load_sip_conn`），FFI（`SipUaTrait`）与隧道帧 `[4B channelId][json]` 不变 |
| 4 | `SipConfig` 线格式不变 | ✅ | `rex-sip::SipConfig` 字段集未改；Agent `parse_sip_config` 仍解析扁平 `SipConfig`，Hub 发送前已解析 profile→active 账户→扁平 `SipConfig` |
| 5 | 仅支持新形状（无迁移/旧兼容） | ✅ | `load_sip_conn` 仅接受 `SipProfile`（accounts[] + activeAccount）；无旧形状兼容分支 |
| 6 | 依赖声明 `workspace = true` | ✅ | 仅新增类型定义（`SipAccount`/`SipProfile`），无新增依赖 |
| 7 | 子任务实现与详细设计一致 | ✅ | #1 后端模型拆分 + `load_sip_conn`；#2 `test_connection` sip 复用 `load_sip_conn` 校验 server+username（含 port 0 拒绝）；#3 向导多账户（id 单调、≥1 账户校验、server 派生 host）；#4 `SipPage` 多账户切换写回 activeAccount；#5 文档对齐 `PRODUCT.md`/`data-models.md` |
| 8 | 打回修复已落地（步骤5 3 🟡） | ✅ | commit `40864894`（port 0 拒绝）、`fc1ca46f`（id 碰撞 + ≥1 账户校验）；`SipPage` JSON.parse try/catch 与 `sip.title` 标题（首轮 5 🟡）均已就位 |

## 代码实现 vs 里程碑文档确认

- **后端**：`resource_conn.rs::load_sip_conn` 解析 `SipProfile` → 选 active（回退首账户）→ 取该账户 server/port/transport/凭据构造 `SipConfig`；显式拒绝 server 空 / port 0 / username 空。`resource_api.rs::test_connection` sip 分支复用 `load_sip_conn`。
- **前端向导**：`WizardModal.vue` 多账户编辑，每账户自带 server/port/transport；`buildConfig` 过滤空账户并将 `activeAccount` 指向被保留账户（与 UI 单选一致）；`resourceHost` 由账户派生生效账户 server；`submit` 校验 ≥1 有效账户。
- **前端面板**：`SipPage.vue` 解析 `config_json` 的 `accounts`/`activeAccount`，切换账户 GET 全量后仅写回 `activeAccount`（带 try/catch），面板标题用 `sip.title`。
- **文档**：`PRODUCT.md §3.10`、`reference/data-models.md` 已对齐「按名称 + 多账户、账户自带 server profile」模型。

## 汇总

- **通过维度**：8/8
- **结论**：✅ 通过

## 发现的问题

无（实现与里程碑文档一致，产品语义与用户可见行为均未变）。
