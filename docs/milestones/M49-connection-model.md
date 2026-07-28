# M49: 连接模型重构 — 统一 resource_id 连接

## Context

M48 完成后，当前版本 v0.41.0。在实际使用中发现严重的架构问题：`ConnectRequest` 只有 SQL 字段（host/port/username/password/database），各协议的特殊参数（SQLite 的 file_path、S3 的 endpoint/access_key、Redis 的 db 等）散落在 `config_json` 中，前端需要手动构造连接参数并传递，但 `ResourceInfo` 和 `Tab` 类型不携带 password 和 config_json，导致**除 SSH 外所有协议的连接参数断裂**（password 始终为 undefined，config_json 从未被读取）。

SSH 终端是唯一正常工作的协议 — 它通过 `load_resource_conn` 从 DB 读取全部连接参数，前端只传 `resource_id`。

本里程碑将所有协议统一为 SSH 模式：**前端只传 resource_id，后端从 DB 读取并解密全部连接参数**。

版本类型：minor（重构，改变 API 契约）
版本号：0.42.0

## 产品边界

**做什么**：
- 后端所有 connect 端点统一为 resource_id-based 连接
- 前端所有协议组件简化为只传 resource_id
- 修复 SQLite config_json 未解密的 bug
- 修复 Redis/SFTP/S3 连接参数断裂问题

**不做什么**：
- 不改变资源创建流程（WizardModal 保持不变）
- 不改变 test-connection 逻辑
- 不改变 SSH 终端连接方式（已经正确）
- 不引入新的协议或功能

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 后端：提取公共 load_resource_config 函数 | ✅ |
| 1a | 前端：修复 WizardModal S3 验证 bug（validateStep 跳过 S3 host 检查） | ✅ |
| 1b | 前端：修复 S3 提交时 host 字段为空（应使用 endpoint） | ✅ |
| 2 | 后端：重构 sql_api.rs connect handler | ✅ |
| 3 | 后端：重构 redis_api.rs connect handler | ✅ |
| 4 | 后端：重构 file_api.rs connect handler | ✅ |
| 5 | 前端：简化 SQL API 和 SqlPage | ✅ |
| 6 | 前端：简化 Redis API 和 RedisPage | ✅ |
| 7 | 前端：简化 Files API 和 FilesPage | ✅ |
| 8 | 前端：简化 WorkspacePage Tab 创建 | ✅ |
| 9 | 前端：ResourceProperties 按协议类型显示编辑字段 | ✅ |
| 10 | 前端：修复 /api/settings PUT 422 session_timeout 类型错误 | ✅ |
| 11 | 前端：修复 StatusDot 连接中状态一直闪烁黄灯 | ✅ |
| 12 | 前端：修复 SSH 终端底部 2 行被截断 | ✅ |
| 13 | 前端：修复 SSH vim 操作后终端卡死 | ✅ |
| 14 | 后端：修复 SSH 空闲连接断开过快（keepalive） | ✅ |
| 15 | 前端：修复审计日志页面无法下翻滚动 | ✅ |
| 16 | 前端：修复收藏夹无收藏资源按钮 | ✅ |
| 17 | 前端：修复 SSH SFTP 按钮点击后底部空栏 | ✅ |
| 18 | 前端：修复 MobileTerminalBar 上下键显示 ~A ~B | ✅ |
| 19 | 前端：修复 SQLite 修改配置后无法读取内容 | ✅ |
| 20 | 前端：添加 Agent 连接操作指南页面 | ✅ |
| 21 | 质量验证 | ✅ |

## 子任务详细设计

### 1 后端：提取公共 load_resource_config 函数

- **功能目标**：创建一个可复用的函数，从 DB 读取资源记录并解密 config_json，返回结构化的连接参数
- **文件结构**：
  - 修改：`crates/rex-hub/src/resource_conn.rs`（新建）
