# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).


## [0.47.1] - 2026-07-29

### Changed
- Agent 部署指南：二进制下载改为直接下载按钮 + 架构选择器（Linux/macOS/Windows）

## [0.47.0] - 2026-07-29

### Added
- Agent 注册流程：每个环境创建时自动生成独立注册令牌，Agent 用该令牌连接 Hub 时自动注册
- 环境详情页：显示当前环境的注册令牌，支持一键复制

### Changed
- Agent 认证：改用环境级注册令牌（environments.registration_token），每个环境独立
- 数据库：移除 agents 表未使用的 agent_token 列，token_hash 列不再使用

## [0.46.1] - 2026-07-29

### Fixed
- 登录页面：修复首次运行时不会跳转密码设置页的问题
- 资源创建向导：修复步骤2错误提示"主机地址为必填项"（验证条件与渲染步骤不匹配）
- 环境详情页：无 Agent 注册时隐藏无效的复制/重置按钮，显示提示信息

## [0.46.0] - 2026-07-29

### Fixed
- SSH 终端：修复底部内容截切（overflow: hidden）和切换标签重连问题（v-show 保持 DOM）
- 资源编辑模态框：根据协议类型动态显示对应字段（SSH/MySQL/Redis/SQLite/S3）
- 环境详情页：Agent Token 正确显示且支持重置

### Changed
- 隐藏 xterm.js char-measure-element 测量元素（CSS 强制隐藏）

## [0.45.0] - 2026-07-28

### Added
- Hub 自动更新机制（阶段2）：supervisor + worker 进程模型，GitHub Release 检查、下载、SHA256 校验、原子替换、健康检查、回滚
- 更新状态 REST API（check/trigger/status/rollback）
- 设置页更新 UI（版本显示、检查更新、进度、回滚）

### Changed
- 重构 redis_codec 模块从 rex-common 移至 rex-redis

### Fixed
- agent_token 字段在环境详情 API 中正确返回
- 资源编辑模态框支持协议特定字段
- SSH 终端切换标签不再重连
- SSH 终端底部行裁剪问题

## [0.44.0] - 2026-07-28

### Added
- 登录页「记住我」功能：勾选后 token 持久化到 localStorage，未勾选则仅存 sessionStorage
- 设置页密码修改：安全区块新增密码修改表单（当前密码 + 新密码 + 确认密码），后端 `/api/auth/change-password` 端点 + 审计日志
- 终端配置即时生效：设置页保存终端主题/透明度/背景图后，已打开终端通过 CustomEvent 即时更新

### Fixed
- 终端生命周期：合并重复 onMounted，修复 settings listener 潜在内存泄漏

## [0.43.0] - 2026-07-28

### Fixed
- SSH 终端：修复底部行裁剪（等待字体加载后再 fit）和顶部 ^^^^ 字符问题
- 资源管理：修复修改资源后侧栏不刷新（新增 updateResource store 方法）
- Logout 按钮：替换 Unicode 字符为 SVG 图标
- S3 连接测试：修复日志中 host 为空的问题，改为显示 endpoint

### Added
- 收藏功能：右键菜单添加收藏/取消收藏入口
- 环境详情页：展示 Agent 注册令牌并支持复制

## [0.42.0] - 2026-07-28

### Changed
- 连接模型重构：所有协议（SQL/Redis/Files）connect 端点统一为 resource_id-based 连接，前端不再传递敏感连接参数
- 新增 `resource_conn.rs` 公共模块，从 DB 读取资源记录并解密 config_json
- ResourceProperties 组件按协议类型显示 Connection/Auth 字段
- SSH 终端：alt-screen resize guard、debounced fit()、移动端箭头键修复

### Fixed
- FilesDrawer SFTP 布局：修复空栏问题 + 错误传播
- AuditLog 页面：修复无法滚动的问题
- Settings API：修复 session_timeout number→string 类型转换
- StatusDot：修复 Redis/Files 连接中状态闪烁问题
- SSH 终端底部截断：CSS flex 修复
- SSH vim 卡死：alt-screen resize guard + setTimeout/setInterval Bun 兼容类型
- MobileTerminalBar 箭头键：修复缺失的 template 标签
- SQLite 重连：修改配置后正确断开旧 session 再重连
- 工作区 + 按钮：移除无用的 tab bar 新建连接按钮

### Added
- Agent 部署操作指南：可折叠指南组件 + i18n 支持

## [0.41.0] - 2026-07-27

### Added
- 侧栏收藏/最近使用 Tab：资源可标记收藏，自动记录最近打开的资源（localStorage 持久化）
- 侧栏全局搜索：侧栏顶部搜索框，跨环境/资源名实时搜索，debounce 200ms
- 双击 Tab 分屏：双击 Tab 标签自动创建左右分屏布局
- 拖 Tab 到目标 Pane：HTML5 拖拽 API 实现 Tab 在 Pane 间移动
- i18n：新增 sidebar.search/favorites/recent/unfavorite/noFavorites/noRecent 等 key

