# M19: SFTP 文件管理增强（SSH 抽屉 + 传输队列）

## Context

M0–M18 完成了基础设施和核心功能。当前 SFTP 作为独立协议，与 SSH 终端完全分离（两次连接、两个 Tab）。实际上 SFTP 是 SSH File Transfer Protocol。本里程碑在保留独立 SFTP Tab 的基础上，增加 SSH Tab 内集成 SFTP 抽屉的能力，同时完善传输队列和右键菜单。

本里程碑版本类型：minor（新功能），版本号 0.19.0 → 0.20.0。

## 产品边界

**本阶段做：**
- SSH Tab 内集成 SFTP 文件管理抽屉（底部可折叠/可调整高度）
- 抽屉自动使用 SSH 连接的 credentials 建立 SFTP 会话
- 保留独立 SFTP Tab（连接树中 SFTP 协议不变）
- 传输队列面板（进度条、传输速率、暂停/恢复、取消）
- 面包屑路径导航
- 右键菜单增强（重命名、新建文件夹、属性）

**本阶段不做：**
- S3 协议保持独立 Tab（S3 与 SSH 无关）
- 同步浏览（两侧联动）
- 文件夹同步对话框
- 传输队列持久化（刷新后清空）
- 后端 SSH/SFTP 连接复用（当前 SFTP 仍通过独立 HTTP API 连接，后续优化）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | SSH Tab 集成 SFTP 抽屉 UI | ✅ |
| 2 | 传输队列 + 面包屑 + 右键菜单增强 | ✅ |

## 子任务详细设计

### 1 SSH Tab 集成 SFTP 抽屉 UI

**功能目标**

SSH Tab 内增加可折叠的 SFTP 文件管理抽屉。独立 SFTP Tab 保持不变。两种模式共存：
- **独立模式**：连接树中 SFTP 资源 → 打开独立 FilesPage Tab（现有行为不变）
- **抽屉模式**：SSH Tab 内点击 📁 按钮 → 底部展开 FilesDrawer（新增）

**文件结构**

修改：
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — SSH Tab 模板增加抽屉区域
- `packages/rex-console-web/src/features/terminal/TerminalView.vue` — 状态栏增加 📁 文件按钮 + emit toggle 事件

新建：
- `packages/rex-console-web/src/features/files/FilesDrawer.vue` — SSH Tab 内的 SFTP 抽屉组件（复用 filesApi，单面板远程文件浏览）

不变：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 独立 SFTP Tab 继续使用
- `packages/rex-console-web/src/features/resource/protocols.ts` — SFTP 协议保留
- `packages/rex-console-web/src/features/workspace/ConnectionTree.vue` — SFTP 协议保留
- `packages/rex-console-web/src/features/resource/WizardModal.vue` — SFTP 协议保留

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

**FilesDrawer 组件设计**

与 FilesPage 的区别：
- **单面板**（非双面板）：抽屉空间有限，只显示一个远程目录
- **自动连接**：从 props 获取 credentials，展开时自动连接
- **精简工具栏**：只有面包屑路径 + 刷新 + 上传按钮
- **共享 API**：使用同一个 `filesApi` 模块

```typescript
// FilesDrawer props
defineProps<{
  resourceId?: string
  host?: string
  port?: number
  username?: string
  password?: string
}>()
```

**WorkspacePage 改造**

SSH Tab 模板改为上下布局（终端 + 可折叠抽屉）：
```vue
<div v-if="activeTabInfo?.protocol === 'ssh'" class="ws-ssh-area">
  <TerminalView
    ...existing props...
    @toggle-sftp="toggleSftpDrawer"
  />
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
<span class="tv-status-item tv-file-btn" @click.stop="$emit('toggle-sftp')" title="Toggle file browser (Ctrl+B)">
  📁
</span>
```

**快捷键**
- `Ctrl+B`：切换 SFTP 抽屉展开/折叠（SSH Tab 内）

**后端不变**
- Files API 保持不变（`/api/files/connect` 等端点照常工作）
- SFTP 抽屉在后端创建独立的 HTTP-based SFTP 会话（与独立 SFTP Tab 使用相同的 API）
- 后续里程碑可以优化为 SSH/SFTP 连接复用

**测试标准**

- 打开 SSH Tab → 点击状态栏 📁 → 底部出现 SFTP 抽屉
- 抽屉内浏览远程文件目录
- 拖拽分隔条调整终端和文件区域比例
- 折叠/展开抽屉
- Ctrl+B 快捷键切换抽屉
- 关闭 SSH Tab → SFTP 会话正确清理
- 独立 SFTP Tab 仍然可用（打开 SFTP 资源 → 全屏双面板）
- type-check + build 通过

**提交信息**

```
feat(ssh): integrate SFTP file browser drawer in SSH tab
```

### 2 传输队列 + 面包屑 + 右键菜单增强

**功能目标**

完善文件管理体验：传输队列、面包屑导航、完整右键菜单。同时应用于 FilesDrawer（SSH 抽屉）和 FilesPage（独立 Tab）。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesDrawer.vue` — 增加传输队列、面包屑、右键菜单
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 增加传输队列、面包屑（与 FilesDrawer 共享逻辑）
- `packages/rex-console-web/src/api/files.ts` — 上传改为支持进度回调（XMLHttpRequest）

**传输队列**

位于底部，高度约 80px，可折叠：

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
- [ ] 独立 SFTP Tab 保持不变
- [ ] 传输队列显示进度和速率
- [ ] 暂停/恢复/取消传输
- [ ] 面包屑路径导航
- [ ] 右键菜单完整（rename, new folder, delete, download, properties）
- [ ] type-check + build 通过

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| 2026-07-20 | 步骤2后 | 用户要求 SFTP 同时支持独立 Tab 和 SSH 抽屉两种模式 |
