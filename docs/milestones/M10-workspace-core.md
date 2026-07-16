# M10: 工作区核心

## Context

M9 完成了环境和资源管理 CRUD API、管理页面、资源创建向导，侧栏连接树已对接真实 API。但工作区 Tab 系统仍使用硬编码假数据（Web Server / DB Primary），点击连接树的资源不会在工作区打开对应控制台。M10 将工作区从假数据壳变成真正可用的操作画布。

本里程碑版本类型：minor（新功能），版本号 0.10.0 → 0.11.0。

## 产品边界

**本阶段做：**
- Tab 系统改造（协议路由 + 资源关联 + 删除写死 tab）
- TerminalView 嵌入（从资源读取连接参数，WebSocket 连接）
- SqlPage 嵌入（自动 connect/disconnect）
- RedisPage 嵌入（自动 connect）
- FilesPage 嵌入（SFTP/S3 模式）
- Quick Connect 降级为临时连接
- 状态栏改造（显示真实资源信息）

**本阶段不做：**
- SqlConnectorFactory 修复（M11）
- Agent 注册/心跳/WebSocket 隧道（M12）
- Dashboard/审计日志/设置页改造（M13）
- i18n 完整翻译（M14）
- 凭据加密（M14）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Tab 系统改造（协议路由 + 资源关联） | ✅ |
| 2 | TerminalView 嵌入改造 | ✅ |
| 3 | SqlPage 嵌入改造 | ⬜ |
| 4 | RedisPage 嵌入改造 | ⬜ |
| 5 | FilesPage 嵌入改造 | ⬜ |
| 6 | Quick Connect 改造 | ✅ |
| 7 | 状态栏改造 | ✅ |

## 子任务详细设计

### 1 Tab 系统改造（协议路由 + 资源关联）

**功能目标**

重写 WorkspacePage 的 Tab 系统，删除写死假数据，根据协议动态路由到对应功能组件。

**文件结构**

修改：
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — 大幅重构

**接口设计**

```typescript
interface Tab {
  id: string
  label: string
  protocol: 'ssh' | 'mysql' | 'postgresql' | 'redis' | 'sqlite' | 'sftp' | 's3'
  resourceId?: string      // 关联真实资源 ID
  environmentId?: string
  host?: string
  port?: number
  username?: string
  password?: string         // 从 config_json 解析
  database?: string         // SQL 类协议
  status: 'connecting' | 'connected' | 'disconnected' | 'error'
  color?: string
  renaming?: boolean
}
```

**交互设计**

- 删除硬编码的初始 tabs（Web Server / DB Primary）
- 连接树点击资源 → `openResourceFromTree()` 创建 Tab 并激活
- Tab 内容区域根据 `tab.protocol` 动态渲染：
  - `ssh` → `<TerminalView v-bind="tabProps" />`
  - `mysql` / `postgresql` / `sqlite` → `<SqlPage v-bind="tabProps" />`
  - `redis` → `<RedisPage v-bind="tabProps" />`
  - `sftp` / `s3` → `<FilesPage v-bind="tabProps" />`
- 去重：相同 resourceId 不重复打开
- Tab 关闭时清理对应 session

**提交信息**

```
feat(workspace): refactor Tab system with protocol routing and resource binding
```

### 2 TerminalView 嵌入改造

**功能目标**

TerminalView 从资源读取连接参数，自动在 mount 时通过 WebSocket 连接到指定 SSH 服务器。

**文件结构**

修改：
- `packages/rex-console-web/src/features/terminal/TerminalView.vue` — 添加 props 接收资源参数
- `packages/rex-console-web/src/features/terminal/useTerminal.ts` — 支持外部传入连接参数

**接口设计**

```typescript
// TerminalView props
interface Props {
  resourceId?: string
  host?: string
  port?: number
  username?: string
  password?: string
}
```

**交互设计**

- mount 时如果有 host 参数，自动发起 WebSocket 连接
- 连接参数通过 `useTerminal()` composable 传入
- 断开时显示重连界面（保持现有 reconnect 逻辑）
- 无 host 参数时显示空状态（等待连接）

