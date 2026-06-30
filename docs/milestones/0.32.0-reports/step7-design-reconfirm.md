# 0.32.0 步骤7：设计再确认报告

## 对照检查

### 里程碑文档 vs 实际实现

| 检查项 | 里程碑文档 | 实际实现 | 结论 |
|--------|-----------|----------|------|
| TransferItem 状态文本 i18n | `t('files.transfer.completed')` 等 | TransferItem.vue 使用 i18n keys | ✅ |
| 源/目标/错误标签 i18n | `t('files.transfer.source')` 等 | TransferItem.vue:36-46 使用 i18n keys | ✅ |
| 传输速度显示 | `1.2 MB/s` 格式 | `formatSpeed(bytesPerSec)` → B/s, KB/s, MB/s | ✅ |
| 剩余时间显示 | `剩余 2:30` 或 `< 1 分钟` | `formatEta(seconds)` → m:ss 或 i18n etaLessThanMinute | ✅ |
| 速度计算方式 | useTransferQueue 3s 轮询 delta | `prevBytes` Map + `computeSpeedAndEta` | ✅ |
| useTransferQueue 导出 prevTasks | 导出 prevTasks ref | 返回值包含 prevTasks | ✅ |
| Files.vue watch transferTasks | watch 变化触发 Toast | `useTransferToast(transferTasks, prevTasks)` | ✅ |
| TerminalSftp watch transferTasks | watch 变化触发 Toast | `useTransferToast(transferTasks, prevTasks)` | ✅ |
| TerminalSftp 传输队列 | TransferQueuePanel 折叠式 | 模板包含 TransferQueuePanel | ✅ |
| Toast i18n keys | completedToast / failedToast | zh.ts:554-555, en.ts:554-555 | ✅ |
| 无后端改动 | 无需后端改动 | 仅前端文件变更 | ✅ |

### 设计核对点（里程碑文档中列出）

- [x] 单用户设计：无权限检查 — 无新权限逻辑
- [x] 自托管：文件操作通过已有 API — 未引入新 API 调用
- [x] 数据不经浏览器：文件传输由后端完成 — 前端仅展示进度
- [x] 不引入多用户/RBAC — 未引入
- [x] 深色主题一致性 — 使用 CSS 变量
- [x] i18n 覆盖：新增 UI 文本使用 i18n key — 全部 i18n 化

### 产品边界一致性

- 未引入新协议支持 ✅
- 未引入冲突处理弹窗 ✅
- 未创建传输队列独立页面 ✅

## 结论

已实现代码与里程碑文档完全一致。✅