- **接口设计**：
  ```rust
  // crates/rex-hub/src/resource_conn.rs
  
  use anyhow::Result;
  use crate::AppState;
  use crate::crypto::CryptoManager;
  
  /// 从 DB 加载资源连接信息（host/port/username + config_json 解密）
  pub struct ResourceConnInfo {
      pub resource_id: String,
      pub protocol: String,
      pub host: String,
      pub port: Option<u16>,
      pub username: String,
      pub config: serde_json::Value,  // 解密后的 config_json
  }
  
  /// 从 DB 读取资源连接信息（含解密）
  /// 所有协议共用此函数，确保连接参数从 DB 而非前端获取
  pub fn load_resource_config(
      state: &AppState,
      resource_id: &str,
  ) -> Result<ResourceConnInfo> {
      let resource = state.db.get_resource(resource_id)?
          .ok_or_else(|| anyhow::anyhow!("resource not found: {}", resource_id))?;
      
      // 解密 config_json
      let config = if !resource.config_json.is_empty() && resource.config_json != "{}" {
          let decrypted = state.crypto.decrypt(&resource.config_json)?;
          serde_json::from_str(&decrypted)?
      } else {
          serde_json::Value::Null
      };
      
      Ok(ResourceConnInfo {
          resource_id: resource.id,
          protocol: resource.protocol,
          host: resource.host,
          port: resource.port,
          username: resource.username,
          config,
      })
  }
  ```
- **后端流程**：
  1. 创建 `resource_conn.rs` 文件
  2. 实现 `load_resource_config` 函数
  3. 在 `rex-hub.rs` 中添加 `mod resource_conn`
  4. 确保 `crypto.rs` 和 `db.rs` 的依赖正确
- **测试标准**：`cargo check` 通过
- **提交信息**：`feat(backend): add load_resource_config helper for unified resource connection`
### 1a 前端：修复 WizardModal S3 验证 bug

- **功能目标**：修复 `WizardModal.vue` 中 `validateStep()` 函数对 S3 协议的验证逻辑错误
- **问题根因**：`validateStep()` 第 99 行条件 `selectedProtocol.value !== 'sqlite'` 未排除 S3，导致 S3 资源在创建时被错误地要求填写 `host` 字段（S3 使用 `endpoint` 而非 `host`）
- **文件结构**：
  - 修改：`packages/rex-console-web/src/components/WizardModal.vue`
- **修复方案**：将条件从 `!== 'sqlite'` 改为 `!['sqlite', 's3'].includes(selectedProtocol.value)`，跳过 S3 的 host 必填验证
- **测试标准**：S3 资源在 WizardModal 中可正常创建，不被 host 验证阻断
- **提交信息**：`fix(wizard): skip host validation for S3 protocol in wizard modal`


### 2 后端：重构 sql_api.rs connect handler

- **功能目标**：SQL connect 端点改为 resource_id-based，从 DB 读取连接参数
- **文件结构**：
  - 修改：`crates/rex-hub/src/sql_api.rs`
- **接口设计**：
  ```rust
  // 新的 ConnectBody（简化）
  #[derive(Debug, Deserialize)]
  struct ConnectBody {
      #[serde(rename = "type")]
      db_type: String,
      resource_id: String,  // 必填，不再有 host/port/username/password
  }
  ```
- **后端流程**：
  1. 使用 `load_resource_config` 从 DB 读取资源
  2. 根据 `db_type` 构造对应的连接请求：
     - MySQL/PostgreSQL：`ConnectRequest { host, port, username, password: config.password, database: config.database_name }`
     - SQLite：`ConnectRequest { host: config.file_path, port: 0, username: "", password: None, database: None }`
  3. 调用对应的 connector 连接
  4. 移除旧的 `resource_id: Option<String>` 和 SQLite 特殊分支
- **测试标准**：`cargo check` 通过，SQLite 连接测试通过
- **提交信息**：`refactor(sql): connect handler uses resource_id-based DB lookup`

### 3 后端：重构 redis_api.rs connect handler

- **功能目标**：Redis connect 端点改为 resource_id-based
- **文件结构**：
  - 修改：`crates/rex-hub/src/redis_api.rs`
