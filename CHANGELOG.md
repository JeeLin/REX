# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

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
