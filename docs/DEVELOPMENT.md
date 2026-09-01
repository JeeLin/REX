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
├── rex-common        通用类型、错误定义、CLI、supervisor 模块、sip_media 媒体帧编解码、更新状态
├── rex-ssh           SSH/SFTP 协议实现
├── rex-mysql         MySQL 协议实现
├── rex-postgresql    PostgreSQL 协议实现
├── rex-sqlite        SQLite 协议实现
├── rex-redis         Redis 协议实现（含 FormatViewer 解码）
├── rex-s3            S3/MinIO 协议实现（含 multipart 续传）
├── rex-sip           SIP 电话（baresip FFI：UA/音频桥/视频桥/抓包/CDR/录音）
├── rex-transfer      文件传输引擎（FileConnector trait）
├── rex-hub           Hub 二进制入口（整合所有 crate + 前端静态资源 + WebSocket 隧道模块）
└── rex-agent         Agent 二进制入口（整合所有 crate + WebSocket 隧道）
```

Hub 和 Agent 共享所有协议 crate，区别在于：

```text
rex-hub   = 所有 crate + 前端静态资源（embedded）
rex-agent = 所有 crate（无前端）
```

---

## 3. 里程碑总览

> **里程碑基线**：开发按里程碑（M0 → M80）串行推进，历史进度记录于此表。早期里程碑保留在 `backup/before-reset` 分支作参考。

| 里程碑 | 标题 | 对标 | 状态 |
|--------|------|------|------|
| **M0** | 项目骨架重建 | — | ✅ |
| **M1** | 设计系统与组件库 | REX 自有 | ✅ |
| **M2** | 工作空间外壳 | Xshell | ✅ |
| **M3** | SSH 终端 | Xshell | ✅ |
| **M4** | 数据库控制台 | Navicat | ✅ |
| **M5** | Redis 控制台 | ARDM | ✅ |
| **M6** | 文件管理（SFTP+S3） | Xftp | ✅ |
| **M7** | 管理模块 + 打磨收尾 | REX 自有 | ✅ |
| **M8** | 基础设施层（DB + Auth + 路由） | — | ✅ |
| **M9** | 环境 + 资源管理 | — | ✅ |
| **M10** | 工作区核心（Tab 协议路由 + 组件嵌入） | — | ✅ |
| **M11** | SQL 控制台接通（SqlConnectorFactory 修复） | — | ✅ |
| **M12** | Agent 管理（注册 + 心跳 + 管理页面） | — | ✅ |
| **M13** | 管理页面（Dashboard + 审计日志 + 设置） | — | ✅ |
| **M14** | 收尾（i18n + 主题 + 响应式 + 加密 + 测试） | — | ✅ |
| **M15** | Agent WebSocket 隧道（内网穿透） | — | ✅ |
| **M16** | Agent 自动更新 + Docker 打包 | — | ✅ |
| **M17** | TLS/HTTPS 支持 | — | ✅ |
| **M18** | 工作区核心增强（分屏拖拽 + Tab 管理 + 状态栏） | — | ✅ |
| **M19** | SFTP 文件管理增强（SSH 抽屉 + 传输队列） | — | ✅ |
| **M20** | 工作区高级功能（广播模式 + 深度属性 + Quick Connect 增强） | Xshell | ✅ |
| **M21** | SQL 控制台高级功能（表设计器 + DDL 预览 + 导出向导） | Navicat | ✅ |
| **M22** | SQL 控制台完善（全局查询 + AI 助手 + 导入向导 + 内联编辑） | Navicat | ✅ |
| **M23** | Redis 控制台增强（批量操作 + 值查看器增强 + 导入导出） | ARDM | ✅ |
| **M24** | SFTP 文件管理增强（同步浏览 + chmod + 编辑功能） | Xftp | ✅ |
| **M25** | 安全加固 + 工作区增强（SQL 注入修复 + 全局搜索） | — | ✅ |
| **M26** | 控制台增强（SQL 编辑器工具栏 + Redis Stream/FormatViewer + SFTP 拖拽/同步） | Navicat/ARDM/Xftp | ✅ |
| **M27** | Bugfix & Polish（SQL 执行模式修复 + UX 完善） | — | ✅ |
| **M28** | Redis FormatViewer 高级格式解码（Msgpack/Pickle/PHP/Java/压缩） | ARDM | ✅ |
| **M29** | SSH 终端主题增强（背景图 + 透明度） | Xshell | ✅ |
| **M30** | SFTP 移动端适配（浮动工具栏 + 上下分栏） | Xftp | ✅ |
| **M31** | S3 文件管理增强（连接参数/Storage Class/Presigned URL/断点续传） | Xftp | ✅ |
| **M32** | S3 ACL 管理（ACL 列 + 编辑对话框） | Xftp | ✅ |
| **M33** | 文件传输断点续传（S3/SFTP 上传续传 + 下载续传 + 传输队列改进） | Xftp | ✅ |
| **M34** | 文件编辑器 + 连接导入导出 + SSH 保活 | Xftp/Xshell | ✅ |
| **M35** | 工作区 Xshell 体验补全（快捷键面板 + 编码 + 状态栏 + Quick Connect） | Xshell | ✅ |
| **M36** | 后端日志增强（请求日志中间件 + 审计日志） | — | ✅ |
| **M37** | i18n 完整翻译 + 性能优化 + 生产加固 | — | ✅ |
| **M38** | 测试覆盖 + Lint 清理 | — | ✅ |
| **M39** | 集成完善 + 会话管理 | — | ✅ |
| **M40** | 工作区快捷键 + Agent 日志查看 | — | ✅ |
| **M41** | Agent 部署指南 + 审计日志增强 | — | ✅ |
| **M42** | Axum 0.8 升级 + 路由参数修复 | — | ✅ |
| **M43** | 前端交互修复 + WebSocket 鉴权 | — | ✅ |
| **M44** | 后端操作日志补全 | — | ✅ |
| **M45** | 工作区 Bug 修复 | — | ✅ |
| **M46** | 右键上下文菜单补全 | — | ✅ |
| **M47** | i18n 全面补全 | — | ✅ |
| **M48** | 侧栏增强 + 工作空间 Tab 交互 | Xshell | ✅ |
| **M49** | 连接模型重构（resource_id 统一） | — | ✅ |
| **M50** | UX Bug 修复与交互打磨 | — | ✅ |
| **M51** | 登录安全增强 + 设置页完善 | — | ✅ |
| **M52** | Hub 自动更新机制（阶段2） | — | ✅ |
| **M53** | Bug fix + UX polish | — | ✅ |
| **M54** | Bug fixes | — | ✅ |
| **M55** | Agent 注册流程修复 | — | ✅ |
| **M56** | UX polish & stability | — | ✅ |
| **M57** | Stability, Mobile & Security | — | ✅ |
| **M58** | Performance, Accessibility & Documentation | — | ✅ |
| **M59** | Production Hardening & Integration Tests | — | ✅ |
| **M60** | i18n Completion, Data Export & Search Enhancement | — | ✅ |
| **M61** | Theme Optimization & Notification System | — | ✅ |
| **M62** | Health Monitoring & WebSocket Enhancement | — | ✅ |
| **M63** | Performance Optimization & Stability | — | ✅ |
| **M64** | Bug Fix & UX Polish | — | ✅ |
| **M65** | Auth & Environment Improvements | — | ✅ |
| **M66** | Mobile Adaptation & Interaction Enhancement | — | ✅ |
| **M67** | Security Hardening & Audit Enhancement | — | ✅ |
| **M68** | Performance Optimization | — | ✅ |
| **M69** | Developer Experience | — | ✅ |
| **M70** | Data Export & Backup | — | ✅ |
| **M71** | i18n Completion | — | ✅ |
| **M72** | Quality & Documentation | — | ✅ |
| **M73** | Test Coverage & Integration Tests | — | ✅ |
| **M74** | Bug Fix Round | — | ✅ |
| **M75** | Bug Fix & UX Polish | — | ✅ |
| **M76** | Bug Fix | — | ✅ |
| **M77** | Bug Fix | — | ✅ |
| **M78** | 全系统 UX/UI 重设计 | — | ✅ |
| **M79** | 任意方向分屏系统 + 工作空间重构 | — | ✅ |
| **M80** | M78 重设计收尾（feature 组件 token 化迁移） | — | ✅ |
| **M81** | SQL 查询保存 + SSH 初始化脚本 + 缺陷修复与覆盖率补全 | — | ✅ |
| **M82a** | SIP 电话资源基础（信令打通 + Agent 链式 UA + 资源模型） | 软电话 | ✅（0.70.0） |
| **M82b** | 浏览器实时双向音频（PCM-over-WebSocket 媒体通道） | 软电话 | ✅（0.70.1） |
| **M82c** | 音视频增强 + 通话记录 + 录音 + 抓包 + 质量监控 | 软电话 | ✅（0.70.2） |
| **M82d** | 测试补全 + 文档重写 + 全量 review/优化 + 压测 | — | ✅ 已完成（0.70.3） |
| **M82e** | SIP 资源按名称管理 + 多账户切换 | 软电话 | ✅ 已完成（0.70.4） |
| **M82f** | 缺陷池清理 + SIP 配置收口（3 🟢 收口） | 软电话/SIP | ✅ 已完成（0.70.5） |
| **M83** | 统一 Hub↔Agent 隧道架构（Agent 终结协议、Hub 仅作传输隧道） | — | ✅ 已完成（v0.70.6） |
| **v0.70.8** | Hub/Agent 服务化管理（CLI 子命令 + 开机自启） | — | ✅ 已完成（v0.70.8） |
| **v0.71.0** | 全站 UI/UX 对齐原型重设计 | — | ✅ 已完成（v0.71.0） |
| **v0.71.1** | 原型对齐修复（patch，修复 v0.71.0 遗留偏差） | — | ✅ 已完成（v0.71.1） |
### M0：项目骨架重建

**核心功能**：清空 `packages/rex-console-web` 与 `crates/*` 源码，按新设计系统重建最小可运行骨架。

子任务：
1. Rust workspace 骨架（Cargo.toml `[workspace]` + `[workspace.dependencies]`、rex-common/rex-hub/rex-agent 空 crate、supervisor+worker 入口）
2. 前端骨架（Vue3 + Vite + TS + Pinia + Router + i18n 初始化、目录按功能域组织）
3. 设计 token 基础（CSS 变量、深色主题、字体）
4. 导航框架（侧栏 + 路由 + 登录/仪表盘空壳页）+ `bun run build` / `cargo build` 跑通

依赖：git 基线
版本类型：minor
版本号：0.1.0

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

**核心功能**：仪表盘/环境/Agent/审计日志/设置用 REX 设计系统统一，全局查询、AI 助手整合，最终 i18n、移动端、测试、文档收尾。

子任务：
1. 仪表盘（统计卡片 + 快速连接 + 环境卡片）
2. 环境管理（列表/详情/创建向导）
3. Agent 管理（卡片/配置/日志/令牌/部署指南/更新状态）
4. 审计日志（筛选/统计/详情/导出）
5. 设置（外观/终端/安全/更新版本总览）
6. 全局打磨：i18n 补全、移动端全适配、性能、无障碍
8. 测试与收尾（cargo fmt/clippy/test + 前端 type-check/lint/build 全绿、CHANGELOG、DEVELOPMENT 状态更新）

依赖：M2, M3, M4, M5, M6
版本类型：minor

### M34：文件编辑器 + 连接导入导出 + SSH 保活

**核心功能**：实现应用内文件编辑器（临时下载→编辑→保存回传）、连接配置导入/导出（JSON 格式批量管理）、SSH 连接保活（KeepAlive 防断线）。

子任务：
1. 文件编辑器后端：临时下载 API + 保存回传 API（SFTP/S3 统一）
2. 文件编辑器前端：Monaco Editor 内嵌编辑（语法检测、保存/另存为、编辑历史）
3. 连接配置导入/导出：JSON 格式导入/导出资源和环境配置
4. SSH 连接保活：后端 KeepAlive 配置 + 前端资源属性对话框保活设置
5. 测试与收尾

依赖：M33
版本类型：minor
版本号：0.33.0

### M20：工作区高级功能（广播模式 + 深度属性 + Quick Connect 增强）

**核心功能**：Tab 广播模式（输入同步到多个 SSH Tab）、深度资源属性对话框（per-session 配置）、Quick Connect 协议感知增强。

子任务：
1. Tab 广播模式（「发送到全部」开关，输入同步到所有 SSH Tab，状态栏广播指示器）
2. 深度资源属性对话框（连接/认证/终端/外观/保活/隧道分类 Tab，复用创建向导配置）
3. Quick Connect 增强（协议自动补全端口、密码字段、连接历史下拉）
4. 测试与收尾

依赖：M18, M19
版本类型：minor
版本号：0.21.0


### M47：i18n 全面补全

**核心功能**：审计并补全所有仍有硬编码英文的组件（文件管理、SQL 控制台、Redis Status、设置），确保切换语言时全站无遗漏。

子任务：
1. 文件管理 i18n：FilesPage / FileEditorDialog / FolderSyncDialog / MobileFilesBar
2. SQL 控制台 i18n：SqlPage / ExportWizard / TableDesigner / GlobalQueryModal / AiAssistantDrawer / SqlResultGrid / ColumnEditor
3. Redis Status i18n：RedisStatus 组件
4. 设置页 i18n：语言选择器选项文本
5. locale 文件同步 + type-check/lint 验证

依赖：M46
版本类型：minor
版本号：0.40.0
### M48：侧栏增强 + 工作空间 Tab 交互

**核心功能**：侧栏增加收藏/最近使用 tab + 全局搜索；工作空间支持双击 Tab 分屏和拖 Tab 到目标 Pane。

子任务：
1. 侧栏收藏/最近使用 tab：Pinia store 持久化到 localStorage，资源右键菜单收藏/取消收藏
2. 侧栏全局搜索：实时搜索资源名/描述，按环境分组，高亮匹配
3. 双击 Tab 进入左右分屏：Tab 添加 dblclick 事件，自动创建分屏布局
4. 拖 Tab 到目标 Pane：Tab draggable + Pane dragover/drop，移动 Tab 归属
5. i18n + type-check/lint 验证

依赖：M47
版本类型：minor
版本号：0.41.0
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
├── M47-i18n-completion.md
├── M48-sidebar-workspace.md
└── 0.7.0-reports/
```

### M64：Bug 修复与 UX 优化

**核心功能**：修复已知 bug、优化用户体验、完善交互细节。

子任务：
1. 右键菜单图标大小统一
2. 资源创建向导优化（协议图标、颜色选择器）
3. 连接树状态显示优化
4. 前端组件样式一致性检查
5. i18n 翻译补全

依赖：M63
版本类型：patch
版本号：0.55.1

### M66：移动端适配与交互增强

**核心功能**：响应式布局优化、触摸手势支持、移动端键盘适配、文件管理移动优化。

子任务预估：5 个（底部导航优化、触摸手势、键盘适配、文件管理移动优化、测试）
依赖：M65
版本类型：minor
版本号：0.57.0

### M67：安全加固与审计增强

**核心功能**：CSP 安全头、CSRF 保护、审计日志增强（操作回溯）、安全审计报告。

子任务预估：4 个（CSP/CSRF、审计日志增强、安全报告、测试）
依赖：M66
版本类型：minor
版本号：0.58.0

### M68：性能优化

**核心功能**：懒加载、虚拟滚动、资源压缩、缓存策略。

子任务预估：4 个（懒加载路由、虚拟滚动、资源压缩、测试）
依赖：M67
版本类型：minor
版本号：0.59.0

### M69：开发体验

**核心功能**：开发者工具、调试面板、错误追踪、日志增强。

子任务预估：4 个（调试面板、错误追踪、日志增强、测试）
依赖：M68
版本类型：minor
版本号：0.60.0

### M81：SQL 查询保存 + SSH 初始化脚本 + 缺陷修复与覆盖率补全 ✅ 已完成

**核心功能**：
1. SQL 查询文件保存（后端支撑的全局命名列表，支持保存/打开/重命名/删除）
2. SSH 连接成功后执行初始化脚本（如 cd 到指定目录；`SshConfig` 增加 `init_script` 字段并在 terminal 会话建立后执行）
3. 修复 M80 阶段沉淀的缺陷（🔴 更新检查降级、🟡 审计日志分页不可见、🟡 分栏不作用于聚焦 pane）
4. 测试覆盖率补全至 90% 门槛

**子任务预估**：
- SQL 查询保存：
  - 后端 API + 持久化（复用 `settings` 表存储命名查询列表的 JSON）
  - 前端保存/打开/重命名/删除 UI（命名查询列表弹层）
- SSH 初始化脚本：
  - `rex-ssh` 的 `SshConfig` 增加 `init_script`，`SshSession` 建立后逐行发送执行
  - 前端在资源连接配置中增加「初始化脚本」输入框，写入 `config_json`
- 缺陷修复：
  - 🔴 更新检查：改用 `/releases` 列表取真正最高语义化版本，或仅当 latest 严格大于 current 才更新
  - 🟡 审计日志分页：常显分页控件（数据少显示「共 N 条」），补每页条数/跳页/总数，后端 `ORDER BY time DESC, id DESC` 稳定排序
  - 🟡 分栏聚焦：让「当前聚焦/最近交互的 pane」可靠写回 `activePaneId`（监听 `focusin`/`pointerdown` 或 split 时取最近聚焦 leaf）
- 测试覆盖率：补齐 Rust 单元/集成测试 + 前端测试，使 `cargo llvm-cov --workspace` 与前端覆盖率达 90%

依赖：M80
版本类型：minor
版本号：0.69.0

### M82a：SIP 电话资源基础（信令打通 + Agent 链式 UA + 资源模型） ✅ 已完成

**核心功能**：新增第 8 种资源类型 `sip`（SIP 电话）。Hub 侧 baresip UA（UA₁）打通拨号/接听/挂断/保持/转 DTMF；Agent 侧 baresip UA（UA₂）经现有 WebSocket 隧道链式转发（channel_id 多路复用，复用 `agent_ws.rs`），内网 SIP 服务器由 Agent 出网；前端 `features/sip/` 拨号盘 + 通话状态；复用 terminal_ws 的 `/ws` 范式建立 `/ws/sip` 控制/事件通道。M82a 只打通信令层（能发起并结束一通电话），音频在 M82b。

**子任务预估**：
- `crates/rex-sip`：baresip FFI 封装（C 库 libre/retask，需 `-sys`/预编译 + FFI），Hub 与 Agent 两个二进制共用，分别当 UA₁/UA₂
- 资源模型加 `sip` 协议（第 8 种）+ 资源创建向导 SIP 配置段（服务器/账号/密码/认证）
- Hub UA₁ + `/ws/sip` 控制/事件消息模型（dial/answer/hangup/hold/unhold/dtmf ↔ registered/incoming/callState/sipMessage）
- Agent UA₂ 隧道链式转发（channel_id 复用现有 `agent_ws.rs`，无需新隧道）
- 前端 `features/sip/`：Dialpad + 通话状态组件（复用设计 token）
- 前后端联调：经 Hub 直连与经 Agent 链式各拨通一通电话（信令层）

**依赖**：M81
**版本类型**：minor
**版本号**：0.70.0

> 技术风险：baresip 是 C 库，FFI 封装与跨平台编译（预编译 `.a` 或 build.rs 编 libre）是 M82a 唯一显著风险点，规划时优先验证；baresip 不能跑在浏览器，必须在 Hub/Agent 服务端。

### M82b：浏览器实时双向音频（PCM-over-WebSocket 媒体通道） ✅ 已完成

**核心功能**：Hub/Agent 终止 RTP，抽出原始 S16LE PCM，经 WebSocket 媒体通道（**原始 PCM 二进制帧，不线上编码**）实时推流到浏览器；浏览器用 Web Audio 原生消费播放，麦克风经 `getUserMedia` 采集原始 PCM、反向回传，实现浏览器与对端**实时双向**通话（满足「时时对话」）。这是产品原则「媒体不经过浏览器」的显式例外（用户明确要求浏览器实时听/说），媒体为实时流而非批量文件传输。

**子任务预估**：
- Hub/Agent 侧 RTP 收包 + 原始 S16LE PCM 抽取（baresip `ausrc`/`auplay` 驱动，从 UA 抽 PCM，不碰 `call_audio()` 私有结构）
- `/ws/sip` 二进制媒体帧协议（**原始 PCM 小端 i16，无 kind 字节**）+ 浏览器↔Hub 上行麦克风帧（与信令文本帧区分）
- Agent UA₂ 媒体经隧道 binary 帧（首字节 kind 区分媒体/信令，channel_id 多路复用）链式转发
- 前端 Web Audio 播放（ScriptProcessor 原生消费 PCM）+ 麦克风采集（getUserMedia）+ 回声抑制（gain=0）+ 延迟/抖动基础优化
- 延迟/回声优化与联调（端到端可听可说）

**依赖**：M82a
**版本类型**：minor
**版本号**：0.70.1

### M82c：音视频增强 + 通话记录 + 录音 + 抓包 + 质量监控（0.70.2） ✅ 已完成

**核心功能**（承接用户规划）：在 M82b 浏览器实时双向音频基础上，补齐通话可观测性与媒体能力：
- **视频支持**：浏览器实时视频（复用 M82b 的 `vidbridge`/`vidsrc` 同构机制 + 前端 WebCodecs + `<video>`，与音频媒体通道共用隧道/WebSocket 帧）。
- **通话录音**：Hub 落盘（baresip aufile / 媒体帧捕获）为 mp3/wav，前端回放（进度/暂停/下载）。
- **信令抓包**：UA₁/UA₂ 两侧 SIP 报文捕获导出 pcap，前端回看/下载。
- **通话记录（CDR）**：起止时间/对端/时长/状态持久化（SQLite）+ 前端表格（筛选/排序/详情）。
- **音视频质量监控**：实时质量指标——丢包率、端到端延迟大小、抖动，前端可视化展示。
- **多种音视频格式支持**：可选，按需扩展线上封装/存储格式（默认维持原始 PCM 边界，必要时再引入）。

**子任务预估**：
- 视频媒体通道：vidbridge/vidsrc 驱动 + 前端 `<video>`/WebCodecs 播放与采集
- CDR 数据模型 + Hub 持久化 + 列表/详情 API + 前端表格组件
- 录音：Hub 媒体帧捕获落盘（mp3/wav）+ 前端回放器
- 信令抓包：UA₁/UA₂ 报文捕获 + pcap 导出 + 前端回看/下载
- 质量监控：丢包率/延迟/抖动采集 + 前端可视化
- 关联与联调：CDR 挂录音 + 抓包，前后端联调与测试

**依赖**：M82b
**版本类型**：minor
**版本号**：0.70.2
**缺陷池 bug**（从 docs/BUGS.md 全量纳入，规划时已清空缺陷池，dev-flow 步骤1 落成里程碑文档 Bugs 表 ⬜）：
- 🔴 版本更新检查会降级下载旧版（`update_checker.rs` 仅比 `releases/latest`，需改 `/releases` 列表取最高语义化版本或仅 latest 严格大于 current 才更新）
- 🟡 审计日志分页不可见/单薄（常显分页控件 + 每页条数/跳页/总数；后端 `ORDER BY time DESC, id DESC` 稳定排序）
- 🟡 分栏不作用于当前聚焦 pane（`activePaneId` 仅 click 更新，需 `focusin`/交互入口或 split 时取最近聚焦 leaf）
- 🟡 S3 双栏无意义应合并单栏 + 移动操作移右键菜单
- 🟡 SQL 后端子类维护分散，按 mysql/pgsql/sqlite 归并分组
- 🟢 手机端不需要快捷键按钮（隐藏/改触屏友好）
- 🟢 手机端仪表盘 Quick Connect 仍两列（统一单列）
- 🟢 `sip_media.rs` 冗余 `u8` 类型转换（clippy `cast_same_type`）
- 🟢 baresip FFI 封装 clippy 样式建议（`Arc not Send+Sync` 为已知误报，重构时一并处理）

### M82d：测试补全 + 文档重写 + 全量 review/优化 + 单侧/整体/压测（0.70.3） ✅ 已完成

**核心功能**（承接用户规划，在 0.70.0–0.70.2 功能稳定后做质量收口，范围是**整个 REX 代码库**，不限于 SIP）：
- **测试代码补充**：补齐全仓库各模块单侧与集成测试覆盖。
- **文档重写**：基于当前实际功能与逻辑重写相关文档（以代码事实为准，去除过时描述），涵盖产品/开发/架构/里程碑文档。
- **注释更新**：补全/修正全仓库代码注释（模块头、FFI 契约、关键流程说明），与最终实现一致。
- **全量代码 review + 简化、优化**：对整个代码库做 review，消除重复、过度设计、拆大函数、统一风格。
- **单侧、整体测试、压测**：单元测试 + 端到端集成测试 + 媒体通道等热点压测（并发通话/高帧率稳定性）。

**子任务预估**：
- 测试补全：全仓库（rex-* crates / 前端 features）单侧 + 集成测试
- 文档重写：PRODUCT/DEVELOPMENT/架构/里程碑文档对齐实际实现
- 注释更新：全仓库代码注释核对与更新
- review + simplify：按 dev-flow 步骤4/5 维度全量过一遍
- 压测：媒体帧吞吐/多路并发等热点验证

**依赖**：M82c
**版本类型**：minor
**版本号**：0.70.3

### M82e：SIP 资源按名称管理 + 多账户切换（0.70.4）

**核心功能**（承接用户反馈 /dev-bug：当前 SIP 资源管理按「账户」维度，应改为按「名称」维度——一个名称注册到某个 SIP 服务器，并在该名称下支持多个账户切换）。这是对 M82a 信令层资源/账户数据模型的重构，属结构性变更，**不在 0.70.1（M82b）范围内**（0.70.1 明确「不改动信令层行为」，仅增媒体通道）。

**核心功能**：
- 资源模型维度调整：`sip` 资源从「账户」维度改为「名称」维度——一个名称绑定一个 SIP 服务器（注册域），名称下挂载多个账户（用户名/密码/认证）。
- 资源创建向导改造：先选名称 + 服务器，再在名称下维护多个账户并可切换当前生效账户。
- 后端配置模型（`SipConfig` 等）拆分「server profile」与「account」两层，Hub/Agent UA 按当前生效账户注册。
- 前端 `features/sip/` 资源选择/切换 UI 适配名称 + 多账户模型。
- 数据迁移：存量「账户」维度资源平滑升级为「名称（单账户）」维度。

**依赖**：M82d
**版本类型**：minor（数据模型迁移属向后兼容的渐进改造）
**版本号**：0.70.4

### M82f：缺陷池清理 + SIP 配置收口（0.70.5）✅ 已完成

**核心功能**：消费 0.70.4 步骤5 代码审查遗留的 3 个 🟢 缺陷池条目，收口 SIP 配置层的镜像与逃生舱，消除类型/逻辑漂移并减少多余往返。无新功能，纯质量收口。

**子任务预估**：
1. `load_sip_conn` 移除顶层 `host` 回退逃生舱（模型已声明 server 完全下沉账户；移除属行为收敛，仅影响 legacy/异常 payload）
2. 抽出共享 `SipProfile` TS 类型 + "active 或 first" 解析规则（消除 `load_sip_conn` / `SipPage.parseSipProfile` / `WizardModal.buildConfig` 三处镜像的类型/逻辑漂移）
3. 新增 `set_active_account` 专用端点，消除 `SipPage.selectAccount` 切换账户先 `get` 再 `update` 的多余 GET 往返

**依赖**：M82e
**版本类型**：patch（无新功能，仅缺陷池 🟢 收口 + 重构）
**版本号**：0.70.5
**缺陷池 bug**：移除 load_sip_conn 顶层 host 回退逃生舱（🟢）、抽出共享 SipProfile TS 类型与 active/first 解析规则（🟢）、selectAccount 减少多余 GET 往返（🟢）— 已从 docs/BUGS.md 纳入并在规划时删除对应行

### v0.70.6：统一 Hub↔Agent 隧道架构（Agent 终结协议、Hub 仅作传输隧道） ✅ 已完成
- **核心功能**：把 SSH / SQL / Redis / 文件(S3/SFTP) 的协议终结从 Hub 下沉到 Agent——Agent 成为私网内协议执行引擎，Hub 退化为单条 WebSocket 上的通用传输隧道 + 控制面 + UI。复用既有「单 WS + `channel_id` 多路复用」范式（M82 验证）。修掉 agent 模式 SSH 现仅做裸字节桥接、看到服务端 banner 却进不了 shell 的缺陷；并让 SQL/Redis/文件在 agent 模式下真正经隧道送达 Agent 执行（当前 Hub 直接连目标、agent 模式形同虚设）。同时收敛 connect 消息中 `password`/`privateKey` 的信任边界。
- **子任务预估**：7 个（隧道契约固化、协议会话消息 schema、Agent SSH 执行层 + 修 banner bug、Agent 终结 SQL、Agent 终结 Redis、Agent 终结 S3/SFTP、Hub 隧道侧重构 + 凭据信任边界）
- **依赖**：M82f（0.70.5）
- **版本类型**：minor（向后兼容，前端连接树/工作空间交互无变化，仅 agent 模式资源真正可用）
- **版本号**：v0.70.6
- **缺陷池 bug**：无（docs/BUGS.md 为空，属新架构落地而非缺陷修复）

### v0.70.7：SQL 资源模型合并（单一 SQL 资源 + 连接时自动识别 dialect） ✅ 已完成（v0.70.7）
- **核心功能**：将并列的 MySQL / PostgreSQL / SQLite 三种资源合并为单一「SQL」资源。用户创建时只选「SQL」并填连接信息，**连接时自动识别 dialect**（SQLite：无 host / 有 file_path；MySQL vs PostgreSQL：端口预判 3306/5432 → 双线缆协议握手回退 → `SELECT VERSION()` 确认；识别后回写资源 config，之后直连秒开）。后端 `sql_api` / agent `agent_sql` 已按 db_type 分支，本里程碑仅新增「连接入口 dialect 探测」+ 收敛资源模型 + 前端 + 一次性 in-place 迁移，协议/隧道层零改动（与 v0.70.6 正交）。**当前仅支持 MySQL / PostgreSQL / SQLite 三种；新方言（如 MariaDB/ClickHouse/Oracle/SQL Server）不在本里程碑范围，探测表预留扩展位但本次不实现。**
- **子任务预估**：6 个（① 资源模型 `db_type: Option` 收敛为单一「SQL」+ 探测结果回写字段；② 连接入口 dialect 探测：Hub 直连侧 + Agent 隧道侧对称实现，含端口预判/握手回退/VERSION() 确认；③ 前端连接树合并 3 类 SQL 为单「SQL」类型、按探测结果着色、创建向导不强制选 subtype；④ 前端 SQL 控制台按实际 dialect 路由连接器（Navicat 对标，行为不变）；⑤ 旧 mysql/postgresql/sqlite 资源 in-place 升级为 `sql`+探测字段；⑥ 回归测试：直连 / agent 模式、三种 dialect 均可用）
- **依赖**：v0.70.6（隧道/协议下沉已就绪）
- **版本类型**：minor（向后兼容：旧 mysql/postgresql/sqlite 资源 in-place 升级为 `sql`；单用户自托管，迁移脚本无破坏性）
- **版本号**：v0.70.7
- **缺陷池 bug**：无（docs/BUGS.md 为空）

### v0.70.8：Hub/Agent 服务化管理（CLI 子命令 + 开机自启） ✅ 已完成（v0.70.8）

**核心功能**：为 Hub / Agent 引入统一的 `clap` CLI 子命令入口（`run`/`version`/`service`），并实现「一键注册为操作系统服务」的开机自启能力（Linux systemd + macOS launchd）。`service install` 把当前相关 env 写入生成的单元文件，使服务以与手工启动一致的方式运行。

**子任务预估**：
- `rex-common::cli`：共享 CLI 框架（`Cli`/`Commands`/`ServiceCmd` + `dispatch()`）
- `rex-common::service`：平台检测 + systemd 单元 / launchd plist 生成 + install/uninstall/start/stop/restart/status（生成逻辑可单测）
- Hub CLI 接入：重构 `crates/rex-hub/src/rex-hub.rs` 的 `main()`，`run` 保留 supervisor/worker 分支
- Agent CLI 接入：重构 `crates/rex-agent/src/rex-agent.rs` 的 `main()`，复用同一套 CLI
- 配置注入与文档：`service install` 写入当前相关 env；更新部署文档
- 测试与收尾：单测（单元内容 / 平台检测 / plist XML / CLI 解析）+ fmt/clippy/test/build 全绿

**依赖**：v0.70.7（进程模型、更新机制已稳定）
**版本类型**：minor（新增功能，向后兼容；现有 env 配置方式不变）
**版本号**：v0.70.8
