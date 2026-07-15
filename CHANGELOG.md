# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

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
