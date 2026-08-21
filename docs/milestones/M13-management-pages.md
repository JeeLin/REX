# M13: 管理页面

## Context

M8-M12 完成了基础设施、环境/资源、工作区、SQL 控制台和 Agent 管理。Dashboard、审计日志、设置页目前是假数据。M13 将它们改为真实功能。

本里程碑版本类型：minor（新功能），版本号 0.13.0 → 0.14.0。

## 产品边界

**本阶段做：**
- Dashboard 统计 API + 页面重写
- 审计日志查询 API + 页面重写
- 设置 API + 页面重写（主题/语言持久化）

**本阶段不做：**
- 凭据 AES 加密（M14）
- i18n 完整翻译（M14）
- 响应式适配（M14）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Dashboard 统计 API + 页面 | ✅ |
| 2 | 审计日志查询 API + 页面 | ✅ |
| 3 | 设置 API + 页面（主题/语言持久化） | ✅ |

## 子任务详细设计

### 1 Dashboard 统计 API + 页面

**功能目标**

提供系统概览统计和最近使用资源。

**接口设计**

```
GET /api/dashboard/stats → { environment_count, resource_count, online_agents, today_operations }
GET /api/dashboard/recent → Resource[]
```

**文件结构**

修改：
- `crates/rex-hub/src/rex-hub.rs` — 添加路由
- `packages/rex-console-web/src/pages/DashboardPage.vue` — 重写

**提交信息**

```
feat(dashboard): add stats API and rewrite dashboard page
```

### 2 审计日志查询 API + 页面

**功能目标**

提供审计日志查询、筛选和导出。

**接口设计**

```
GET /api/audit-log?time_from=&time_to=&action=&result= → AuditEntry[]
GET /api/audit/stats → { total, success_count, failure_count }
```

**文件结构**

修改：
- `packages/rex-console-web/src/pages/AuditLogPage.vue` — 重写

**提交信息**

```
feat(audit): add audit log query API and rewrite page
```

### 3 设置 API + 页面

**功能目标**

提供设置持久化（主题/语言/终端配置）。

**接口设计**

```
GET    /api/settings → Settings
PUT    /api/settings → Settings    { theme, language, terminal_font, ... }
PUT    /api/user/password → { ok }  { old_password, new_password }
```

**文件结构**

修改：
- `packages/rex-console-web/src/pages/SettingsPage.vue` — 重写

**提交信息**

```
feat(settings): add settings API and rewrite page with persistence
```

## 设计核对点

- [ ] Dashboard 统计数据真实
- [ ] 审计日志可查询/筛选
- [ ] 设置修改后刷新页面仍保留
- [ ] type-check + cargo check 通过

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
