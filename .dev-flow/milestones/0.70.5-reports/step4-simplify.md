# 代码精简：0.70.5（重跑 · 打回修复后）

## 背景

步骤4 首次审查（报告已归档为 `step4-simplify.md.rejected`）发现 7 项（1 黄 6 绿）。因本版本为最终版本，用户要求全部修复，故登记入本里程碑 Bugs 表并打回步骤3。7 项均已修复并各自提交（见提交 `f017727b` / `2c925449` / `b301d954`）。

本文件为修复后的**重跑**结论。

## 重跑范围与结论

对修复后代码再次按 Reuse / Simplification / Efficiency / Altitude 四角度核对：

| 原发现 | 修复情况 | 重跑结论 |
|--------|----------|----------|
| WizardModal 重复 SIP 账户变换（🟡） | 抽出 `toSipAccounts()`，`buildConfig`/`resourceHost` 共用 | ✅ 已消除 |
| `load_sip_conn` 注释描述已删回退（🟢） | 删除矛盾注释句 | ✅ 已消除 |
| `set_resource_active_account` 未校验 protocol（🟢） | 增加 `protocol != "sip"` 前置拒绝 | ✅ 已消除 |
| 取两次连接池（🟢） | 单次查询 + 同一 conn 执行 UPDATE | ✅ 已消除 |
| 全列读取仅取 config_json（🟢） | 改为 `SELECT id, environment_id, name, protocol, config_json` | ✅ 已消除 |
| test_connection SIP 分支构造已不读取字段（🟢） | 顶层 host/port/username 置空 | ✅ 已消除 |
| 审计日志串行 await（🟢） | 改为 fire-and-forget（去除 `.await`） | ✅ 已消除 |

重跑未发现新的 🔴/🟡/🟢。

## 门禁

- 🔴：0　🟡：0　🟢：0（均为已修复项，无新增）
- 门禁通过，勾选步骤4，进入步骤5。
