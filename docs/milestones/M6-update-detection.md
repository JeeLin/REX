# M6: 自动更新检测

## Context

M0-M5b 完成了项目骨架、Hub 管理 API、Agent 连接、SSH 终端、文件传输、SQL 控制台。M6 实现自动更新检测能力：Hub 定期查询 GitHub Releases 检测新版本，Agent 心跳上报版本信息，设置页展示更新状态。参考 `docs/PRODUCT.md` §自更新机制 阶段 1 和 `docs/DEVELOPMENT.md` §27.8 里程碑 7。

## 产品边界

**做什么：**
- Hub worker 定期查询 GitHub Releases 检测新版本
- 解析当前平台对应 release asset，比较版本号
- 设置页显示"有新版本"更新提示
- Agent 心跳上报版本号和 SHA256
- Agent 页面显示 Hub/Agent 版本总览
- Hub 判断 Agent 是否需要更新
- 前端设置页更新区块（开关、版本信息）

**不做什么：**
- 自动下载/替换二进制（M7）
- SHA256 校验（M7）
- rollback 机制（M7）
- staging 目录（M7）
- Windows supervisor 副本逻辑（M7）

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 6.1 | 版本信息模块（version.rs） | 后端 | ✅ |
| 6.2 | GitHub Releases 更新检查器 | 后端 | ✅ |
| 6.3 | Agent 心跳上报版本 + Hub 版本对比 | 后端 | ✅ |
| 6.4 | 更新状态 REST API | 后端 | ✅ |
| 6.5 | 设置页更新区块 | 前端 | ✅ |
| 6.6 | Agent 页面版本总览 | 前端 | ✅ |

---

## 子任务 6.1：版本信息模块

### 功能目标

创建版本信息模块，提供当前版本号、构建时间、Git commit 等静态信息，供更新检查和 Agent 心跳使用。

### 文件结构

```text
crates/rex-common/src/
└── version.rs    新增：版本信息模块
```

### 接口设计

```rust
/// 当前版本（编译时嵌入）
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Git commit hash（编译时嵌入）
pub const GIT_COMMIT: &str = env!("GIT_COMMIT_HASH");

/// 构建时间
pub const BUILD_TIME: &str = env!("BUILD_TIME");

/// 版本信息响应
#[derive(Debug, Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub git_commit: String,
    pub build_time: String,
    pub rust_version: String,
}

impl VersionInfo {
    pub fn current() -> Self;
}
```

### 测试标准

- `VersionInfo::current()` 返回正确的版本信息
- 编译时嵌入的版本号与 Cargo.toml 一致

### 提交信息

```
feat: add version info module with compile-time constants
```

---

## 子任务 6.2：GitHub Releases 更新检查器

### 功能目标

实现 GitHub Releases 更新检查器，定期查询最新版本并比较版本号。

### 文件结构

```text
crates/rex-common/src/
└── updater.rs    新增：更新检查器
```

### 接口设计

```rust
/// GitHub Release 信息
#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub published_at: String,
    pub assets: Vec<GitHubAsset>,
}

/// 版本比较结果
#[derive(Debug, PartialEq)]
pub enum UpdateStatus {
    UpToDate,
    UpdateAvailable {
        current: String,
        latest: String,
        download_url: String,
        release_notes: String,
    },
    CheckFailed(String),
}

/// 更新检查器
pub struct UpdateChecker {
    repo: String,          // "owner/repo"
    current_version: String,
    check_interval: Duration,
    http_client: reqwest::Client,
}

impl UpdateChecker {
    pub fn new(repo: &str, current_version: &str) -> Self;
    
    /// 检查是否有新版本
    pub async fn check_for_update(&self) -> UpdateStatus;
    
    /// 后台定期检查（spawn 一个 tokio task）
    pub fn start_periodic_check(
        self,
        state: Arc<AppState>,
    ) -> tokio::task::JoinHandle<()>;
}
```

### GitHub API 调用

