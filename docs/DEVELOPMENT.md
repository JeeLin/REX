# REX Hub — 开发文档

本文档是开发索引。产品功能和设计决策请参阅 [PRODUCT.md](PRODUCT.md)。架构决策、数据模型、API 设计等详细参考信息见各子文档。

---

## 1. 技术栈

| 层 | 技术 |
|---|---|
| 后端 | Rust（tokio async） |
| 前端 | Vue 3 + Vite + TypeScript |
| 终端 | xterm.js |
| 通信 | WebSocket + HTTPS |
| 加密 | TLS 1.3, AES-256-GCM, ECDHE-X25519 |
| 数据库 | SQLite（Hub 本地） |

---

## 2. Rust crate 结构

```text
crates/
├── rex-common        通用类型、错误定义、CLI、supervisor、版本、更新状态
├── rex-ssh           SSH/SFTP 协议实现
├── rex-mysql         MySQL 协议实现
├── rex-postgresql    PostgreSQL 协议实现
├── rex-transfer      文件传输引擎（FileConnector trait）
├── rex-hub           Hub 二进制入口（整合所有 crate + 前端静态资源）
└── rex-agent         Agent 二进制入口（整合所有 crate）
```

Hub 和 Agent 共享所有协议 crate，区别在于：

```text
rex-hub   = 所有 crate + 前端静态资源（embedded）
rex-agent = 所有 crate（无前端）
```

---

## 3. 里程碑总览

> **2.0 重设计起点**：在 `main` 分支从 0 开始全面重写，git 历史已清理为单一 2.0 基线提交。视觉语言自有（现代化/极客化/易用化），交互布局对标 Xshell / Navicat / ARDM / Xftp。旧 0.x 里程碑（M0–M26、0.2.0–0.87.0）保留在 `backup/before-reset` 分支作参考，不在此重复罗列。

| 里程碑 | 标题 | 对标 | 状态 |
|--------|------|------|------|
| **M0** | 项目骨架重建 | — | ⬜ |
| **M1** | 设计系统与组件库 | REX 自有 | ⬜ |
| **M2** | 工作空间外壳 | Xshell | ⬜ |
| **M3** | SSH 终端 | Xshell | ⬜ |
| **M4** | 数据库控制台 | Navicat | ⬜ |
| **M5** | Redis 控制台 | ARDM | ⬜ |
| **M6** | 文件管理（SFTP+S3） | Xftp | ⬜ |
| **M7** | 管理模块 + 打磨收尾 | REX 自有 | ⬜ |

### M0：项目骨架重建

**核心功能**：清空 `packages/rex-console-web` 与 `crates/*` 源码，按新设计系统重建最小可运行骨架。

子任务：
1. Rust workspace 骨架（Cargo.toml `[workspace]` + `[workspace.dependencies]`、rex-common/rex-hub/rex-agent 空 crate、supervisor+worker 入口）
2. 前端骨架（Vue3 + Vite + TS + Pinia + Router + i18n 初始化、目录按功能域组织）
3. 设计 token 基础（CSS 变量、深色主题、字体）
4. 导航框架（侧栏 + 路由 + 登录/仪表盘空壳页）+ `bun run build` / `cargo build` 跑通

依赖：git 基线（redesign-2.0 从 main 派生）
版本类型：minor
版本号：1.0.0（2.0 重设计首个版本）

### M1：设计系统与组件库

**核心功能**：建立 REX 统一设计系统 + 基础组件库，供所有模块复用。

子任务：
1. 设计 token 完善（spacing/radius/shadow/语义色/组件令牌）
2. 基础组件：Button / Badge / Card / Table / Tabs / Drawer / Modal / Toast / ContextMenu / Tooltip / Scrollbar
3. 布局框架：AppLayout（侧栏+顶栏+内容）、移动端底部导航
4. 设计系统测试 + 收尾

依赖：M0
版本类型：minor

### M2：工作空间外壳（Xshell 模式）

**核心功能**：连接树侧栏、Quick Connect、Tab 管理（设色/重命名/广播）、Pane 拖拽分屏、状态栏、快捷键面板、资源属性对话框。

