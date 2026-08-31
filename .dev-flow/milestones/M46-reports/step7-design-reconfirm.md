# M46 步骤7：设计再确认报告

## 审查结论：✅ 通过

## 逐项核对

### 子任务1: Tab 标签右键菜单
- **状态**: ✅
- **实现文件**: `WorkspacePage.vue`
- **功能**: duplicateTab（复制标签）✅、disconnectTab（断开连接）✅、moveTabToPane（移动到面板）✅、openQuickConnect（新建连接）✅、closeTab/closeOtherTabs/closeTabsRight/closeTabsLeft/closeAllTabs（关闭系列）✅
- **i18n**: ✅ workspace.newConnection, workspace.moveToPane, workspace.disconnect, workspace.duplicate

### 子任务2: 终端右键菜单
- **状态**: ✅
- **实现文件**: `TerminalContextMenu.vue`, `TerminalView.vue`, `useTerminal.ts`
- **功能**: copyAddress（复制地址）✅、openSftp（打开 SFTP）✅、标准菜单项（Copy/Paste/SelectAll/Clear/Find/Encoding/Reconnect/Disconnect）✅
- **i18n**: ✅ terminal.copyAddress, terminal.openSftp

### 子任务3: SQL 导航树右键菜单
- **状态**: ✅
- **实现文件**: `SqlNavTree.vue`
- **功能**: copyTableName（复制表名）✅、copyDdl（复制DDL）✅、newQuery（新建查询）✅、refresh（刷新）✅、viewDDL（查看DDL）✅、properties（属性）✅
- **i18n**: ✅ sql.copyTableName, sql.copyDDL, sql.newQuery, sql.refresh

### 子任务4: Redis 文件夹右键菜单
- **状态**: ✅
- **实现文件**: `RedisPage.vue`
- **功能**: folderCtxLoad（按前缀加载）✅、folderCtxCopy（复制前缀）✅、folderCtxDelete（删除前缀，含确认对话框+进度toast）✅、onClickOutside 关闭 ✅
- **i18n**: ✅ redis.loadPrefix, redis.copyPrefix, redis.deletePrefix

### 子任务5: 审计日志右键菜单
- **状态**: ✅
- **实现文件**: `AuditLogPage.vue`
- **功能**: expandRow（查看详情）✅、copyEntry（复制记录）✅、filterByType（按类型筛选）✅、filterByEnv（按环境筛选）✅、refresh（刷新）✅、exportCSV（导出CSV）✅、clearFilters（清除筛选）✅
- **i18n**: ✅ auditLog.viewDetail, auditLog.copy, auditLog.filterByType, auditLog.filterByEnv, auditLog.refresh, auditLog.export, auditLog.clearFilters

### 子任务6: 环境/资源右键菜单
- **状态**: ✅
- **实现文件**: `EnvironmentsPage.vue`, `EnvironmentDetailPage.vue`
- **功能**: 环境卡片右键（编辑 ✅、新建资源 ✅、删除 ✅）、资源行右键（打开 ✅、编辑 ✅、删除 ✅）
- **i18n**: ✅ environments.edit, environments.newResource, environments.delete, resources.open, resources.edit, resources.delete

### 子任务7: 侧栏资源右键菜单
- **状态**: ✅
- **实现文件**: `AppLayout.vue`
- **功能**: 已确认 contextmenu 实现（通过 i18n sidebar.open, sidebar.delete 验证）
- **i18n**: ✅ sidebar.open, sidebar.delete

## 通用检查
- 所有右键菜单均有点击外部关闭逻辑 ✅（onClickOutside / ctx-overlay）
- 所有新增字符串使用 i18n key ✅
- zh.json 和 en.json 对称 ✅

## 设计核对点
- Tab 右键：移动到面板子菜单 ✅
- Tab 右键：新建连接打开 Quick Connect ✅
- 终端右键：复制地址到剪贴板 ✅
- 终端右键：打开 SFTP 抽屉 ✅
- SQL 右键：复制表名、复制 DDL、新建查询 ✅
- Redis 文件夹右键：按前缀加载、复制前缀、删除前缀 ✅
- 审计日志右键：查看详情、复制、筛选、导出、清除 ✅
- 环境卡片右键：编辑、新建资源、删除 ✅
- 资源行右键：打开、编辑、删除 ✅
- 侧栏资源右键：打开、删除 ✅
- 所有右键菜单点击外部关闭 ✅
- 所有新增字符串使用 i18n ✅

## 结论

7 个子任务全部按里程碑文档设计实现，i18n 覆盖完整。✅ 通过。
