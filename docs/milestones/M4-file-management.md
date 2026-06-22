# M4: 文件管理 + 前端 files 页面

## Context

M3 完成了 SSH 终端 + 前端 Terminal 页面。M4 实现文件传输功能，包含后端 FileConnector trait 和前端文件管理界面。文件传输数据不经过浏览器，前端只负责创建任务、展示进度和处理冲突。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | FileConnector trait（rex-transfer crate） | ⬜ |
| 2 | Hub 文件传输 API（/api/transfers） | ⬜ |
| 3 | 前端 files 页面初始化 | ⬜ |
| 4 | 前端文件列表 + 右键菜单 + 传输队列 | ⬜ |
| 5 | 资源创建向导 SFTP/FTP 表单 + 工作空间面板集成 | ⬜ |

## 子任务详细设计

### 1 FileConnector trait（rex-transfer crate）

- **功能目标**
  定义文件传输的统一接口，支持 SFTP（直连/Agent 代理）和 FTP 协议。

- **文件结构**
  - 修改：`crates/rex-transfer/Cargo.toml` — 添加 async-trait、tokio 依赖
  - 修改：`crates/rex-transfer/src/lib.rs` — 添加 FileConnector trait、TransferTask、FileItem 等类型

- **接口设计**
  ```rust
  pub struct FileItem {
      pub name: String,
      pub path: String,
      pub is_dir: bool,
      pub size: u64,
      pub modified: Option<chrono::DateTime<chrono::Utc>>,
  }

  pub struct TransferTask {
      pub id: String,
      pub source: String,
      pub target: String,
      pub direction: TransferDirection,
      pub status: TransferStatus,
      pub progress: f64,
      pub total_bytes: u64,
      pub transferred_bytes: u64,
  }

  pub enum TransferDirection { Upload, Download, RemoteCopy }
  pub enum TransferStatus { Pending, Running, Completed, Failed(String), Cancelled }

  #[async_trait]
  pub trait FileConnector: Send + Sync {
      async fn connect(&mut self) -> Result<()>;
      async fn list(&self, path: &str) -> Result<Vec<FileItem>>;
      async fn read(&self, path: &str) -> Result<Bytes>;
      async fn write(&self, path: &str, data: Bytes) -> Result<()>;
      async fn stat(&self, path: &str) -> Result<FileItem>;
      async fn rename(&self, from: &str, to: &str) -> Result<()>;
      async fn delete(&self, path: &str) -> Result<()>;
      async fn mkdir(&self, path: &str) -> Result<()>;
      async fn close(&self) -> Result<()>;
  }
  ```

- **测试标准**
  - FileItem 序列化/反序列化
  - TransferTask 状态机转换
  - FileConnector trait object safety

- **提交信息**
  `feat: add FileConnector trait and transfer types in rex-transfer`

### 2 Hub 文件传输 API

- **功能目标**
  Hub 端实现文件传输的 REST API 和 WebSocket 进度推送。

- **文件结构**
  - 新增：`crates/rex-hub/src/routes/transfers.rs` — 传输路由
  - 修改：`crates/rex-hub/src/routes/mod.rs` — 注册传输路由

- **接口设计**
  - `POST /api/transfers` — 创建传输任务（source, target, direction）
  - `GET /api/transfers` — 列出传输任务
  - `DELETE /api/transfers/:id` — 取消传输任务
  - `WS /ws/transfers/:id` — 传输进度 WebSocket

- **测试标准**
  - 路由注册正确
  - 传输任务 API 序列化

- **提交信息**
  `feat: add file transfer API in Hub`

### 3 前端 files 页面初始化

- **功能目标**
  初始化 files 页面骨架，为后续文件列表和传输队列做准备。

- **文件结构**
  - 新增：`packages/rex-console-web/src/features/files/FilesPage.vue`
  - 修改：`packages/rex-console-web/src/router/index.ts` — 添加 /files 路由
  - 修改：`packages/rex-console-web/src/layouts/AppLayout.vue` — 添加文件导航项

- **交互设计**
  参考 `prototype/files.html`：
  - 左侧资源选择器
  - 双栏文件浏览器（源 → 目标）
  - 顶部路径导航

- **测试标准**
  - FilesPage 渲染成功
  - 路由 /files 正确加载

- **提交信息**
  `feat: add files page skeleton`

### 4 前端文件列表 + 右键菜单 + 传输队列

- **功能目标**
  实现完整的文件管理界面，包括文件列表、右键菜单、传输队列面板。

- **文件结构**
  - 新增：`packages/rex-console-web/src/features/files/FileList.vue` — 文件列表组件
  - 新增：`packages/rex-console-web/src/features/files/ContextMenu.vue` — 右键菜单
  - 新增：`packages/rex-console-web/src/features/files/TransferQueue.vue` — 传输队列
  - 新增：`packages/rex-console-web/src/features/files/useFileBrowser.ts` — 文件浏览逻辑
  - 新增：`packages/rex-console-web/src/api/files.ts` — 文件 API 客户端

- **交互设计**
  参考 `prototype/files.html`：
  - 右键菜单：发送到…、重命名、删除、新建文件夹、刷新
  - 传输队列：底部面板，显示进度条和速度
  - 拖拽上传（可选）

- **测试标准**
  - FileList 渲染文件列表
  - 右键菜单显示正确选项
  - TransferQueue 显示传输进度

- **提交信息**
  `feat: add file list, context menu and transfer queue`

### 5 资源创建向导 SFTP/FTP 表单 + 工作空间面板集成

- **功能目标**
  在资源创建向导中添加 SFTP 和 FTP 连接表单，在工作空间中将文件协议映射到文件面板。

- **文件结构**
  - 修改：`packages/rex-console-web/src/pages/ResourceNew.vue` — 添加 SFTP/FTP 表单
  - 修改：`packages/rex-console-web/src/features/workspace/useTabs.ts` — 添加文件协议映射
  - 新增：`packages/rex-console-web/src/features/workspace/panels/WorkspaceFile.vue`

- **交互设计**
  SFTP 表单：主机、端口（22）、用户名、密码/密钥、工作目录
  FTP 表单：主机、端口（21）、用户名、密码、被动模式

- **测试标准**
  - ResourceNew 显示 SFTP/FTP 表单
  - useTabs 正确映射文件协议

- **提交信息**
  `feat: add SFTP/FTP resource form and workspace panel`

## 设计核对点

- [ ] FileConnector trait 是否支持 SFTP 和 FTP
- [ ] 文件传输数据是否不经过浏览器
- [ ] 前端交互是否与 prototype/files.html 一致
- [ ] 右键菜单是否包含所有必要操作
- [ ] 传输队列是否显示进度和速度

## Flow Status

- [ ] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
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
