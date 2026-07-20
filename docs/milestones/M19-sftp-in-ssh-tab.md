# M19: SSH Tab 集成 SFTP 文件管理

## Context

M0–M18 完成了基础设施和核心功能。当前 SFTP 作为独立协议，与 SSH 终端完全分离（两次连接、两个 Tab）。实际上 SFTP 是 SSH File Transfer Protocol，应复用 SSH 连接。参考 Xshell / WinSCP 的交互模式，在 SSH Tab 内集成可折叠文件管理抽屉。

本里程碑版本类型：minor（新功能），版本号 0.19.0 → 0.20.0。

## 产品边界

**本阶段做：**
- SSH Tab 内集成 SFTP 文件管理抽屉（底部可折叠/可调整高度）
- 抽屉自动使用 SSH 连接的 credentials 建立 SFTP 会话
- 文件管理基本操作（浏览、上传、下载、删除、重命名、新建文件夹）
- 传输队列面板（进度条、传输速率、暂停/恢复、取消）
- 面包屑路径导航
- 从连接树中移除 SFTP 作为独立协议类型（SFTP 资源不再单独存在）

**本阶段不做：**
- S3 协议保持独立 Tab（S3 与 SSH 无关）
- 同步浏览（两侧联动）
- 文件夹同步对话框
- 传输队列持久化（刷新后清空）
- 后端 SSH/SFTP 连接复用（当前 SFTP 仍通过独立 HTTP API 连接，后续优化）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | SSH Tab 集成 SFTP 抽屉 UI | ⬜ |
| 2 | 传输队列 + 面包屑 + 右键菜单增强 | ⬜ |

## 子任务详细设计

### 1 SSH Tab 集成 SFTP 抽屉 UI

**功能目标**

SSH Tab 内集成可折叠的 SFTP 文件管理抽屉，替代独立的 SFTP Tab。抽屉位于终端下方，可拖拽调整高度，可折叠/展开。

**文件结构**

修改：
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — 移除 SFTP 作为独立 Tab 协议，SSH Tab 模板增加抽屉
- `packages/rex-console-web/src/features/terminal/TerminalView.vue` — 增加 `showSftp` prop，传递 SFTP 抽屉状态
- `packages/rex-console-web/src/features/files/FilesDrawer.vue` — **新建**，从 FilesPage 精简而来的抽屉组件
- `packages/rex-console-web/src/features/workspace/ConnectionTree.vue` — 移除 SFTP 协议类型显示

**交互设计**

```
┌─ SSH Tab: My Server ──────────────────────────┐
│ ● SSH 192.168.1.100:22          80×24  📁 UTF-8 │  ← 状态栏增加 📁 文件按钮
├─────────────────────────────────────────────────┤
│                                                 │
│            xterm.js 终端区域                     │
│                                                 │
│                                                 │
├─ ═══ 拖拽分隔条 ═══════════════════════════════ ┤
│ 📁 SFTP · /var/www/    ↑ ⬆ ↻  [−]              │  ← 抽屉标题栏
│ Name              Size    Modified              │
│ 📁 html            -     2026-07-20             │
│ 📄 index.html     2.1KB  2026-07-19             │
│ 📄 config.json    456B   2026-07-18             │
│ 3 items                                    2 sel │
└─────────────────────────────────────────────────┘
```

**抽屉行为**：
- 默认折叠，点击状态栏 📁 按钮展开（底部 30% 高度）
- 拖拽分隔条调整终端/文件区域比例
- 最小高度 120px，最大高度 70%
- 再次点击 📁 或快捷键折叠
- 抽屉展开时自动连接 SFTP（使用 Tab 的 SSH credentials）
- 折叠时不断开 SFTP 连接
- Tab 关闭时断开 SFTP 连接

**数据模型**

```typescript
// FilesDrawer 使用 Tab 的 credentials 连接
// props: resourceId, host, port, username, password
// 自动在展开时建立 SFTP 会话
interface SftpSession {
  sessionId: string
  connected: boolean
  loading: boolean
}
```

**WorkspacePage 改造**

```vue
<!-- SSH Tab 模板 -->
<div v-if="activeTabInfo?.protocol === 'ssh'" class="ws-ssh-area">
  <TerminalView ... />
  <!-- SFTP 抽屉 -->
  <div v-if="showSftpDrawer" class="ws-sftp-drawer" :style="{ height: sftpDrawerHeight + 'px' }">
    <div class="ws-sftp-drag-handle" @mousedown="startSftpDrag" />
    <FilesDrawer
      :resource-id="activeTabInfo.resourceId"
      :host="activeTabInfo.host"
      :port="activeTabInfo.port"
      :username="activeTabInfo.username"
      :password="activeTabInfo.password"
    />
  </div>
</div>
```