### Fixed
- 代码质量：修复 52 个 ESLint warnings（unused vars、any 类型、default props、template shadow）
- 代码审查：修复 onPaneDrop 拖拽 Tab 时未清除源 Pane 的 critical bug
- 收藏按钮 title 属性：区分收藏/取消收藏提示文案

## [0.40.0] - 2026-07-25

### Changed
- i18n 全面补全：13 个组件的硬编码英文字符串替换为 i18n key，locale 文件从 382 增至 578 个 key（files 66、sql 93、redis 58、settings 2）
- 文件管理模块：FilesPage、FileEditorDialog、FolderSyncDialog、MobileFilesBar 全部接入 i18n
- SQL 控制台模块：SqlPage、ExportWizard、TableDesigner、GlobalQueryModal、AiAssistantDrawer、SqlResultGrid、ColumnEditor 全部接入 i18n
- Redis 模块：RedisStatus 接入 i18n
- AiAssistantDrawer：actions 数组从静态硬编码改为 computed 动态生成

## [0.39.2] - 2026-07-25

### Added
- Tab 标签右键菜单：新建连接、移动到面板（子菜单）、断开连接
- 终端右键菜单：复制地址、打开 SFTP
- SQL 导航树右键菜单：复制表名、复制 DDL、新建查询、刷新、查看 DDL、属性
- Redis 文件夹右键菜单：按前缀加载、复制前缀、删除前缀下所有键（SCAN+DEL）
- 审计日志行右键菜单：查看详情、复制记录、按类型/环境筛选、刷新、导出 CSV、清除筛选
- 环境卡片右键菜单：编辑、新建资源、删除
- 资源表格行右键菜单：在工作区打开、编辑、删除
- 侧栏资源右键菜单：在工作区打开、属性、删除

### Fixed
- SSH 终端 resize：连接建立后立即发送实际终端尺寸，修复 SSH 默认80x24 导致 vim 卡死和显示不完整

## [0.39.1] - 2026-07-25

### Fixed
- 工作区分栏：所有面板渲染同一连接的 bug，实现 per-pane 标签绑定
- 工作区标签切换：终端组件添加 :key 绑定，切换 Tab 时 Vue 正确重建组件
- 工作区侧栏打开资源：openResourceFromTree 同步更新 paneTabs，Alt+6-9 快捷键同步更新
- 退出登录按钮：topbar 添加退出按钮
- 页面标题间距：内容区域添加 padding
- 页面布局：移除多个页面的 max-width 约束，内容撑满可用宽度
- Agent 页面：直连模式下显示引导说明（Agent 仅用于隧道模式）
- Agent 页面：并行加载各环境 Agent，Token 重置/日志查看添加错误反馈
- Settings 日志：使用 HashMap 替代固定 struct，只记录实际变更的 key
- IPv6 支持：SSH/SFTP/Redis/MySQL/PostgreSQL 连接测试支持 IPv6 地址格式
- 资源创建校验：向导步骤添加必填字段验证，防止空值提交
- 审计日志：展开详情中资源 ID 替换为资源名称

### Changed
- Settings API：`SettingsUpdate` struct 替换为 `HashMap<String, String>`，支持任意 key
- Agent 页面：直连模式下隐藏快速开始按钮，显示隧道模式说明

## [0.39.0] - 2026-07-25

### Added
- Redis 操作日志：连接/断开/DB切换/Key操作/命令执行 全部添加结构化 tracing 和审计日志
- 环境/资源 CRUD 审计日志：创建/更新/删除/导入 操作写入审计日志表
- 文件传输操作日志：连接/断开/列表/重命名/创建文件夹/下载/编辑保存/ACL 操作补全日志
- Settings 变更日志 + Agent token 重置日志
- Agent 隧道统计日志：记录隧道持续时间、数据转发量、错误计数
- 日志级别规范化：统一 action 字段命名（`PREFIX_ACTION`），全局审查敏感信息不入日志

### Changed
- 日志格式统一：所有 tracing 调用添加 `action` 字段，错误日志包含 resource_id/session_id 上下文
- terminal_ws.rs：~25 个 tracing 调用添加 SSH_* action 字段
- agent_ws.rs：6 个 tracing 调用添加 AGENT_* action 字段
- file_api.rs：8 个 tracing 调用添加 FILE_* action 字段
- tunnel_ws.rs：3 个 tracing 调用添加 TUNNEL_* action 字段


## [0.38.2] - 2026-07-24

### Fixed
- WebSocket 鉴权：terminal WebSocket 连接携带 JWT token，后端 AuthUser 中间件支持 `?token=` 提取
- 侧栏资源点击：点击资源项在工作区打开对应 Tab（新增 workspace store）
- 实时资源更新：新增/删除资源后侧栏立即反映变更（envResources 响应式 Map）
- 全屏布局修复：内容区域撑满视口，移除右侧空白
- Agent 部署指南：恢复部署指南入口，支持 binary/docker/compose/config 四种方式

### Removed
- 移除工作区冗余 ConnectionTree 组件（由侧栏 ResourcePanel 替代）
- 移除侧栏收起功能（收起后资源连接信息混乱）
- 删除死代码 ConnectionTree.vue

