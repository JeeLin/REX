# M68: Performance Optimization

## Context
M67 完成安全加固。本里程碑聚焦前端性能优化，提升页面加载速度和运行时响应性。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段做什么：
- 路由懒加载（Vue 异步组件）
- 虚拟滚动（长列表优化）
- 静态资源压缩与缓存策略
- 前端构建优化

本阶段不做什么：
- 不修改后端 API
- 不改变核心功能逻辑
- 不引入新的外部依赖（除非必要）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 路由懒加载 | ✅ |
| 2 | 长列表虚拟滚动 | ✅ |
| 3 | 静态资源缓存策略 | ✅ |
| 4 | 前端构建优化 | ✅ |

## 子任务详细设计

### 1 路由懒加载

- **功能目标**：所有页面路由使用 Vue 异步组件，减少初始 bundle 大小
- **文件结构**（修改）：
  - `packages/rex-console-web/src/router/index.ts` — 路由组件改为动态 import
- **实现方式**：
  - 使用 `() => import('../pages/XxxPage.vue')` 替代静态 import
  - 添加 webpackChunkName 注释分组
- **测试标准**：
  - 构建后检查 chunk 分割是否正确
  - 页面切换无闪烁
- **提交信息**：`perf: add lazy loading for all page routes`

### 2 长列表虚拟滚动

- **功能目标**：审计日志、Agent 列表等长列表使用虚拟滚动
- **文件结构**（修改）：
  - `packages/rex-console-web/src/components/` — 新增 VirtualList 组件
  - `packages/rex-console-web/src/pages/AuditLogPage.vue` — 使用 VirtualList
- **实现方式**：
  - 简单虚拟滚动：仅渲染可视区域 + buffer 行
  - 支持固定行高
- **测试标准**：
  - 10000 条数据滚动流畅
  - 内存占用稳定
- **提交信息**：`perf: add virtual list component for long lists`

### 3 静态资源缓存策略

- **功能目标**：配置正确的 HTTP 缓存头，减少重复下载
- **文件结构**（修改）：
  - `crates/rex-hub/src/middleware.rs` — 添加 Cache-Control 头
- **实现方式**：
  - 静态资源（js/css）：`Cache-Control: public, max-age=31536000, immutable`
  - API 响应：`Cache-Control: no-store`
- **提交信息**：`perf: add Cache-Control headers for static assets`

### 4 前端构建优化

- **功能目标**：优化 Vite 构建配置，减小 bundle 大小
- **文件结构**（修改）：
  - `packages/rex-console-web/vite.config.ts` — 优化构建配置
- **优化方向**：
  - 分包策略（vendor/ui/logic 分离）
  - 移除 console.log（生产环境）
  - 启用 brotli 压缩（可选）
- **测试标准**：
  - 构建产物大小 < 500KB（gzipped）
  - 构建时间无明显增加
- **提交信息**：`perf: optimize Vite build config`

## 设计核对点

- 不改变用户可见功能
- 桌面端和移动端性能均有提升
- 构建产物体积可量化验证

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
