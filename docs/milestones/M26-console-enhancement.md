# M26: 控制台增强（SQL 编辑器工具栏 + Redis Stream/FormatViewer + SFTP 拖拽/同步）

## Context

M0–M25 完成了从项目骨架到安全加固的全部开发。功能审计发现 PRODUCT.md 中定义的多项功能尚未实现：SQL 编辑器缺少格式化工具栏/剪贴板栈/缩放；Redis 不支持 Stream 类型和高级格式查看器；SFTP 缺少拖拽传输和文件夹同步对话框。本里程碑补全这些核心功能缺口。

本里程碑版本类型：minor（新功能），版本号 0.25.1 → 0.26.0。

## 产品边界

**本阶段做：**
- SQL 编辑器增强：格式化工具栏（格式化/注释/大小写/剪贴板栈/缩放）+ 快捷键
- Redis Stream 支持：Stream 类型值查看 + 消费者组表格
- Redis FormatViewer：通用格式查看器（Text/Hex/JSON/Binary/Msgpack/PHPSerialize）
- Redis 管理功能：内存分析 + 慢日志 + FlushDB
- SFTP 拖拽传输：面板间拖拽文件传输
- SFTP 文件夹同步对话框：方向/比较/掩码/孤儿删除/预览

**本阶段不做：**
- SQL XML 导入格式（需要额外 XML 解析库，后续里程碑）
- Redis CMD 批量导入（后续里程碑）
- SFTP 移动端浮动工具栏（后续里程碑）
- SSH SFTP 抽屉通道复用（需要后端 SSH session 共享，后续里程碑）
- 终端背景图/透明度（后续里程碑）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | SQL 编辑器增强（工具栏 + 快捷键） | ✅ |
| 2 | Redis Stream 类型支持（值查看 + 消费者组） | ✅ |
| 3 | Redis FormatViewer（通用格式查看器组件） | ✅ |
| 4 | Redis 管理功能（内存分析 + 慢日志 + FlushDB） | ✅ |
| 5 | SFTP 拖拽传输（面板间拖拽文件） | ✅ |
| 6 | SFTP 文件夹同步对话框 | ✅ |

## 子任务详细设计

### 1 SQL 编辑器增强（工具栏 + 快捷键）

**功能目标**

为 SQL 编辑器添加格式化工具栏和键盘快捷键，对标 Navicat 的编辑器体验。

**文件结构**

修改：
- `packages/rex-console-web/src/features/sql/SqlEditor.vue` — 添加缩放、剪贴板栈、注释切换
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — 添加工具栏按钮

新建：
- `packages/rex-console-web/src/features/sql/sql-format.ts` — SQL 格式化工具函数

**交互设计**

工具栏（编辑器上方）：

```
┌──────────────────────────────────────────────────────────────────────┐
│ [Run ▾] [Run Current] [Run Selected]  │  [_format] [//] [Aa▼] [📋] [+][-][0] │
│                                                                        │
│  1  SELECT * FROM users                                                 │
│  2  WHERE status = 'active'                                            │
│  3  ORDER BY created_at DESC;                                           │
└──────────────────────────────────────────────────────────────────────┘
```

工具栏按钮（右侧）：
- 格式化（`Ctrl+Shift+F`）— 美化 SQL 缩进和换行
- 注释（`Ctrl+/`）— 切换选中行注释（`-- ` 前缀）
- 大小写（`Ctrl+Shift+U`）— 切换选中文本大小写
- 剪贴板栈（`Ctrl+Shift+V`）— 显示最近 10 项复制历史，选择粘贴
- 缩放（`Ctrl+=` / `Ctrl+-` / `Ctrl+0`）— 编辑器字体缩放

**实现流程**

1. `sql-format.ts`：实现简单的 SQL 格式化函数（关键字大写、缩进对齐、逗号换行）
2. `SqlEditor.vue`：
   - 添加 `fontSize` state，`Ctrl+=/-/0` 快捷键调整
   - 添加 `clipboardHistory` array（最多 10 项），监听 `copy` 事件自动记录
   - `Ctrl+Shift+V` 打开剪贴板历史选择弹窗
   - `Ctrl+/` 切换注释（选中行添加/移除 `-- ` 前缀）
   - `Ctrl+Shift+U` 切换大小写
   - 暴露 `format()`, `toggleComment()`, `toggleCase()` 方法供工具栏调用
3. `SqlPage.vue`：
   - 在编辑器上方添加工具栏按钮行
   - 格式化按钮调用 `editorRef.format()`
   - 注释/大小写/剪贴板/缩放同理

