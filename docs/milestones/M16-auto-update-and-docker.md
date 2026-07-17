# M16: Agent 自动更新 + Docker 打包

## Context

M0–M15 完成了从项目骨架到 Agent WebSocket 隧道的全部功能。Agent 现在可以通过 WebSocket 隧道代理内网资源连接，但更新机制尚未实现——Hub 和 Agent 各自独立部署，版本一致性无法保证。

M16 实现完整的更新链路：Hub 提供版本检查 API，Agent 接收更新指令并执行替换，Docker 镜像打包多架构 Agent 二进制。这是产品"自托管"定位的关键一环——用户不需要手动 SSH 到每台服务器更新 Agent。

本里程碑版本类型：minor（新功能），版本号 0.16.0 → 0.17.0。

## 产品边界

**本阶段做：**
- Hub 版本检查 API（当前版本 + 最新版本 + 下载链接）
- Hub 更新触发 API（向 Agent 发送更新指令）
- Agent 更新处理器（接收指令 → 下载 → 校验 → 替换 → 重启）
- WebSocket 更新协议扩展（Hub ↔ Agent）
- Agent 更新状态跟踪（phase / progress / error）
- Hub Docker 镜像打包多架构 Agent 二进制（linux/amd64 + linux/arm64）
- Agent 二进制下载端点（优先从 Hub 下载，fallback 到 GitHub Releases）
- 自动更新开关（Agent 配置）

**本阶段不做：**
- Supervisor + worker 进程模型（简化为直接重启）
- 更新回滚机制（失败时回退到备份，但不做自动回滚轮询）
- Hub 自动更新（只做 Agent 更新，Hub 由用户手动或 Docker pull）
- 更新进度实时推送到前端（前端轮询即可）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Hub 版本检查 + 更新触发 API | ✅ |
| 2 | Agent 更新处理器 + WebSocket 协议扩展 | ✅ |
| 3 | Hub Docker 多架构打包 + Agent 二进制下载端点 | ✅ |
| 4 | 前端更新状态展示（Agent 管理页 + 设置页） | ✅ |

## 子任务详细设计

### 1 Hub 版本检查 + 更新触发 API

**功能目标**

Hub 提供版本信息查询和更新触发的 REST API，供前端和 Agent 使用。

**文件结构**

新建：
- `crates/rex-hub/src/update_api.rs` — 更新相关 API handler

修改：
- `crates/rex-hub/src/rex-hub.rs` — 注册更新路由
- `crates/rex-hub/src/app.rs` — 添加 `update_state: Arc<UpdateState>` 到 AppState
- `crates/rex-common/src/update.rs` — 更新状态模型（新增，替代现有的 update-state.json 逻辑）

**接口设计**

```
GET  /api/version                         → VersionInfo
GET  /api/agents/:id/update/check         → UpdateCheckResult
POST /api/agents/:id/update/trigger       → { ok: true }
GET  /api/agents/:id/update/status        → UpdateStatus
GET  /api/agents/download?target=x86_64-unknown-linux-musl → Binary file
```

**数据模型**

```rust
// crates/rex-common/src/update.rs

/// 版本信息
pub struct VersionInfo {
    pub hub_version: String,        // 当前 Hub 版本
    pub latest_version: String,     // 最新可用版本（从 GitHub API 获取）
    pub download_url: String,       // 最新版本下载链接
    pub agent_version: String,      // Hub 打包的 Agent 二进制版本
    pub agents: Vec<AgentVersion>,  // 各 Agent 的版本信息
}

pub struct AgentVersion {
    pub agent_id: String,
    pub name: String,
    pub version: String,
    pub is_online: bool,
    pub is_up_to_date: bool,        // version == hub_version
}

/// 更新检查结果
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub download_url: String,
    pub release_notes: String,
}

/// 更新状态（Agent 侧）
pub struct UpdateStatus {
    pub phase: UpdatePhase,         // idle / downloading / verifying / replacing / restarting / error
    pub progress: f64,              // 0.0 ~ 1.0
    pub current_version: String,
    pub target_version: String,
    pub error: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UpdatePhase {
    Idle,
    Downloading,
    Verifying,
    Replacing,
    Restarting,
    Error,
}

/// 更新指令（Hub → Agent）
pub struct UpdateCommand {
    pub version: String,
    pub download_url: String,       // 优先 Hub 下载地址
    pub fallback_url: String,       // GitHub Releases 下载地址
    pub sha256: String,             // 校验和
}

/// 更新进度（Agent → Hub）
pub struct UpdateProgress {
    pub phase: UpdatePhase,
    pub progress: f64,
    pub error: Option<String>,
}
```

