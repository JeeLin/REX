# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.13.0] - 2026-07-16

### Added
- Agent 管理：注册 API（POST /api/agents/register）+ 心跳 API（POST /api/agents/:id/heartbeat）
- Agent 管理页：重写为真实 API 数据，支持令牌重置

## [0.12.0] - 2026-07-16

### Fixed
- SQL 控制台：修复 SqlConnectorFactory 断路，连接分发到 MySQL/PostgreSQL/SQLite 真实 connector

## [0.11.0] - 2026-07-16

### Added
- 工作区核心：Tab 系统协议路由（7 种协议动态渲染对应组件）
- 连接树重写：从 API 读取环境→资源数据，替代硬编码假数据
- SqlPage/RedisPage/FilesPage 嵌入工作区 Tab，支持 auto-connect/disconnect

### Changed
- Quick Connect 降级为临时连接模式（不保存为资源）
- 状态栏显示当前 Tab 真实资源信息

## [0.10.0] - 2026-07-16

### Added
- 环境管理：CRUD API（列表/详情/创建/编辑/删除）+ 环境详情页（Agent 面板 + 资源表格）
- 资源管理：CRUD API + 测试连接（SSH/MySQL/PG/Redis/SQLite/S3）+ 4 步创建向导
- 环境管理页：卡片网格展示，对接真实 API，支持创建/编辑/删除
- 资源创建向导：选择协议 → 基本信息 → 连接详情（含测试连接） → 确认
- 侧栏连接树：从 API 读取环境→资源数据，替代硬编码假数据
- 前端 API 层：environments.ts + resources.ts + environments Pinia store
- 共享协议常量：PROTOCOL_ICONS / PROTOCOL_COLORS / PROTOCOL_NAMES

## [0.9.0] - 2026-07-15

### Added
- 数据库层：SQLite schema + 迁移 + Database struct（环境/资源/Agent/审计日志/设置表）
- 认证系统：单用户密码认证（argon2 + JWT），首次设置密码 → 登录 → 路由守卫
- 路由框架：统一 AppState + auth 中间件，公开路由（认证）+ 受保护路由分离
- 审计日志：写入基础设施，所有关键操作自动记录
- 前端 API 客户端：统一 fetch 封装，自动注入 auth header，处理 401
- 前端 auth store：Pinia 状态管理，token 持久化
- 前端路由守卫：未登录跳转登录页，首次使用引导设置密码
- 登录页 / 设置密码页：真实认证对接

### Changed
- 现有 API 模块（SQL/Redis/Files）注入 auth header，支持认证后正常工作
- WebSocket 终端支持 token query param 认证

## [0.8.0] - 2026-07-15

### Added
- 管理模块：仪表盘/环境管理/Agent 管理/审计日志/设置页面使用 REX 设计系统统一视觉
- SFTP 连接器：修复 russh-sftp API 兼容性

### Changed
- 所有管理页面统一使用 Card/Badge/Button/StatusDot/Table 等 REX 设计系统组件

## [0.7.0] - 2026-07-15

### Added
- 文件管理：后端 SFTP 连接器（基于 russh-sftp，支持 list/upload/download/delete/rename/mkdir）
- 文件管理：后端 S3 连接器（基于 AWS SDK，支持 multipart 上传）
- 文件管理：文件传输 REST API（connect/list/stat/upload/download/delete/rename/mkdir）
- 文件管理：前端双面板文件浏览器（SFTP/S3 支持、活动面板模型、路径栏、面包屑、右键菜单）

## [0.6.0] - 2026-07-15

### Added
- Redis 控制台：后端 Redis 连接器（`RedisConnector` trait + REST API）
- Redis 控制台：前端键树（命名空间分组、搜索、DB 选择器、右键菜单）
- Redis 控制台：值查看器（String/Hash/List/Set/ZSet 类型表格展示）
- Redis 控制台：CLI（命令输入 + 历史 + 结果日志）
- Redis 控制台：Server Status 卡片仪表盘（版本/内存/统计/键空间）

