# 步骤4 代码精简报告 — v0.70.7

## 范围

对比里程碑起始 ref `milestone-v0.70.7-start` 至 HEAD，本里程碑共改动 34 个代码文件
（crates + 前端），均为 v0.70.7 统一 SQL 资源合并相关。

## 检查维度（AGENTS.md / Rust workspace + Vue 功能域）

- 是否引入重复代码
- 是否过度设计 / 提前实现下一阶段能力
- 是否符合 `workspace = true` 依赖规则与 Rust workspace 结构
- 是否符合前端按功能域组织
- 是否有死代码 / 未完成 rename 残留

## 发现

| # | 位置 | 级别 | 说明 |
|---|------|------|------|
| — | 全量 | 无 | 未发现 🔴/🟡/🟢 任一问题 |

### 逐项确认

1. **db_type → subtype 重命名完整性**：`git grep db_type` 在变更文件中残留的均为
   连接处理器内的局部变量名（非资源字段），以及 `sql.rs` 中 `DetectedDialect` 共享层
   的无关字段；资源模型字段、列、API、前端类型均已统一为 `subtype`。无半吊子 rename。
2. **重复代码**：前端 SQL 分支在三处（WizardModal / EnvironmentDetailPage / ResourceProperties）
   均按 `protocol === 'sql' + subtype` 分支，逻辑清晰无冗余拷贝；旧 mysql/postgresql/sqlite
   协议选项已从向导移除，仅作为 dialect 子类保留，符合「合并为单一 SQL」目标。
3. **死代码**：`migrate_unified_sql_resources` / `set_resource_subtype` / `detect_dialect`
   均被调用；前端 `SUBTYPE_META` / `resProtoIcon` 等辅助函数均被模板使用。无未使用项。
4. **结构一致性**：后端 `subtype` 经 `ResourceConnInfo` → `SessionOpened.subtype` →
   `set_resource_subtype` 回写链路对称（Hub 直连 / Agent 隧道一致）；前端
   `ResourceInfo.subtype` → `Tab.subtype` → `PaneLeaf.sqlDbType` → `SqlPage.dbType` 透传完整。
5. **依赖规则**：无新增 crate 依赖，全部 `workspace = true`，符合约束。

## 结论

✅ 精简检查无 🔴/🟡/🟢 发现，功能不变、组织方式合理。步骤4 通过，勾选步骤4。
