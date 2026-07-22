# M33: 文件传输断点续传

## Context

M31 完成了 S3 multipart 断点续传后端 API，但前端未集成。当前文件传输（SFTP/S3）失败后需要重新开始，无续传能力。PRODUCT.md 3.8 要求「断点续传：SFTP 重开 seek；S3 multipart 续传」。本里程碑实现前后端完整的断点续传功能。

版本类型：minor（新功能），版本号 0.31.0 → 0.32.0。

## 产品边界

**本阶段做：**
- S3 multipart 上传前端续传集成（保存 upload_id，失败后自动续传）
- SFTP 上传断点续传（通过文件偏移量 seek）
- 传输队列 UI 改进（显示断点续传状态）
- 下载断点续传（HTTP Range 请求）

**本阶段不做：**
- 传输队列抽屉重写（已在 M6 实现基本功能）
- 并发传输优化
- 跨会话续传（会话结束后续传失效）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | S3 上传前端续传集成 | ✅ |
| 2 | SFTP 上传断点续传 | ✅ |
| 3 | 下载断点续传 | ✅ |
| 4 | 传输队列状态改进 | ✅ |

## 子任务详细设计

### 1 S3 上传前端续传集成

**功能目标**

FilesPage.vue 上传 S3 大文件时，保存 upload_id，失败后自动续传。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 上传逻辑增加续传
- `packages/rex-console-web/src/api/files.ts` — 上传函数支持续传参数

**接口设计**

前端状态管理：
```typescript
// 上传状态
interface UploadState {
  sessionId: string
  remotePath: string
  uploadId?: string      // S3 multipart upload_id
  completedParts?: Array<{ part_number: number; e_tag: string }>
  failedPart?: number    // 失败的分片号
}
```

**交互设计**

- 上传 >5MB 文件时：
  1. 首次上传获取 upload_id
  2. 上传过程中保存每个 completed part
  3. 失败时显示「Retry from failed part」按钮
  4. 点击重试时调用 resume_multipart_upload API
- 传输队列显示：上传中 / 失败（可重试）/ 续传中

**实现流程**

1. `files.ts`：uploadFile 返回 upload_id（首次上传时）
2. `FilesPage.vue`：维护上传状态 map
3. 失败时保存 upload_id 和 completed parts
4. 重试时调用 resumeMultipartUpload API

**测试标准**

- S3 大文件上传失败后可续传
- 续传不重复已上传的分片
- 传输队列正确显示状态
- type-check + build 通过

**提交信息**: `feat(files): add S3 multipart upload resume in frontend`

### 2 SFTP 上传断点续传

**功能目标**

SFTP 上传失败后，从已上传的字节位置继续。

**文件结构**

修改：
- `crates/rex-ssh/src/sftp.rs` — upload 支持 offset 参数
- `crates/rex-hub/src/file_api.rs` — 上传端点支持 offset
- `crates/rex-common/src/file_transfer.rs` — FileConnector trait upload 方法签名
- `packages/rex-console-web/src/api/files.ts` — 上传函数支持 offset
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 上传逻辑

**接口设计**

后端扩展：
```rust
// FileConnector trait
async fn upload(
    &mut self,
    remote_path: &str,
    data: Vec<u8>,
    offset: u64,  // 新增：从指定偏移开始写入
    progress: Option<&ProgressCallback>,
) -> Result<()>;

// file_api.rs 上传端点
POST /api/files/upload  (multipart form)
// 新增可选字段：offset
```

**交互设计**

- SFTP 上传失败时显示「Retry」按钮
- 重试时从已上传字节数继续
- 进度条从上次位置继续

**实现流程**

1. `SftpConnector::upload`：打开文件时使用 `open_options().write(true).create(true).open()`
2. seek 到 offset 位置后继续写入
3. 前端记录已上传字节数，失败时传入 offset

**测试标准**

- SFTP 上传失败后可从断点续传
- 续传后文件内容正确
- type-check + build 通过

**提交信息**: `feat(sftp): add upload resume with offset support`

### 3 下载断点续传

**功能目标**

下载失败后，从已下载的字节位置继续。

**文件结构**

修改：
- `crates/rex-s3/src/lib.rs` — download 支持 range 参数
- `crates/rex-ssh/src/sftp.rs` — download 支持 range 参数
- `crates/rex-hub/src/file_api.rs` — 下载端点支持 Range header
- `packages/rex-console-web/src/api/files.ts` — 下载函数支持 offset
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 下载逻辑

**接口设计**

后端：
```rust
// FileConnector trait
async fn download_range(&mut self, path: &str, offset: u64, limit: u64) -> Result<Vec<u8>>;

// file_api.rs
GET /api/files/download?session_id=...&path=...
// 支持 Range header: Range: bytes=1024-
```

**交互设计**

- 下载失败时显示「Retry」按钮
- 重试时从已下载位置继续
- 进度条从上次位置继续

**实现流程**

1. `S3Connector`：使用 `get_object().range(format!("bytes={offset}-"))`
2. `SftpConnector`：使用 `session.read_at(path, offset, limit)`
3. 前端 Blob 拼接：新数据追加到已下载部分

**测试标准**

- 下载失败后可从断点续传
- 续传后文件内容完整正确
- type-check + build 通过

**提交信息**: `feat(files): add download resume with range support`

### 4 传输队列状态改进

**功能目标**

传输队列 UI 显示续传状态，提供重试按钮。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 传输队列状态

**交互设计**

传输项状态：
- `pending` — 等待中
- `uploading` — 上传中（显示进度）
- `downloading` — 下载中（显示进度）
- `failed` — 失败（显示「Retry」按钮）
- `resuming` — 续传中
- `completed` — 完成

重试按钮行为：
- 点击重试 → 根据协议调用续传 API
- S3：resume_multipart_upload
- SFTP：upload with offset
- Download：download with Range

**测试标准**

- 传输队列正确显示各种状态
- 重试按钮可用且功能正确
- type-check + build 通过

**提交信息**: `feat(files): improve transfer queue with resume status`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ PRODUCT.md 3.8 要求「断点续传：SFTP 重开 seek；S3 multipart 续传」
- ✅ 不引入跨会话续传（复杂度过高）
- ✅ 与 M31 S3 multipart API 衔接

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
