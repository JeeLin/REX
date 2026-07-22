# M32: S3 ACL 管理

## Context

M31 完成了 S3 连接参数、Storage Class 列、Presigned URL、Multipart 断点续传。PRODUCT.md 3.8 要求 S3 文件列表显示「Storage Class·ACL」，Storage Class 已实现，ACL 管理尚未实现。本里程碑补齐 S3 ACL 功能。

版本类型：minor（新功能），版本号 0.30.0 → 0.31.0。

## 产品边界

**本阶段做：**
- S3 文件列表增加 ACL 列（显示 Canned ACL）
- S3 右键菜单「Permissions」→ ACL 编辑对话框
- S3Connector 实现 get_object_acl / put_object_acl

**本阶段不做：**
- Bucket Policy 管理（后续里程碑）
- 复杂 ACL（bucket policy / ACL rules beyond canned）
- SFTP Permissions 功能变更

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | S3Connector ACL 方法 + API 端点 | ✅ |
| 2 | 前端 ACL 列 + 编辑对话框 | ✅ |

## 子任务详细设计

### 1 S3Connector ACL 方法 + API 端点

**功能目标**

S3Connector 实现 get_object_acl / put_object_acl，file_api.rs 新增 ACL 端点。

**文件结构**

修改：
- `crates/rex-s3/src/lib.rs` — 添加 get_acl / put_acl 方法
- `crates/rex-hub/src/file_api.rs` — 添加 `/acl` GET/PUT 端点
- `crates/rex-common/src/file_transfer.rs` — FileEntry 新增 acl 字段

**接口设计**

后端：
```rust
// S3Connector
pub async fn get_acl(&self, key: &str) -> Result<String>  // returns canned ACL: "private"|"public-read"|...
pub async fn put_acl(&self, key: &str, canned_acl: &str) -> Result<()>

// file_api.rs
GET  /api/files/acl?session_id=...&path=...  → { acl: "private" }
PUT  /api/files/acl  → { ok: true }  body: { session_id, path, acl }
```

FileEntry 新增：
```rust
pub struct FileEntry {
    // ... existing fields
    pub acl: Option<String>,  // S3: "private"|"public-read"|"public-read-write"|"authenticated-read"
}
```

**交互设计**

- S3 文件列表：显示 ACL 列（替换 SFTP 的 Permissions 列）
- 右键文件 →「Permissions」→ 弹出 ACL 编辑对话框
- 对话框：下拉选择 Canned ACL（Private / Public Read / Public Read Write / Authenticated Read）
- 保存后刷新文件列表

**测试标准**

- S3 文件列表显示 ACL 列
- 右键「Permissions」弹出编辑对话框
- 修改 ACL 后文件列表更新
- cargo test + type-check + build 通过

**提交信息**: `feat(s3): add ACL display and editing for S3 objects`

### 2 前端 ACL 列 + 编辑对话框

**功能目标**

FilesPage.vue 显示 ACL 列，提供 ACL 编辑对话框。

**文件结构**

修改：
- `packages/rex-console-web/src/api/files.ts` — 添加 getAcl / putAcl API
- `packages/rex-console-web/src/features/files/FilesPage.vue` — ACL 列 + 编辑对话框

**交互设计**

- ACL 列：显示 Canned ACL 值（private/public-read 等）
- ACL 编辑对话框：下拉选择 + 保存按钮
- SFTP 协议时隐藏 ACL 列和菜单项

**测试标准**

- S3 显示 ACL 列，SFTP 不显示
- 右键「Permissions」打开 ACL 编辑对话框
- 保存后文件列表刷新
- type-check + build 通过

**提交信息**: `feat(files): add ACL column and edit dialog for S3`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ PRODUCT.md 3.8 要求「Storage Class·ACL（S3）」
- ✅ 不引入复杂 ACL（仅 Canned ACL）
- ✅ 与 M31 S3 增强衔接

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
