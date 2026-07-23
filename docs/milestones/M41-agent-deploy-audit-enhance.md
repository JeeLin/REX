# M41: Agent 部署指南 + 审计日志增强

## Context

M40 补全了工作区快捷键和 Agent 日志查看。本里程碑实现 PRODUCT.md §3.10 中 Agent 部署指南和配置弹窗，以及 §3.11 审计日志的增强功能（统计卡片、行展开详情、CSV 导出、时间范围筛选），提升运维管理效率。

版本类型：minor（新功能），版本号 0.37.0 → 0.38.0。

## 产品边界

**本阶段做：**
- Agent 部署指南弹窗（二进制/Docker/Compose/配置 4 种，按 OS/架构推荐）
- Agent 配置弹窗（只读元信息 + auto_update 开关 + 更新状态展示）
- 审计日志统计卡片（总数/成功/失败）
- 审计日志时间范围筛选 + 环境筛选
- 审计日志行展开详情（target、detail、agent_id 结构化展示）
- 审计日志 CSV 导出
- 审计日志 AGENT_ONLINE/AGENT_OFFLINE action 过滤选项

**本阶段不做：**
- Agent 实时日志流
- 审计日志清除功能（涉及数据删除，需谨慎设计）
- 设置页改造（移至后续里程碑）
- 新页面或新路由

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Agent 部署指南弹窗 + 配置弹窗 | ⬜ |
| 2 | 审计日志增强（统计卡片 + 行展开 + CSV 导出 + 时间筛选） | ⬜ |

## 子任务详细设计

### 1 Agent 部署指南弹窗 + 配置弹窗

**功能目标**

在 Agent 管理页实现两种弹窗：
1. **部署指南弹窗**：按 Agent 的 OS/架构生成对应的部署命令（二进制下载、Docker、Docker Compose、配置文件），帮助用户快速部署新 Agent。
2. **配置弹窗**：展示 Agent 元信息（环境、Agent ID、版本只读）和可配置项（服务器地址、令牌、auto_update 开关 + 更新状态）。

**文件结构**

修改：
- `packages/rex-console-web/src/pages/AgentsPage.vue` — 新增部署指南弹窗和配置弹窗
- `packages/rex-console-web/src/i18n/locales/zh.json` — 新增翻译 key
- `packages/rex-console-web/src/i18n/locales/en.json` — 新增翻译 key

**部署指南弹窗设计**

Agent 卡片新增「部署」按钮。弹窗内容：

1. **检测 Agent 的 OS 和架构**（从 `agent.os` / `agent.arch` 读取）
2. **4 种部署方式**（Tab 或分段展示）：

**a) 二进制下载**
```bash
# 下载（按 OS/架构推荐）
curl -LO https://{hub-host}/api/agents/download?os={os}&arch={arch}
chmod +x rex-agent
# 注册
./rex-agent register --server https://{hub-host} --token {env-token}
```

**b) Docker**
```bash
docker run -d \
  --name rex-agent \
  -e REX_SERVER=https://{hub-host} \
  -e REX_TOKEN={env-token} \
  rex/rex-agent:latest
```

**c) Docker Compose**
```yaml
services:
  rex-agent:
    image: rex/rex-agent:latest
    environment:
      REX_SERVER: https://{hub-host}
      REX_TOKEN: {env-token}
    restart: unless-stopped
```

**d) 配置文件**
```ini
# ~/.rex/config.toml
[agent]
server = "https://{hub-host}"
token = "{env-token}"
auto_update = true
```

- Hub 地址从 `window.location.origin` 获取
- 环境令牌从 `agent` 关联的 environment 的 `agent_token` 获取
- 每段配置旁有「复制」按钮

**配置弹窗设计**

Agent 卡片新增「配置」按钮。弹窗内容：

只读信息：
- 环境名、Agent ID、当前版本、OS/架构、主机名

可配置项：
- 服务器地址（文本框，显示 Hub URL）
- 令牌（只读，与重置令牌弹窗联动提示）
- auto_update 开关（Toggle 组件）

更新状态（仅 auto_update 开启时展示）：
- 当前版本 vs Hub 版本
- 更新阶段（无更新 / 检查中 / 下载中 / 安装中 / 失败）
- 更新错误信息

> 注意：后端暂无 update_status API。auto_update 开关和更新状态展示为 UI 占位，数据读取 agent 的 `config_json` 字段。后端 API 可在后续里程碑补全。

