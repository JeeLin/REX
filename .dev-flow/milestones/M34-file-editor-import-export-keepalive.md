# M34: 文件编辑器 + 连接导入导出 + SSH 保活

## Context

M33 完成了 S3/SFTP 上传续传功能。M34 为文件管理模块补全「编辑」功能（临时下载→应用内编辑→保存回传），增加连接配置的导入/导出（方便备份和迁移），以及 SSH 连接保活（防止长连接被中间设备断开）。

版本类型：minor（新功能），版本号 0.32.0 → 0.33.0。

## 产品边界

**本阶段做：**
- 应用内文件编辑器（Monaco Editor 集成，临时下载→编辑→保存回传）
- 连接配置导入/导出（JSON 格式，环境+资源批量管理）
- SSH 连接保活（KeepAlive 配置，防止断线）

**本阶段不做：**
- 本地文件浏览器（真正的双面板本地/远程，复杂度高，留待后续）
- 在线文件 diff/merge
- 编辑器多 Tab（当前每次编辑打开一个编辑器对话框即可）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 文件编辑器后端：read_for_edit + save_from_edit API | ✅ |
| 2 | 文件编辑器前端：CodeMirror Editor 编辑对话框 | ✅ |
| 3 | 连接配置导入/导出 API + 前端 | ✅ |
| 4 | SSH 连接保活：后端 KeepAlive | ✅ |

## 子任务详细设计

### 1 文件编辑器后端：read_for_edit + save_from_edit API

**功能目标**

为文件编辑器提供「读取文件内容」和「保存回写」两个 API 端点。支持 SFTP 和 S3 两种协议。

**文件结构**

修改：
- `crates/rex-common/src/file_transfer.rs` — FileConnector trait 新增 `read_for_edit` 和 `save_from_edit` 方法
- `crates/rex-ssh/src/sftp.rs` — 实现 SFTP read_for_edit / save_from_edit
- `crates/rex-s3/src/lib.rs` — 实现 S3 read_for_edit / save_from_edit
- `crates/rex-hub/src/file_api.rs` — 新增 read-for-edit / save-from-edit 路由

**接口设计**

FileConnector trait 扩展：
```rust
/// 读取文件内容用于编辑（小文件）
async fn read_for_edit(&mut self, path: &str) -> Result<Vec<u8>>;

/// 从编辑器保存文件内容（覆盖写入）
async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> Result<()>;
```

HTTP API：
```
GET  /api/files/read-for-edit?session_id=...&path=...
POST /api/files/save-from-edit  { session_id, path, content(base64) }
```

**设计决策**
- `read_for_edit` 限制最大 5MB，超过返回错误（不适合编辑的大文件）
- `save_from_edit` 直接覆盖写入（编辑器场景不需要 append）
- SFTP：直接读取整个文件 / 直接写入覆盖
- S3：get_object / put_object 覆盖写入

**SFTP 实现**
```rust
async fn read_for_edit(&mut self, path: &str) -> Result<Vec<u8>> {
    let mut file = self.session.open(path).await?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).await?;
    // 限制 5MB
    if data.len() > 5 * 1024 * 1024 {
        return Err(anyhow!("File too large for editing (>5MB)"));
    }
    Ok(data)
}

async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> Result<()> {
    let mut file = self.session.create(path).await?;
    file.write_all(&data).await?;
    file.flush().await?;
    Ok(())
}
```

**S3 实现**
```rust
async fn read_for_edit(&mut self, path: &str) -> Result<Vec<u8>> {
    let resp = self.client.get_object()
        .bucket(&self.bucket)
        .key(path)
        .send().await?;
    let bytes = resp.body.collect().await?.into_bytes().to_vec();
    if bytes.len() > 5 * 1024 * 1024 {
        return Err(anyhow!("File too large for editing (>5MB)"));
    }
    Ok(bytes)
}

async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> Result<()> {
    self.client.put_object()
        .bucket(&self.bucket)
        .key(path)
        .body(data.into())
        .send().await?;
    Ok(())
}
```

**后端流程**

