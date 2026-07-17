# M14: 收尾

## Context

M0–M13 完成了从设计系统到管理页面的全部功能开发。但关键的非功能性需求尚未完成：页面全部使用硬编码中文（无 i18n 调用）、主题切换不持久化、响应式布局覆盖不足、凭据明文存储、无任何测试。M14 是产品发布前的收尾里程碑。

本里程碑版本类型：minor（新功能/增强），版本号 0.14.0 → 0.15.0。

## 产品边界

**本阶段做：**
- i18n 完整翻译（所有页面使用 $t()，zh/en 完整覆盖）
- 主题持久化（深色/浅色/跟随系统 → 保存到后端设置 + localStorage 缓存）
- 响应式适配（移动端底部导航、工作区移动端、各页面响应式布局）
- 凭据 AES-256-GCM 加密存储（资源密码/私钥）
- Rust 测试（公开函数和关键逻辑路径）

**本阶段不做：**
- 新功能开发
- 前端组件测试（仅验证当前功能，不依赖外部服务）
- 性能优化（已有功能已满足需求）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | i18n 完整翻译（所有页面 $t() + zh/en 翻译文件） | ✅ |
| 2 | 主题持久化（后端设置 API + 前端同步） | ✅ |
| 3 | 响应式适配（移动端底部导航 + 页面响应式） | ✅ |
| 4 | 凭据 AES-256-GCM 加密存储 | ✅ |
| 5 | Rust 测试覆盖 | ✅ |

## 子任务详细设计

### 1 i18n 完整翻译

**功能目标**

所有页面文本使用 `$t()` 调用，中文/英文翻译文件完整覆盖。

**文件结构**

修改：
- `packages/rex-console-web/src/i18n/zh.ts` — 补全所有页面的中文翻译 key
- `packages/rex-console-web/src/i18n/en.ts` — 补全所有页面的英文翻译 key
- `packages/rex-console-web/src/pages/DashboardPage.vue` — 替换硬编码为 $t()
- `packages/rex-console-web/src/pages/EnvironmentsPage.vue` — 替换硬编码为 $t()
- `packages/rex-console-web/src/pages/EnvironmentDetailPage.vue` — 替换硬编码为 $t()
- `packages/rex-console-web/src/pages/AgentsPage.vue` — 替换硬编码为 $t()
- `packages/rex-console-web/src/pages/AuditLogPage.vue` — 替换硬编码为 $t()
- `packages/rex-console-web/src/pages/SettingsPage.vue` — 替换硬编码为 $t()
- `packages/rex-console-web/src/pages/LoginPage.vue` — 替换硬编码为 $t()
- `packages/rex-console-web/src/layouts/AppLayout.vue` — 替换硬编码为 $t()
- `packages/rex-console-web/src/features/workspace/ConnectionTree.vue` — 替换硬编码为 $t()
- `packages/rex-console-web/src/features/resource/WizardModal.vue` — 替换硬编码为 $t()
- 所有 features/ 下的组件 — 替换硬编码为 $t()

**翻译 key 命名规范**

```typescript
// 按功能域组织
{
  // 通用
  common: { save, cancel, delete, confirm, loading, success, error, ... },
  // 导航
  nav: { workspace, dashboard, environments, agents, auditLog, settings, ... },
  // 各页面
  dashboard: { title, envCount, resourceCount, onlineAgents, todayOps, ... },
  environments: { title, create, edit, delete, description, ... },
  agents: { title, status, online, offline, version, ... },
  auditLog: { title, time, action, target, result, ... },
  settings: { title, appearance, terminal, security, language, theme, ... },
  workspace: { connectionTree, search, noConnections, ... },
  login: { title, password, login, setupPassword, ... },
  wizard: { step1Title, step2Title, step3Title, step4Title, ... },
  // 协议
  protocol: { ssh, mysql, postgresql, redis, sqlite, sftp, s3, ... },
}
```

**测试标准**

- 所有页面模板中无裸露中文字符串（grep 确认）
- 切换语言后所有文本正确切换
- type-check + lint + build 通过

**提交信息**

```
feat(i18n): complete zh/en translations for all pages
```

### 2 主题持久化

**功能目标**

主题选择（深色/浅色/跟随系统）持久化到后端设置 API，页面加载时恢复。

**文件结构**

修改：
- `packages/rex-console-web/src/stores/settings.ts`（或相关 store） — 主题状态同步到后端
- `packages/rex-console-web/src/i18n/index.ts` — 初始化时从 localStorage 读取主题
- `packages/rex-console-web/src/App.vue` — 启动时应用持久化的主题

流程：
1. 用户在设置页切换主题 → 调用 `PUT /api/settings` 保存 `theme` 字段
2. 同时写入 `localStorage.setItem('rex-theme', value)` 作为缓存
3. 页面加载时：先从 localStorage 读取立即应用（避免闪烁），再从后端 API 同步

**后端已有 `PUT /api/settings` 支持 `theme` 字段**，无需新建 API。

**测试标准**

