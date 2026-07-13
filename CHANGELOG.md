# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.86.0] - 2026-07-13

### Added
- 文件传输并发数量控制：用户可在设置页面配置同时传输的最大任务数（1-10），修改后立即生效
- Redis 批量导入：支持 JSON 格式批量导入 Redis 数据，带预览和进度展示
- Agent 二进制下载 GitHub Release 兜底：本地缺失时自动从 GitHub 下载
- SQL 查询历史数据库筛选：历史面板新增数据库下拉筛选器
- SQL 结果表格固定列与列宽拖拽：首列固定，列宽可拖拽调整

### Fixed
- 前端 API 路径修正：settings.ts /api/ 前缀重复（5处）、notebook.ts /api/ 前缀重复（10处）
- 备份导出/导入 API 路径修正：backup.ts raw fetch 缺少 /api 前缀（3处）
- 备份 API 认证头作用域修正：token 变量作用域修复
- Agent 下载路径匹配：同时支持扁平布局和嵌套布局（CI 子目录结构）
- 标签页拖拽同步：reorderTab 后内容面板跟随激活
- 传输并发管理器内部可变性修复：使用 parking_lot::Mutex 替代裸字段赋值

## [0.85.1] - 2026-07-13

### Changed
- CI 构建优化：Docker layer caching（GHA cache）复用构建层
- CI 构建优化：rust-cache 添加 shared-key 和 save-if，提升缓存命中率
- CI 构建优化：build-hub 解耦 build-frontend 依赖，构建并行度提升
- CI 构建优化：aarch64 交叉编译从 cross 切换到 cargo-zigbuild，缩短编译时间

## [0.85.0] - 2026-07-13

### Fixed
- 工作空间稳定性确认：分屏拖拽、CI 构建已验证正常
- SqlResults 测试修复：更新表头索引以匹配 checkbox 列

### Removed
- 删除废弃的 continue.sh 脚本


## [0.84.3] - 2026-07-13

### Fixed
- SSH 终端复制粘贴修复：Ctrl+C/V 在终端容器任意位置聚焦时均可用（不再要求 xterm textarea 必须获得焦点）
- 终端复制操作增加 toast 反馈：复制成功时右上角显示「已复制」
- 工作空间终端同步修复：WorkspaceTerminal 同样支持容器级快捷键和 toast 反馈

## [0.84.2] - 2026-07-13

### Fixed
- 工作空间布局持久化：分屏模式切换后刷新页面自动恢复
- 全局快捷键作用域修复：Ctrl+K/Ctrl+N/F1 在终端聚焦时仍可用
- 工作空间终端快捷键补全：Ctrl+Shift+F 切换 SFTP 面板
- 移除 AppLayout 中未使用的 `globalShortcutsEnabled` 死代码

## [0.84.1] - 2026-07-12

### Fixed
- SQLite 连接失败错误提示修复：前端 onclose 不再覆盖后端返回的真实错误消息
- Agent 状态显示修正：基于 last_seen_at 新鲜度判定在线状态，消除 Hub 重启后的虚假在线
- 未配置 Agent 提示：在线但未下发配置的 Agent 显示「⚠ 未配置」提示

## [0.84.0] - 2026-07-11

### Added
- 审计日志详情丰富化：REST API 操作（登录、环境 CRUD、资源 CRUD）的 detail_json 字段现在包含结构化上下文信息（IP、User-Agent、资源名称、协议、连接信息等）
- WebSocket 层操作审计：SSH/MySQL/PostgreSQL/SQLite/Redis 连接与断开事件写入审计日志
- 前端审计详情展示优化：操作类型和详情键使用中文标签显示
- 新增审计类型翻译：ssh_connect、ssh_disconnect、mysql_connect、mysql_disconnect、postgresql_connect、postgresql_disconnect、sqlite_connect、sqlite_disconnect、redis_connect、redis_disconnect

### Fixed
- 审计日志操作值显示英文而非中文的问题（AGENTS.md #10）：通过 i18n 翻译表解决
- 审计日志详情无实际价值的问题（AGENTS.md #11）：通过丰富 detail_json 解决
- 删除环境/资源时审计日志不包含名称的问题：在删除前查询记录名称

## [0.83.1] - 2026-07-11

