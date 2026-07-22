# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.33.0] - 2026-07-22

### Added
- 文件编辑器：read_for_edit / save_from_edit API（支持 SFTP 和 S3）
- 文件编辑器前端：CodeMirror 编辑对话框，语法高亮、Ctrl+S 保存
- 连接配置导入/导出 API（JSON 格式，环境+资源批量管理）
- SSH 连接保活：keepalive_interval 配置防止长连接断开

## [0.32.0] - 2026-07-22

### Added
- S3 上传断点续传：大文件上传失败后自动从已完成的分片继续
- SFTP 上传断点续传：使用 APPEND 模式从已上传字节位置继续
- upload API 支持 offset 参数：SFTP 上传支持从指定偏移开始
- 下载断点续传：download_range trait 方法，支持 Range header 从断点继续
- 传输队列 UI：显示上传/下载进度、失败状态，支持重试按钮（S3/SFTP/Download）

### Fixed
- S3 resume_multipart_upload：使用 list_parts 获取已完成的 parts，不再信任前端参数
- SFTP upload：clamp offset 防止越界 panic
- download_range limit 类型修复：改为 Option<u64>，None 表示到文件末尾
- Range header 解析：使用 splitn(2, '-') 正确处理无 end 的 Range 格式

## [0.31.0] - 2026-07-21

### Added
- S3 ACL 管理：get_acl/put_acl API + 前端 Canned ACL 编辑对话框
- S3 文件列表：显示 ACL 列
- S3 上传返回 upload_id：为断点续传做准备

### Changed
- FileEntry 新增 acl 字段（前后端一致）
- 重构：移除死代码（SessionQuery、AclQuery），统一 auth header 构造

## [0.30.0] - 2026-07-21

### Added
- S3 连接对话框：接入 S3 专用参数（Bucket/Region/Endpoint/AccessKey/SecretKey）
- S3 Storage Class 列：文件列表显示 Storage Class（Standard/IA/Glacier）
- S3 Presigned URL：右键菜单生成临时访问链接并复制到剪贴板
- S3 Multipart 断点续传：后端支持 list/resume/abort multipart uploads API

## [0.29.0] - 2026-07-21

### Added
- SFTP 移动端适配：单面板布局 + 面板切换（Left/Right segmented control）
- MobileFilesBar：底部浮动工具栏（Upload/Download/New Folder/Refresh/More 菜单）
- 响应式对话框：Chmod/删除确认/FolderSync 对话框移动端适配
- 列隐藏：移动端文件列表隐藏 Modified 列，只显示 Name + Size

## [0.28.0] - 2026-07-21

### Added
- SSH 终端主题增强：ResourceProperties 设置（主题/光标/字体/透明度/背景图）接通 TerminalView
- 终端背景透明度：opacity 滑块控制终端背景半透明效果
- 终端背景图预设：Grid（网格线）、Dots（圆点矩阵）、Gradient（对角渐变）三种纯 CSS 预设
- 全局设置页终端主题控件：Terminal Theme / Background Opacity / Background Image 下拉
- 全局终端设置缓存：SettingsPage 保存时写入 localStorage，TerminalView 读取作为 fallback

## [0.27.0] - 2026-07-21

### Added
- Redis FormatViewer 高级格式解码：支持 Msgpack / PHPSerialize / JavaSerialize / Pickle 自动探测与解码
- Redis 压缩格式检测：zlib / gzip / zstd 压缩数据自动解压并递归检测内部格式
- 后端 redis-codec 模块：rex-common 新增格式检测引擎，get_value 返回格式元数据
- 前端 FormatViewer 扩展：动态格式标签（Msgpack蓝/PHP紫/Java橙/Pickle绿/Compressed红）+ 解码结果展示

## [0.26.1] - 2026-07-20

