# 步骤7 设计再确认报告 — v0.70.7

## 审查维度

对比已实现代码 vs 里程碑文档的「设计核对点」。

| # | 设计核对点 | 结论 | 说明 |
|---|-----------|------|------|
| 1 | 单一「SQL」资源取代三种并列类型，旧资源 in-place 升级（无破坏性） | ✅ | 向导仅剩 `sql` 协议；`migrate_unified_sql_resources` 幂等迁移旧资源；Resource/Model/db 列统一为 `subtype`；PROTOCOL_ICONS/COLORS/NAMES 新增 `sql` 条目，保留 mysql/postgresql/sqlite 用于旧 tab 向后兼容着色。 |
| 2 | dialect 仅连接时缺省才探测，识别后回写缓存（后续无额外往返） | ✅ | `sql_api.rs` / `agent_sql.rs` 的 `detect_dialect` 仅在 `subtype` 缺省（`auto`）时触发；成功后调用 `set_resource_subtype` 回写，后续连接由 `res.subtype` 直接取用，零额外往返。前端 PaneLeaf 也遵循此逻辑：`sqlDbType` 在 subtype 缺省时传 `'auto'`，探测后 tabsubtype 已缓存。 |
| 3 | Hub 直连与 Agent 隧道两侧探测逻辑对称，探测结果经 `SessionOpened.subtype` 回传 | ✅ | Hub 直连侧（`sql_api::detect_dialect`）与 Agent 侧（`agent_sql::detect_dialect`）共用同一端口预判→握手→VERSION() 确认规则；Agent 探测成功后通过 `SessionOpened { subtype: detected }` 回传 Hub，Hub 经 `take_session_subtype` + `set_resource_subtype` 持久化。 |
| 4 | 前端连接树/向导/控制台合并为单「SQL」，按 dialect 着色，交互行为不变 | ✅ | WizardModal：三个 SQL 选项合并为单一 `sql`，可选 dialect 下拉（默认空 = 自动识别）。EnvironmentDetailPage：`resProtoIcon/Color/Name/Tone` 按 `subtype` 解析。PaneLeaf：`sqlDbType()` 为 `sql` tab 解析 dialect，支持旧 tab 向后兼容（mysql/postgresql/sqlite 直传 protocol）。 |
| 5 | 当前仅支持 MySQL/PostgreSQL/SQLite；新方言本次不实现、不引入依赖 | ✅ | `detect_dialect` 的 `candidates` 列表仅含 `MySQL` / `PostgreSQL`；SQLite 经 host/port 为 0 直接判定。无 MariaDB/ClickHouse/Oracle/SQL Server 分支；前端 dialect 下拉仅三项。 |
| 6 | 不引入多用户 / RBAC / 企业协作（AGENTS.md 硬约束） | ✅ | 无变更。AgentsPage.vue 中的 `multi-user.target` 为 systemd 部署模板的无关内容。 |
| 7 | 文件传输数据不经浏览器（本里程碑不涉及文件传输变更） | ✅ | 本次里程碑仅涉及 SQL 协议，文件传输层零改动。 |
| 8 | 单用户自托管定位不变 | ✅ | 登录/资源/环境模型未变更（PRODUCT.md 的 login 描述是早期遗留写入，非本里程碑新增）。 |

## 汇总

- **通过维度**：8/8
- **结论**：✅ 通过

## 发现的问题

无。
