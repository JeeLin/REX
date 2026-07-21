# M31: S3 文件管理增强

## Context

M30 完成了 SFTP 移动端适配。M6 已实现 S3 基础连接器（S3Connector + FileConnector trait），但 S3 连接对话框未接入 S3 专用参数（bucket/region/endpoint/credentials），文件列表缺少 Storage Class 列，无 presigned URL 功能，multipart 上传无断点续传。本里程碑补齐 S3 特有能力。

版本类型：minor（新功能），版本号 0.29.0 → 0.30.0。

## 产品边界

**本阶段做：**
- S3 连接对话框接入 S3 专用参数（bucket/region/endpoint/access_key/secret_key）
- S3 文件列表增加 Storage Class 列
- S3 presigned URL 生成（右键菜单 + 工具栏）
- S3 multipart 上传断点续传（upload_id 持久化 + 恢复）

**本阶段不做：**
- ACL 管理（后续里程碑）
- Bucket 策略管理（后续里程碑）
- S3 lifecycle/版本控制/跨区域复制等高级功能

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | S3 连接对话框参数接入 | ⬜ |
| 2 | S3 文件列表 Storage Class 列 | ⬜ |
| 3 | S3 presigned URL 生成 | ⬜ |
| 4 | S3 multipart 上传断点续传 | ⬜ |

## 子任务详细设计

### 1 S3 连接对话框参数接入

**功能目标**

FilesPage.vue 连接对话框在选择 S3 协议时，显示 S3 专用字段（bucket/region/endpoint/access_key/secret_key），替代 SFTP 的 host/port/username/password。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 连接对话框 S3 字段

**交互设计**

- 协议选择 S3 时：
  - 隐藏 Host / Port / Username / Password
  - 显示 Bucket（必填）、Region（选填）、Endpoint URL（选填，用于 MinIO 等兼容服务）
  - 显示 Access Key（选填）、Secret Key（选填，密码输入框）
- 连接时将 S3 参数传入 `filesApi.connect()`

**实现流程**

1. FilesPage.vue 模板：在连接对话框中添加 S3 专用字段
2. `connProtocol === 's3'` 时 `v-show` S3 字段组，隐藏 SFTP 字段组
3. `doConnect()` 函数：根据协议构造不同的请求参数
4. 确保 `FileConnectRequest` 的 S3 字段正确传递到后端

**测试标准**

- SFTP 连接：字段不变，功能正常
- S3 连接：显示 Bucket/Region/Endpoint/AccessKey/SecretKey
- S3 连接成功后文件列表正常加载
- type-check + build 通过

**提交信息**: `feat(files): wire S3 connection fields in connect dialog`

### 2 S3 文件列表 Storage Class 列

**功能目标**

S3 文件列表显示 Storage Class 列（Standard / IA / Glacier 等），替代 SFTP 的 Permissions 列。

**文件结构**

修改：
- `crates/rex-common/src/file_transfer.rs` — FileEntry 增加 `storage_class` 字段
- `crates/rex-s3/src/lib.rs` — list() 和 stat() 填充 storage_class
- `crates/rex-ssh/src/sftp.rs` — 适配 FileEntry 新字段
- `packages/rex-console-web/src/api/files.ts` — FileEntry 类型增加 storage_class
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 文件列表条件列

**接口设计**

FileEntry 新增字段：
```rust
pub struct FileEntry {
    // ... existing fields
    pub storage_class: Option<String>,  // S3: "STANDARD" | "STANDARD_IA" | "GLACIER" | ...
}
```

**交互设计**

- S3 协议时：显示 Storage Class 列（替代 Permissions 列）
- SFTP 协议时：显示 Permissions 列（不变）
- Storage Class 值用 Badge 展示：Standard=绿、IA=蓝、Glacier=紫

**实现流程**

1. `file_transfer.rs`：FileEntry 添加 `storage_class: Option<String>`
2. `S3Connector::list()`：从 `Object::storage_class()` 提取值
3. `S3Connector::stat()`：从 `HeadObjectOutput::storage_class()` 提取值
4. `SftpConnector`：所有 `FileEntry` 构造处添加 `storage_class: None`
5. 前端 `files.ts`：FileEntry 类型添加 `storage_class`
6. `FilesPage.vue`：根据协议条件显示列（S3 → Storage Class，SFTP → Permissions）

**测试标准**

