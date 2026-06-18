# M4b: 文件管理前端

## Context

M4a 完成了 FileConnector trait、SFTP/Local connector、传输任务模型和 REST API。M4b 实现文件浏览的后端 API 和前端文件管理页面。参考原型 `prototype/files.html`。

## 产品边界

**做什么：**
- 文件浏览 REST API（列表、元数据、创建、删除、重命名）
- 前端文件管理页面（路由 `/files/:resourceId`）
- 文件列表表格（名称、大小、修改时间）
- 面包屑路径导航
- 右键菜单（打开、下载、复制路径、重命名、删除）
- 工具栏（新建文件夹、新建文件、上传、下载、删除、重命名）
- 传输队列面板（展示传输任务列表和进度）

**不做什么：**
- 跨连接传输（后续阶段）
- 拖拽传送（后续阶段）
- SSH 内置 SFTP 面板（后续阶段）
- 移动端适配（后续阶段）
- 文件内容预览/编辑器

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 4b.1 | 文件浏览 REST API | 后端 | ✅ |
| 4b.2 | 前端文件管理页面 | 前端 | ✅ |
| 4b.3 | 右键菜单 + 工具栏交互 | 前端 | ✅ |
| 4b.4 | 传输队列面板 | 前端 | ✅ |

---

## 子任务 4b.1：文件浏览 REST API

### 功能目标

为文件操作提供 REST API，通过资源关联的 connector 执行文件浏览、创建、删除、重命名。

### 文件结构

```text
crates/rex-hub/src/
├── files.rs           新增：文件 API handlers
└── routes.rs          修改：注册路由
```

### 接口设计

```
GET  /api/resources/:resource_id/files?path=/           — 列出目录内容
GET  /api/resources/:resource_id/files/meta?path=/      — 获取文件/目录元数据
POST /api/resources/:resource_id/files/mkdir             — 创建目录
POST /api/resources/:resource_id/files/touch             — 创建文件
DELETE /api/resources/:resource_id/files?path=/data/file — 删除文件/目录
PUT  /api/resources/:resource_id/files/rename            — 重命名
```

### 数据模型

```rust
/// 目录列表响应
pub struct FileListResponse {
    pub path: String,
    pub entries: Vec<FileEntry>,  // 复用 rex_transfer::FileEntry
}

/// 重命名请求
pub struct RenameRequest {
    pub old_path: String,
    pub new_path: String,
}

/// 创建目录请求
pub struct MkdirRequest {
    pub path: String,
}

/// 创建文件请求
pub struct TouchRequest {
    pub path: String,
}
```

### 后端流程

1. 根据 resource_id 查询数据库获取资源的连接类型和配置
2. 根据连接类型创建对应的 FileConnector（SFTP 或 Local）
3. 调用 connector 的 list/metadata/write 等方法
4. 返回结果

### 测试标准

- 目录列表返回正确结构
- 文件元数据返回正确信息
- 创建目录/文件成功
- 删除文件/目录成功
- 重命名成功
- 不存在的资源返回 404
- 无文件权限返回 403

### 提交信息

```
feat: add file browsing REST API
```

---

## 子任务 4b.2：前端文件管理页面

### 功能目标

实现文件管理的全屏页面，展示远程文件列表和面包屑导航。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/Files.vue                    新增
├── features/files/
│   ├── FileList.vue                   新增：文件列表表格
│   ├── FileBreadcrumb.vue             新增：面包屑路径
│   └── useFileManager.ts             新增：文件管理 composable
├── api/files.ts                       新增：文件 API
└── router.ts                          修改：添加 /files/:resourceId 路由
```

### 前端交互（参考 prototype/files.html）

**全屏布局：**
- 顶部导航栏：面包屑路径 + 刷新按钮
- 工具栏：新建文件夹、新建文件、上传、下载、删除、重命名
- 主区域：文件列表表格（名称、大小、修改时间）
- 右侧面板：传输队列（可折叠）

**文件列表表格：**
- 文件类型图标：📁 文件夹、📄 代码文件、📦 压缩包、🖼 图片
- 双击文件夹进入
- 双击 `..` 返回上级
- 点击列头排序

**面包屑导航：**
- 可点击的路径层级导航
- 最后一项为当前目录

### 提交信息

```
feat: add file management page with list and breadcrumb
```

---

## 子任务 4b.3：右键菜单 + 工具栏交互

### 功能目标

实现文件列表的右键菜单和工具栏按钮的完整交互。

### 文件结构

```text
packages/rex-console-web/src/features/files/
├── FileContextMenu.vue                新增：右键菜单
├── FileToolbar.vue                    新增：工具栏组件
└── useFileManager.ts                  修改：添加操作方法
```

### 右键菜单（参考 prototype/files.html）

**文件/文件夹右键菜单：**
- 打开（文件夹进入，文件下载）
- 下载
- 复制路径
- 复制文件名
- 重命名（内联编辑）
- 删除（确认对话框）

**空白区域右键菜单：**
- 上传文件
- 新建文件
- 新建文件夹
- 粘贴
- 刷新
- 全选

**工具栏按钮：**
- 📁 新建文件夹 → 弹出输入框
- 📄 新建文件 → 弹出输入框
- ⬆ 上传 → 文件选择器
- ⬇ 下载 → 下载选中文件
- 🗑 删除 → 确认对话框
- ✏ 重命名 → 内联编辑

### 测试标准

- 右键菜单在正确位置显示
- 工具栏按钮触发对应操作
- 新建文件/文件夹弹出输入框
- 删除有确认对话框
- 重命名支持内联编辑

### 提交信息

```
feat: add file context menu and toolbar
```

---

## 子任务 4b.4：传输队列面板

### 功能目标

实现右侧传输队列面板，展示传输任务列表和进度。

### 文件结构

```text
packages/rex-console-web/src/features/files/
├── TransferQueuePanel.vue             新增：传输队列面板
├── TransferItem.vue                   新增：单个传输项
└── useTransferQueue.ts                新增：传输队列 composable
```

### 前端交互（参考 prototype/files.html）

**传输队列面板：**
- 可折叠/展开控制
- 面板头：传输队列 + 任务数量
- 传输项列表：
  - 状态图标（✓ 完成 / ⬆ 上传中 / ⏳ 等待）
  - 文件名
  - 进度条
  - 状态文字

**传输项交互：**
- 点击展开详情（源 → 目标路径）
- 取消按钮（仅等待/进行中状态）
- 删除按钮（仅完成/失败状态）

### 数据来源

轮询 `GET /api/transfers` 获取任务列表，更新传输状态。

### 测试标准

- 面板可折叠/展开
- 传输项正确显示状态和进度
- 取消按钮只在可取消状态显示
- 删除按钮只在完成/失败状态显示

### 提交信息

```
feat: add transfer queue panel
```

## 设计核对点

- [ ] 文件数据不经过浏览器
- [ ] 文件浏览通过后端 connector 执行
- [ ] 传输任务状态实时更新
- [ ] 右键菜单交互与原型一致
- [ ] 工具栏按钮功能完整
