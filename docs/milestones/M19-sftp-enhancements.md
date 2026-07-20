# M19: SFTP 文件管理增强

## Context

M0–M18 完成了基础设施和核心功能。FilesPage.vue 已有双面板布局、文件列表、多选、可拖拽分隔条。但产品文档描述的 Xftp 体验还缺少传输队列和完整右键菜单。

本里程碑版本类型：minor（新功能），版本号 0.19.0 → 0.20.0。

## 产品边界

**本阶段做：**
- 传输队列面板（进度条、传输速率、暂停/恢复、取消）
- 右键菜单增强（重命名、新建文件夹、属性/权限）
- 拖拽传输（从一个面板拖到另一个面板）
- 面包屑路径导航

**本阶段不做：**
- 同步浏览（两侧联动）
- 文件夹同步对话框
- 传输队列持久化（刷新后清空）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 传输队列面板 | ⬜ |
| 2 | 右键菜单增强 + 拖拽传输 + 面包屑 | ⬜ |

## 子任务详细设计

### 1 传输队列面板

**功能目标**

底部可折叠面板，显示所有进行中和已完成的传输任务。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 添加传输队列 UI

**交互设计**

- 传输队列固定在底部，高度约 120px，可折叠
- 每个传输项：文件名 + 方向（↑上传/↓下载）+ 进度条 + 速率 + 状态
- 底部状态栏：总进度 + 总速率 + 活跃任务数
- 传输完成后保留 30 秒后自动清除
- 暂停/恢复/取消按钮（每项 + 全局）

**数据模型**

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

**实现要点**

- 上传：使用 `filesApi.uploadFile` 时包装进度回调（XMLHttpRequest 或 fetch + ReadableStream）
- 下载：`filesApi.downloadFile` 已返回 blob，改为流式下载以获取进度
- 速率计算：每秒采样 transferred 差值

**测试标准**

- 上传多个文件 → 队列显示每项进度 → 全部完成
- 下载文件 → 队列显示进度 → 完成后触发浏览器下载
- 暂停/恢复传输
- 取消传输
- 折叠/展开队列面板

**提交信息**

```
feat(files): add transfer queue panel with progress and speed tracking
```

### 2 右键菜单增强 + 拖拽传输 + 面包屑

**功能目标**

完善文件管理交互：完整右键菜单、拖拽传输、路径面包屑。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 添加右键菜单项、拖拽事件、面包屑

**右键菜单新增项**

| 项目 | 操作 |
|------|------|
| Open | 双击进入目录/下载文件 |
| Rename | 内联编辑文件名 |
| New Folder | 弹出输入框创建目录 |
| Delete | 确认后删除 |
| Download | 下载到本地 |
| Upload Here | 上传到当前目录 |
| Copy Path | 复制完整路径 |
| Properties | 显示文件大小、权限、修改时间 |

**拖拽传输**

- 从左面板拖文件到右面板 → 上传（本地→远程）
- 从右面板拖文件到左面板 → 下载（远程→本地）
- 拖拽时显示目标面板高亮
- 松开后加入传输队列

**面包屑路径**

- 路径栏改为可点击的面包屑
- 每级目录可点击跳转
- 根目录显示 `/`

**测试标准**

- 右键 → Rename → 输入新名 → 回车 → 文件重命名
- 右键 → New Folder → 输入名 → 创建目录
- 拖拽文件到另一面板 → 传输队列显示进度
- 面包屑点击 → 跳转到对应目录
- type-check + build 通过

**提交信息**

```
feat(files): enhance context menu, drag transfer, and breadcrumb navigation
```

## 设计核对点

- [ ] 传输队列显示进度和速率
- [ ] 暂停/恢复/取消传输
- [ ] 右键菜单完整（rename, new folder, delete, download, properties）
- [ ] 拖拽传输可用
- [ ] 面包屑路径导航
- [ ] type-check + build 通过

## Flow Status

- [x] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
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