**TerminalView 状态栏改造**

在状态栏右侧增加文件按钮：
```vue
<span class="tv-status-item tv-file-btn" @click="$emit('toggle-sftp')" title="Toggle file browser (Ctrl+B)">
  📁
</span>
```

**快捷键**
- `Ctrl+B`：切换 SFTP 抽屉展开/折叠（SSH Tab 内）

**连接树改造**
- 移除 SFTP 协议选项：`ConnectionTree.vue` 和 `WizardModal.vue` 中不再显示 SFTP 作为可选协议
- SFTP 功能完全由 SSH Tab 内的抽屉提供
- 保留 S3 作为独立协议

**后端不变**
- Files API 保持不变（`/api/files/connect` 等端点照常工作）
- SFTP 抽屉在后端仍然创建独立的 HTTP-based SFTP 会话
- 后续里程碑可以优化为 SSH/SFTP 连接复用

**测试标准**

- 打开 SSH Tab → 点击状态栏 📁 → 底部出现 SFTP 抽屉
- 抽屉内浏览远程文件目录
- 拖拽分隔条调整终端和文件区域比例
- 折叠/展开抽屉
- Ctrl+B 快捷键切换抽屉
- 关闭 SSH Tab → SFTP 会话正确清理
- 连接树中不再有 SFTP 协议选项
- S3 资源仍可独立创建和使用
- type-check + build 通过

**提交信息**

```
feat(ssh): integrate SFTP file browser drawer in SSH tab
refactor(files): remove SFTP as independent protocol type
```

### 2 传输队列 + 面包屑 + 右键菜单增强

**功能目标**

完善 SSH Tab 内 SFTP 抽屉的文件管理体验：传输队列、面包屑导航、完整右键菜单。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesDrawer.vue` — 增加传输队列、面包屑、右键菜单
- `packages/rex-console-web/src/api/files.ts` — 上传改为支持进度回调

**传输队列**

位于抽屉底部，高度约 80px，可折叠：

```typescript
interface TransferItem {
  id: string
  fileName: string
  direction: 'upload' | 'download'
  progress: number        // 0-100
  speed: number           // bytes/s
  totalSize: number
  transferred: number
  status: 'pending' | 'transferring' | 'paused' | 'completed' | 'error' | 'cancelled'
  error?: string
}
```

- 上传使用 XMLHttpRequest（获取 progress 事件）
- 下载仍使用 fetch blob（无进度，显示不确定进度条）
- 速率计算：每秒采样 transferred 差值
- 完成后保留 30 秒自动清除
- 支持暂停/恢复/取消（取消时 abort XHR）

**面包屑路径**

路径栏改为可点击面包屑：
```
/ > var > www > html
```
每级可点击跳转到对应目录。

**右键菜单增强**

| 项目 | 操作 |
|------|------|
| Open | 双击进入目录 / 下载文件 |
| Rename | 内联编辑文件名 |
| New Folder | 弹出输入框创建目录 |
| Delete | 确认后删除 |
| Download | 下载到本地 |
| Upload Here | 上传到当前目录 |
| Copy Path | 复制完整路径 |

**测试标准**

- 上传文件 → 队列显示进度 → 完成后自动清除
- 下载文件 → 队列显示不确定进度 → 触发浏览器下载
- 暂停/恢复/取消传输
- 面包屑每级可点击跳转
- 右键 Rename → 输入新名 → 回车 → 文件重命名
- 右键 New Folder → 输入名 → 创建目录
- type-check + build 通过

**提交信息**

```
feat(sftp): add transfer queue, breadcrumb, and enhanced context menu
```

## 设计核对点

- [ ] SSH Tab 内可展开 SFTP 抽屉
- [ ] 抽屉使用 SSH 连接的 credentials 自动连接
- [ ] 抽屉高度可拖拽调整
- [ ] 传输队列显示进度和速率
- [ ] 暂停/恢复/取消传输
- [ ] 面包屑路径导航
- [ ] 右键菜单完整（rename, new folder, delete, download, properties）
- [ ] 连接树中 SFTP 已移除为独立协议
- [ ] S3 仍可独立使用
- [ ] type-check + build 通过

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