**测试标准**

- 点击 Agent「部署」按钮打开部署指南弹窗
- 4 种部署方式正确展示
- 复制按钮正常工作
- 点击 Agent「配置」按钮打开配置弹窗
- 只读信息正确展示
- auto_update 开关可切换
- `bun run type-check` + `bun run lint` 通过

**提交信息**: `feat(web): add Agent deploy guide and config modals`

### 2 审计日志增强

**功能目标**

增强审计日志页面的查询能力和信息展示密度，对标 PRODUCT.md §3.11。

**文件结构**

修改：
- `packages/rex-console-web/src/pages/AuditLogPage.vue` — 重写，增加统计卡片、行展开、CSV 导出、时间范围筛选
- `packages/rex-console-web/src/i18n/locales/zh.json` — 新增翻译 key
- `packages/rex-console-web/src/i18n/locales/en.json` — 新增翻译 key

后端修改：
- `crates/rex-hub/src/audit_api.rs` — 新增统计端点 `GET /api/audit-log/stats`
- `crates/rex-hub/src/db.rs` — 新增 `query_audit_stats()` 方法

**后端 API**

```
GET /api/audit-log/stats?action=&start_time=&end_time=&environment_id=
  → { total: i64, success_count: i64, failure_count: i64 }
```

现有 `GET /api/audit-log` 已支持 `action`、`result`、`environment_id` 筛选。新增 `start_time` 和 `end_time` 参数支持时间范围筛选。

**前端交互**

1. **统计卡片**（顶部 3 个卡片）：
   - 总操作数（蓝色）
   - 成功数（绿色）
   - 失败数（红色）
   - 筛选条件变化时自动刷新

2. **筛选栏**（增强）：
   - 操作类型：下拉多选（原有 + 新增 AGENT_ONLINE / AGENT_OFFLINE）
   - 结果：下拉（success / failure / all）
   - 环境：下拉（从 environments store 读取）
   - 时间范围：预设（今天 / 最近 7 天 / 最近 30 天 / 全部）+ 自定义日期选择
   - 刷新按钮

3. **行展开详情**（点击表格行展开）：
   - 展开区域显示：完整 target、environment_id、resource_id、agent_id、detail（格式化展示）
   - 如果 detail 是 JSON，用 `<pre><code>` 展示
   - 如果 action 是 SSH_CONNECT / SQL_QUERY，detail 区域用代码块样式

4. **CSV 导出**（筛选栏右侧按钮）：
   - 导出当前筛选条件下的所有记录
   - 字段：时间、操作、目标、环境、资源、Agent、结果、详情
   - 文件名：`audit-log-{date}.csv`

5. **action 过滤选项补全**：
   ```
   ENV_CREATE / ENV_UPDATE / ENV_DELETE
   RESOURCE_CREATE / RESOURCE_DELETE
   AGENT_REGISTER / AGENT_ONLINE / AGENT_OFFLINE
   SSH_CONNECT / SQL_QUERY / REDIS_COMMAND / FILE_OPERATION
   AUTH_LOGIN / AUTH_LOGOUT
   ```

**数据模型**

```typescript
// 新增
interface AuditStats {
  total: number
  success_count: number
  failure_count: number
}

// 增强
interface AuditQuery {
  action?: string
  result?: string
  environment_id?: string
  agent_id?: string
  start_time?: string
  end_time?: string
  limit?: number
}
```

**测试标准**

- 统计卡片正确显示总数/成功/失败
- 筛选条件变化时统计自动更新
- 时间范围筛选正确工作
- 环境筛选正确工作
- 点击行展开详情
- CSV 导出正确生成文件
- action 过滤包含所有操作类型
- `bun run type-check` + `bun run lint` 通过

**提交信息**: `feat(web): enhance audit log page with stats, row details, CSV export, and time filter`

## 设计核对点

- ✅ 不引入多用户、RBAC、企业协作概念
- ✅ Agent 部署指南数据来自 Hub 地址 + 环境令牌，不引入新的数据源
- ✅ 审计日志增强基于现有 API，新增统计端点和时间筛选参数
- ✅ 前端命令使用 bun
- ✅ 依赖声明符合 workspace 规则
- ✅ 所有用户文本使用 i18n 翻译

## Flow Status

- [x] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

（打回时追加一条，创建里程碑文档时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