### Changed
- 终端 SSH 连接：host/port/username 改为从 Resource 顶层字段读取（而非 config_json）
- SSH 连接日志增强：每个步骤添加详细 tracing（资源加载、认证方式、连接尝试、Agent 隧道）

## [0.38.1] - 2026-07-24

### Changed
- 升级 axum 从 0.7 到 0.8，tower 从 0.4 到 0.5，tower-http 从 0.5 到 0.6
- middleware: 移除 `#[async_trait]`，使用原生 `async fn` 语法
- WebSocket: 适配 `Message::Text(Utf8Bytes)` 和 `Message::Binary(Bytes)` 类型变更

### Fixed
- 移除 rex-hub 中未使用的 `async-trait` 依赖

## [0.38.0] - 2026-07-24

### Added
- Agent 部署指南弹窗：支持二进制下载、Docker、Docker Compose、配置文件 4 种部署方式，按 OS/架构推荐
- Agent 配置弹窗：展示元信息、服务器地址、auto_update 开关
- 审计日志统计卡片：总数/成功/失败实时统计
- 审计日志时间范围筛选：今天/7天/30天/全部预设
- 审计日志 CSV 导出：导出当前筛选条件下的所有记录
- 审计日志行展开详情：结构化展示 target、detail、agent_id
- 后端审计统计 API：GET /api/audit-log/stats

## [0.37.0] - 2026-07-23

### Added
- 工作区快捷键补全：Ctrl+N 新建连接、F11 全屏切换、Alt+6-9 标签跳转
- Agent 审计日志：WebSocket 连接/断开时写入 AGENT_ONLINE/AGENT_OFFLINE 事件
- Agent 日志查看：Agent 管理页新增日志弹窗，按 agent_id 筛选审计日志

### Fixed
- 路由参数名冲突：resource_api 中 {env_id} 与 env_api 中 {id} 导致 Axum 合并失败，统一参数命名
- unwrap() 安全：agent_ws.rs 中 4 处 serde_json::to_string().unwrap() 改为错误处理
- os/arch 输入验证：Agent 心跳上报的 os/arch 值增加白名单校验
- AuditQuery 补全 environment_id 参数传递
- 日志搜索：Agents 页面日志过滤改为大小写不敏感

## [0.36.0] - 2026-07-23

### Added
- CommandPalette：从 environments store 读取真实数据，支持搜索环境/命令/设置
- 会话超时：idle 检测（5 秒轮询）+ 60 秒警告对话框 + 自动登出
- 会话超时配置：设置页支持 15/30/60/120 分钟超时时间选择
- 警告对话框：显示倒计时，支持「续期」和「登出」操作

### Changed
- CommandPalette：所有标签使用 i18n 翻译，支持中英文切换
- 设置页：主题/语言设置双写 localStorage + 后端 settings API

### Fixed
- 会话超时登出按钮：清除 auth token 后重定向到登录页
- NaN timeout 防护：localStorage 损坏时降级到默认值 30 分钟
- CommandPalette 全局 keydown 泄漏：面板隐藏时不监听键盘事件
- session_timeout 加载：从后端读取设置时不覆盖 localStorage 值

## [0.35.1] - 2026-07-23

### Added
- Rust 单元测试：auth（JWT/密码）、db（CRUD）、error 模块共 54 个测试
- 前端单元测试：auth store、environments store 共 16 个测试

### Fixed
- 前端 lint：修复 138 个 vue/attributes-order warnings（138 → 0）

## [0.35.0] - 2026-07-23

### Added
- i18n 完整翻译：所有页面和功能组件使用 $t() 调用，中英文 231 个 key 完整覆盖
- WebSocket 心跳：终端连接每30秒发送 ping，防止代理/负载均衡器超时断开
- 安全 HTTP 响应头：X-Content-Type-Options、X-Frame-Options、Referrer-Policy、Permissions-Policy
- 语言切换即时预览：设置页切换语言后界面立即变化，无需刷新

### Fixed
- 语言切换不生效：设置页切换英文后界面无变化
- 安全头 X-XSS-Protection 已弃用：替换为 Referrer-Policy 和 Permissions-Policy
- 路由守卫：已登录用户访问 /setup 页面时正确重定向到 /workspace
- ClientMsg Ping 变体：后端正确处理客户端心跳消息

## [0.34.0] - 2026-07-23

### Added
- 后端请求日志中间件：记录 method、path、status、latency，跳过静态文件
- 审计日志增强：SSH 连接/断开、SQL 连接/查询、文件上传/删除、认证事件
- 快捷键面板 Escape 键关闭支持
- 编码子菜单：每个资源独立保存编码设置
- WorkspacePage chunk 优化：SqlPage/RedisPage/FilesPage 懒加载，689KB → 103KB

### Fixed
- Axum 路由顺序：resource_routes 注册在 env_routes 之前，防止 /{id} 拦截子路由
- 编码硬编码：Properties 对话框使用 tab.encoding 而非固定 UTF-8

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