1. 前端发送 `GET /api/files/read-for-edit?session_id=...&path=...`
2. 后端从连接池获取 connector，调用 `read_for_edit`
3. 返回 base64 编码的文件内容 + 文件名 + 大小
4. 前端编辑完成后发送 `POST /api/files/save-from-edit`
5. 后端解码 base64，调用 `save_from_edit` 写回

**测试标准**

- SFTP 文件可通过 read_for_edit 读取
- S3 文件可通过 read_for_edit 读取
- 大文件（>5MB）返回错误
- save_from_edit 正确覆盖写入
- type-check + build 通过

**提交信息**: `feat(files): add read_for_edit and save_from_edit API for file editor`

### 2 文件编辑器前端：Monaco Editor 编辑对话框

**功能目标**

在文件管理右键菜单「编辑」触发后，打开 Monaco Editor 对话框，支持语法高亮编辑、保存回传、另存为。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 右键菜单「编辑」触发编辑器
- `packages/rex-console-web/src/features/files/FileEditorDialog.vue` — **新建**，Monaco Editor 编辑对话框

新增依赖：
- `monaco-editor` — Monaco Editor

**接口设计**

FileEditorDialog props：
```typescript
interface Props {
  visible: boolean
  sessionId: string
  filePath: string
  protocol: 'sftp' | 's3'
  onClose: () => void
  onSaved: () => void  // 保存后回调（刷新文件列表）
}
```

files.ts 新增：
```typescript
export async function readForEdit(sessionId: string, path: string): Promise<{
  content: string  // base64
  filename: string
  size: number
}>

export async function saveFromEdit(sessionId: string, path: string, content: string): Promise<void>
```

**交互设计**

- 右键文件 → 点击「编辑」→ 打开编辑器对话框
- 对话框全屏（或 90vw × 85vh），顶部显示文件名 + 路径
- Monaco Editor 区域：语法高亮（根据文件扩展名自动检测语言）
- 底部工具栏：保存（Ctrl+S）| 另存为 | 文件信息（大小/编码）
- 保存时显示 loading，成功后 Toast 提示 + onSaved 回调
- 超过 5MB 的文件：显示「文件过大，无法编辑」提示

**文件扩展名→语言映射**
```typescript
const LANG_MAP: Record<string, string> = {
  '.ts': 'typescript', '.js': 'javascript', '.vue': 'html',
  '.py': 'python', '.rs': 'rust', '.go': 'go',
  '.json': 'json', '.yaml': 'yaml', '.yml': 'yaml',
  '.sql': 'sql', '.sh': 'shell', '.bash': 'shell',
  '.md': 'markdown', '.html': 'html', '.css': 'css',
  '.xml': 'xml', '.toml': 'ini', '.ini': 'ini',
  '.conf': 'plaintext', '.log': 'plaintext', '.txt': 'plaintext',
}
```

**实现流程**

1. 右键菜单点击「编辑」→ 调用 `readForEdit` API
2. 打开 FileEditorDialog，传入文件路径
3. Monaco Editor 加载内容，自动检测语言
4. 用户编辑，Ctrl+S 触发保存
5. 保存时调用 `saveFromEdit` API
6. 成功后 Toast + onSaved 回调

**测试标准**

- 右键文件 → 编辑 → 对话框打开 → 内容正确加载
- 语法高亮根据扩展名正确显示
- Ctrl+S 保存 → Toast 成功 → 文件列表刷新
- 超过 5MB 文件显示错误提示
- type-check + build 通过

**提交信息**: `feat(files): add Monaco Editor dialog for file editing`

### 3 连接配置导入/导出 API + 前端

**功能目标**

支持将环境+资源连接配置导出为 JSON 文件，以及从 JSON 文件导入。方便备份和跨实例迁移。

**文件结构**

修改：
- `crates/rex-hub/src/env_api.rs` — 新增 export / import 路由
- `packages/rex-console-web/src/api/environments.ts` — 新增 export/import API 调用
- `packages/rex-console-web/src/pages/EnvironmentsPage.vue` — 新增导入/导出按钮

**接口设计**