**后端流程**

1. `GET /api/version` — 读取 Hub 当前版本（从 Cargo.toml 编译时注入）、GitHub 最新版本（缓存 5 分钟）、所有 Agent 版本
2. `POST /api/agents/:id/update/trigger` — 向指定 Agent 的 WebSocket 连接发送 `update` 消息
3. `GET /api/agents/:id/update/status` — 查询 Agent 的更新状态（从内存中的 UpdateState 读取）
4. `GET /api/agents/download?arch=x86_64` — 从 Hub 内嵌的 Agent 二进制目录提供下载，不存在则返回 404
5. GitHub 版本检查使用 `reqwest` 调用 GitHub Releases API，结果缓存在内存中（TTL 5 分钟）

**GitHub Releases API**

```
GET https://api.github.com/repos/{owner}/{repo}/releases/latest
→ { tag_name, body, assets: [{ name, browser_download_url, size }] }
```

Agent 二进制命名约定：`rex-agent-{version}-{os}-{arch}`
- `rex-agent-0.17.0-linux-amd64`
- `rex-agent-0.17.0-linux-arm64`
- `rex-agent-0.17.0-windows-amd64.exe`

下载端点：`GET /api/agents/download?os=linux&arch=amd64`
Docker 内嵌路径：`/usr/local/lib/rex/agents/{os}/{arch}/rex-agent`

**测试标准**

- `GET /api/version` 返回正确版本信息
- `POST /api/agents/:id/update/trigger` 向 Agent 发送更新消息
- `GET /api/agents/download` 返回二进制文件或 404
- GitHub API 缓存生效（5 分钟内不重复请求）
- cargo clippy + cargo test 通过

**提交信息**

```
feat(hub): add version check and update trigger API
```

### 2 Agent 更新处理器 + WebSocket 协议扩展

**功能目标**

Agent 端实现 supervisor + worker 进程模型。Worker 处理业务逻辑和更新下载，Supervisor 负责进程监控和二进制替换。

核心原则：**Worker 从不替换正在运行的二进制，只写 update-state.json 后退出。Supervisor（PID 1）负责实际替换。**

```
┌─ Supervisor (PID 1) ──────────────────┐
│  1. 启动 worker                       │
│  2. 监控 worker 退出码                 │
│  3. 退出码=42 → 读 update-state.json   │
│  4. rename(tmp → rex-agent)           │
│  5. 启动新 worker                     │
│  6. 退出码≠42 → 正常退出              │
└───────────────────────────────────────┘
```

Docker / systemd / 手动运行都适用：
- Docker：PID 1 = supervisor，容器不销毁
- systemd：`Restart=always` 拉起 supervisor
- 手动：supervisor 本身就是前台进程

**文件结构**

新建：
- `crates/rex-agent/src/supervisor.rs` — supervisor 进程（启动 worker、监控退出、替换二进制）
- `crates/rex-agent/src/updater.rs` — 更新处理器（下载、校验、写 update-state.json）

修改：
- `crates/rex-agent/src/agent_ws.rs` — 处理 `update` 消息，扩展 WebSocket 协议
- `crates/rex-agent/src/rex-agent.rs` — 入口改为启动 supervisor（而非直接运行 worker）

**WebSocket 协议扩展**

