# 代码审查：v0.70.7

## 变更概览

- **变更文件**：34 个代码文件（crates + 前端 packages），对比 `milestone-v0.70.7-start`
- **审查维度**：内置默认集（正确性 / 安全性 / 健壮性 / 可维护性 / 性能 / 规范）
  —— AGENTS.md 无 `## 代码审查维度` 段落，故不传 dimensions，按内置默认维度审查。

## 问题列表

（无 🔴 / 🟡 / 🟢 发现）

| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| — | — | — | — | 无 |

## 逐维度确认

1. **正确性**：dialect 探测逻辑（端口预判 → 线缆握手 → `SELECT VERSION()` 确认 → 必要时重连确认类型）在 Hub 直连侧（`sql_api::detect_dialect`）与 Agent 侧（`agent_sql::detect_dialect`）对称一致；`subtype` 透传链路完整（资源 → tab → PaneLeaf → SqlPage；`ResourceConnInfo.subtype` → `SessionOpened.subtype` → `set_resource_subtype` 回写）。`ConnectBody` 字段虽由 `db_type` 改名 `subtype`，但保留 `#[serde(rename = "type")]`，前端 wire 契约不变，向后兼容。旧 `mysql/postgresql/sqlite` 资源经迁移为 `sql+subtype`，前端对旧协议 tab 仍走向后兼容分支。
2. **安全性**：`config_json` 仍加密存储/读取；`subtype` 为不透明字符串透传，无拼接执行；无新增注入/XSS 面。
3. **健壮性**：探测失败时 `anyhow::bail!` 返回可读错误；`SELECT VERSION()` 结果 `unwrap_or_default` 容错；迁移 `migrate_unified_sql_resources` 幂等（按旧 protocol 匹配，已迁移资源不受影响）。
4. **可维护性**：三处前端 SQL 分支（向导/编辑/属性）按 `protocol==='sql' + subtype` 清晰拆分，无冗余拷贝；`SUBTYPE_META` / `resProto*` 辅助函数均被使用；`DetectedDialect` 枚举被 `from_connector` 使用（sql_api.rs:307）。
5. **性能**：探测仅在 `subtype` 缺省（`auto`）时发生一次，成功后回写资源缓存，后续连接零额外往返；无新增循环/查询。
6. **规范**：依赖全部 `workspace = true`；提交信息遵循 `feat:/fix:` 约定；Rust `cargo fmt` 通过；前端 `bun run type-check` + `vite build` 通过，`eslint` 仅 0 error（45 个预存在 warning，均在无关文件）。

## 汇总

- 🔴 必须修复：0
- 🟡 应该修复：0
- 🟢 可选改进：0
- **结论**：通过（🔴/🟡/🟢 均为 0）
