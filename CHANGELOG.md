# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.75.0] - 2026-07-08

### Changed
- 代码清理：应用 clippy 建议，使用 derive(Default) 简化手写 Default 实现
- 代码清理：移除冗余的类型转换和借用
- 代码格式化：统一 cargo fmt 格式

## [0.74.0] - 2026-07-07

### Added
- 单元测试：ConfirmDialog 组件测试（7 个用例）
- 单元测试：useWorkspacePersistence 测试（5 个用例）
- 单元测试：useSidebar 测试（7 个用例）
- 单元测试：useGlobalQuery 测试（8 个用例）
- 单元测试：useNetworkStatus 测试（4 个用例）
- 单元测试：useSort 测试（9 个用例）

## [0.73.0] - 2026-07-07

### Added
- 工作空间布局持久化：记住用户的分屏布局偏好，刷新页面后恢复
- 标签操作增强：新标签在当前活动面板打开
- 快捷键系统完善：Ctrl+D 复制当前标签
- 工作空间状态恢复：刷新页面后恢复已打开的标签和布局（24 小时过期）

## [0.72.0] - 2026-07-07

### Added
- Vite 构建优化：手动 chunk 分割（CodeMirror/xterm/vendor 独立 chunk）、构建目标升级 es2020、rollup-plugin-visualizer 产物分析
- 可访问性补全：侧边栏 aria-expanded/aria-label、移动端焦点陷阱、prefers-reduced-motion、Toast 角色分级、Dashboard aria-live
- TypeScript 配置加固：启用 noUncheckedIndexedAccess、forceConsistentCasingInFileNames

### Fixed
- SqlResults.test.ts：修复 vue-i18n mock 缺少 createI18n 导出导致的测试失败
- i18n/zh.ts、i18n/en.ts：__APP_VERSION__ 添加 typeof 回退值，兼容测试环境

## [0.71.0] - 2026-07-07

### Added
- i18n 基础设施：创建可独立导入的 i18n 实例（t() 函数），供非 Vue 模块使用
- API 错误消息国际化：client.ts 4 处硬编码中文替换为 i18n 翻译（429/5xx/超时/网络错误）

### Fixed
- GlobalQueryModal.vue：修复 12+ 处硬编码中文 UI 文本
- WorkspaceSql.vue：修复 8 处硬编码中文 UI 文本（执行、格式化、保存、清空、快捷键提示等）
- ProfileSection.vue：修复确认密码标签硬编码中文
- TabBar.vue：修复新建连接 tooltip 硬编码中文

## [0.70.0] - 2026-07-07

### Added
- 后端安全加固：登录速率限制（同IP 5分钟内最多5次失败，超出返回429）、HTTP安全响应头（CSP、HSTS、X-Frame-Options、X-Content-Type-Options、Cache-Control）
- 前端错误处理：Vue全局ErrorBoundary组件捕获渲染错误、API错误统一拦截器（401/429/5xx/超时/网络错误Toast提示）
- 网络状态监听：useNetworkStatus composable（离线/在线Toast提示）、WebSocket自动重连（指数退避1s→16s，最多5次）

## [0.69.0] - 2026-07-07

### Added
- rex-common 测试：ViewInfo/ProcedureInfo 序列化、CLI TLS flags、版本号边缘情况、UpdatePhase 序列化
- 协议连接器测试：配置序列化往返（SQLite/MySQL/PostgreSQL/Redis）、表名验证、RESP 编解码边界
- 传输引擎测试：TransferManager 边缘情况、LocalConnector 边界测试

## [0.68.0] - 2026-07-07

### Added
- 通用组件测试：SkeletonLoader、EmptyState、ErrorState、LoadingSpinner 单元测试
- useToast composable 单元测试
- 页面组件测试：Dashboard、Environments、Agents 单元测试

## [0.67.0] - 2026-07-07

### Added
- 统一骨架屏加载状态：Dashboard、环境列表、Agent 列表使用 SkeletonLoader 组件
- 空态引导优化：侧边栏收藏/最近使用、仪表盘快速连接显示引导文案和图标
- Toast 通知：自动关闭动画和手动关闭按钮

## [0.66.0] - 2026-07-06

### Added
- S3/MinIO 控制台：移动端桶列表单列、工具栏紧凑排列、路径可水平滚动
- SQLite 控制台：移动端工具栏换行、侧边栏全宽、结果表格水平滚动
- 工作空间面板：通用移动端工具栏紧凑、padding 调整

## [0.65.0] - 2026-07-06

### Added
- 仪表盘：移动端统计卡片 2 列、环境卡片单列、快速连接紧凑显示
- 环境管理：移动端卡片单列、资源表格水平滚动
- 资源创建向导：移动端协议选择 2 列、表单全宽

## [0.64.0] - 2026-07-06

### Added
- 工作空间：连接菜单触摸目标增大、标签栏触摸区域优化、快捷键面板移动端适配

