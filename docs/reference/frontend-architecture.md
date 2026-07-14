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
| `Settings.vue` | `/settings` | 标准布局 | 个人设置 |

## 功能域组织

按功能域组织组件：

```text
packages/rex-console-web/src/
├── pages/          只做路由入口
├── features/       按功能域组织组件
│   ├── terminal/
│   ├── sql/
│   ├── files/
│   ├── agents/
│   ├── settings/
│   └── workspace/
├── components/     跨功能通用组件
├── api/            按接口域拆分
├── composables/    组合式函数
├── layouts/        布局组件
├── styles/         主题和全局样式
└── i18n/           国际化
```

## 全局组件

通用组件：

| 组件 | 说明 |
|------|------|
| `AppLayout` | 桌面侧边栏 + 移动端底部导航 |
| `FullScreenLayout` | 终端、SQL、文件管理等全屏页面 |
| `WorkspaceLayout` | 多标签 + 分屏工作区 |
| `ResourceIcon` | 协议图标、颜色、状态点 |
| `ContextMenu` | 统一右键菜单，支持 divider、danger、submenu |
| `ConfirmDialog` | 删除、断开、重启、重置令牌等确认弹窗 |
| `ToastProvider` | 全局 Toast |
| `ThemeToggle` | 深色/浅色/跟随系统切换 |
| `I18nSwitch` | 中文/英文切换 |
| `TransferQueuePanel` | 文件传输队列 |
| `VersionOverview` | Hub 和 Agent 版本总览 |

## 全局状态

前端状态按模块拆分：

```text
stores/
├── auth.ts        登录态、会话过期、登出
├── user.ts        用户设置、主题、语言
├── env.ts         环境列表、环境详情
├── resource.ts    资源列表、连接方式、凭据引用
├── agent.ts       Agent 列表、在线状态、版本、平台
├── workspace.ts   标签页、分屏布局、连接状态
├── transfer.ts    传输任务、进度、冲突处理
├── audit.ts       审计日志筛选条件和列表
└── ui.ts          Toast、Modal、Loading
```

状态数据优先从 Hub API 获取，本地只做 UI 状态缓存。敏感字段不在 localStorage 中明文保存。