- **接口设计**：
  ```rust
  // 新的 ConnectBody（简化）
  #[derive(Debug, Deserialize)]
  struct ConnectBody {
      resource_id: String,  // 必填
  }
  ```
- **后端流程**：
  1. 使用 `load_resource_config` 从 DB 读取资源
  2. 从 config 提取 Redis 特有参数：`password`, `db`
  3. 构造 `RedisConnectRequest { host, port, password, db }`
  4. 调用 `RedisConnectorImpl::connect`
- **测试标准**：`cargo check` 通过
- **提交信息**：`refactor(redis): connect handler uses resource_id-based DB lookup`

### 4 后端：重构 file_api.rs connect handler

- **功能目标**：SFTP/S3 connect 端点改为 resource_id-based
- **文件结构**：
  - 修改：`crates/rex-hub/src/file_api.rs`
- **接口设计**：
  ```rust
  // 新的 ConnectBody（简化）
  #[derive(Debug, Deserialize)]
  struct ConnectBody {
      resource_id: String,  // 必填
  }
  ```
- **后端流程**：
  1. 使用 `load_resource_config` 从 DB 读取资源
  2. 根据 protocol 构造连接请求：
     - SFTP：从 config 提取 `password`, `private_key`，构造 `SshConfig`
     - S3：从 config 提取 `endpoint`, `access_key`, `secret_key`, `bucket`, `region`
  3. 调用对应的 connector 连接
- **测试标准**：`cargo check` 通过
- **提交信息**：`refactor(file): connect handler uses resource_id-based DB lookup`

### 5 前端：简化 SQL API 和 SqlPage

- **功能目标**：前端 SQL 连接只传 resource_id
- **文件结构**：
  - 修改：`packages/rex-console-web/src/api/sql.ts`
  - 修改：`packages/rex-console-web/src/features/sql/SqlPage.vue`
- **接口设计**：
  ```typescript
  // sql.ts - 简化 ConnectRequest
  export interface ConnectRequest {
    type: string
    resource_id: string  // 必填，移除 host/port/username/password/database
  }
  
  // SqlPage.vue - 简化 props
  const props = defineProps<{
    resourceId: string  // 必填
    protocol?: string   // mysql/postgresql/sqlite
  }>()
  ```
- **前端流程**：
  1. 修改 `sql.ts` 的 `ConnectRequest` 接口
  2. 修改 `SqlPage.vue` 的 props，移除 host/port/username/password/database
  3. 修改 `onMounted` 中的连接逻辑，只传 `{ type, resource_id }`
  4. 更新 i18n（如有需要）
- **测试标准**：`bun run type-check` 通过
- **提交信息**：`refactor(sql-frontend): simplify connect to resource_id only`

### 6 前端：简化 Redis API 和 RedisPage

- **功能目标**：前端 Redis 连接只传 resource_id
- **文件结构**：
  - 修改：`packages/rex-console-web/src/api/redis.ts`
  - 修改：`packages/rex-console-web/src/features/redis/RedisPage.vue`
- **接口设计**：
  ```typescript
  // redis.ts - 简化 connect 函数
  export async function connect(resource_id: string): Promise<string> {
    const res = await fetch(`${API_BASE}/connect`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ resource_id }),
    })
    if (!res.ok) throw new Error((await res.json()).error?.message || 'Connection failed')
    return (await res.json()).session_id
  }
  
  // RedisPage.vue - 简化 props
  const props = defineProps<{
    resourceId: string  // 必填，移除 host/port/password/db
  }>()
  ```
- **前端流程**：
  1. 修改 `redis.ts` 的 `connect` 函数签名
  2. 修改 `RedisPage.vue` 的 props，移除 host/port/password/db
  3. 修改 `onMounted` 和 `doConnect` 中的连接逻辑
  4. 移除连接表单中的 host/port/password 字段（保留手动连接作为调试入口）
