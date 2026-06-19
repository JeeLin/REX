# M7: 自动更新替换和回滚

## Context

M6 实现了更新检测能力（GitHub Releases 查询、版本对比、设置页提示）。M7 实现自动更新的下载、校验、替换和回滚。参考 `docs/PRODUCT.md` §自更新机制 阶段 2 和 `docs/DEVELOPMENT.md` §27.9 里程碑 8。

## 产品边界

**做什么：**
- update-state.json 状态管理（原子写入）
- staging 下载目录 + rollback 备份目录
- SHA256 校验
- Unix/Linux/macOS 原子替换
- worker 退出码（10 请求更新、11 健康失败、12 崩溃）
- REX_UPDATE_PENDING 环境变量
- Hub 新 worker 健康检查
- Agent 新 worker 健康检查
- 连续 3 次失败回滚
- 设置页"下载更新"按钮触发实际下载和替换流程

**不做什么：**
- Windows supervisor 副本逻辑（后续阶段）
- Docker 内更新（依赖镜像拉取，不做二进制替换）
- Agent 自主下载更新（Hub 下载后 Agent 通过 API 获取）
- 签名验证（仅 SHA256 校验）

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 7.1 | update-state.json 状态模型 + 原子读写 | 后端 | ✅ |
| 7.2 | 下载 + staging + rollback 目录管理 | 后端 | ✅ |
| 7.3 | SHA256 校验 | 后端 | ✅ |
| 7.4 | supervisor 替换逻辑（退出码 + 状态机） | 后端 | ✅ |
| 7.5 | 健康检查 + 回滚机制 | 后端 | ✅ |
| 7.6 | 下载/更新 REST API | 后端 | ✅ |
| 7.7 | 前端设置页"下载更新"按钮 | 前端 | ✅ |

---

## 子任务 7.1：update-state.json 状态模型 + 原子读写

### 功能目标

实现更新状态文件的 Schema 定义、原子读写（写入临时文件后 rename）。

### 文件结构

```text
crates/rex-common/src/
└── update_state.rs    新增：更新状态模型和读写
```

### 接口设计

```rust
/// 更新状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateState {
    pub phase: UpdatePhase,
    pub target_version: String,
    pub old_version: String,
    pub staged_path: String,
    pub rollback_path: String,
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UpdatePhase {
    Idle,
    Requested,
    StartingNew,
    Committed,
    RollingBack,
    RolledBack,
    Failed,
}

impl UpdateState {
    /// 读取状态文件，不存在返回 Idle
    pub fn read(path: &Path) -> Self;
    
    /// 原子写入状态文件（先写 .tmp 再 rename）
    pub fn write(&self, path: &Path) -> Result<()>;
}
```

### 原子写入

```text
1. 写入 {path}.tmp
2. fsync
3. rename {path}.tmp → {path}（原子操作）
```

### 测试标准

- 默认状态为 Idle
- 写入后读取一致
- 不存在的文件返回默认 Idle
- 原子写入不产生中间状态

### 提交信息

```
feat: add update state model with atomic read/write
```

---

## 子任务 7.2：下载 + staging + rollback 目录管理

### 功能目标

实现二进制下载到 staging 目录、备份当前二进制到 rollback 目录。

### 文件结构

```text
crates/rex-common/src/
└── updater.rs    修改：增加下载和目录管理功能
```

### 目录结构

```text
{data-dir}/
├── updates/
│   ├── staging/      下载的新二进制
│   └── rollback/     备份的旧二进制
└── update-state.json
```

### 接口设计

```rust
impl UpdateChecker {
    /// 下载新版本二进制到 staging 目录
    pub async fn download_update(
        &self,
        data_dir: &Path,
    ) -> Result<String>;  // 返回 staged_path
    
    /// 备份当前二进制到 rollback 目录
    pub fn backup_current(data_dir: &Path) -> Result<String>;  // 返回 rollback_path
}
```

### 下载流程

```text
1. 确定当前平台（linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64, windows-x86_64）
2. 从 GitHub Release assets 找对应文件
3. 下载到 {data-dir}/updates/staging/rex-{version}
4. chmod +x
5. 返回路径
```

### 测试标准

- 目录创建正确
- 下载文件路径正确
- 备份路径正确

### 提交信息

```
feat: add binary download and staging/rollback directories
```

---

## 子任务 7.3：SHA256 校验

### 功能目标

下载后校验文件 SHA256，与 release 中的 checksums 比较。

### 文件结构

```text
crates/rex-common/src/
└── updater.rs    修改：增加校验功能
```

### 接口设计

```rust
/// 计算文件 SHA256
pub fn sha256_file(path: &Path) -> Result<String>;

/// 下载并验证 SHA256SUMS
pub async fn verify_download(
    binary_path: &Path,
    checksums_url: &str,
    expected_filename: &str,
) -> Result<bool>;
```

### 测试标准

- 计算 SHA256 正确
- 校验通过返回 true
- 校验失败返回 false
- checksums 文件缺失时返回 error

### 提交信息

```
feat: add SHA256 verification for updates
```

---

## 子任务 7.4：supervisor 替换逻辑

### 功能目标

实现 supervisor 的二进制替换逻辑：读取 update-state.json，执行替换，启动新 worker。

### 文件结构

```text
crates/rex-common/src/
└── supervisor.rs    修改：增加更新替换逻辑
```

### 替换流程

