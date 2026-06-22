# 0.2.0 步骤7：设计再确认报告

## 确认范围

里程碑文档 `0.2.0-terminal-sftp-and-polish.md` vs 已实现代码

## 逐项核对

### 1 终端内置 SFTP 面板

| 检查项 | 里程碑文档 | 实际实现 | 结果 |
|--------|-----------|----------|------|
| SFTP 按钮展开/折叠面板 | ✅ | `Terminal.vue`: toolbar 按钮 + `toggleSftp()` | ✅ |
| Split view 终端+SFTP | ✅ | `.terminal-sftp-wrap` flex 容器 | ✅ |
| 文件列表显示名称、大小 | ✅ | `TerminalSftp.vue`: `.sfile-name` + `.sfile-size` | ✅ |
| 右键菜单下载/复制路径/删除 | ✅ | `TerminalSftp.vue`: context menu | ✅ |
| 拖拽文件路径到终端 | ✅ | `@drag-path` emit → `handleSftpDragPath()` | ✅ |
| Ctrl+Shift+F 快捷键 | ✅ | `handleKeydown` 中绑定 | ✅ |
| 复用现有文件 API | ✅ | 使用 `@/api/files` 的 `listFiles`, `uploadFile` 等 | ✅ |
| i18n 键已存在 | ✅ | 使用已有 `ws.terminal.toolbar.sftp` 等键 | ✅ |
| `useTerminalSftp.ts` 新增 | ⬜ 未创建 | 面板逻辑直接在组件内，无需单独 composable | ✅ 可接受 |

### 2 终端全屏按钮 + 死代码清理

| 检查项 | 里程碑文档 | 实际实现 | 结果 |
|--------|-----------|----------|------|
| ⛶ 全屏按钮 | ✅ | `Terminal.vue`: toolbar 按钮 | ✅ |
| F11 快捷键 | ✅ | `handleKeydown` 中绑定 | ✅ |
| `requestFullscreen` API | ✅ | `toggleFullscreen()` 函数 | ✅ |
| 清理 `handlePasteEvent` | ✅ | 已删除 | ✅ |

### 3 Settings 个人信息区块

| 检查项 | 里程碑文档 | 实际实现 | 结果 |
|--------|-----------|----------|------|
| ProfileSection.vue | ✅ | 创建完成 | ✅ |
| GET /api/user/profile | ✅ | `user.rs::get_profile` | ✅ |
| PUT /api/user/profile | ✅ | `user.rs::update_profile` | ✅ |
| PUT /api/user/password | ✅ | `user.rs::change_password` | ✅ |
| 密码验证当前密码 | ✅ | `verify_password` 检查 | ✅ |
| Settings.vue 集成 | ✅ | 顶部添加 ProfileSection | ✅ |

### 4 Dashboard 审计统计 + SQL 消息标签 + 标签关闭改进

| 检查项 | 里程碑文档 | 实际实现 | 结果 |
|--------|-----------|----------|------|
| GET /api/audit/stats | ✅ | `audit.rs::get_stats` | ✅ |
| Dashboard 调用统计 API | ✅ | `Dashboard.vue`: `getAuditStats('today')` | ✅ |
| SQL 消息标签 | ✅ | `SqlResults.vue`: 消息 tab | ✅ |
| 执行成功消息 | ✅ | `useSqlTabActions.ts`: `Query OK, N rows affected` | ✅ |
| 执行错误消息 | ✅ | `useSqlTabActions.ts`: `ERROR: msg` | ✅ |
| 关闭全部标签 | ✅ | `closeAll()` + `SqlTabs.vue` 菜单项 | ✅ |
| 关闭已保存标签 | ✅ | `closeSaved()` + `SqlTabs.vue` 菜单项 | ✅ |
| i18n 键 | ✅ | `closeAll`, `closeSaved`, `noMessage` | ✅ |

## 产品语义确认

| 检查项 | 结果 |
|--------|------|
| 单用户、自托管 | ✅ 未引入多用户/RBAC |
| 文件传输不经过浏览器 | ✅ REST API 直连后端 |
| 深色优先 | ✅ 新组件遵循现有主题 |
| 不引入全局查询/AI 助手 | ✅ 未实现 |
| 不引入执行计划标签 | ✅ 未实现 |

## 结论

✅ 通过。所有子任务实现与里程碑文档一致，产品语义未被污染。
