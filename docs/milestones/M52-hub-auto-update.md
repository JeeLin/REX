# M52: Hub 自动更新机制（阶段2）

## Context

M51 完成登录安全增强 + 设置页完善（v0.44.0）后，产品核心功能已齐全。当前 Hub 进程模型是简单的 `if REX_WORKER → worker_main()`，缺少真正的 supervisor 监控和自动更新能力。Agent 更新机制（阶段1）已在 M16/M17 实现，但 Hub 自身无法自动更新。本里程碑实现 Hub 的自动更新闭环：检查 → 下载 → 校验 → 替换 → 健康检查 → 回滚。

版本类型：minor（新功能）
版本号：0.45.0

## 产品边界

**做什么**：
- Hub supervisor 真正的进程监控（spawn worker、监控退出码、重启）
- Hub 更新检查器（定期检查 GitHub Release）
- Hub 下载与校验（SHA256、备份、原子替换）
- 更新状态管理（update-state.json 读写）
- 更新状态 REST API（检查/状态/触发/回滚）
- 前端更新 UI（设置页显示版本、检查更新、进度、回滚）

**不做什么**：
- 不改变 Agent 更新机制（已实现）
- 不实现 ACME 自动证书（M17 已有基础框架）
- 不做 Docker 内自动更新（Docker 由用户管理镜像版本）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 后端：Supervisor 进程管理 + 更新引擎 | ✅ |
| 2 | 后端：更新状态 REST API | ✅ |
| 3 | 前端：设置页更新 UI | ✅ |
| 4 | 质量验证 | ✅ |

## 子任务详细设计

### 1 后端：Supervisor 进程管理 + 更新引擎

- **功能目标**：将 Hub 从简单的 `if/else` 改造为真正的 supervisor + worker 模式，supervisor 监控 worker 进程，处理 exit(10) 更新信号，执行二进制替换、健康检查和回滚
- **文件结构**：
  - 新建：`crates/rex-common/src/supervisor.rs`（进程监控、重启、退出码处理）
  - 新建：`crates/rex-hub/src/update_checker.rs`（GitHub Release 检查、下载、校验）
  - 修改：`crates/rex-hub/src/rex-hub.rs`（main 函数改造为 supervisor 模式）
  - 修改：`crates/rex-common/src/update.rs`（扩展 UpdateStateFile，增加 phase/attempt 字段）
  - 修改：`crates/rex-common/src/lib.rs`（导出 supervisor 模块）
- **接口设计**：
  ```rust
  // supervisor.rs
  pub struct SupervisorConfig {
      pub data_dir: PathBuf,
      pub health_url: String,         // e.g. "http://127.0.0.1:3000/api/health"
      pub max_restart_attempts: u32,  // default 3
  }

  pub fn run_supervisor(config: SupervisorConfig) -> Result<()>;
  // 内部：循环 spawn worker，监控退出码
  // exit(0) → 检查 update-state.json，决定是否替换二进制后重启
  // exit(10) → 读取 update-state.json，执行替换，重启
  // exit(11) / exit(12) → attempt+1，连续 3 次回滚
  // 其他退出码 → 重启 worker（attempt < max_restart_attempts）

  // update_checker.rs
  pub struct UpdateChecker {
      current_version: String,
      github_repo: String,  // e.g. "user/rex"
      data_dir: PathBuf,
  }

  impl UpdateChecker {
      pub async fn check_for_update(&self) -> Result<Option<UpdateInfo>>;
      pub async fn download_and_stage(&self, info: &UpdateInfo) -> Result<PathBuf>;
      pub fn write_update_state(&self, state: &UpdateStateFile) -> Result<()>;
  }

  // update.rs 扩展
  pub struct UpdateStateFile {
      pub phase: UpdatePhase,      // idle / requested / starting_new / committed / rolling_back / rolled_back / failed
      pub target_version: String,
      pub old_version: String,
      pub staged_path: String,
      pub rollback_path: String,
      pub attempt: u32,
  }

  pub enum UpdatePhase {
      Idle, Requested, StartingNew, Committed, RollingBack, RolledBack, Failed,
  }
  ```
- **交互设计**：无前端交互，纯后端逻辑
- **后端流程**：
  1. Hub 启动 → 父进程进入 supervisor 模式
  2. Supervisor spawn worker 子进程，设置 `REX_WORKER=1`
  3. Supervisor 后台线程每 6 小时检查 GitHub Release
  4. 发现新版本 → 下载二进制 + SHA256SUMS → 校验 → 写入 staging 路径
  5. 写 update-state.json（phase=requested）→ Worker 检测到更新信号 → 优雅退出（exit(10)）
  6. Supervisor 检测到 exit(10) → 读取 update-state.json → 原子替换二进制 → 重启 worker
  8. 新 worker 启动 → 检测到 REX_UPDATE_PENDING=1 → 只做健康检查
  9. 健康通过 → 写 phase=committed；健康失败 → exit(1) → supervisor attempt+1 → 回滚
- **测试标准**：supervisor 正确 spawn/monitor worker；exit(10) 触发更新流程；SHA256 校验正确；回滚逻辑正确
- **提交信息**：`feat(update): implement Hub supervisor process management and update engine`

### 2 后端：更新状态 REST API

- **功能目标**：提供更新状态查询、手动触发检查、触发更新的 REST 端点
- **文件结构**：
  - 修改：`crates/rex-hub/src/update_api.rs`（添加 Hub 更新相关 handler）
  - 修改：`crates/rex-hub/src/rex-hub.rs`（注册新路由）
