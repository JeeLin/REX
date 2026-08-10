# M78: 全系统 UX/UI 重设计

## Context
M77 完成 Bug Fix。本里程碑对 REX 全系统进行 UX/UI 重设计，使用专业设计工具（ui-ux-pro-max）提升界面质量和一致性。覆盖所有页面和核心组件。

版本类型：minor
版本号：0.66.0

## 产品边界
本阶段做什么：全系统 UX/UI 重设计（所有页面、组件、交互模式）
本阶段不做什么：不新增后端功能、不修改数据模型、不新增页面

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 1 | 设计系统建立（CSS 变量、组件 token、深色主题） | 前端 | ✅ |
| 2 | 通用组件优化 | 前端 | ✅ |
| 3 | 登录页重设计 | 前端 | ✅ |
| 4 | 仪表盘重设计 | 前端 | ✅ |
| 5 | 环境管理页重设计（列表+详情） | 前端 | ✅ |
| 6 | Agent 管理页重设计 | 前端 | ✅ |
| 7 | 审计日志页重设计 | 前端 | ✅ |
| 8 | 设置页重设计 | 前端 | ✅ |
| 9 | 侧栏+导航重设计 | 前端 | ✅ |
| 10 | 工作空间（Tab+分屏+状态栏）重设计 | 前端 | ✅ |

## 子任务详细设计

### 1 设计系统建立

- **功能目标**：使用 ui-ux-pro-max 生成设计系统，统一全站视觉语言
- **文件结构**：`packages/rex-console-web/src/styles/`
- **设计方向**：
  - 基于 Dark Mode (OLED) 风格，深色优先
  - JetBrains Mono 等宽字体体系
  - 语义化颜色 token（primary/success/danger/warning/info）
  - 间距/圆角/阴影/动画 token
  - 组件级 token（按钮/卡片/输入框/徽章）
- **提交信息**：`feat: establish design system with dark mode tokens`

### 2 登录页重设计

- **功能目标**：重新设计登录页，提升品牌感和专业度
- **文件结构**：`packages/rex-console-web/src/pages/LoginPage.vue`
- **设计方向**：
  - 全屏深色背景 + CRT 扫描线效果（极客风）
  - 居中登录卡片，毛玻璃效果
  - 表单交互优化（焦点状态、错误提示）
  - 底部版本号 + 环境信息
- **提交信息**：`feat: redesign login page with CRT aesthetic`

### 3 仪表盘重设计

- **功能目标**：重新设计仪表盘，提升信息密度和可读性
- **文件结构**：`packages/rex-console-web/src/pages/DashboardPage.vue`
- **设计方向**：
  - 统计卡片网格（环境数/资源数/Agent 在线/今日操作）
  - 快速连接区域（最近使用资源）
  - 环境卡片网格（状态指示、资源分布）
  - 响应式布局（桌面侧栏+卡片网格，移动端单列）
- **提交信息**：`feat: redesign dashboard with metrics cards`

### 4 环境管理页重设计

- **功能目标**：重新设计环境列表和详情页
- **文件结构**：`packages/rex-console-web/src/pages/EnvironmentsPage.vue`、`EnvironmentDetailPage.vue`
- **设计方向**：
  - 环境卡片（名称、Agent 状态、描述、资源类型分布）
  - 详情页（Agent 面板 + 资源表格）
  - 创建/编辑向导优化
  - 右键菜单样式统一
- **提交信息**：`feat: redesign environment pages`

### 5 Agent 管理页重设计

- **功能目标**：重新设计 Agent 管理页面
- **文件结构**：`packages/rex-console-web/src/pages/AgentsPage.vue`
- **设计方向**：
  - 卡片式 Agent 展示（状态、环境、设备信息、元信息）
  - 部署指南仅在无 Agent 时显示
  - 配置文件下载合并到部署指南
  - Agent 详情面板（配置、日志、令牌管理）
  - 移除无用的独立配置文件按钮
- **提交信息**：`feat: redesign agent management page`

### 6 审计日志页重设计

- **功能目标**：重新设计审计日志页面
- **文件结构**：`packages/rex-console-web/src/pages/AuditLogPage.vue`
- **设计方向**：
  - 统计卡片（总数/成功/失败/活跃用户）
  - 表格（时间/用户/环境/操作彩色标签/摘要/结果）
  - 筛选器优化（时间范围/用户/环境/操作类型）
  - 行展开详情 + 命令/SQL 代码块