### Fixed
- SQL 编辑器：修复 Run Current/Run Selected 模式未传递 cursorPos/selectedText 导致始终执行全部 SQL
- SQL 编辑器：剪贴板弹窗添加 onClickOutside 点击外部关闭
- SQL 编辑器：实现 onSave（Ctrl+S）下载 .sql 文件
- Redis：selectDb 检查 res.ok，切换 DB 失败时显示错误提示
- SFTP：删除文件前弹出确认对话框，防止误删
- SFTP：拖拽文件夹时自动过滤（目录不参与拖拽传输）

## [0.26.0] - 2026-07-20

### Added
- SQL 编辑器工具栏：格式化 (Ctrl+Shift+F)、注释切换 (Ctrl+/)、大小写切换 (Ctrl+Shift+U)、剪贴板栈 (Ctrl+Shift+V)、缩放 (Ctrl+=/-/0)
- Redis Stream 类型支持：消息列表 + 消费者组表格，Min/Max ID 过滤
- Redis FormatViewer：通用格式查看器，自动探测 Text/Hex/JSON/Binary
- Redis 管理功能：内存分析（键分布）、慢日志、FlushDB（带确认）
- SFTP 拖拽传输：面板间拖拽文件进行上传/下载
- SFTP 文件夹同步对话框：方向选择、比较规则、掩码配置、孤儿删除、Preview 差异列表

### Fixed
- CommandPalette.vue：移除未使用变量，修复 lint error

## [0.25.1] - 2026-07-20

### Fixed
- SQL 注入修复：MySQL/PostgreSQL connector 标识符转义
- 查询超时：30 秒超时 + 10000 行限制

### Added
- 全局搜索：Ctrl+K 打开命令面板，搜索资源/命令/设置

## [0.25.0] - 2026-07-20

### Added
- SFTP 同步浏览：双面板同步进入文件夹
- SFTP chmod 权限管理：复选框矩阵设置文件权限
- SFTP 编辑功能：临时下载文件进行编辑

## [0.24.0] - 2026-07-20

### Added
- Redis 批量操作：批量删除、批量 TTL、导入导出（JSON 格式）
- Redis 值查看器：支持「在新标签打开」，Tab 栏管理多个键值
- Redis 连接管理：编辑、删除、复制连接功能

## [0.23.0] - 2026-07-20

### Added
- SQL 全局查询：Ctrl+Shift+Q 打开跨库查询模态，多选库执行查询
- SQL AI 助手：Ctrl+Shift+A 打开右侧抽屉，上下文感知的快捷操作（分析/优化/生成/关系）
- SQL 导入向导：拖拽文件到表，支持 CSV/JSON/SQL 格式解析和导入
- SQL 结果网格内联编辑：双击单元格编辑，变更追踪，Apply/Discard 按钮
- SQL 表单视图：网格/表单切换，一次一条记录编辑

## [0.22.0] - 2026-07-20

### Added
- SQL 表设计器：多 Tab 界面（Columns/Indexes/Foreign Keys/DDL 预览），可视化查看表结构
- SQL DDL 预览抽屉：右键表 → View DDL → 底部抽屉显示 CREATE TABLE 语句
- SQL 结果网格增强：列头点击排序（asc/desc/none）、状态栏显示行数和执行时间
- SQL 导出向导：查询结果导出为 CSV/JSON/SQL 文件（前端生成）
- 后端 SQL API：新增 indexes/foreign_keys/ddl 三个端点，支持 MySQL/PostgreSQL/SQLite 三引擎

## [0.21.0] - 2026-07-20

### Added
- Tab 广播模式：「发送到全部」开关，输入同步到所有 SSH Tab，状态栏广播指示器，Ctrl+Shift+B 快捷键
- 深度资源属性对话框：连接/认证/终端/外观/保活 5 个分类 Tab，per-session 配置
- Quick Connect 增强：协议切换自动补全端口、密码字段、连接历史下拉（最近 10 条）

## [0.20.0] - 2026-07-20