## [0.5.0] - 2026-07-15

### Added
- SQL 控制台：后端 MySQL/PostgreSQL/SQLite 协议接入（`SqlConnector` trait + REST API）
- SQL 控制台：前端导航树（库→表/视图分组、搜索过滤、右键菜单）
- SQL 控制台：CodeMirror 6 查询编辑器（多 Tab、SQL 语法高亮、`.` 补全、代码折叠、查找替换）
- SQL 控制台：结果网格（表格视图、状态栏、行数/耗时显示）
- SQL 控制台：执行模式工具栏（Run All / Run Current / Run Selected）

## [0.4.0] - 2026-07-15

### Added
- SSH 终端：后端 WebSocket 桥接（`/ws/terminal`），通过 russh 建立 SSH 连接并双向转发数据
- SSH 终端：前端 xterm.js 集成，Pane 内自适应渲染，支持 FitAddon 自动 resize
- SSH 终端：Ctrl+F 非模态查找栏（高亮匹配、上下导航、区分大小写/整词/正则）
- SSH 终端：右键菜单（复制/粘贴/全选/清屏/查找/编码子菜单/重连/断开）
- SSH 终端：3 套主题预设（REX Default / Ubuntu / Solarized Dark）
- SSH 终端：移动端浮动工具栏（方向键、Tab、Ctrl+C/L、粘贴、字体缩放）

## [0.3.0] - 2026-07-15

### Added
- 工作空间外壳：连接树侧栏（可折叠、搜索、环境→资源分组、颜色标签）
- Tab 管理增强：右键菜单（关闭/关闭其他/关闭右侧）、设色（8 色圆点）、重命名（双击编辑）
- 分屏布局预设（Alt+1~5：单面板/左右/上下/四宫格/主+侧）
- Quick Connect 栏（协议选择 + 主机/端口/用户名 + 连接按钮）
- F1 快捷键面板（分组展示所有快捷键）
- 资源属性对话框（连接/认证/终端/笔记 4 个分类 Tab）
- UI 组件：Input、Select、Checkbox、Switch、Avatar、Alert、ToggleGroup、Scrollbar

## [0.2.0] - 2026-07-14

### Added
- 设计系统 token 完善（组件令牌、动画变量、亮色主题完整映射）
- 组件增强：Button (loading/block)、Input (clearable/error)、Select、Badge (size/dot)、Card (hoverable/footer)、Table (striped/compact/empty)、Modal (ESC/scroll lock)、Drawer (width)
- 新增组件：Checkbox、Switch、Avatar、Alert、ToggleGroup、Scrollbar
- 设计预览页更新（全部组件变体展示）

## [0.1.0] - 2026-07-14

### Added
- Rust workspace 骨架（10 crate，supervisor + worker 进程模型）
- 前端骨架（Vue 3 + Vite + TypeScript + Pinia + Router + i18n）
- 基础组件库（11 个 UI 组件：Button/Card/Badge/StatusDot/Tabs/Table/Drawer/Modal/ContextMenu/Tooltip/Toast）
- 设计系统预览页（/design-preview，token 可视化 + 暗/亮切换）
- 导航框架（AppLayout 侧栏 + 6 个 stub 页面）
- 后端代理前端（RexHub axum 静态文件服务，单端口 REX_PORT 默认 3000）
- 侧栏收起（localStorage 持久化）
- 工作区全屏模式
- 资源栏嵌入侧栏（连接树/收藏/最近）
- 工作区分栏（splitpanes，Ctrl+\ 水平/垂直分栏）
- Agent 绑定环境（环境卡片显示 Agent 状态，Agent 表格显示所属环境）
- 手机端适配（汉堡菜单 + 浮动快捷按钮）
- 多语言 zh/en 骨架
- 主题暗/亮切换