**测试标准**

- `Ctrl+Shift+F` 格式化 SQL（缩进对齐）
- `Ctrl+/` 切换注释
- `Ctrl+Shift+U` 切换大小写
- `Ctrl+Shift+V` 显示剪贴板历史
- `Ctrl+=/-/0` 缩放编辑器
- type-check + build 通过

**提交信息**: `feat(sql): add editor toolbar with format, comment, case, clipboard stack, and zoom`

### 2 Redis Stream 类型支持（值查看 + 消费者组）

**功能目标**

支持 Redis Stream 数据类型的查看和管理，包括消息列表、消费者组表格。

**文件结构**

修改：
- `packages/rex-console-web/src/features/redis/RedisPage.vue` — 添加 Stream 值查看分支 + 消费者组 Tab

**交互设计**

Stream 值查看器：

```
┌─ Stream: mystream ──────────────────────────────────────────────────┐
│ [Messages] [Consumer Groups]  |  Min ID: 0  Max ID: +  [Filter]   │
│                                                                         │
│ Messages Tab:                                                          │
│ ┌───┬─────────────┬────────────────────────────┬──────────────────┐   │
│ │ # │ ID          │ Field                      │ Value            │   │
│ ├───┼─────────────┼────────────────────────────┼──────────────────┤   │
│ │ 1 │ 1234567890-0│ {"event":"login","user":1} │                  │   │
│ │ 2 │ 1234567890-1│ {"event":"logout","user":2}│                  │   │
│ └───┴─────────────┴────────────────────────────┴──────────────────┘   │
│                                                                         │
│ Consumer Groups Tab:                                                    │
│ ┌───┬─────────────┬──────────┬────────┬──────────┬───────────────┐   │
│ │ # │ Group       │ Consumers│ Pending│ Last Delivered│ Lag      │   │
│ ├───┼─────────────┼──────────┼────────┼──────────┼───────────────┤   │
│ │ 1 │ mygroup     │ 2        │ 5      │ 12345    │ 3             │   │
│ └───┴─────────────┴──────────┴────────┴──────────┴───────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

**实现流程**

1. 在 `RedisPage.vue` 的值查看器中添加 `Stream` 分支
2. Messages Tab：使用 `XRANGE` / `XREVRANGE` 获取消息，分页加载
3. Consumer Groups Tab：使用 `XINFO GROUPS` / `XINFO CONSUMERS` 获取消费者组信息
4. 支持 Min/Max ID 过滤
5. 工具栏：添加 Stream 消息（`XADD`）、删除消息（`XDEL`）
6. 键树添加 `type-stream` 样式类

**测试标准**

- Stream 键在键树中正确显示（紫色图标）
- 点击 Stream 键显示消息列表
- 消费者组 Tab 显示组信息
- Min/Max ID 过滤正常
- type-check + build 通过

**提交信息**: `feat(redis): add Stream type support with messages and consumer groups`

### 3 Redis FormatViewer（通用格式查看器组件）

**功能目标**

创建通用格式查看器组件，自动探测值的格式并支持多种显示模式。

**文件结构**

新建：
- `packages/rex-console-web/src/features/redis/FormatViewer.vue` — 通用格式查看器

修改：
- `packages/rex-console-web/src/features/redis/RedisPage.vue` — String 值使用 FormatViewer

**交互设计**

```
┌─ FormatViewer ──────────────────────────────────────────────────────┐
│ Format: [Auto ▾]  Size: 256 bytes                                   │
│                                                                      │
│ ┌─ Text ─────────────────────────────────────────────────────────┐  │
│ │ {"name":"John","age":30,"scores":[95,87,92]}                    │  │
│ └─────────────────────────────────────────────────────────────────┘  │
│                                                                      │
│ ┌─ JSON (pretty) ───────────────────────────────────────────────┐   │
│ │ {                                                               │   │
│ │   "name": "John",                                               │   │
│ │   "age": 30,                                                    │   │
│ │   "scores": [95, 87, 92]                                        │   │
│ │ }                                                               │   │
│ └─────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

格式标签（Format tabs）：
- Text — 原始文本
- Hex — 十六进制显示
- JSON — 尝试解析 JSON 并美化
- Binary — 不可打印字符显示为 `[Hex]`

自动探测逻辑：
1. 尝试 UTF-8 解码 → 成功则为 Text
2. 尝试 JSON.parse → 成功则显示 JSON 标签
3. 包含不可打印字符 → 显示 Hex 标签
4. 检测 Msgpack 前缀（`0xc0`-`0xdf` 等）→ 显示 Msgpack 标签（暂不解码，标记为二进制）