```text
GET https://api.github.com/repos/{owner}/{repo}/releases/latest
Accept: application/vnd.github.v3+json
```

### 版本比较

使用 semver 比较：
- 移除 `v` 前缀
- 比较 major.minor.patch
- 仅比较同平台（linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64, windows-x86_64）

### 测试标准

- 版本比较逻辑正确（v0.1.0 < v0.2.0 < v1.0.0）
- 无效版本号处理
- 网络错误处理
- API 限流处理（GitHub 60 req/h 未认证）

### 提交信息

```
feat: add GitHub Releases update checker
```

---

## 子任务 6.3：Agent 心跳上报版本 + Hub 版本对比

### 功能目标

扩展 Agent 心跳消息，上报版本号和二进制 SHA256。Hub 收到后判断 Agent 是否需要更新。

### 文件结构

```text
crates/rex-common/src/
└── agent.rs    修改：心跳消息增加版本字段

crates/rex-hub/src/
└── agent.rs    修改：心跳处理增加版本对比
```

### 接口设计

Agent 心跳消息扩展：

```rust
/// Agent 心跳请求
pub struct AgentHeartbeat {
    pub agent_id: String,
    pub status: String,
    pub version: String,           // 新增：Agent 版本号
    pub binary_sha256: String,     // 新增：二进制 SHA256
    pub device_info: DeviceInfo,
}
```

Hub 心跳响应扩展：

```rust
/// Agent 心跳响应
pub struct AgentHeartbeatResponse {
    pub hub_version: String,       // 新增：Hub 版本号
    pub needs_update: bool,        // 新增：是否需要更新
    pub update_url: Option<String>,// 新增：更新下载 URL
}
```

### 后端流程

```text
Agent 心跳到达
  ↓
Hub 提取 version + binary_sha256
  ↓
与 Hub 当前版本比较
  ↓
version != Hub version?
  ├─ 是 → needs_update = true, update_url = Some(...)
  └─ 否 → needs_update = false
  ↓
返回响应
```

### 测试标准

- Agent 心跳正确携带版本信息
- Hub 正确判断版本差异
- 版本相同返回 needs_update = false

### 提交信息

```
feat: add version reporting to agent heartbeat
```

---

## 子任务 6.4：更新状态 REST API

### 功能目标

提供更新状态的 REST API，供前端设置页和 Agent 页面查询。

### 文件结构

```text
crates/rex-hub/src/
├── update.rs    新增：更新状态 API handlers
└── routes.rs    修改：注册路由
```

### 接口设计

```
GET /api/update/status         — 获取 Hub 更新状态
GET /api/update/check          — 手动触发更新检查
GET /api/agents/versions       — 获取所有 Agent 版本信息
```

### 数据模型

```rust
/// Hub 更新状态
#[derive(Debug, Serialize)]
pub struct UpdateStatusResponse {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub last_checked: Option<String>,
    pub auto_check_enabled: bool,
}

/// Agent 版本信息
#[derive(Debug, Serialize)]
pub struct AgentVersionInfo {
    pub agent_id: String,
    pub name: String,
    pub version: String,
    pub binary_sha256: String,
    pub needs_update: bool,
    pub connected_at: String,
}
```

### 测试标准

- `/api/update/status` 返回当前版本和更新状态
- `/api/update/check` 触发检查并返回结果
- `/api/agents/versions` 返回所有 Agent 版本信息

### 提交信息

```
feat: add update status REST API
```

---

## 子任务 6.5：设置页更新区块

### 功能目标