**提交信息**

```
feat(workspace): integrate TerminalView into workspace tabs
```

### 3 SqlPage 嵌入改造

**功能目标**

SqlPage 从资源读取连接参数，mount 时自动调用 `/api/sql/connect` 建立 session。

**文件结构**

修改：
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — 添加 props + 自动连接

**接口设计**

```typescript
// SqlPage props
interface Props {
  resourceId?: string
  host?: string
  port?: number
  username?: string
  password?: string
  database?: string
  dbType?: 'mysql' | 'postgresql' | 'sqlite'
}
```

**交互设计**

- mount 时如果有 host 参数，调用 `POST /api/sql/connect` 建立 session
- 卸载时调用 `POST /api/sql/disconnect` 清理 session
- 导航树从 session 加载 databases → tables → columns
- 连接失败时显示错误信息
- 无参数时显示空状态

**提交信息**

```
feat(workspace): integrate SqlPage into workspace tabs
```

### 4 RedisPage 嵌入改造

**功能目标**

RedisPage 从资源读取连接参数，mount 时自动调用 `/api/redis/connect`。

**文件结构**

修改：
- `packages/rex-console-web/src/features/redis/RedisPage.vue` — 添加 props + 自动连接

**接口设计**

```typescript
// RedisPage props
interface Props {
  resourceId?: string
  host?: string
  port?: number
  password?: string
  db?: number
}
```

**交互设计**

- mount 时如果有 host 参数，调用 `POST /api/redis/connect` 建立 session
- 卸载时调用 `POST /api/redis/disconnect`
- 连接后自动加载键树和 Server Status
- 断开时显示重连界面

**提交信息**

```
feat(workspace): integrate RedisPage into workspace tabs
```

### 5 FilesPage 嵌入改造

**功能目标**

FilesPage 从资源读取连接参数，支持 SFTP 和 S3 两种模式。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 添加 props + 自动连接

**接口设计**

```typescript
// FilesPage props
interface Props {
  resourceId?: string
  protocol?: 'sftp' | 's3'
  host?: string
  port?: number
  username?: string
  password?: string
}
```

**交互设计**

- mount 时如果有 host 参数，调用 `POST /api/files/connect` 建立 session
- 根据 protocol 决定使用 SFTP 还是 S3 模式
- 卸载时调用 `POST /api/files/disconnect`
- 连接后显示文件浏览器

**提交信息**

```
feat(workspace): integrate FilesPage into workspace tabs
```

### 6 Quick Connect 改造

**功能目标**

Quick Connect 降级为临时连接模式，不保存为资源。

**文件结构**

修改：
- `packages/rex-console-web/src/features/workspace/QuickConnect.vue` — 改为直接打开 Tab

**交互设计**

- 输入协议 + 主机 + 端口 + 用户名 → 点击连接
- 创建临时 Tab（resourceId 为空）
- Tab 标记为临时（可选保存为资源，后续 M13 实现）
- 不调用资源创建 API

**提交信息**

```
refactor(workspace): downgrade QuickConnect to ad-hoc-only mode
```

### 7 状态栏改造

**功能目标**

状态栏显示当前 Tab 关联的真实资源信息。

**文件结构**

修改：
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — 状态栏区域

**交互设计**

- 显示当前 Tab 的协议 + 主机 + 端口
- 连接状态实时更新（connecting → connected → disconnected）
- 编码显示（SSH 终端）
- 广播/锁状态指示

**提交信息**

```
feat(workspace): update status bar with real resource info
```

## 设计核对点

- [ ] Tab 数据模型包含 resourceId、environmentId
- [ ] 协议路由覆盖 7 种协议
- [ ] 各组件 props 与后端 API 参数一致
- [ ] mount 自动连接、unmount 自动断开
- [ ] Quick Connect 不保存为资源
- [ ] 状态栏显示真实资源信息
- [ ] 前端 type-check + lint + build 通过

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