子任务：
1. 连接树侧栏（环境→资源分组、可折叠、搜索、颜色标签、拖拽组织）
2. Tab 管理（单排滚动、状态点、右键菜单、设色/重命名/广播）
3. Pane 分屏（拖拽缩放、5 种布局、Tab 跨 Pane 拖动）
4. 状态栏 + 快捷键面板（F1）+ Quick Connect 栏
5. 资源属性对话框（分类树深度配置）
6. 测试与收尾

依赖：M1
版本类型：minor

### M3：SSH 终端（Xshell 模式）

**核心功能**：xterm.js 终端、光标/选择/滚动缓冲、Ctrl+F 查找、右键菜单（bracketed paste/编码子菜单）、内置 SFTP 抽屉、主题与 Highlight Sets。

子任务：
1. 终端核心（xterm.js 集成、光标/选择/滚动缓冲、状态栏联动）
2. 终端内查找栏（Ctrl+F 非模态）
3. 右键菜单（复制/粘贴/bracketed/清屏/编码子菜单/重连）
4. 内置 SFTP 抽屉（复用 SSH 通道，引用 M6 文件面板）
5. 主题系统（ANSI 调色板 + 透明度 + Highlight Sets 正则高亮）
6. 移动端浮动工具栏
7. 测试与收尾

依赖：M2
版本类型：minor

### M4：数据库控制台（Navicat 模式）

**核心功能**：两栏布局（导航树+对象区）、库表树、查询编辑器（`.` 补全/折叠/查找/格式化/Run 模式）、结果网格（内联编辑/Apply-Discard/导出）、表设计器多 Tab、导入导出向导、DDL 抽屉。后端 MySQL/PostgreSQL/SQLite 协议接入。

子任务：
1. 后端协议接入（get_sql_connector 支持三库、REST/WebSocket 端点）
2. 导航树（连接组→库→表/视图/函数/过程/事件+字段/索引/外键、搜索、右键）
3. 查询编辑器（Tab 多开、语法高亮、补全、折叠、查找替换、剪贴板栈、执行模式）
4. 结果网格（内联编辑、排序/过滤、Apply/Discard、表单视图、导出）
5. 表设计器（字段/索引/外键/约束/选项/DDL 多 Tab）
6. DDL 抽屉 + 导入/导出向导 + 全局查询 + AI 助手整合
7. 测试与收尾

依赖：M1
版本类型：minor

### M5：Redis 控制台（ARDM 模式）

**核心功能**：双面板（连接+键树 / 固定右栏值查看器，可开 Tab）、键树虚拟滚动+命名空间+SCAN 分页+多选、DB 选择器、FormatViewer、集合表格编辑器、Monaco CLI、Server Status、批量操作。

子任务：
1. 后端 Redis 连接器（AUTH/SELECT/INFO/SCAN、各类型 GET/SET、DB 列表）
2. 连接+键树（虚拟滚动、分隔符命名空间+count 徽章、SCAN 流式分页、搜索、Shift 多选、右键）
3. 值查看器（通用 FormatViewer + 集合表格编辑器：String/Hash/List/Set/ZSet/Stream）
4. CLI（Monaco 只读日志 + 命令输入）
5. Server Status 卡片仪表盘 + 自动刷新
6. 批量操作（删除/TTL/导入导出）
7. 测试与收尾

依赖：M1
版本类型：minor

### M6：文件管理（Xftp 模式，SFTP+S3）

**核心功能**：双面板（本地/远程或 S3 prefix）、活动面板传输模型、传输队列抽屉（进度/吞吐/暂停恢复/续传）、路径栏+面包屑、同步浏览+文件夹同步、chmod 矩阵、Edit 临时下载回传。后端 SFTP+S3 连接器接入。