```
═══ 更新指令（Hub → Agent）═══
← Hub:   { "type": "update", "payload": { "version": "0.17.0", "download_url": "http://hub:3000/api/agents/download?arch=x86_64", "fallback_url": "https://github.com/.../rex-agent-0.17.0-x86_64-unknown-linux-musl.tar.gz", "sha256": "abc123..." } }

═══ 更新进度（Agent → Hub，定期报告）═══
→ Agent: { "type": "update_progress", "payload": { "phase": "downloading", "progress": 0.35 } }
→ Agent: { "type": "update_progress", "payload": { "phase": "verifying", "progress": 1.0 } }
→ Agent: { "type": "update_progress", "payload": { "phase": "replacing", "progress": 1.0 } }
→ Agent: { "type": "update_progress", "payload": { "phase": "restarting" } }

═══ 更新失败（Agent → Hub）═══
→ Agent: { "type": "update_progress", "payload": { "phase": "error", "error": "SHA256 mismatch" } }
```

**更新流程**

```
Worker 运行中 ← Hub 发送 update 消息
  ↓
Worker: 检查 auto_update → 关闭则忽略
  ↓
Worker: 报告 phase: downloading → 下载二进制（优先 download_url，失败用 fallback_url）
  ↓
Worker: 报告 phase: verifying → 计算 SHA256 → 与预期对比
  ↓
Worker: 校验通过 → 写 update-state.json { version, sha256, tmp_path }
  ↓
Worker: report phase: restarting → exit(42)    ← 特殊退出码表示有更新
  ↓
Supervisor (PID 1) 检测到 worker 退出
  ↓
Supervisor: 退出码 == 42 → 读 update-state.json
  ↓
Supervisor: rename(tmp → rex-agent) → 设置权限
  ↓
Supervisor: 启动新 worker（新版本）
```

如果 worker 异常崩溃（退出码非 0 且非 42）：
- Supervisor 检查 update-state.json 是否存在 → 存在则替换 → 启动新版本
- 不存在则直接重启同一版本（最多重试 3 次，超过则退出）


**下载实现**

```rust
// crates/rex-agent/src/updater.rs

pub struct AgentUpdater {
    hub_url: String,
    agent_token: String,
    current_exe: PathBuf,  // 当前二进制路径
}

impl AgentUpdater {
    /// 执行更新
    pub async fn update(&self, cmd: UpdateCommand) -> Result<()> {
        // 1. 下载
        let bytes = self.download(&cmd).await?;

        // 2. 校验 SHA256
        let hash = sha256::digest(&bytes);
        if hash != cmd.sha256 {
            bail!("SHA256 mismatch: expected {}, got {}", cmd.sha256, hash);
        }

        // 3. 备份
        let backup = self.current_exe.with_extension("bak");
        fs::copy(&self.current_exe, &backup)?;

        // 4. 替换（原子写入：写临时文件 → rename）
        let tmp = self.current_exe.with_extension("tmp");
        fs::write(&tmp, &bytes)?;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
        fs::rename(&tmp, &self.current_exe)?;

        // 5. 重启（退出当前进程，由外部管理器重启）
        std::process::exit(0);
    }

    /// 下载二进制（优先 Hub，fallback GitHub）
    async fn download(&self, cmd: &UpdateCommand) -> Result<Vec<u8>> {
        match self.try_download(&cmd.download_url).await {
            Ok(bytes) => Ok(bytes),
            Err(e) => {
                tracing::warn!("Hub download failed: {}, trying fallback", e);
                self.try_download(&cmd.fallback_url).await
            }
        }
    }

    async fn try_download(&self, url: &str) -> Result<Vec<u8>> {
        // HTTP GET → stream → bytes
        // 支持 .tar.gz 解压（只取 rex-agent 二进制）
        todo!()
    }
}
```

**备份与恢复**

- 备份路径：`{current_exe}.bak`
- 每次更新前备份上一版本
- 如果新版本启动失败（如 SHA256 不匹配导致写入失败），备份仍在
- 用户可手动恢复：`cp rex-agent.bak rex-agent`

**配置**

```rust
// 环境变量配置
REX_AUTO_UPDATE=true       // 自动更新开关（默认 true）
REX_HUB_URL=ws://hub:3000  // Hub URL
REX_AGENT_TOKEN=xxx        // 注册令牌
```

**测试标准**

- Agent 收到 update 消息 → 下载 → 校验 → 替换 → 退出
- SHA256 不匹配 → 报告 error → 不替换
- Hub 下载失败 → 自动 fallback 到 GitHub
- auto_update=false → 忽略更新指令
- 备份文件正确创建
- cargo clippy + cargo test 通过

