# M23 步骤2：设计核对报告

## 审查维度

### 1. 产品定位 ✅
- 单用户模型：无多用户概念
- 自托管：上传下载通过浏览器与 Hub 交互，不涉及第三方服务
- 深色优先：不涉及 UI 主题

### 2. 架构一致性 ✅
- 单二进制 + supervisor + worker：无变更
- Hub API 端点风格与现有 files.rs 一致（`/api/resources/:resource_id/files/...`）
- 传输使用现有 `TransferManager`，不引入新架构

### 3. 文件传输不经过浏览器 ✅
- **下载**：浏览器从 Hub 下载文件（Hub 从 SFTP 读取），这是正常行为
- **上传**：浏览器上传文件到 Hub（Hub 写入 SFTP），这是正常行为
- **跨连接传输**：前端只创建传输任务，实际传输由后端 TransferManager 完成，文件数据不经过浏览器 ✅

### 4. 不引入不该有的概念 ✅
- 无 RBAC、多用户、企业协作
- 无新的外部依赖

### 5. 是否跳阶段实现 ✅
- 文件管理是 M4 级别功能，M23 补充上传/下载和跨连接传输合理
- 不涉及新的协议 crate

### 6. 子任务拆分粒度 ✅
- 23.1 下载、23.2 上传、23.3 跨连接传输各自独立
- 每个子任务可独立提交

### 7. 接口设计与现有代码一致性 ✅
- 下载端点：`GET /api/resources/:resource_id/files/download?path=...` 与现有 `GET .../files?path=...` 风格一致
- 上传端点：`POST /api/resources/:resource_id/files/upload` 使用 multipart form data，是文件上传标准方式
- 传输创建：使用现有 `POST /api/transfers` 端点

### 8. 安全性 🟡
- 现有文件端点（delete、rename、touch、mkdir）未做路径遍历防护，M23 新增的 upload/download 端点也未规划路径遍历检查
- **建议**：M23 实现时在 `get_connector` 或各端点中统一添加路径规范化（canonicalize）检查，防止 `../../etc/passwd` 类攻击
- 这是预存问题，非 M23 引入，但 M23 是修复的好时机

---

## 结论

✅ 设计与 PRODUCT.md §3.8 一致，架构合理，子任务拆分得当。安全性有一个 🟡 建议（路径遍历防护），可在开发时一并处理。
