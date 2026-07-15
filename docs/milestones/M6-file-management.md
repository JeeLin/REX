# 0.7.0: M6 文件管理

## Context
M0-M5 已完成项目骨架、设计系统、工作空间外壳、SSH 终端、数据库控制台、Redis 控制台。M6 在工作空间内接入文件管理功能（SFTP + S3），是最后一个核心功能模块。

前序：M3 SSH 终端（rex-ssh crate 已有 SFTP 基础）、M4/M5（连接器 + REST API 模式已验证）。
后续：M7 管理模块收尾。

版本类型：minor

## 产品边界
- **做**：后端 SFTP/S3 连接器、传输引擎、REST API、前端双面板 UI、传输队列、右键菜单、同步浏览
- **不做**：文件编辑器集成（仅临时下载回传）、文件预览（图片/PDF）、WebDAV

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 后端 SFTP 连接器（rex-ssh SFTP + FileConnector trait） | ⬜ |
| 2 | 后端 S3 连接器（rex-s3：bucket/prefix、multipart） | ⬜ |
| 3 | 后端传输引擎 + REST API（任务调度/进度/取消） | ⬜ |
| 4 | 前端双面板 UI（活动面板、路径栏、面包屑、文件表格） | ⬜ |
| 5 | 传输队列抽屉（进度/吞吐/暂停恢复/取消） | ⬜ |
| 6 | 右键菜单 + 同步浏览 | ⬜ |
| 7 | 测试与收尾 | ⬜ |

## 子任务详细设计

### 1 后端 SFTP 连接器

- **功能目标**：通过 SSH 通道提供 SFTP 文件操作（列表/上传/下载/删除/重命名/mkdir/rstat）
- **文件结构**：
  - `crates/rex-ssh/src/sftp.rs`（新增：SftpConnector 实现 FileConnector trait）
  - `crates/rex-common/src/file_transfer.rs`（新增：FileConnector trait + 共享类型）
- **接口设计**：
  ```rust
  #[async_trait]
  pub trait FileConnector: Send + Sync {
      async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>>;
      async fn stat(&mut self, path: &str) -> Result<FileEntry>;
      async fn upload(&mut self, remote: &str, data: Bytes, progress: Option<ProgressCallback>) -> Result<()>;
      async fn download(&mut self, path: &str) -> Result<Bytes>;
      async fn delete(&mut self, path: &str) -> Result<()>;
      async fn rename(&mut self, from: &str, to: &str) -> Result<()>;
      async fn mkdir(&mut self, path: &str) -> Result<()>;
      async fn close(&mut self) -> Result<()>;
  }

  pub struct FileEntry {
      pub name: String,
      pub path: String,
      pub is_dir: bool,
      pub size: u64,
      pub modified: Option<String>,
      pub permissions: Option<String>,
  }
  ```
- **提交**：`feat(ssh): add SFTP file connector`

### 2 后端 S3 连接器

- **功能目标**：通过 AWS SDK 提供 S3 文件操作（list/get/put/delete/multipart upload）
- **文件结构**：
  - `crates/rex-s3/src/lib.rs`（实现：S3Connector 实现 FileConnector trait）
- **提交**：`feat(s3): add S3 file connector`

### 3 后端传输引擎 + REST API

- **功能目标**：统一文件传输 API（上传/下载/列表/删除/重命名），支持进度回调
- **文件结构**：
  - `crates/rex-hub/src/file_api.rs`（新增：REST 路由）
  - `crates/rex-hub/src/bin/rex-hub.rs`（修改：注册 `/api/files/*` 路由）
- **REST API**：
  ```http
  POST /api/files/connect      # 建立 SFTP/S3 连接
  POST /api/files/disconnect   # 断开连接
  GET  /api/files/list         # 列出目录内容
  GET  /api/files/stat         # 获取文件信息
  POST /api/files/upload       # 上传文件
  GET  /api/files/download     # 下载文件
  POST /api/files/delete       # 删除文件/目录
  POST /api/files/rename       # 重命名
  POST /api/files/mkdir        # 创建目录
  ```
- **提交**：`feat(files): add file transfer REST API`

### 4 前端双面板 UI

- **功能目标**：Xftp 风格双面板文件管理界面
- **文件结构**：
  - `src/features/files/FilesPage.vue`（新增：双面板布局）
  - `src/features/files/FilePanel.vue`（新增：单面板组件）
  - `src/features/files/useFileBrowser.ts`（新增：文件浏览 composable）
  - `src/api/files.ts`（新增：文件 API 封装）
- **交互设计**：
  - 左右双面板，每面板独立路径栏 + 面包屑
  - 活动面板模型：聚焦面板高亮，上传/下载作用于活动面板
  - Details 表格：名称（图标）/ 大小 / 修改时间 / 权限
  - 双击文件夹进入，`..` 返回上级
  - 路径栏可编辑输入跳转
- **提交**：`feat(web): add dual-panel file browser`

### 5 传输队列抽屉

- **功能目标**：底部常驻传输队列，显示进度/吞吐/方向
- **文件结构**：
  - `src/features/files/TransferQueue.vue`（新增）
- **交互设计**：
  - 每传输项：文件名 / 方向（↑↓）/ 大小 / 进度条 / 速率 / 状态
  - 总吞吐聚合
  - 暂停/恢复/取消按钮
- **提交**：`feat(web): add transfer queue drawer`

### 6 右键菜单 + 同步浏览

- **功能目标**：文件右键操作菜单、双面板同步浏览
- **文件结构**：
  - `src/features/files/FileContextMenu.vue`（新增）
- **菜单项**：上传 / 下载 / 重命名 / 删除 / 新建文件夹 / 复制路径 / 刷新
- **提交**：`feat(web): add file context menu and sync browse`

### 7 测试与收尾

- **功能目标**：验证全部功能，修复问题
- **测试标准**：type-check + lint + build + cargo build + cargo clippy 全通过
- **提交**：`fix(web): file manager polish and fixes`

## 设计核对点
- [ ] SFTP 连接器可通过 API 列出/上传/下载文件
- [ ] S3 连接器可通过 API 操作 bucket/prefix
- [ ] 双面板正确展示文件列表，活动面板模型工作
- [ ] 传输队列正确显示进度和速率
- [ ] 右键菜单项完整（上传/下载/重命名/删除/新建/复制路径）
- [ ] 同步浏览功能正常
- [ ] 路径栏可编辑跳转

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