## [0.63.0] - 2026-07-06

### Added
- 设置页：移动端全宽布局、设置区块减小内边距、设置行垂直排列

## [0.62.0] - 2026-07-06

### Added
- 审计日志：移动端筛选栏垂直排列、表格水平滚动、统计卡片 2 列布局

## [0.61.0] - 2026-07-06

### Added
- Agent 管理：移动端卡片单列布局和操作按钮紧凑排列
- Agent 日志查看器：移动端全屏显示、筛选栏换行、日志行水平滚动
- Agent 配置弹窗：移动端全屏显示、表单单列布局

## [0.60.0] - 2026-07-06

### Changed
- 提取 MobileToolbar 共享组件，消除三个移动端浮动工具栏的重复 CSS

### Fixed
- 清理前端 Lint 警告（39 → 0）

### Added
- MobileToolbar 共享组件单元测试

## [0.59.0] - 2026-07-06

### Added
- SQL 控制台移动端浮动工具栏：执行、格式化、清空操作按钮
- SQL 控制台移动端浮动工具栏：保存、历史、全局查询、更多操作
- SQL 控制台移动端浮动工具栏单元测试

## [0.58.0] - 2026-07-06

### Added
- SFTP 移动端浮动工具栏：上传、新建文件、新建文件夹、刷新操作按钮
- SFTP 移动端浮动工具栏：下载、删除、全选更多操作
- SFTP 移动端浮动工具栏单元测试

## [0.57.0] - 2026-07-06

### Added
- 终端移动端浮动工具栏：方向键、Tab、Enter、^C、^L 按钮
- 终端移动端浮动工具栏：历史记录、粘贴、字体缩放（A-/A+）、更多选项菜单
- 终端移动端浮动工具栏单元测试

### Fixed
- 终端移动端工具栏：修复 i18n 键名错误（`terminal.mobile.*` → `ws.terminal.mobile.*`）
- 终端移动端工具栏：添加 `toolbar-action` 事件监听，修复更多菜单功能无响应问题

## [0.56.0] - 2026-07-06

### Added
- SQL 标签：未保存标签显示蓝色圆点标记，已保存标签显示图标
- SQL 标签副标题：显示 SQL 首行内容（截断到 30 字符）
- SQL 结果表格：奇偶行斑马纹交替背景色
- SQL 结果表格：点击行高亮选中，再次点击取消
- SQL 历史记录面板：按时间分组（今天/昨天/更早）
- SQL 历史记录面板：右键菜单支持复制 SQL、新标签打开、删除
- SQL 功能单元测试：SqlTabs、useSqlTabActions、SqlResults 斑马纹测试

### Fixed
- SQL 结果表格：结果变化时重置选中行和排序状态

### Changed
- SQL 结果表格：合并重复 CSS 样式（pagination-controls → results-footer）
- SQL 标签：优化 subtitle computed 中重复的 trim() 调用

## [0.55.0] - 2026-07-06

### Fixed
- Dashboard "在新标签中打开"现在真正创建新标签（而非复用已有标签）
- 系统主题模式下切换 OS 偏好时自动切换深色/浅色
- SqlSidebar 新建表标签名国际化（修复硬编码中文）

### Changed
- i18n 清理：消除 `ws.workspace.*` 与 `ws.*` 的重复 key（减少约 94 行冗余翻译）
- settings store 重构为 Pinia `defineStore`，与其他 store 架构一致

## [0.54.0] - 2026-07-06

### Changed
- 快速连接改为显示最近使用的资源（而非全部），卡片补充地址和环境名
- 仪表盘自动刷新：每 60 秒定时更新统计数据
- 环境卡片右键菜单增加"在工作区打开所有资源"
- 统计卡片手动刷新改为静默更新（不再整页重载）

## [0.53.0] - 2026-07-06

### Changed
- 工作空间面板组件懒加载：6 个面板组件使用 `defineAsyncComponent` 优化代码分割
- TabBar 可访问性：添加 `role="tablist"`/`role="tab"`、`aria-selected`、键盘导航支持
- AppLayout 可访问性：添加 skip-to-content 链接、底部导航 `aria-current`、`aria-label`
- 测试兼容性修复：修复 Vue 3.5 + @vue/test-utils 的 WeakMap 兼容性问题

## [0.52.0] - 2026-07-05

### Added
- 移动端体验优化：工作空间标签栏横向滚动、面板全屏显示
- 触摸手势：触觉反馈支持（长按和双击时振动）
- 页面过渡动画：移动端 slide 过渡，桌面端 fade 过渡
- 返回手势：主内容区从屏幕边缘右滑可返回上一页
- 底部导航激活动画：图标弹跳效果

## [0.51.0] - 2026-07-04

### Added
- SQL 工作空间面板增强：保存查询、格式化功能
- SQL 编辑器快捷键：Ctrl+Shift+F 格式化
- 工作空间移动端响应式布局优化