### Added
- SSH Tab 集成 SFTP 文件管理抽屉（状态栏 📁 按钮 + Ctrl+B 快捷键）
- SFTP 抽屉：单面板远程文件浏览、面包屑路径导航、拖拽调整高度
- 传输队列：上传进度条、传输速率、取消传输、完成后自动清除
- 右键菜单增强：重命名、新建文件夹、上传、下载、复制路径

### Changed
- SFTP 同时支持独立 Tab 和 SSH Tab 抽屉两种模式

## [0.19.0] - 2026-07-20

### Added
- 工作区状态栏增强：显示协议+主机+端口、终端尺寸（cols×rows）、连接状态
- Tab 拖拽排序（HTML5 drag-and-drop）
- Tab 右键菜单：关闭左侧、关闭全部、复制标签

## [0.18.0] - 2026-07-17

### Added
- Hub TLS 配置框架：支持自签名（REX_TLS_SELF_SIGNED）、手动证书（REX_TLS_CERT/KEY）、ACME（REX_ACME_DOMAIN/EMAIL）三种模式
- Agent WSS 支持：自动识别 https Hub 并使用 wss 连接
- Agent TLS insecure 模式（REX_TLS_INSECURE=true）：自签名证书开发环境跳过验证
- Docker TLS 环境变量配置文档

### Changed
- TLS serve 实际监听待实现（hyper 1.x 集成复杂），当前所有 TLS 模式回退到 HTTP 并输出警告

## [0.17.0] - 2026-07-17

### Added
- Agent 自动更新机制：supervisor/worker 进程模型，版本不一致时自动推送更新
- Agent 更新处理器：下载 → SHA256 校验 → 写 update-state.json → exit(42) → supervisor 替换
- Hub 版本 API（GET /api/version）：返回 Hub 版本 + 所有 Agent 版本信息
- Agent 二进制下载端点（GET /api/agents/download?os=&arch=）
- Docker Hub 镜像打包多架构 Agent 二进制（linux/amd64, linux/arm64）
- 交叉编译脚本（scripts/build-agent-bins.sh）
- 前端 Agent 管理页：版本标签（最新/可更新）、更新进度条
- 前端设置页：Hub 版本 + Agent 版本总览

### Changed
- 版本检查通过 WebSocket 心跳完成（Agent 上报 version → Hub 对比 → 推送 update）
- Agent 更新通过 WebSocket 指令触发，不暴露 REST API

## [0.16.0] - 2026-07-17

### Added
- Agent WebSocket 隧道：单一 WebSocket 连接完成认证、心跳、资源连接、数据转发
- Hub 自动路由：前端只传 resourceId，Hub 从 DB 读取连接信息，自动判断直连/Agent 隧道
- Agent 部署指南：二进制、Docker、Docker Compose 三种方式
- Agent 隧道协议测试（8 个新测试）

### Changed
- Agent 认证：token-only（不再需要 agent_id，Hub 通过 token 查找）
- 终端 WebSocket：统一为 /ws/terminal?resourceId=xxx，前端无感知连接方式
- get_resource API：简化为只用 resource_id（resource ID 全局唯一）

### Fixed
- 全项目 0 clippy warnings（修复 rex-s3/rex-mysql/rex-postgresql/rex-hub 所有预存 warning）

## [0.15.0] - 2026-07-17

### Added
- i18n：所有页面文本使用 $t() 调用，中英文翻译完整覆盖
- 主题持久化：localStorage + 后端设置 API 双重持久化
- 响应式适配：移动端底部导航栏，各页面响应式布局
- 凭据加密：AES-256-GCM 加密存储资源密码和私钥
- 单元测试：crypto（4 tests）+ auth（3 tests）

### Fixed
- Auth middleware：修复 state 注入方式，从 extensions 获取改为 from_extractor_with_state
- Auth 错误消息：中文硬编码 → 英文

## [0.14.0] - 2026-07-16

### Added
- Dashboard：统计 API（环境数/资源数/在线 Agent）+ 页面重写
- 审计日志：查询 API（时间/操作/结果筛选）+ 页面重写
- 设置：设置 API + 页面重写，主题/语言/终端配置持久化

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
