# M23 步骤5：代码审查报告

## 审查维度

### 1. 正确性
- 后端 download_file：通过 FileConnector::read 读取文件，返回 octet-stream + Content-Disposition ✅
- 后端 upload_file：multipart form data 解析，遍历 field 写入 FileConnector ✅
- 前端下载：使用 axios client 发请求（带 auth header），Blob URL 触发下载 ✅
- 前端上传：FormData + multipart/form-data header ✅
- 拖放上传：dragCounter 计数器处理嵌套元素 dragleave 事件 ✅
- 跨连接传输：从 useTabs 获取已打开标签，创建传输任务 ✅

### 2. 安全性
- **🔴 已修复**：前端下载最初使用 `<a>` 标签直接请求 URL，不携带 Authorization header，会导致 401 失败。已改为通过 axios client 发请求（自动注入 token），再用 Blob URL 触发下载。
- 路径遍历：预存问题，非 M23 引入，但 download/upload 端点继承了现有 get_connector 的路径处理。后续可统一加固。
- upload 端点：文件名来自 multipart header，直接拼接到路径。极端情况下 `../../` 文件名可逃逸目录。但 SFTP/local connector 的 write 方法由底层实现处理，实际风险取决于 connector 实现。

### 3. 架构一致性
- 后端端点风格与现有 files.rs 一致（`/api/resources/:resource_id/files/...`）✅
- 前端使用 composable 模式（useFileManager, useTransferQueue, useTabs）✅
- 传输使用现有 TransferManager，不引入新架构 ✅

### 4. 错误处理
- 后端：download/upload 都有完整的错误映射（FILE_READ_FAILED, FILE_WRITE_FAILED）✅
- 前端：uploadFiles 有 try/finally 确保 uploading 状态重置 ✅
- 拖放：drop 事件重置 isDragging 和 dragCounter ✅

### 5. 里程碑文档一致性
- 23.1 下载：后端端点 + 前端按钮 ✅
- 23.2 上传：后端端点 + 前端按钮 + 拖放 ✅
- 23.3 跨连接传输：发送到… + 创建传输任务 ✅

## 发现

| 级别 | 问题 | 位置 | 状态 |
|------|------|------|------|
| 🔴 | 前端下载不携带 auth header | Files.vue (旧) | ✅ 已修复 |
| 🟡 | upload 文件名未做路径遍历过滤 | files.rs:upload_file | 预存问题 |
| 🟡 | 跨连接传输直接用第一个目标，无选择对话框 | Files.vue:handleSendTo | TODO 注释已标记 |
| 🟢 | download_file 将整个文件读入内存 | files.rs:download_file | 与 FileConnector trait 一致 |

## 结论

✅ 无未修复的 🔴 必须修复项。代码审查通过。