```text
supervisor 收到 worker 退出（code 0 或 10）
  ↓
读取 update-state.json
  ↓
phase == requested?
  ├─ 否 → 正常重启
  └─ 是 → 执行替换
       ↓
    backup current → rollback_path
       ↓
    rename staged_path → 当前二进制路径
       ↓
    写入 phase = starting_new
       ↓
    启动新 worker，传入 REX_UPDATE_PENDING=1
       ↓
    等待退出
```

### 退出码处理

```rust
// supervisor 核心循环（伪代码）
fn handle_exit(code: i32, state: &mut UpdateState) -> Action {
    match code {
        0 => {
            if state.phase == UpdatePhase::Requested {
                Action::ReplaceAndRestart
            } else {
                Action::Restart
            }
        }
        10 => {
            if state.attempt >= 3 {
                Action::Rollback
            } else {
                state.attempt += 1;
                Action::ReplaceAndRestart
            }
        }
        11 | 12 => {
            state.attempt += 1;
            if state.attempt >= 3 {
                Action::Rollback
            } else {
                Action::Restart
            }
        }
        _ => Action::Restart,
    }
}
```

### REX_UPDATE_PENDING

新 worker 启动时通过环境变量传入：
- `REX_UPDATE_PENDING=1` — 表示这是更新后的首次启动
- supervisor 启动新 worker 时设置此变量

### 测试标准

- 替换逻辑正确执行
- 退出码 10 + phase requested → 替换
- 退出码 11/12 + attempt >= 3 → 回滚
- REX_UPDATE_PENDING 正确传递

### 提交信息

```
feat: add supervisor binary replacement logic
```

---

## 子任务 7.5：健康检查 + 回滚机制

### 功能功能

新 worker 启动后检查健康状态，连续失败则回滚到旧版本。

### 文件结构

```text
crates/rex-common/src/
└── supervisor.rs    修改：增加健康检查和回滚
```

### 健康检查流程

```text
新 worker 启动（REX_UPDATE_PENDING=1）
  ↓
supervisor 等待 30 秒
  ↓
检查 worker 是否仍在运行
  ├─ 是 → 健康，写入 phase = committed，删除 rollback
  └─ 否 → 不健康
       ↓
    attempt + 1
    attempt >= 3?
       ├─ 是 → 回滚：rename rollback_path → 当前二进制，重启
       └─ 否 → 重启新 worker
```

### Hub 健康检查

```text
新 worker 启动后调用自身 GET /healthz
  成功 → 健康
  失败 → 不健康
```

### Agent 健康检查

```text
新 worker 启动后尝试连接 Hub WebSocket
  成功 → 健康
  失败 → 不健康
```

### 回滚

```text
1. 写入 phase = rolling_back
2. rename rollback_path → 当前二进制路径
3. 写入 phase = rolled_back
4. 重启 worker（不传 REX_UPDATE_PENDING）
5. worker 正常运行 → 写入 phase = idle
```

### 测试标准

- 健康检查通过 → committed
- 健康检查失败 → 重启
- 连续 3 次失败 → 回滚
- 回滚后旧版正常运行

### 提交信息

```
feat: add health check and rollback mechanism
```

---

## 子任务 7.6：下载/更新 REST API

### 功能目标

提供下载和更新的 REST API，供前端设置页调用。

### 文件结构

```text
crates/rex-hub/src/
└── update.rs    修改：增加下载和更新 API
```

### 接口设计

```
POST /api/update/download     — 下载新版本（异步）
GET  /api/update/download/progress  — 查询下载进度
POST /api/update/apply        — 应用更新（写入 state + 重启）
```

### 数据模型

```rust
#[derive(Debug, Serialize)]
pub struct DownloadProgress {
    pub status: String,    // "downloading" | "verifying" | "ready" | "error"
    pub percent: u32,
    pub message: String,
}
```

### 测试标准

- 下载 API 返回进度信息
- apply API 正确写入 update-state.json
- 状态流转正确

### 提交信息

```
feat: add download and apply update REST API
```

---

## 子任务 7.7：前端设置页"下载更新"按钮

### 功能目标

在设置页更新区块增加"下载更新"按钮，触发实际下载和替换流程。

### 文件结构

```text
packages/rex-console-web/src/
├── features/settings/
│   └── UpdateSection.vue    修改：增加下载按钮和进度
└── api/
    └── update.ts            修改：增加下载 API
```

### 前端交互

```text
发现新版本 v0.2.0
  ↓
点击 [下载更新]
  ↓
显示进度：下载中... 45%
  ↓
校验中...
  ↓
就绪：可以重启更新
  ↓
点击 [立即更新]
  ↓
页面显示"更新中，请稍候..."
  ↓
自动刷新页面
```

### i18n keys

```typescript
settings: {
  update: {
    downloading: '下载中...',
    verifying: '校验中...',
    ready: '可以重启更新',
    applyNow: '立即更新',
    updating: '更新中，请稍候...',
    downloadFailed: '下载失败',
  }
}
```

### 测试标准

- 下载按钮可用
- 进度正确显示
- 校验成功后显示"立即更新"
- 点击更新后页面刷新

### 提交信息

```
feat: add download and apply buttons to settings page
```

## 设计核对点

- [ ] 不引入 Windows supervisor 副本逻辑
- [ ] 不在 Docker 内做二进制替换
- [ ] 原子写入不产生中间状态
- [ ] SHA256 校验在替换前执行
- [ ] 回滚后旧版正常运行
- [ ] 不会进入无限更新循环（attempt < 3 限制）
- [ ] Agent 更新后保持同一 Agent ID（agent.json 不动）
- [ ] REX_UPDATE_PENDING 只在更新后首次启动时设置

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [ ] 步骤8：提交