在设置页实现更新区块，显示当前版本、检查更新按钮、更新提示。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/Settings.vue          新增：设置页
├── features/settings/
│   └── UpdateSection.vue       新增：更新区块组件
└── router.ts                   修改：添加设置页路由
```

### 前端交互

参考 `prototype/settings.html` 更新区块：

```text
┌─────────────────────────────────────────┐
│ 更新                                      │
│                                          │
│ 当前版本：v0.1.0 (abc1234)              │
│ 检查时间：2024-01-01 12:00:00           │
│                                          │
│ [检查更新]                               │
│                                          │
│ ✓ 已是最新版本                           │
│ ─ 或 ─                                   │
│ ⚠ 发现新版本 v0.2.0                     │
│ [下载更新]                               │
└─────────────────────────────────────────┘
```

### 组件设计

```vue
<!-- UpdateSection.vue -->
<template>
  <div class="update-section">
    <h3>{{ t('settings.update.title') }}</h3>
    <div class="version-info">
      <span>{{ t('settings.update.currentVersion') }}：{{ status.current_version }}</span>
      <span v-if="status.git_commit">({{ status.git_commit.slice(0, 7) }})</span>
    </div>
    <div class="last-checked" v-if="status.last_checked">
      {{ t('settings.update.lastChecked') }}：{{ formatTime(status.last_checked) }}
    </div>
    <button @click="checkUpdate" :disabled="checking">
      {{ checking ? t('settings.update.checking') : t('settings.update.checkNow') }}
    </button>
    <div v-if="status.update_available" class="update-available">
      ⚠ {{ t('settings.update.foundNew') }} {{ status.latest_version }}
      <button @click="downloadUpdate">{{ t('settings.update.download') }}</button>
    </div>
    <div v-else-if="checked" class="up-to-date">
      ✓ {{ t('settings.update.upToDate') }}
    </div>
  </div>
</template>
```

### i18n keys

```typescript
settings: {
  update: {
    title: '更新',
    currentVersion: '当前版本',
    lastChecked: '检查时间',
    checkNow: '检查更新',
    checking: '检查中...',
    foundNew: '发现新版本',
    download: '下载更新',
    upToDate: '已是最新版本',
    autoCheck: '自动检查更新',
  }
}
```

### 测试标准

- 设置页可访问
- 显示当前版本信息
- 检查更新按钮可用
- 更新提示正确显示
- i18n 中英文切换正常

### 提交信息

```
feat: add update section to settings page
```

---

## 子任务 6.6：Agent 页面版本总览

### 功能目标

在 Agent 页面显示所有 Agent 的版本信息和更新状态。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/Agents.vue            修改：增加版本列
└── features/agents/
    └── AgentVersionBadge.vue   新增：版本徽章组件
```

### 前端交互

Agent 表格增加版本列：

```text
| Agent 名称 | 版本 | 状态 | 设备信息 | 连接时间 | 操作 |
|-----------|------|------|----------|----------|------|
| prod-1    | v0.1.0 | 🟢 在线 | Linux x86_64 | 2h 前 | [下载] |
| dev-1     | v0.0.9 | 🟢 在线 | macOS arm64 | 1d 前 | [下载] ⚠ |
```

- 版本相同显示绿色
- 版本不同显示橙色 + ⚠ 图标
- 下载按钮根据平台推荐命令

### 组件设计

```vue
<!-- AgentVersionBadge.vue -->
<template>
  <span class="version-badge" :class="{ 'needs-update': needsUpdate }">
    {{ version }}
    <span v-if="needsUpdate" class="update-icon">⚠</span>
  </span>
</template>

<script setup lang="ts">
defineProps<{
  version: string
  needsUpdate: boolean
}>()
</script>
```

### i18n keys

```typescript
agent: {
  version: '版本',
  needsUpdate: '需要更新',
  download: '下载',
}
```

### 测试标准

- Agent 表格显示版本列
- 版本徽章正确显示状态
- 下载按钮可点击

### 提交信息

```
feat: add version overview to agent page
```

## 设计核对点

- [ ] 单用户、自托管定位：不引入多用户、RBAC
- [ ] 不自动下载/替换二进制（M7 再实现）
- [ ] GitHub API 限流处理（60 req/h 未认证）
- [ ] 版本比较使用 semver 规范
- [ ] 前端设置页与原型 settings.html 一致
- [ ] Agent 版本信息通过心跳上报，不引入额外连接
- [ ] 审计日志记录更新检查操作

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交