- **接口设计**：
  ```
  GET  /api/update/check     → { has_update: bool, current_version: String, latest_version: String, download_url: String }
  POST /api/update/trigger   → { ok: true }  // 触发后台下载+更新
  GET  /api/update/status    → { phase: String, target_version: String, attempt: u32, error: Option<String> }
  POST /api/update/rollback  → { ok: true }  // 回滚到旧版本
  ```
- **交互设计**：前端通过这些端点获取更新状态
- **后端流程**：
  - `/check`：调用 UpdateChecker 检查 GitHub Release，返回版本对比
  - `/trigger`：启动后台 tokio task 下载+校验+写 update-state → worker 优雅退出
  - `/status`：读取 update-state.json 返回当前阶段
  - `/rollback`：写 update-state.json phase=rolling_back → supervisor 恢复旧版
- **测试标准**：所有端点返回正确格式；并发调用安全
- **提交信息**：`feat(update): add Hub update status REST API`

### 3 前端：设置页更新 UI

- **功能目标**：在设置页添加 Hub 更新区块，显示当前版本、检查更新、显示进度、支持回滚
- **文件结构**：
  - 修改：`packages/rex-console-web/src/pages/SettingsPage.vue`（添加更新区块）
  - 修改：`packages/rex-console-web/src/api/settings.ts`（添加更新 API）
  - 修改：`packages/rex-console-web/src/i18n/locales/zh.json`（更新相关 i18n key）
  - 修改：`packages/rex-console-web/src/i18n/locales/en.json`（更新相关 i18n key）
- **接口设计**：
  ```typescript
  // settings.ts
  updateApi: {
    check: () => api.get<{ has_update: boolean; current_version: string; latest_version: string; download_url: string }>('/update/check'),
    trigger: () => api.post<{ ok: boolean }>('/update/trigger'),
    status: () => api.get<{ phase: string; target_version: string; attempt: number; error: string | null }>('/update/status'),
    rollback: () => api.post<{ ok: boolean }>('/update/rollback'),
  }
  ```
- **交互设计**：
  - 设置页「更新」区块：当前版本号 +「检查更新」按钮
  - 检查后有新版本：显示最新版本号 +「下载并更新」按钮
  - 更新中：进度文字（下载中/校验中/替换中/重启中）+ loading 动画
  - 更新失败：显示错误 +「回滚」按钮
  - 回滚后：显示已回滚到旧版本
- **测试标准**：设置页正确显示版本；检查更新按钮可用；更新/回滚状态正确显示
- **提交信息**：`feat(settings): add Hub update UI with check/download/rollback`

### 4 质量验证

- **功能目标**：确保所有改动通过质量门禁
- **后端流程**：`cargo fmt --check` + `cargo clippy` + `cargo test`
- **前端流程**：`bun run type-check` + `bun run lint` + `bun run build`
- **测试标准**：所有检查通过
- **提交信息**：`chore: quality gate verification for M52`

## 设计核对点

1. **进程安全**：supervisor 正确处理 worker 异常退出（非 exit(10)），避免重启死循环
2. **原子替换**：二进制替换使用 rename 原子操作，避免替换中断导致二进制损坏
3. **回滚可靠**：健康检查失败时正确恢复旧版二进制，attempt 达到上限后停止重试
4. **版本检查**：GitHub Release 检查有超时和错误处理，不影响正常服务
5. **update-state.json**：原子写入（tmp → fsync → rename），避免 supervisor 读到半写入文件
6. **兼容性**：update-state.json schema 扩展向后兼容（新字段使用 default）

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

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
| [x] | 🟡 | 资源编辑字段不匹配协议类型 | 用户反馈 | 新建资源时可以选择协议类型，不同类型填入的数据不同，但编辑时只有固定的几个字段，导致无法完整编辑资源 |
| [x] | 🟡 | xterm-char-measure-element 显示乱码 | 用户反馈 | xterm-char-measure-element 有时显示 "dddddddddddddddddddddddddddddddd" 或 "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~" |
| [x] | 🟡 | SSH 终端最后两行看不到 | 用户反馈 | SSH 终端底部内容被截断，最后两行无法显示 |
| [x] | 🟡 | 切换标签 SSH 终端会重连 | 用户反馈 | 从 SSH 标签切换到其他标签再切回时，SSH 连接会重新建立 |
| [x] | 🟡 | agent-token-row 显示空的 | 用户反馈 | Agent token 行显示为空，按键显示 common.copy |
| [x] | 🟡 | redis_codec.rs 放置位置不合理 | 架构审查 | redis_codec.rs 只被 rex-redis 使用，应移至 rex-redis 而非 rex-common |
| [x] | 🔴 | write_update_state 缺少 fsync | 步骤5代码审查 | supervisor.rs:162-169 只做 write+rename，缺少 File::sync_all()，崩溃时可能读到半写入 JSON |
| [x] | 🔴 | Supervisor 未实现健康检查 | 步骤5代码审查 | health_url 配置存在但从未使用，替换二进制后不执行健康检查 |
| [x] | 🔴 | 回滚流程断裂 | 步骤5代码审查 | rollback_update API 写入 RollingBack 但 supervisor 从不检查此 phase |
| [x] | 🔴 | exit(10) 在 tokio::spawn 中强制终止 | 步骤5代码审查 | 会中断异步清理（DB连接池、日志刷盘），应改为退出标志+优雅退出 |
| [x] | 🔴 | background_update_task 中 exit(10) | 步骤5代码审查 | 同上问题，应改为退出标志+优雅退出 |