- **测试标准**：`bun run type-check` 通过
- **提交信息**：`refactor(redis-frontend): simplify connect to resource_id only`

### 7 前端：简化 Files API 和 FilesPage

- **功能目标**：前端 SFTP/S3 连接只传 resource_id
- **文件结构**：
  - 修改：`packages/rex-console-web/src/api/files.ts`
  - 修改：`packages/rex-console-web/src/features/files/FilesPage.vue`
- **接口设计**：
  ```typescript
  // files.ts - 简化 connect 函数
  export async function connect(resource_id: string): Promise<string> {
    const res = await fetch(`${API_BASE}/connect`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...authHeaders() },
      body: JSON.stringify({ resource_id }),
    })
    if (!res.ok) throw new Error((await res.json()).error?.message || 'Connection failed')
    return (await res.json()).session_id
  }
  
  // FilesPage.vue - 简化 props
  const props = defineProps<{
    resourceId: string  // 必填，移除 host/port/username/password/protocol
    protocol?: string   // sftp/s3（可选，用于 UI 显示）
  }>()
  ```
- **前端流程**：
  1. 修改 `files.ts` 的 `connect` 函数签名
  2. 修改 `FilesPage.vue` 的 props，移除 host/port/username/password
  3. 修改 `doConnect` 中的连接逻辑
  4. 移除连接表单中的手动输入字段（保留作为调试入口）
- **测试标准**：`bun run type-check` 通过
- **提交信息**：`refactor(files-frontend): simplify connect to resource_id only`

### 8 前端：简化 WorkspacePage Tab 创建

- **功能目标**：Tab 创建时只传 resourceId，不传连接参数
- **文件结构**：
  - 修改：`packages/rex-console-web/src/pages/WorkspacePage.vue`
- **接口设计**：
  ```typescript
  // 简化 Tab interface
  interface Tab {
    id: string
    label: string
    protocol: 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3'
    resourceId: string  // 必填
    environmentId?: string
    status: 'connecting' | 'connected' | 'disconnected' | 'error'
    color?: string
    renaming?: boolean
    broadcast?: boolean
    // Terminal settings（仅 SSH）
    theme?: string
    fontSize?: number
    opacity?: number
    cursorStyle?: string
    cursorBlink?: boolean
    backgroundImage?: string
    encoding?: string
  }
  
  // 简化 openResourceFromTree
  function openResourceFromTree(node: {
    id: string
    name: string
    protocol?: string
    environmentId?: string
  }) {
    // 只传 resourceId 和 protocol，不传 host/port/username
    tabs.value.push({
      id: `tab-${Date.now()}`,
      label: node.name,
      protocol: (node.protocol || 'ssh') as Tab['protocol'],
      resourceId: node.id,
      environmentId: node.environmentId,
      status: 'connecting',
    })
  }
  ```
- **前端流程**：
  1. 修改 `Tab` interface，移除 host/port/username/password/database
  2. 修改 `openResourceFromTree` 函数，只传必要的字段
  3. 修改模板中各协议组件的 props 绑定，只传 resourceId
  4. 更新 `ResourceInfo` interface（stores/workspace.ts）移除不必要的字段
- **测试标准**：`bun run type-check` 通过，`bun run lint` 通过
- **提交信息**：`refactor(workspace): simplify Tab creation to resource_id only`

### 9 前端：ResourceProperties 按协议类型显示编辑字段

- **功能目标**：修复 ResourceProperties 弹窗按协议类型显示对应字段，而非固定显示 SSH 字段
- **问题根因**：`ResourceProperties.vue` 的 Connection tab 硬编码了 Name/Protocol/Host/Port/Color，Auth tab 硬编码了 SSH 认证方式；不支持 MySQL/PostgreSQL 的 database_name、Redis 的 db、SQLite 的 file_path、S3 的 endpoint/access_key/secret_key/bucket 等协议特有字段
- **文件结构**：
  - 修改：`packages/rex-console-web/src/features/workspace/ResourceProperties.vue`
  - 修改：`packages/rex-console-web/src/pages/WorkspacePage.vue`（Tab interface 增加 config_json 字段）
