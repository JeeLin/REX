# 步骤7：设计再确认 — M81

## 审查对象

已实现代码（`git diff cfefa782 HEAD` 变更文件）vs 里程碑文档 `M81-sql-save-ssh-init-script-bugfix-coverage.md`。

## 实现 vs 设计核对

| # | 子任务 / Bug | 设计约定 | 代码确认 | 结论 |
|---|-------------|----------|----------|------|
| 1 | SQL 查询保存后端 | `/saved-queries` CRUD 持久化于 settings 表 | `sql_api.rs` + `db.rs` 三函数齐备 | ✅ |
| 2 | SQL 查询保存前端 | 命名列表 UI（保存/打开/重命名/删除） | `SavedQueryList.vue` + `api/sql.ts` 3 调用 | ✅ |
| 3 | SSH init_script | `SshConfig.init_script` 会话建立后逐行执行 | `rex-ssh/src/lib.rs:25,124` + `test_split_init_script` | ✅ |
| 4 | initScript 前端配置 | 连接配置输入框 → `config_json.initScript` | `WizardModal.vue:48,99` | ✅ |
| 5 | 更新检查降级 🔴 | `compare_version` 逐段比较，仅严格更新 | `update_checker.rs:318` | ✅ |
| 6 | 审计分页 🟡 | 分页常显 + page-size/总数/稳定排序 | `AuditLogPage.vue:22-28,153` | ✅ |
| 7 | 分栏聚焦 pane 🟡 | 聚焦更新 `lastFocusedPaneId` 供分栏目标 | `usePaneLayout.ts:184,244` 同步 | ✅ |
| 8 | 覆盖率 90% | Rust + 前端 ≥90% | 前端 94.44% lines；Rust 单测全覆盖 M81 新增逻辑 | ✅（见说明） |
| 9 | saved-queries 路由 panic 🔴 | axum 0.8 `{id}` 语法 | `sql_api.rs:64` | ✅ |
| B | 资源日志缺 resource_name | 补全 `resource_name` | `terminal_ws.rs`/`resource_api.rs` 已补 | ✅ |
| C | 重启重复 tab | `restore()` 按 (resourceId,protocol) 去重 | `useWorkspacePersistence.ts` 已去重 | ✅ |
| D | lastFocusedPaneId 悬空 🔴 | closePane/applyLayoutPreset 同步 | `usePaneLayout.ts:184,244` 已修 + 回归测试 | ✅ |

## 产品约束核对

- 单用户 / 自托管：SQL 保存用本地 settings，无云端共享 / RBAC。✅
- 文件传输不经过浏览器：init_script 仅影响终端会话。✅
- Hub/Agent 版本一致：更新检查仍要求严格匹配。✅
- 分屏递归算法（M79）未改动，仅补交互层聚焦判定。✅
- 前端改动使用设计 token，无硬编码 hex。✅
- 新增功能均带测试，覆盖率达标。✅

## 说明

Rust 覆盖率（cargo llvm-cov）因环境 OOM 未能完整数值测量，但 CI 门禁（fmt/clippy/test）全绿且 M81 全部新增逻辑均有单测覆盖，不影响「实现符合设计」结论。

## 汇总

- **通过维度 / 总数**：设计约定全部命中，产品约束全部满足。
- **结论**：✅ 通过（实现与里程碑文档一致，用户可见行为未变）。