**提交信息**

```
feat(agent): add update handler with download, verify, and replace
```

### 3 Hub Docker 多架构打包 + Agent 二进制下载端点

**功能目标**

Hub Docker 镜像内置多架构 Agent 二进制，通过 API 提供下载。用户部署 Agent 时直接从 Hub 下载，无需访问外网。

**文件结构**

新建：
- `Dockerfile`（workspace 根）— 多阶段构建，编译 Hub + Agent
- `docker-compose.yml`（workspace 根）— 开发用 compose
- `scripts/build-agent-bins.sh` — 多架构交叉编译脚本
- `.github/workflows/docker.yml`（可选）— CI 自动构建

修改：
- `crates/rex-hub/src/update_api.rs` — 添加 Agent 二进制下载端点

**Dockerfile 设计**

```dockerfile
# 阶段 1：编译所有架构的 Agent 二进制
FROM --platform=$BUILDPLATFORM rust:1.82-slim AS agent-builder
WORKDIR /src
COPY . .
# 交叉编译各架构 Agent（只编译 rex-agent crate）
RUN cargo build --release --bin rex-agent --target x86_64-unknown-linux-musl
RUN cargo build --release --bin rex-agent --target aarch64-unknown-linux-musl

# 阶段 2：编译 Hub（native 架构）
FROM rust:1.82-slim AS hub-builder
WORKDIR /src
COPY . .
RUN cargo build --release --bin rex-hub

# 阶段 3：运行时
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=hub-builder /src/target/release/rex-hub /usr/local/bin/
COPY --from=agent-builder /src/target/x86_64-unknown-linux-musl/release/rex-agent /usr/local/lib/rex/agents/linux/amd64/
COPY --from=agent-builder /src/target/aarch64-unknown-linux-musl/release/rex-agent /usr/local/lib/rex/agents/linux/arm64/
COPY packages/rex-console-web/dist /usr/local/lib/rex/static/
RUN mkdir -p /usr/local/lib/rex/agents/linux/amd64 /usr/local/lib/rex/agents/linux/arm64
EXPOSE 3000
ENV REX_PORT=3000
CMD ["rex-hub"]
```

**Agent 二进制下载端点**

```
GET /api/agents/download?arch=x86_64
  → 200: Agent 二进制（application/octet-stream）
  → 404: 该架构二进制不存在

GET /api/agents/download?arch=aarch64
  → 同上
```

**下载目录优先级**

```
1. /usr/local/lib/rex/agents/{arch}/  (Docker 内嵌)
2. ~/.rex/agents/{arch}/              (本地缓存目录)
3. GitHub Releases                    (外部下载)
```

**支持的平台**

| 平台 | os | arch | 二进制文件名 |
|------|-----|------|-------------|
| Linux x86_64 | `linux` | `amd64` | `rex-agent-0.17.0-linux-amd64` |
| Linux ARM64 | `linux` | `arm64` | `rex-agent-0.17.0-linux-arm64` |
| Linux ARMv7 | `linux` | `armv7` | `rex-agent-0.17.0-linux-armv7` |
| Windows x86_64 | `windows` | `amd64` | `rex-agent-0.17.0-windows-amd64.exe` |
| macOS x86_64 | `darwin` | `amd64` | `rex-agent-0.17.0-darwin-amd64` |
| macOS ARM64 | `darwin` | `arm64` | `rex-agent-0.17.0-darwin-arm64` |

M16 先做 Linux + Windows，macOS 后续添加。格式与 Docker 一致：`{os}/{arch}`。

**交叉编译脚本**

```bash
#!/bin/bash
# scripts/build-agent-bins.sh
# 编译所有平台的 Agent 二进制

# {rust_target} → {os}/{arch}
declare -A PLATFORMS=(
  ["x86_64-unknown-linux-musl"]="linux/amd64"
  ["aarch64-unknown-linux-musl"]="linux/arm64"
  ["x86_64-pc-windows-msvc"]="windows/amd64"
)

OUT_DIR="target/agent-bins"
mkdir -p "$OUT_DIR"

for rust_target in "${!PLATFORMS[@]}"; do
  os_arch="${PLATFORMS[$rust_target]}"
  echo "Building for $rust_target → $os_arch"
  cargo build --release --bin rex-agent --target "$rust_target"

  # 复制到统一输出目录
  mkdir -p "$OUT_DIR/$os_arch"
  cp "target/$rust_target/release/rex-agent" "$OUT_DIR/$os_arch/"
done

echo "All agent binaries built in $OUT_DIR/"
```