- 切换主题 → 刷新页面 → 主题保持
- 清除 localStorage → 从后端恢复主题
- type-check + lint + build 通过

**提交信息**

```
feat(web): persist theme selection across sessions
```

### 3 响应式适配

**功能目标**

移动端底部导航、工作区移动端适配、各管理页面响应式布局。

**文件结构**

修改：
- `packages/rex-console-web/src/layouts/AppLayout.vue` — 移动端底部导航（5 图标：仪表盘/环境/+/Agent/设置）
- `packages/rex-console-web/src/pages/DashboardPage.vue` — 卡片单列、统计卡片堆叠
- `packages/rex-console-web/src/pages/EnvironmentsPage.vue` — 卡片单列
- `packages/rex-console-web/src/pages/AgentsPage.vue` — 卡片单列
- `packages/rex-console-web/src/pages/AuditLogPage.vue` — 表格横向滚动
- `packages/rex-console-web/src/pages/SettingsPage.vue` — 单列布局
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — 侧栏折叠、Tab 栏紧凑

**交互设计**

移动端（< 768px）：
- 侧栏隐藏，通过汉堡菜单临时打开
- 底部固定导航栏（5 个图标）
- 卡片网格 → 单列
- 表格 → 横向滚动或卡片列表
- 工作区 Tab 栏紧凑滚动

**测试标准**

- 768px 以下断点布局正确
- 底部导航可跳转
- type-check + lint + build 通过

**提交信息**

```
feat(web): add responsive layout for mobile devices
```

### 4 凭据 AES-256-GCM 加密存储

**功能目标**

资源密码和 SSH 私钥使用 AES-256-GCM 加密后存储到 SQLite。

**文件结构**

新建：
- `crates/rex-hub/src/crypto.rs` — AES-256-GCM 加密/解密工具函数

修改：
- `crates/rex-hub/src/db.rs` — create_resource/update_resource 时加密 password，get_resource 时解密
- `crates/rex-hub/src/models.rs` — Resource 返回类型不包含解密后的密码（安全）
- `crates/rex-hub/src/resource_api.rs` — test-connection 时解密密码再传给 connector

**接口设计**

```rust
// crypto.rs
pub struct CredentialCrypto {
    key: [u8; 32],  // AES-256 key
}

impl CredentialCrypto {
    pub fn new(master_key: &[u8]) -> Self { ... }
    pub fn encrypt(&self, plaintext: &str) -> Result<String> { ... }  // base64(nonce + ciphertext + tag)
    pub fn decrypt(&self, encrypted: &str) -> Result<String> { ... }
}
```

**密钥派生**

使用 PBKDF2 从固定盐值 + Hub 启动时生成的 machine-id 派生加密密钥。密钥存储在 `REX_DATA_DIR/.master-key` 文件中（仅首次生成）。

**后端流程**

1. Resource 创建/更新：`password` 字段 → `crypto.encrypt(password)` → 存储到 `password_enc` 列
2. Resource 查询（详情）：不返回密码字段（安全考虑）
3. 测试连接 / 实际连接：从 DB 读取 `password_enc` → `crypto.decrypt()` → 传给 connector
4. `private_key` 字段同理加密

**测试标准**

- 创建资源（含密码）→ 数据库中 password_enc 为加密文本
- 测试连接能正确解密密码并连接
- 直接查看数据库看不到明文密码
- cargo clippy + cargo test 通过

**提交信息**

```
feat(security): add AES-256-GCM encryption for stored credentials
```

### 5 Rust 测试覆盖

**功能目标**

rex-common 和 rex-hub 的公开函数和关键逻辑路径有测试覆盖。

**文件结构**

修改：
- `crates/rex-common/src/config.rs` — 添加配置解析测试
- `crates/rex-common/src/models.rs` — 添加模型测试
- `crates/rex-hub/src/crypto.rs` — 加密/解密单元测试
- `crates/rex-hub/src/db.rs` — 数据库操作测试（使用临时 SQLite）
- `crates/rex-hub/src/auth.rs` — JWT 签发/验证测试

**测试策略**

- 单元测试放在模块内 `#[cfg(test)] mod tests`
- 数据库测试使用 `tempfile::NamedTempFile` 创建临时 SQLite
- 不依赖外部服务（不测真实 SSH/MySQL 连接）
- 测试覆盖：配置解析、加密解密、JWT 签发验证、CRUD 操作

**测试标准**

- `cargo test --workspace` 全部通过
- `cargo clippy --workspace --all-targets` 无 error
- 关键路径覆盖：auth、crypto、db CRUD、config parsing

**提交信息**

```
test: add unit tests for crypto, auth, db, and config
```

## 设计核对点

- [ ] 所有页面使用 $t()，无硬编码中文
- [ ] 中英文切换功能正常
- [ ] 主题切换刷新后保持
- [ ] 移动端底部导航可用
- [ ] 密码在数据库中为加密文本
- [ ] 测试连接能正确解密密码
- [ ] cargo test 通过
- [ ] type-check + lint + build 通过

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