- **接口设计**：
  - Tab interface 新增 `configJson?: Record<string, unknown>` 字段
  - ResourceProperties props 新增 `configJson?: Record<string, unknown>`
  - Connection tab 根据 `form.protocol` 条件渲染不同字段：
    - SSH/SFTP/MySQL/PostgreSQL/Redis：Host + Port
    - SQLite：File Path（替代 Host/Port）
    - S3：Endpoint + Bucket + Region
  - Auth tab 仅 SSH 显示 Password/Key File；其他协议显示 Password（MySQL/PG/Redis/SFTP）或不显示
  - 非 SSH 协议隐藏 Terminal/Appearance/Keepalive tabs
- **测试标准**：`bun run type-check` 通过
- **提交信息**：`fix(ui): ResourceProperties shows protocol-specific fields`

### 10 前端：修复 /api/settings PUT 422 session_timeout 类型错误

- **功能目标**：修复 SettingsPage 保存时 session_timeout 类型不匹配导致 422 错误
- **问题根因**：后端 `update_settings` 期望 `HashMap<String, String>`，前端发送数字 `30` 而非字符串 `"30"`
- **修复方案**：在 `saveSettings()` 中将 `session_timeout` 转为字符串后再发送
- **文件结构**：修改 `packages/rex-console-web/src/pages/SettingsPage.vue`
- **提交信息**：`fix(settings): send session_timeout as string to match backend API`

### 11 前端：修复 StatusDot 连接中状态一直闪烁黄灯

- **功能目标**：修复所有连接标签右侧 StatusDot 始终显示黄色脉冲（connecting 状态）
- **问题根因**：Tab 的 `status` 字段初始值为 `'connecting'`，连接成功后未更新为 `'connected'`
- **修复方案**：检查各协议页面的连接状态回调，确保连接成功后更新 Tab status 为 `'connected'`
- **文件结构**：修改 `WorkspacePage.vue`、各协议页面的连接回调
- **提交信息**：`fix(ui): update tab status to connected after successful connection`

### 12 前端：修复 SSH 终端底部 2 行被截断

- **功能目标**：修复 SSH 终端底部约 2 行内容被容器裁剪
- **问题根因**：`.ws-ssh-area` 或 `TerminalView` 容器的 CSS 高度/溢出设置不当
- **修复方案**：检查终端容器 CSS，确保 `overflow: hidden` 不裁剪内容，调整 `height: 100%` 配合 flexbox
- **文件结构**：修改 `WorkspacePage.vue`（`.ws-ssh-area` 样式）、`TerminalView.vue`（终端容器样式）
- **提交信息**：`fix(terminal): prevent bottom rows from being clipped by container`

### 13 前端：修复 SSH vim 操作后终端卡死

- **功能目标**：修复 SSH 终端中使用 vim 编辑文件后终端无法响应
- **问题根因**：vim 使用alternate screen buffer，xterm.js 可能未正确处理 CSI 序列或 resize 事件
- **修复方案**：检查 `useTerminal.ts` 中的 terminal 配置，确保 `altScreen` 相关设置正确；检查 resize 事件处理
- **文件结构**：修改 `packages/rex-console-web/src/features/terminal/useTerminal.ts`
- **提交信息**：`fix(terminal): handle vim alt-screen buffer correctly`

### 14 后端：修复 SSH 空闲连接断开过快（keepalive）

- **功能目标**：修复 SSH 连接空闲一段时间后自动断开
- **问题根因**：`keepalive_interval` 配置未设置或默认值过低，导致服务端或 NAT 超时断开
- **修复方案**：在 SSH 连接初始化时设置默认 keepalive 间隔（如 60 秒），并在资源配置中允许用户自定义
- **文件结构**：修改 `crates/rex-ssh/src/lib.rs`（keepalive 默认值）
- **提交信息**：`fix(ssh): set default keepalive interval to prevent idle disconnect`