### Fixed
- 分屏拖拽：修复标签拖拽到面板时 splice 索引偏移导致换位错误，新增 drop position 感知（左/右半区）
- 全局快捷键：修正 Ctrl+K/Ctrl+N/Alt+1~5/F1 在 overlay 打开时误触发，contentEditable 元素跳过快捷键，i18n key 语义修正
- SSH 终端复制粘贴：修复 Ctrl+C 选中文本时误发 SIGINT，统一复制粘贴处理逻辑

## [0.83.0] - 2026-07-10

### Changed
- 设计令牌重构：消除 phantom tokens、补齐语义令牌（--bg-panel/--bg-deep/--border/--text-*）与组件令牌层、间距/字阶、浅色主题兼容
- 响应式布局优化：AppLayout 新增平板断点（768px–1024px），独立路由页（Terminal/SqlConsole/Files）100vh→100% 修复移动端溢出
- 交互与反馈增强：模态框错误反馈不再静默、统一 Toast；共享 <Transition name="modal"> 统一组件动画
- 无障碍补齐：所有模态框（ResourceEditModal/EnvironmentEditModal/ConfirmDialog/Agent 三弹窗）增加焦点陷阱、Esc 关闭、ARIA 角色（沿用既有内联 a11y 约定，不引入新抽象）

### Added
- 全局 reduced-motion 守卫已覆盖组件 keyframes 动画


## [0.82.0] - 2026-07-10

### Added
- 文件传输并发数量控制：基于 Semaphore 的并发限制器（默认 3 个并发）
- 传输统计 API：GET /api/transfers/stats 返回并发数、可用槽位等信息
- 传输队列面板：显示当前活跃/最大并发数

### Changed
- SQLite 自动连接优化：添加指数退避自动重连（最多 5 次）
- sqlx 升级：从 0.8.0 升级到 0.8.6，消除 future-incompat 警告
- TypeScript 目标版本：从 ES2020 升级到 ES2024

### Fixed
- SQLite 连接错误处理：移除静默错误吞没，连接失败时显示错误
- sqlx-postgres future-incompat 警告：禁用默认 features 避免 sqlite 冲突
- MySQL connector：移除 sqlx::types::JsonValue 使用，改用 serde_json::from_str

## [0.80.0] - 2026-07-10

### Added
- CSS 变量修复与设计 token 补齐：新增 --bg-panel, --bg-header, --bg-input, --bg-muted, --border-hover 等变量
- Light 主题适配：CodeMirror 和 xterm.js 支持动态主题切换
- 弹窗过渡动画：所有模态框添加 opacity + scale 过渡动画
- useThemeObserver composable：统一主题变化监听逻辑
- 面板拖拽调整大小：支持水平和垂直方向的面板边框拖拽
- 全局键盘快捷键：Ctrl+K 命令面板、Ctrl+N 新建连接、F1 快捷键面板

### Changed
- 样式去重：AgentCard 移除重复 badge/btn 样式，复用全局类
- Dashboard 内联样式提取为 scoped CSS 类
- lightTheme 硬编码颜色值改为 CSS 变量引用
- SQL 格式化器优化（复合关键字处理顺序）

### Fixed
- SSH 终端 Ctrl+C/V 快捷键冲突：有选中文本时复制，无选中时发送 SIGINT
- AppLayout 布局过渡动画：修复 --transition-normal 未定义问题
- GlobalQueryModal 未定义 CSS 变量修复
- useThemeObserver 内存泄漏：生命周期钩子移至顶层作用域
- GlobalQuery API 请求添加 Authorization header
- ESLint 错误修复（BatchOperationDialog switch default case）

## [0.79.0] - 2026-07-10

### Added
- Redis数据类型可视化器：Hash、Set、List、Sorted Set、Stream 五种类型查看器
- 批量操作：支持多键删除、批量设置TTL、导出功能
- 实时监控面板：内存使用、客户端连接、统计、命令统计
- 高级搜索与过滤：键模式搜索、类型过滤、TTL范围过滤
- 键浏览器增强：多选、批量操作工具栏、客户端侧过滤
- 完整单元测试：所有新组件共61个测试用例

### Changed
- 优化Redis控制台：批量TTL时的错误处理
- 更新国际化：新增redis.keys.filter、redis.monitor等国际化键
- 修复TS类型：RedisKeyBrowser中的FilterCriteria类型定义
- 优化布局：搜索过滤器组件布局和响应式行为

