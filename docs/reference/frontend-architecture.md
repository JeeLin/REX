# 前端工程结构

## 前端包

前端作为独立 Vue 3 + Vite 包开发，由 Hub 在构建或启动时嵌入/托管。

```text
packages/
└── rex-console-web/
    ├── package.json
    ├── index.html
    ├── src/
    │   ├── main.ts
    │   ├── App.vue
    │   ├── router.ts
    │   ├── api/
    │   ├── stores/
    │   ├── components/
    │   ├── layouts/
    │   ├── pages/
    │   ├── styles/
    │   └── i18n/
    ├── public/
    └── vite.config.ts
```

Hub 二进制只负责静态资源托管和 API 服务；前端不持有敏感凭据，不中转文件数据。

## 页面路由

| 页面 | 路由 | 布局 | 说明 |
|------|------|------|------|
| `Login.vue` | `/login` | 全屏 | 登录认证 |
| `Dashboard.vue` | `/dashboard` | 标准布局 | 仪表盘 |
| `Environments.vue` | `/environments` | 标准布局 | 环境列表 |
| `EnvironmentDetail.vue` | `/environments/:id` | 标准布局 | 环境详情 |
| `EnvironmentNew.vue` | `/environments/new` | 标准布局 | 创建环境 |
| `ResourceNew.vue` | `/resources/new` | 标准布局 | 创建资源向导 |
| `Workspace.vue` | `/workspace` | 工作区布局 | 多标签分屏 |
| `Terminal.vue` | `/terminal` | 全屏 | SSH 终端 |
| `SqlConsole.vue` | `/sql` | 全屏 | SQL 控制台 |
| `Files.vue` | `/files` | 全屏 | 文件管理 |
| `Agents.vue` | `/agents` | 标准布局 | Agent 管理 |
| `AuditLog.vue` | `/audit-log` | 标准布局 | 审计日志 |
| `Cdr.vue` | `/cdr` | 标准布局 | SIP 通话记录 |
| `Settings.vue` | `/settings` | 标准布局 | 个人设置 |

## 功能域组织

按功能域组织组件：

```text
packages/rex-console-web/src/
├── pages/          只做路由入口（LoginPage / DashboardPage / EnvironmentsPage / EnvironmentDetailPage / WorkspacePage / AgentsPage / AuditLogPage / CdrPage / SettingsPage / SetupPage）
├── features/       按功能域组织组件
│   ├── terminal/   SSH 终端（含 terminal-themes）
│   ├── sql/        SQL 控制台（含 formatSql）
│   ├── files/      SFTP/S3 文件管理
│   ├── redis/      Redis 控制台
│   ├── resource/   资源协议图标/色/名映射（protocols.ts）
│   ├── resource-panel/  连接树侧栏与资源面板
│   ├── sip/        SIP 软电话（拨号盘/通话/CDR/录音/抓包/质量监控）
│   ├── agents/     Agent 管理
│   ├── settings/   个人设置
│   └── workspace/  工作区外壳（Tab/Pane/Quick Connect/状态栏）
├── components/     跨功能通用组件
├── api/            按接口域拆分（client / auth / environments / resources / sql / redis / files / agents / audit / dashboard / settings / sip）
├── composables/    组合式函数
├── stores/         跨功能状态（Pinia）
├── layouts/        布局组件（AppLayout 等）
├── styles/         主题和全局样式
└── i18n/           国际化
```

## 全局组件

通用组件：

| 组件 | 说明 |
|------|------|
| `AppLayout` | 桌面侧边栏 + 移动端底部导航 |
| `ResourceIcon` | 协议图标、颜色、状态点（取自 `features/resource/protocols.ts`） |
| `ContextMenu` | 统一右键菜单，支持 divider、danger、submenu |
| `ConfirmDialog` | 删除、断开、重启、重置令牌等确认弹窗 |
| `ToastProvider` | 全局 Toast |
| `ThemeToggle` | 深色/浅色/跟随系统切换 |
| `I18nSwitch` | 中文/英文切换 |
| `VersionOverview` | Hub 和 Agent 版本总览 |

## 全局状态

前端状态按模块拆分（`stores/`）：

| store | 说明 |
|-------|------|
| `auth.ts` | 登录态、会话过期、登出 |
| `environments.ts` | 环境列表、环境详情 |
| `favorites.ts` | 收藏的连接 |
| `notification.ts` | 通知/提醒 |
| `update.ts` | Hub/Agent 版本与更新状态 |
| `workspace.ts` | 标签页、分屏布局、连接状态 |

状态数据优先从 Hub API 获取，本地只做 UI 状态缓存。敏感字段不在 localStorage 中明文保存。