**实现流程**

1. 创建 `FormatViewer.vue` 组件
   - Props: `value: string | Uint8Array`, `type?: string`
   - Format tabs 切换显示模式
   - 自动探测格式
2. 在 `RedisPage.vue` 的 String 值查看中使用 FormatViewer
3. 显示值大小信息

**测试标准**

- Text 值正常显示
- JSON 值自动探测并美化
- 二进制值显示 Hex 标签
- 格式切换正常
- type-check + build 通过

**提交信息**: `feat(redis): add FormatViewer with auto-detection and multi-format display`

### 4 Redis 管理功能（内存分析 + 慢日志 + FlushDB）

**功能目标**

为 Redis 连接添加管理功能：内存分析、慢日志查看、FlushDB 操作。

**文件结构**

修改：
- `packages/rex-console-web/src/features/redis/RedisPage.vue` — 连接右键菜单添加管理选项
- `packages/rex-console-web/src/features/redis/RedisStatus.vue` — 添加内存分析和慢日志 Tab

**交互设计**

连接右键菜单（扩展）：

```
┌─ Context Menu ──────────┐
│ Edit Connection         │
│ Copy Connection         │
│ ─────────────────────── │
│ Memory Analysis    📊   │
│ Slow Log           📋   │
│ Flush DB           ⚠️   │
│ ─────────────────────── │
│ Delete Connection       │
└─────────────────────────┘
```

内存分析弹窗：

```
┌─ Memory Analysis: redis-local ─────────────────────────────────────┐
│                                                                     │
│ Memory Used: 12.5 MB    Peak: 15.2 MB    Fragmentation: 1.05       │
│                                                                     │
│ Key Distribution (Top 20):                                         │
│ ┌──────────────────┬──────┬──────────┬────────────────────┐        │
│ │ Pattern          │ Keys │ Est. Size│ % of Total         │        │
│ ├──────────────────┼──────┼──────────┼────────────────────┤        │
│ │ user:*           │ 1250 │ 3.2 MB   │ 25.6%  ██████████  │        │
│ │ session:*        │ 890  │ 2.1 MB   │ 16.8%  ███████     │        │
│ │ cache:*          │ 450  │ 1.8 MB   │ 14.4%  ██████      │        │
│ └──────────────────┴──────┴──────────┴────────────────────┘        │
│                                                                     │
│ Total Keys: 5,847    Key Types: str(3200) hash(1500) list(800)     │
└─────────────────────────────────────────────────────────────────────┘
```

慢日志查看：

```
┌─ Slow Log: redis-local ────────────────────────────────────────────┐
│                                                                     │
│ ┌───┬───────────┬────────────┬──────────┬──────────────────────┐   │
│ │ # │ Time      │ Duration   │ Client   │ Command              │   │
│ ├───┼───────────┼────────────┼──────────┼──────────────────────┤   │
│ │ 1 │ 14:23:01  │ 150ms      │ 10.0.0.1 │ KEYS user:*          │   │
│ │ 2 │ 14:20:45  │ 85ms       │ 10.0.0.2 │ SORT biglist         │   │
│ └───┴───────────┴────────────┴──────────┴──────────────────────┘   │
│                                                     [Clear] [×]    │
└─────────────────────────────────────────────────────────────────────┘
```

FlushDB（带确认）：

```
┌─ Flush Database: redis-local ──────────────────────────────────────┐
│                                                                     │
│ ⚠️ This will permanently delete ALL keys in db0.                   │
│                                                                     │
│ This action cannot be undone.                                       │
│                                                                     │
│ [Cancel]                                    [Flush DB]              │
└─────────────────────────────────────────────────────────────────────┘
```

**实现流程**

1. 连接右键菜单添加三个管理选项
2. 内存分析：调用 `MEMORY USAGE` 采样 + `INFO memory` + `SCAN` 按前缀聚合
3. 慢日志：调用 `SLOWLOG GET 50` 获取慢查询
4. FlushDB：调用 `FLUSHDB`（带确认弹窗），连接方式下才启用
5. 在 RedisStatus.vue 中添加对应的 Tab/弹窗

**测试标准**

- 内存分析显示键分布
- 慢日志显示最近查询
- FlushDB 需要确认才能执行
- type-check + build 通过

**提交信息**: `feat(redis): add memory analysis, slow log, and flush DB features`

### 5 SFTP 拖拽传输（面板间拖拽文件）

**功能目标**