## [0.78.0] - 2026-07-10

### Added
- Notebook 功能：交互式笔记本，支持 block-based 富文本编辑器
- 支持 4 种块类型：段落、标题、代码块、命令块
- 命令块支持资源绑定和执行（7 种协议：SSH、SQL、Redis、S3、MCP、FTP、Terminal）
- 内联结果展示与执行历史
- JSON 导入/导出功能
- 自动保存与脏数据检测
- 后端 Notebook CRUD API 和数据模型
- 前端 Notebook 列表页与编辑器页面
- 单元测试：composable 和 API 客户端测试用例
- 代码审查修复：事务包裹、block_type 验证、级联删除、错误处理

## [0.77.0] - 2026-07-09

### Added
- 标签系统：资源标签数据模型（tags + resource_tags 表）和 CRUD API
- 标签管理 UI：TagSelector 组件，集成到资源创建和编辑流程
- 连接菜单标签筛选：工作空间连接菜单支持按标签 AND 逻辑筛选资源
- 标签颜色预设：12 种品牌色系预设颜色
- 单元测试：标签系统后端和前端测试用例

## [0.76.0] - 2026-07-08

### Changed
- 后端可靠性：清理生产代码中约420处 unwrap()，替换为正确的错误处理（?、unwrap_or、expect）
- WebSocket 重构：提取 ws_common.rs 通用模块，消除4个 handler 文件约300行重复代码
- CI 补全：前端测试（bun run test）纳入 CI 流程，修正过时的 stub 注释

## [0.75.0] - 2026-07-08

### Changed
- 代码清理：应用 clippy 建议，使用 derive(Default) 简化手写 Default 实现
- 提取重复代码：创建通用辅助函数和 closure 减少重复模式
- 函数优化：短路条件、简化 if-let 链、消除未使用变量

### Added
- 测试覆盖：补充后端模块测试（ws_agent、executor、auth）
- 边缘情况与错误处理测试

## [0.74.0] - 2026-07-07

### Added
- 补充前端组件和 composable 的单元测试
- ConfirmDialog、ToastProvider、GlobalQueryModal 等组件测试
- useWorkspacePersistence、useSidebar、useGlobalQuery 等 composable 测试

### Changed
- 代码精简：提取共享的 useWorkspacePersistence composable
- 清理未使用的导入和重复的模式
- 统一错误处理模式

## [0.73.0] - 2026-07-06

### Added
- 工作空间体验增强：多标签支持、分屏布局
- 面板集成：SQL、Redis、文件等面板统一管理
- 标签未保存标记和快捷导航

### Fixed
- 性能优化：按需加载、虚拟滚动
- 内存泄漏修复

## [0.72.0] - 2026-07-05

### Added
- 路由懒加载与代码分割
- 可访问性改进：ARIA 标签、键盘导航、屏幕阅读器支持
- 构建分析、树摇优化、依赖清理

### Changed
- 性能优化：减少首次加载体积约 40%

## [0.71.0] - 2026-07-04

### Added
- i18n 基础设施：创建独立 i18n 实例供非 Vue 模块使用
- API 错误消息国际化
- GlobalQueryModal、WorkspaceSql、ProfileSection、TabBar 国际化

### Changed
- 修复硬编码中文字符串

## [0.70.0] - 2026-07-03

### Added
- 后端安全加固：登录限流、CSP/HSTS/X-Frame-Options 安全头、健康检查端点
- 前端错误处理：Vue 全局错误边界、API 错误统一处理、WebSocket 断线重连
- 离线检测与状态提示：网络状态监听、离线/在线 Toast 提示、连接状态指示器

## [0.66.0] - 2026-07-02

### Added
- S3/MinIO 控制台移动端适配
- SQLite 控制台移动端适配
- 工作空间面板通用移动端样式

## [0.67.0] - 2026-07-01

### Added
- 加载状态骨架屏
- 空态引导优化
- 操作反馈增强

### Changed
- 过渡动画优化

## [0.68.0] - 2026-06-30

### Added
- 通用组件测试（SkeletonLoader、EmptyState、ErrorState、LoadingSpinner）
- Composable 单元测试（useToast、useContextMenu、useProtocol）
- 页面组件测试（Dashboard、Environments、Agents）

## [0.69.0] - 2026-06-29