### 15 前端：修复审计日志页面无法下翻滚动

- **功能目标**：修复审计日志页面内容超出视口时无法滚动查看
- **问题根因**：AuditLogPage 容器缺少 `overflow-y: auto` 或高度未约束
- **修复方案**：为审计日志容器添加滚动样式
- **文件结构**：修改 `packages/rex-console-web/src/pages/AuditLogPage.vue`
- **提交信息**：`fix(audit): enable scrolling for audit log page`

### 16 前端：修复收藏夹无收藏资源按钮

- **功能目标**：在收藏夹面板中添加收藏资源的操作入口
- **问题根因**：收藏夹功能只有列表展示，缺少"收藏"操作按钮
- **修复方案**：在资源列表或资源详情中添加"收藏"按钮，调用 favorites store
- **文件结构**：修改 `packages/rex-console-web/src/stores/favorites.ts`、相关页面组件
- **提交信息**：`fix(favorites): add resource favorite button to UI`

### 17 前端：修复 SSH SFTP 按钮点击后底部空栏

- **功能目标**：修复 SSH 终端中点击 SFTP 按钮后底部出现空面板
- **问题根因**：SFTP Drawer 组件内容未正确加载或布局异常
- **修复方案**：检查 FilesDrawer 组件的加载逻辑和 CSS 布局
- **文件结构**：修改 `packages/rex-console-web/src/features/files/FilesDrawer.vue`
- **提交信息**：`fix(sftp): show file browser content in SFTP drawer`

### 18 前端：修复 MobileTerminalBar 上下键显示 ~A ~B

- **功能目标**：修复移动端终端栏上下键发送错误的转义序列
- **问题根因**：MobileTerminalBar 将上下键作为字符输入而非终端转义序列
- **修复方案**：将上下键映射为正确的 ANSI 转义序列（`\x1b[A` / `\x1b[B`）
- **文件结构**：修改 `packages/rex-console-web/src/features/terminal/MobileTerminalBar.vue`
- **提交信息**：`fix(terminal): send correct escape sequences for arrow keys on mobile`

### 19 前端：修复 SQLite 修改配置后无法读取内容

- **功能目标**：修复 SQLite 资源修改配置（如更换数据库文件路径）后打开仍无法读取
- **问题根因**：修改配置后未重新建立连接，或缓存的 session_id 过期
- **修复方案**：在资源配置变更时清除旧的 session，重新连接
- **文件结构**：修改 SQL 连接逻辑
- **提交信息**：`fix(sqlite): reconnect after config change`

### 20 前端：添加 Agent 连接操作指南

- **功能目标**：在 Agents 页面添加 Agent 连接操作指南，指导用户如何部署和连接 Agent
- **文件结构**：修改 `packages/rex-console-web/src/pages/AgentsPage.vue`
- **交互设计**：在 Agent 列表为空时显示引导卡片，包含部署步骤和配置说明
- **提交信息**：`feat(agents): add agent deployment guide to agents page`
### 21 质量验证

- **功能目标**：确保所有改动通过质量门禁
- **文件结构**：无新文件
- **后端流程**：
  1. `cargo fmt --check`
  2. `cargo clippy --workspace --all-targets`
  3. `cargo test --workspace`
- **前端流程**：
  1. `bun run type-check`
  2. `bun run lint`
  3. `bun run build`
- **测试标准**：所有检查通过
- **提交信息**：`chore: quality gate verification for M49`

## 设计核对点

1. **数据流完整性**：所有协议的连接参数必须从 DB 读取，前端不传递敏感信息
2. **向后兼容**：保持 test-connection 逻辑不变，保持资源创建流程不变
3. **安全性**：password/private_key 等敏感信息不应出现在前端 props 或 API 请求体中
4. **一致性**：所有协议使用相同的 load_resource_config 模式
5. **错误处理**：resource_id 无效时返回清晰的错误信息

## Flow Status

  - [x] 步骤1：编写里程碑文档
  - [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