子任务：
1. 后端 SFTP 连接器（rex-ssh SFTP + rex-transfer 任务调度/并发/续传）
2. 后端 S3 连接器（rex-s3：bucket/prefix、multipart 续传）
3. 双面板 UI（活动面板模型、路径栏、面包屑、Details 表格）
4. 传输队列抽屉（进度/吞吐/↑↓方向/暂停恢复/取消/断点续传）
5. 右键菜单（Edit 临时下载回传、chmod 矩阵）+ 同步浏览 + 文件夹同步
6. 移动端上下分栏 + 浮动工具栏
7. 测试与收尾

依赖：M1
版本类型：minor

### M7：管理模块 + 打磨收尾

**核心功能**：仪表盘/环境/Agent/审计日志/设置/Notebook 用 REX 设计系统统一，全局查询、AI 助手整合，最终 i18n、移动端、测试、文档收尾。

子任务：
1. 仪表盘（统计卡片 + 快速连接 + 环境卡片）
2. 环境管理（列表/详情/创建向导）
3. Agent 管理（卡片/配置/日志/令牌/部署指南/更新状态）
4. 审计日志（筛选/统计/详情/导出）
5. 设置（外观/终端/安全/更新版本总览）
6. Notebook（block 编辑器，如需要保留）
7. 全局打磨：i18n 补全、移动端全适配、性能、无障碍
8. 测试与收尾（cargo fmt/clippy/test + 前端 type-check/lint/build 全绿、CHANGELOG、DEVELOPMENT 状态更新）

依赖：M2, M3, M4, M5, M6
版本类型：minor
版本号：1.0.0

---

## 4. 架构文档

详细架构决策和设计原理：

| 文档 | 内容 |
|------|------|
| [进程模型](architecture/process-model.md) | supervisor + worker、退出码语义、Windows 差异 |
| [更新机制](architecture/update-mechanism.md) | update-state.json、状态流转、原子替换、回滚 |
| [文件传输](architecture/file-transfer.md) | FileConnector trait、传输路径、冲突处理 |
| [连接通道](architecture/connection-channels.md) | 直连资源、Agent 代理、WebSocket 隧道协议 |
| [Docker](architecture/docker.md) | 信号处理、Dockerfile、部署限制 |

---

## 5. 参考文档

开发时查阅的参考信息：

| 文档 | 内容 |
|------|------|
| [数据模型](reference/data-models.md) | SQLite 表结构、凭据加密、资源配置 |
| [API 设计](reference/api-design.md) | 认证、错误格式、分页、WebSocket 消息 |
| [前端工程](reference/frontend-architecture.md) | 页面路由、功能域组织、组件规范 |
| [配置约定](reference/config-conventions.md) | Hub/Agent 配置、目录结构、后端工程结构 |

---

## 6. 里程碑详细文档

每个里程碑的详细设计和实现记录：

```text
docs/milestones/
├── M0-project-skeleton.md
├── M1-hub-management.md
├── M2-agent-connection.md
├── M3a-ssh-backend.md
├── M3b-ssh-terminal-frontend.md
├── M4a-file-transfer-backend.md
├── M4b-file-management-frontend.md
├── M5a-sql-backend.md
├── M5b-sql-frontend.md
├── M6-update-detection.md
├── M7-auto-update.md
├── M8-frontend-refinement.md
├── M9-release.md
├── M10-audit-settings.md
├── M11-context-menus.md
├── M12-agent-management-enhancements.md
├── M13-workspace-shell.md
├── M14-workspace-panel-integration.md
├── M15-agent-deploy-guide.md
├── M16-wizard-and-tabmenu.md
├── M17-workspace-tabmenu.md
├── M18-terminal-sftp-sql-contextmenu.md
├── M19-sql-tree-tab-contextmenu.md
├── M20-workspace-panel-drag.md
├── M21-terminal-mobile-statusbar.md
├── M22-sidebar-favorites-recent.md
├── M23-file-upload-download.md
├── M24-docs-restructure.md
├── M25-sql-query-files.md
└── M26-sql-history-and-polish.md
├── 0.6.0-hub-https-and-agent-download.md
├── 0.7.0-hub-acme-auto-cert.md
├── 0.16.0-sql-ai-assistant.md
└── 0.7.0-reports/
```