**测试标准**

- `docker build` 成功构建包含 Hub + Agent 二进制的镜像
- `docker run` 启动 Hub 后，`GET /api/agents/download?arch=x86_64` 返回 Agent 二进制
- 交叉编译脚本能编译 x86_64 和 aarch64
- cargo clippy + cargo test 通过
- `bun run type-check` + `bun run lint` + `bun run build` 通过

**提交信息**

```
feat(docker): add multi-arch Hub Docker image with embedded Agent binaries
feat(hub): add Agent binary download endpoint with fallback chain
```

### 4 前端更新状态展示

**功能目标**

在 Agent 管理页和设置页展示版本信息和更新状态，提供手动触发更新的入口。

**文件结构**

修改：
- `packages/rex-console-web/src/pages/AgentsPage.vue` — Agent 卡片增加版本和更新状态
- `packages/rex-console-web/src/pages/SettingsPage.vue` — 更新区块：Hub 版本 + 各 Agent 版本 + 检查更新按钮
- `packages/rex-console-web/src/api/agents.ts` — 添加更新相关 API 调用

**Agent 管理页改动**

- Agent 卡片：新增版本标签（`v0.16.0` 灰色 / `v0.17.0` 绿色"最新" / `v0.15.0` 橙色"可更新"）
- Agent 详情弹窗：更新状态区域（phase + progress 进度条 + error 信息）
- 触发更新按钮（仅在线 Agent 显示）→ 确认弹窗 → 触发后显示进度

**设置页改动**

- 更新区块：
  - Hub 当前版本（v0.16.0）
  - 检查更新按钮 → 显示最新版本 + release notes
  - Agent 版本总览表格（名称 / 版本 / 状态 / 操作）

**API 调用**

```typescript
// packages/rex-console-web/src/api/agents.ts

export const updateApi = {
  checkUpdate(agentId: string): Promise<UpdateCheckResult> {
    return api.get(`/api/agents/${agentId}/update/check`)
  },

  triggerUpdate(agentId: string): Promise<void> {
    return api.post(`/api/agents/${agentId}/update/trigger`)
  },

  getUpdateStatus(agentId: string): Promise<UpdateStatus> {
    return api.get(`/api/agents/${agentId}/update/status`)
  },

  getVersion(): Promise<VersionInfo> {
    return api.get('/api/version')
  },
}
```

**交互设计**

1. 用户点击"检查更新" → loading → 显示版本对比
2. 点击"更新 Agent" → 确认弹窗（"将更新 Agent 到 v0.17.0，Agent 将重启"）
3. 确认后 → 轮询更新状态（每 2 秒）→ 进度条展示
4. 更新完成后 → Agent 重连 → 状态恢复在线

**测试标准**

- Agent 管理页正确显示版本信息和更新状态
- 设置页版本总览正确
- 触发更新流程正常（按钮 → 确认 → 进度 → 完成）
- type-check + lint + build 通过

**提交信息**

```
feat(web): add version display and update controls to Agent and Settings pages
```

## 设计核对点

- [ ] `GET /api/version` 返回 Hub + Agent 版本信息
- [ ] `POST /api/agents/:id/update/trigger` 通过 WebSocket 发送更新指令
- [ ] Agent 收到更新指令 → 下载 → SHA256 校验 → 替换 → 重启
- [ ] Hub 下载失败自动 fallback 到 GitHub Releases
- [ ] auto_update=false 时忽略更新指令
- [ ] Docker 镜像包含 x86_64 + aarch64 Agent 二进制
- [ ] `GET /api/agents/download` 按架构返回二进制
- [ ] Agent 管理页显示版本和更新状态
- [ ] 设置页显示版本总览和检查更新
- [ ] 更新进度实时展示（轮询）
- [ ] cargo test 通过
- [ ] type-check + lint + build 通过

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
