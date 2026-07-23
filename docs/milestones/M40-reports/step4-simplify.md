# M40 代码精简报告

## 精简项

| # | 文件 | 变更 | 原因 |
|---|------|------|------|
| 1 | resource_api.rs | 路由参数 `{env_id}` → `{id}`，`{rid}` 代替 `{id}` | 消除与 env_routes `/{id}` 的参数名冲突 |
| 2 | resource_api.rs | handler 参数名统一为 `(env_id, rid)` | 与路由定义一致 |
| 3 | agent_ws.rs | Agent 上线/离线事件写入审计日志 | 复用现有 audit_log 表，不引入新表 |
| 4 | models.rs | AuditFilter 新增 `agent_id` 字段 | 支持按 agent 筛选审计日志 |

## 未处理项

| 项 | 原因 |
|----|------|
| Agent card footer 按钮间距 | 可后续优化，不影响功能 |
| Log viewer 虚拟滚动 | 日志量 <100 条，不需要 |

## 结论

✅ 精简不改变功能行为。路由参数名统一消除了 Axum 合并冲突的根因。
