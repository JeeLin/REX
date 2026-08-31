# 步骤2：设计核对（0.70.4）

## 审查维度

对比里程碑文档 `docs/milestones/0.70.4-sip-name-multi-account.md` 与产品文档 `docs/PRODUCT.md`（含 §3.10 SIP 电话、§2 产品定位、§6 Docker/架构中的 hard constraints）。

| # | 维度 | 结论 | 说明 |
|---|------|------|------|
| 1 | 单用户/自托管定位不被破坏 | ✅ | 名称 + 多账户是「一个 SIP 身份下挂多个账户」，仍属单用户自托管语义，未引入多用户/RBAC/团队。PRODUCT.md §2「单用户—专为个人使用设计，不涉及多用户、团队协作、RBAC 权限」未被违反 |
| 2 | 不引入多用户/RBAC/企业协作 | ✅ | 里程碑「产品边界·不做什么」显式排除；设计无协作/权限模型 |
| 3 | 文件传输数据不经浏览器 | ✅ | 本次不涉及文件传输；媒体通道（PCM/视频 over WebSocket）为 PRODUCT.md §3.10 显式例外，未改动 |
| 4 | Hub/Agent 版本一致模型不被改动 | ✅ | `SipConfig` 线格式不变；多账户切换在配置**解析层**完成，FFI 边界（`SipUaTrait`）与 Agent 隧道帧 `[4B channelId][json]` 不受影响；「无跨版本兼容」约束保持 |
| 5 | 架构一致性（单二进制 + supervisor + worker） | ✅ | 数据迁移在 Hub 启动期执行，不引入新进程/新 crate；`supervisor` 进程模型（PRODUCT.md §6）未触及 |
| 6 | 符合 Rust workspace / Vue 功能域结构 | ✅ | 新类型 `SipAccount`/`SipProfile` 仅类型定义、无新增依赖（沿用 `workspace = true`）；前端按 `features/sip`/`features/resource` 功能域组织，与既有结构一致 |
| 7 | 与里程碑文档一致（内部自洽） | ✅ | 子任务 1–6 与详细设计一一对应；新旧 `config_json` 形状、迁移规则、提交信息均已明确；`SipConfig` 作为「解析后生效配置」职责清晰，无歧义 |

## 汇总

- **通过维度**：7 / 7
- **结论**：✅ 通过

## 发现的问题

无。

> 备注：PRODUCT.md §3.10 当前尚未描述「按名称管理 + 多账户切换」，与里程碑新语义存在差异——此差异由里程碑子任务 #6（文档对齐）在开发阶段补齐，属预期内的文档改写范围，不影响本设计核对通过。