后端 API：
```
GET  /api/environments/export                  → JSON（所有环境+资源，密码加密）
POST /api/environments/import  { environments } → { imported: number, skipped: number }
```

导出格式（JSON）：
```json
{
  "version": "1.0",
  "exported_at": "2026-07-21T10:00:00Z",
  "environments": [
    {
      "name": "Production",
      "description": "...",
      "connection_mode": "direct",
      "resources": [
        {
          "name": "Web Server",
          "protocol": "ssh",
          "host": "192.168.1.100",
          "port": 22,
          "username": "admin",
          "config_json": "{ ... }",
          "color": "#3FB950"
        }
      ]
    }
  ]
}
```

**安全设计**
- 导出时密码/私钥经过加密后包含在 config_json 中
- 导入时按名称去重：同名环境→跳过；同环境下同名资源→跳过
- 导入结果返回 imported 和 skipped 数量

**后端实现**

```rust
async fn export_environments(State(state): State<AppState>) -> Json<ExportData> {
    let envs = state.db.list_environments().unwrap_or_default();
    let mut export_envs = Vec::new();
    for env in &envs {
        let resources = state.db.list_resources_by_env(&env.id).unwrap_or_default();
        export_envs.push(ExportEnvironment {
            name: env.name.clone(),
            description: env.description.clone(),
            connection_mode: env.connection_mode.clone(),
            resources: resources.into_iter().map(|r| ExportResource {
                name: r.name,
                protocol: r.protocol,
                host: r.host,
                port: r.port,
                username: r.username,
                config_json: r.config_json,  // 已加密
                color: r.color,
            }).collect(),
        });
    }
    Json(ExportData { version: "1.0".into(), environments: export_envs })
}
```

**前端交互**

环境管理页顶栏新增：
- 导出按钮：点击 → 下载 `rex-config-{date}.json`
- 导入按钮：点击 → 文件选择对话框 → 上传 → 显示结果 Toast

**测试标准**

- 导出：下载 JSON 文件，内容包含所有环境和资源
- 导入：选择 JSON 文件 → 导入成功 → 环境列表更新
- 重复导入同名环境 → 跳过 → Toast 显示 skipped 数量
- type-check + build 通过

**提交信息**: `feat(env): add environment config export/import API`

### 4 SSH 连接保活：后端 KeepAlive + 前端设置

**功能目标**

SSH 长连接支持 KeepAlive 探测，防止 NAT/防火墙超时断开。前端资源属性对话框可配置保活间隔。

**文件结构**

修改：
- `crates/rex-ssh/src/lib.rs` — SSH 连接支持 keepalive_interval 配置
- `crates/rex-common/src/file_transfer.rs` — FileConnectRequest 新增 keepalive 字段
- `crates/rex-hub/src/terminal_ws.rs` — WebSocket 终端传递 keepalive 配置
- `packages/rex-console-web/src/features/resource/ResourceProperties.vue` — 保活设置 UI

**接口设计**

FileConnectRequest 扩展：
```rust
pub struct FileConnectRequest {
    // ... 现有字段
    pub keepalive_interval: Option<u32>,  // 秒，默认 0（禁用）
}
```

SSH keepalive 实现：
```rust
// 在建立 SSH session 后设置
session.keepalive(true, keepalive_interval).await?;
```

**交互设计**

资源属性对话框 → 连接 Tab → 保活设置区域：
- 「启用 KeepAlive」开关
- 「间隔（秒）」输入框：默认 60，范围 10-300
- 提示文字：「定期发送探活包，防止长时间空闲后连接断开」

**测试标准**

- SSH 连接设置 keepalive 后，长时间空闲不断开
- 资源属性对话框显示保活设置
- 保活设置保存后生效
- type-check + build 通过

**提交信息**: `feat(ssh): add keepalive support for SSH connections`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ PRODUCT.md 3.8 要求「编辑（临时下载→编辑→保存回传）」
- ✅ 不引入多用户/RBAC 概念
- ✅ 文件编辑限制 5MB，不滥用资源
- ✅ 连接配置导入/导出保留加密，不泄露密码
- ✅ SSH KeepAlive 为可选配置，不强制

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

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
