# M23: 文件上传下载 + 跨连接传输

## Context

M22 完成了侧边栏收藏、最近使用和右键菜单增强。文件管理后端已有 `FileConnector` trait（list/mkdir/touch/delete/rename）和 `TransferManager`（创建/查询/取消传输任务），前端 Files 页面可浏览目录、新建/删除/重命名。但缺少两个关键能力：

1. **文件上传/下载** — 后端无 upload/download HTTP 端点，前端无上传下载按钮
2. **跨连接传输** — `TransferManager` 可用，但前端无法创建传输任务，Files 页面右键菜单没有「发送到…」

## 产品边界

**做什么：**
- 文件下载：后端端点 + 前端下载按钮 + 右键菜单「下载」
- 文件上传：后端端点 + 前端上传按钮 + 拖放上传
- 跨连接传输：前端创建传输任务 + 右键菜单「发送到…」+ 传输队列 UI 完善

**不做什么：**
- 不实现文件内容在线预览（直接下载即可）
- 不实现断点续传（当前传输模型不支持）
- 不实现压缩传输（后续优化）

---

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 23.1 | 文件下载（后端端点 + 前端按钮） | ✅ |
| 23.2 | 文件上传（后端端点 + 前端按钮 + 拖放） | ✅ |
| 23.3 | 跨连接传输（发送到… + 创建传输任务） | ✅ |

---

## 子任务详细设计

### 23.1 文件下载

**功能目标：**
用户可以从 Files 页面下载远程文件到本地浏览器。

**修改文件：**
```text
crates/rex-hub/src/
├── files.rs                          修改：添加 download_file 端点
├── routes.rs                         修改：注册下载路由
packages/rex-console-web/src/
├── api/files.ts                      修改：添加 downloadFile 方法
├── features/files/
│   ├── FileList.vue                  修改：添加下载按钮到工具栏
│   └── useFileManager.ts            无需修改（下载通过浏览器原生行为）
├── i18n/zh.ts, en.ts                修改：添加下载相关 i18n 键
```

**接口设计：**

后端端点：
```text
GET /api/resources/:resource_id/files/download?path=/path/to/file
```

响应：直接返回文件二进制内容（`application/octet-stream`），`Content-Disposition: attachment`。

后端流程：
1. 获取 SFTP 连接器
2. 调用 `FileConnector::read(path)` 读取文件内容
3. 返回文件流，设置 `Content-Type: application/octet-stream`、`Content-Disposition: attachment; filename="xxx"`

前端方法：
```ts
// api/files.ts
export function downloadFileUrl(resourceId: string, path: string): string {
  return `${client.defaults.baseURL}/resources/${resourceId}/files/download?path=${encodeURIComponent(path)}`
}
```

前端交互：
- Files 工具栏添加「⬇ 下载」按钮（选中文件时激活）
- 文件右键菜单添加「下载」选项
- 点击后通过 `<a download>` 触发浏览器下载

**测试标准：**
- 下载端点返回正确文件内容和 Content-Disposition
- 未选中文件时下载按钮禁用
- 下载大文件不会阻塞 UI
- 下载不存在的文件返回 404

**提交信息：**
```
feat: add file download endpoint and frontend button
```

---

### 23.2 文件上传

**功能目标：**
用户可以从 Files 页面上传本地文件到远程服务器，支持按钮上传和拖放上传。

**修改文件：**
```text
crates/rex-hub/src/
├── files.rs                          修改：添加 upload_file 端点
├── routes.rs                         修改：注册上传路由
packages/rex-console-web/src/
├── api/files.ts                      修改：添加 uploadFile 方法
├── features/files/
│   ├── FileList.vue                  修改：添加拖放上传区域
│   ├── Files.vue                     修改：添加上传按钮 + 拖放处理
│   └── useFileManager.ts            无需修改
├── i18n/zh.ts, en.ts                修改：添加上传相关 i18n 键
```

**接口设计：**