- **提交信息**：`feat: redesign audit log page`

### 7 设置页重设计

- **功能目标**：重新设计设置页面
- **文件结构**：`packages/rex-console-web/src/pages/SettingsPage.vue`
- **设计方向**：
  - 分区块布局（个人信息/外观/终端/安全/更新）
  - 表单控件优化（开关/选择器/输入框）
  - 保存按钮 sticky 定位
  - 版本信息展示优化
- **提交信息**：`feat: redesign settings page`

### 8 侧栏+导航重设计

- **功能目标**：重新设计侧栏导航和连接树
- **文件结构**：`packages/rex-console-web/src/layouts/AppLayout.vue`、`packages/rex-console-web/src/features/resource-panel/ResourcePanel.vue`
- **设计方向**：
  - 侧栏布局（Logo/搜索/导航/连接树/设置）
  - 连接树（环境→资源分组、可折叠、搜索、颜色标签）
  - 收藏/最近使用 tab
  - 全局搜索优化
  - 右键菜单样式统一
  - 移动端底部导航
- **提交信息**：`feat: redesign sidebar and navigation`

### 9 工作空间重设计

- **功能目标**：重新设计工作空间 Tab、分屏、状态栏
- **文件结构**：`packages/rex-console-web/src/pages/WorkspacePage.vue`
- **设计方向**：
  - Tab 栏（单排滚动、状态点、关闭按钮、设色）
  - 分屏布局（拖拽缩放、布局预设）
  - 状态栏（协议+主机+端口、连接状态、终端尺寸）
  - 快捷键面板样式统一
  - Quick Connect 栏优化
- **提交信息**：`feat: redesign workspace shell`

### 10 通用组件优化

- **功能目标**：优化所有通用 UI 组件
- **文件结构**：`packages/rex-console-web/src/components/ui/`
- **设计方向**：
  - Button（主/次/危险/幽灵，sm/md/lg 尺寸）
  - Card（圆角、边框、hover 效果）
  - Modal/Drawer（毛玻璃遮罩、动画）
  - Table（紧凑行高、斑马纹、排序）
  - Toast（右上角滑入、成功/错误区分）
  - Badge/StatusDot（状态颜色、动画）
  - Form controls（输入框/选择器/开关/复选框）
- **提交信息**：`feat: optimize shared UI components`

## 设计核对点

- 所有设计变更符合 PRODUCT.md 设计基调（现代化/极客化/易用化）
- 对标 Xshell/Navicat/ARDM/Xftp 的交互模式
- 深色优先、高信息密度、键盘驱动
- 不引入多用户/RBAC 概念
- 所有组件 cursor:pointer、hover 过渡 150-300ms
- 对比度 ≥ 4.5:1（WCAG AA）
- 响应式：375px / 768px / 1024px / 1440px

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

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
| [x] | 🔴 | 登录页密码输入未打码 | 用户反馈 | Input 组件 type 写死 text，password 不生效（已修复 3d48a684） |
| [x] | 🟡 | 不存在的页面不跳回首页 | 用户反馈 | 添加 catch-all 路由 redirect 到 /workspace |
| [x] | 🟡 | 登录页版本号只显示 v | 用户反馈 | 构建已确认版本号正确嵌入，浏览器缓存问题 |
| [x] | 🟢 | 连接日志加上资源名称 | 用户反馈 | 后端审计日志 target 字段问题，不在前端重设计范围内 |
| [x] | 🔴 | 新增资源切换模式触发保存 | 用户反馈 | Select 点击事件冒泡到 form 触发 submit（已修复 @click.stop） |
| [x] | 🔴 | 资源详情页路由错误 | 用户反馈 | 从资源页点击任意资源，进的都是同一个资源详情页（KeepAlive 缓存，已修复 caec2546） |
| [x] | 🔴 | 左侧 Agent 导航点击无响应 | 用户反馈 | 浏览器缓存问题，强制刷新后恢复正常（已加 router.push 兜底） |
| [x] | 🔴 | 左侧工作区导航点击无响应 | 用户反馈 | 浏览器缓存问题，强制刷新后恢复正常（已加 router.push 兜底） |
| [x] | 🔴 | 仪表盘环境点击路由错误 | 用户反馈 | 同资源详情页路由错误，KeepAlive 缓存（已修复 caec2546） |
| [x] | 🔴 | 分屏关闭按钮关错 pane | 用户反馈 | 左右分屏时点击左侧关闭按钮关的是右侧（已修复 63581feb） |