- S3 文件列表显示 Storage Class 列
- SFTP 文件列表仍显示 Permissions 列
- Storage Class 值正确显示（Standard/IA/Glacier）
- cargo test + type-check + build 通过

**提交信息**: `feat(s3): add Storage Class column to S3 file list`

### 3 S3 presigned URL 生成

**功能目标**

为 S3 文件生成临时访问 URL（presigned URL），可用于分享或外部访问。

**文件结构**

修改：
- `crates/rex-s3/src/lib.rs` — 添加 `presigned_url()` 方法
- `crates/rex-hub/src/file_api.rs` — 添加 `POST /api/files/presigned-url` 端点
- `packages/rex-console-web/src/api/files.ts` — 添加 `presignedUrl()` API 调用
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 右键菜单添加「Copy Presigned URL」

**接口设计**

后端：
```rust
// S3Connector 新增方法
pub async fn presigned_url(&self, key: &str, expires_in_secs: u64) -> Result<String>

// file_api.rs 新增端点
POST /api/files/presigned-url
Body: { session_id, path, expires_in: 3600 }
Response: { url: "https://..." }
```

前端：
```typescript
// files.ts
presignedUrl(sessionId: string, path: string, expires?: number): Promise<{ url: string }>
```

**交互设计**

- 右键文件 →「Copy Presigned URL」→ 生成 URL 并复制到剪贴板
- 默认有效期 1 小时（3600 秒）
- Toast 提示「Presigned URL copied to clipboard」

**实现流程**

1. `S3Connector`：使用 `aws_sdk_s3::presigning` 生成 presigned URL
2. `file_api.rs`：新增 `/presigned-url` 端点
3. 前端 `files.ts`：新增 `presignedUrl()` 函数
4. `FilesPage.vue`：右键菜单添加「Copy Presigned URL」项
5. 非 S3 协议时隐藏此菜单项

**测试标准**

- S3 文件右键菜单显示「Copy Presigned URL」
- SFTP 文件右键菜单不显示此项
- 点击后 URL 正确复制到剪贴板
- URL 可在浏览器中直接访问文件
- cargo test + type-check + build 通过

**提交信息**: `feat(s3): add presigned URL generation for S3 files`

### 4 S3 multipart 上传断点续传

**功能目标**

S3 大文件上传支持断点续传：中断后可从已上传的分片继续，无需重新上传。

**文件结构**

修改：
- `crates/rex-s3/src/lib.rs` — multipart 上传增加 upload_id 跟踪和恢复
- `crates/rex-hub/src/file_api.rs` — 上传端点支持 resume 参数
- `crates/rex-common/src/file_transfer.rs` — 上传请求增加 resume_id 字段

**接口设计**

后端扩展：
```rust
// S3Connector multipart 上传返回 upload_id
pub struct MultipartUploadState {
    pub upload_id: String,
    pub key: String,
    pub parts: Vec<CompletedPart>,
}

// file_api.rs 上传端点
POST /api/files/upload  (multipart form)
// 新增可选字段：resume_upload_id, part_number_offset

Response: { ok: true, upload_id?: "...", completed?: true }
// 首次上传返回 upload_id，后续分片上传返回 completed: false
// 最后一个分片返回 completed: true
```

**交互设计**

- 上传 >5MB 文件时，后端自动分片
- 上传中断后，再次上传同一文件时，前端检测到已有 upload_id，自动续传
- 传输队列显示「Resuming...」状态

**实现流程**

1. `S3Connector`：拆分 multipart 为可恢复的步骤
   - `init_multipart_upload()` → 返回 upload_id
   - `upload_part(upload_id, part_number, data)` → 返回 CompletedPart
   - `complete_multipart(upload_id, parts)` → 完成
   - `abort_multipart(upload_id)` → 取消
2. `file_api.rs`：上传端点支持 upload_id 跟踪
3. 前端：上传失败时保存 upload_id，重试时传入

**测试标准**

- 上传 >5MB 文件正常完成
- 模拟中断后续传成功（不重复已上传分片）
- 上传取消时正确 abort multipart
- cargo test + type-check + build 通过

**提交信息**: `feat(s3): add multipart upload resume for S3 large files`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ S3 专用参数通过现有 FileConnectRequest 传递，不引入新概念
- ✅ Storage Class 列遵循 PRODUCT.md 3.8 规定的「Storage Class·ACL（S3）」
- ✅ Presigned URL 是 S3 标准能力，不引入外部依赖
- ✅ Multipart resume 基于 AWS SDK 原生支持

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

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