后端端点：
```text
POST /api/resources/:resource_id/files/upload?path=/target/directory
Content-Type: multipart/form-data
Body: file=<File>
```

响应：`201 Created`

后端流程：
1. 接收 multipart form data
2. 获取 SFTP 连接器
3. 将文件写入目标路径（目标目录 + 文件名）
4. 返回 201

前端方法：
```ts
// api/files.ts
export function uploadFile(resourceId: string, dirPath: string, file: File): Promise<void> {
  const formData = new FormData()
  formData.append('file', file)
  return client.post(`/resources/${resourceId}/files/upload`, formData, {
    params: { path: dirPath },
    headers: { 'Content-Type': 'multipart/form-data' },
  }).then(() => {})
}
```

前端交互：
- Files 工具栏添加「⬆ 上传」按钮，点击触发隐藏 `<input type="file">`
- 拖放文件到文件列表区域：显示蓝色虚线边框覆盖层 + 「拖放文件到此处上传」提示
- 上传完成后自动刷新当前目录
- 支持多文件选择上传

**测试标准：**
- 上传端点正确接收文件并写入远程
- 上传后目录列表自动刷新
- 拖放上传显示覆盖层提示
- 上传不存在的目录返回 400
- 上传大文件不会阻塞 UI

**提交信息：**
```
feat: add file upload endpoint and frontend upload UI
```

---

### 23.3 跨连接传输

**功能目标：**
用户可以在 Files 页面通过右键菜单「发送到…」将文件从一个连接传输到另一个已打开的连接。前端创建传输任务，由后端完成实际传输。

**修改文件：**
```text
packages/rex-console-web/src/
├── api/transfer.ts                  修改：添加 createTransfer 方法
├── features/files/
│   ├── FileList.vue                 修改：右键菜单添加「发送到…」
│   ├── TransferQueuePanel.vue       修改：完善传输状态显示
│   └── useTransferQueue.ts         无需修改（已有轮询）
├── features/workspace/
│   └── useTabs.ts                   读取：获取已打开的标签列表
├── i18n/zh.ts, en.ts               修改：添加传输相关 i18n 键
```

**接口设计：**

前端创建传输：
```ts
// api/transfer.ts
export function createTransfer(source: TransferEndpoint, target: TransferEndpoint): Promise<TransferTask> {
  return client.post('/transfers', { source, target }).then(r => r.data.data)
}
```

前端交互：
- 文件右键菜单添加「发送到…」子菜单
- 子菜单列出所有已打开的支持文件传输的标签页（SSH、SFTP）
- 选择目标标签后调用 `createTransfer`，传入：
  - source: 当前连接的 `connector_type="sftp"` + `resource_id` + `path`
  - target: 目标连接的 `connector_type="sftp"` + `resource_id` + 目标路径（当前目录）
- 传输创建后，TransferQueuePanel 自动显示新任务
- Toast 提示「文件传输已创建」

**获取已打开的传输连接：**
从 `useTabs` 获取所有已打开标签，过滤出 protocol 为 ssh/sftp 的标签作为传输目标候选。

**测试标准：**
- 右键菜单「发送到…」列出所有可用目标连接
- 选择目标后成功创建传输任务
- TransferQueuePanel 显示新任务并轮询进度
- Toast 提示传输创建成功
- 无可用目标时「发送到…」菜单项禁用

**提交信息：**
```
feat: add cross-connection file transfer with send-to menu
```

---

## 设计核对点

- [ ] 下载端点与 PRODUCT.md §3.8 的文件管理一致
- [ ] 上传端点与 PRODUCT.md §3.8 的文件管理一致
- [ ] 跨连接传输与 PRODUCT.md §3.8 的「发送到…」一致
- [ ] 传输队列与 PRODUCT.md §3.8 的传输队列一致
- [ ] 文件数据不经过浏览器中转（传输由后端完成）
- [ ] 上传/下载不引入新的安全风险（路径遍历防护）

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

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