### Added
- 通用 crate 测试（rex-common：类型、错误处理、配置解析）
- 协议连接器测试（rex-redis、rex-sqlite、rex-mysql、rex-postgresql）
- 传输引擎测试（rex-transfer：任务管理、执行器）

## [0.64.0] - 2026-06-26

### Added
- 工作空间触控优化：连接菜单、标签栏、快捷键面板
- 移动端触摸目标尺寸优化

## [0.63.0] - 2026-06-25

### Added
- 设置页移动端适配：布局、设置区块、更新区块

## [0.62.0] - 2026-06-24

### Added
- 审计日志移动端优化：筛选栏、表格、统计卡片

## [0.61.0] - 2026-06-23

### Added
- Agent 管理移动端优化：卡片布局、日志查看器、配置弹窗

## [0.60.0] - 2026-06-22

### Changed
- 提取 MobileToolbar 共享组件
- Lint 警告清理
- 补充缺失的单元测试

## [0.59.0] - 2026-06-21

### Added
- SQL 控制台移动端浮动工具栏：执行、格式化、保存等移动端操作入口
- 工具栏集成与 i18n
- 单元测试补充

## [0.58.0] - 2026-06-20

### Added
- SFTP 文件管理移动端浮动工具栏：上传、新建、刷新等移动端操作入口
- 工具栏集成与 i18n
- 单元测试补充

## [0.57.0] - 2026-06-19

### Added
- SSH 终端移动端浮动工具栏：方向键、功能键和字体缩放等移动端交互控件
- 工具栏集成与 i18n
- 单元测试补充

## [0.56.0] - 2026-06-18

### Added
- SQL 标签未保存标记与副标题
- 结果表格斑马纹与行选中高亮
- SQL 历史记录面板增强
- SQL 功能单元测试补充

## [0.55.0] - 2026-06-17

### Changed
- i18n 重复 key 清理（消除 ws.workspace.* 冗余）
- 系统主题自动切换修复
- Dashboard "在新标签中打开" bug 修复
- settings.ts 重构为 Pinia store

## [0.54.0] - 2026-06-16

### Added
- 快速连接改为最近使用的资源
- 仪表盘自动刷新
- 环境卡片右键菜单"在工作区打开所有资源"
- 统计卡片手动刷新

## [0.47.0] - 2026-06-10

### Added
- 后端 Redis 连接器：真实 TCP 连接，支持 AUTH、SELECT、INFO、SCAN
- 键浏览器：树形键列表，支持 SCAN 模式搜索
- 值查看器：按类型显示（String/Hash/List/Set/ZSet）
- 数据库选择器 + 命令自动补全

## [0.48.0] - 2026-06-09

### Added
- 键浏览器增强：树形结构、类型图标、右键菜单

## [0.49.0] - 2026-06-08

### Added
- 新建键对话框：选择类型、输入键名和值
- 值编辑器组件：按类型提供内联编辑界面
- 值编辑器集成到 RedisConsole

## [0.45.0] - 2026-06-05

### Changed
- SQLite 连接器现代化：重构为 SqlConnector trait
- Hub API 集成
- WebSocket 升级
- 前端适配：SqlSidebar 智能过滤

## [0.44.0] - 2026-06-04

### Added
- SQL 存储过程/函数节点支持

## [0.43.0] - 2026-06-03

### Added
- MySQL 控制器支持
- PostgreSQL 控制器支持

## [0.42.0] - 2026-06-02

### Added
- 文件传输引擎
- MCP 协议支持

## [0.41.0] - 2026-06-01

### Added
- SFTP 文件管理增强
- 文件上传统一入口

## [0.40.0] - 2026-05-30

### Added
- 用户信息页面
- 备份与恢复功能

## [0.39.0] - 2026-05-28

### Added
- 系统设置页面
- TLS 配置管理

## [0.38.0] - 2026-05-25

### Added
- 更新检测机制
- 自动更新功能

## [0.37.0] - 2026-05-22

### Added
- Agent 日志查看器
- Agent 配置编辑

## [0.36.0] - 2026-05-20

### Added
- 仪表盘统计数据
- 操作审计日志页面

## [0.35.0] - 2026-05-18

### Added
- 标签系统基础实现
- 工作空间多标签支持

---

Versions before 0.35.0 are not individually tracked in this changelog. Refer to git history for earlier changes.