支持在双面板之间拖拽文件进行传输，对标 Xftp 的拖拽传送。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 添加文件拖拽逻辑

**交互设计**

```
┌─ FilesPage ─────────────────────────────────────────────────────────┐
│ [Toolbar]                                                            │
│ ┌──────────────────────┬────┬──────────────────────┐                │
│ │ Local: /home/user    │ ║  │ Remote: /var/www      │                │
│ │                      │ ║  │                       │                │
│ │ 📁 projects          │ ║  │ 📁 html               │                │
│ │ 📄 readme.md    [拖]→║║══║║→ 📁 logs              │                │
│ │ 📄 config.json       │ ║  │ 📄 index.html         │                │
│ │ 📁 downloads    [拖]→║║══║║→ 📄 app.css            │                │
│ │                      │ ║  │                       │                │
│ └──────────────────────┴────┴──────────────────────┘                │
│ [Transfer Queue: 2 items]                                            │
└─────────────────────────────────────────────────────────────────────┘
```

拖拽行为：
- 拖拽源面板的文件/文件夹 → 拖到目标面板区域
- 释放时触发传输（上传或下载，取决于方向）
- 拖拽时显示视觉反馈（目标面板高亮 + 方向箭头）
- 支持多选文件拖拽
- 拖拽文件夹时递归传输

**实现流程**

1. 文件行添加 `draggable="true"` 属性
2. `ondragstart`：记录拖拽的文件列表
3. 面板区域 `ondragover`：显示高亮反馈（阻止默认行为）
4. 面板区域 `ondrop`：根据方向调用上传/下载 API
5. 传输项推入传输队列
6. 拖拽时显示半透明拖拽预览（可选）

**测试标准**

- 拖拽文件从本地到远程 → 触发上传
- 拖拽文件从远程到本地 → 触发下载
- 多选文件拖拽正常
- 拖拽文件夹递归传输
- type-check + build 通过

**提交信息**: `feat(files): add drag-and-drop transfer between panels`

### 6 SFTP 文件夹同步对话框

**功能目标**

实现文件夹同步对话框，支持双向比较和同步操作。

**文件结构**

新建：
- `packages/rex-console-web/src/features/files/FolderSyncDialog.vue` — 文件夹同步对话框

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 右键菜单添加"同步"选项，集成对话框

**交互设计**

```
┌─ Folder Sync ──────────────────────────────────────────────────────┐
│                                                                     │
│ Source: /home/user/projects         Target: /var/www/html           │
│                                                                     │
│ Direction:                                                          │
│ ○ Upload (Source → Target)                                         │
│ ● Download (Target → Source)                                       │
│ ○ Bidirectional                                                    │
│                                                                     │
│ Compare By:                                                         │
│ ☑ Size    ☑ Modified Time    ☐ Checksum                           │
│                                                                     │
│ Include: [*.html,*.css,*.js    ]  Exclude: [node_modules/**     ]  │
│                                                                     │
│ ☐ Delete orphan files in target                                     │
│                                                                     │
│ Preview (3 changes):                                               │
│ ┌────┬──────────────────┬──────────┬──────────┬──────────────┐     │
│ │ #  │ File             │ Action   │ Size     │ Modified     │     │
│ ├────┼──────────────────┼──────────┼──────────┼──────────────┤     │
│ │ 1  │ index.html       │ Update   │ 2.1 KB   │ 2026-07-20   │     │
│ │ 2  │ new-file.js      │ Copy     │ 0.5 KB   │ 2026-07-20   │     │
│ │ 3  │ old-file.bak     │ Delete   │ 1.2 KB   │ 2026-07-15   │     │
│ └────┴──────────────────┴──────────┴──────────┴──────────────┘     │
│                                                                     │
│ [Cancel]  [Preview]  [Sync Now]                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**实现流程**

1. 右键菜单或工具栏添加"文件夹同步"选项
2. 打开 `FolderSyncDialog.vue`
3. 用户配置方向、比较规则、掩码
4. 点击 Preview：后端比较两个目录（`ls -la` 递归），返回差异列表
5. 用户确认后执行同步（逐文件传输 + 可选删除孤儿）
6. 显示进度

**测试标准**

- 对话框正确显示源/目标路径
- 方向选择正确
- Preview 显示差异列表
- 掩码过滤正确
- type-check + build 通过

**提交信息**: `feat(files): add folder sync dialog with preview and diff`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ 架构一致（前端组件，无新增后端 API）
- ✅ 不引入多用户/RBAC 概念
- ✅ 不跳阶段实现
- ✅ 实现细节不污染产品文档

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

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
