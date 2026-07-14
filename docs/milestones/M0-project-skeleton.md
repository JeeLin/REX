# 0.1.0: M0 项目骨架重建

## Context
2.0 重设计起点，git 历史已清理为单一基线提交（main，版本归零到 0.1.0）。本阶段在空仓库上重建最小可运行骨架：Rust workspace（10 crate）+ Vue 3 前端（按功能域组织），并完成设计系统预览页作为后续模块的可视化基线。

前序：无（从零开始）。
后续：M1 设计系统组件库深化、M2 工作空间外壳。

版本类型：minor（首个可运行骨架）

## 产品边界
- **做**：Rust workspace 骨架（Cargo.toml + 10 crate 空壳 + supervisor/worker 入口）、前端骨架（entry/router/i18n/tokens）、基础组件库（11 个 UI 组件）、设计系统预览页（/design-preview）、导航框架（AppLayout + stub 页）。
- **不做**：任何业务功能（终端/SQL/Redis/文件传输后端逻辑）、真实登录鉴权、持久化。这些留给 M2–M7。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Rust workspace 骨架（Cargo.toml + 10 crate + supervisor/worker 入口） | ✅ |
| 2 | 前端骨架（Vue3 + Vite + TS + Pinia + Router + i18n + tokens） | ✅ |
| 3 | 基础组件库（Button/Card/Badge/StatusDot/Tabs/Table/Drawer/Modal/ContextMenu/Tooltip/Toast） | ✅ |
| 4 | 导航框架（AppLayout 侧栏 + 路由 + 登录/仪表盘等 stub 页） | ✅ |
| 5 | 设计系统预览页（token + 组件可视化 + 暗/亮切换） | ✅ |

## 子任务详细设计

### 1 Rust workspace 骨架
- **文件结构**：`Cargo.toml`（workspace + dependencies）、`crates/*/Cargo.toml`、`crates/rex-common/src/lib.rs`（RExError）、`crates/rex-hub/src/bin/rex-hub.rs`、`crates/rex-agent/src/bin/rex-agent.rs`（supervisor+worker 占位）
- **接口设计**：`rex-common::RExError`、`Result<T>`；bin 入口启动 tokio runtime + tracing
- **测试标准**：`cargo check --workspace` 通过
- **提交**：`build: add 2.0 project init` 等

### 2 前端骨架
- **文件结构**：`src/main.ts`、`src/App.vue`、`src/router/index.ts`、`src/i18n/`、`src/styles/tokens.css`、`src/styles/global.css`
- **接口设计**：router 含 /login /dashboard /environments /agents /audit-log /settings /workspace /design-preview；i18n zh/en
- **设计原则**：深色优先、橙色主色、JetBrains Mono + Inter
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): M0 frontend scaffold`

### 3 基础组件库
- **文件结构**：`src/components/ui/*.vue`（11 个）
- **接口设计**：props 已定义（variant/size/tone/status 等），scoped style 引用 tokens
- **测试标准**：`bun run type-check` + `bun run build` 通过
- **提交**：`feat(web): M1 base/overlay UI components`

### 4 导航框架
- **文件结构**：`src/layouts/AppLayout.vue`、`src/pages/*.vue`（6 stub）
- **交互设计**：侧栏 256px + 顶栏 + 内容区；导航高亮
- **测试标准**：build 通过
- **提交**：`feat(web): M0 AppLayout sidebar + stub pages`

### 5 设计系统预览页
- **文件结构**：`src/features/design-preview/DesignPreview.vue`（路由 /design-preview）
- **交互设计**：可视化 token（配色/间距/字体）+ 全部组件 + 暗/亮切换按钮
- **测试标准**：build 通过，dev server 可访问
- **提交**：`feat(web): M1 design system preview page`

## 设计核对点
- [ ] 视觉语言 REX 自有（现代化/极客化/易用化），非桌面软件外观复刻
- [ ] 深色优先，GitHub 暗色系基调
- [ ] 品牌色橙 #E8912D，协议色与产品文档一致
- [ ] 字体 JetBrains Mono（代码/标题）+ Inter（正文）
- [ ] 高信息密度、自定义 6px 细滚动条
- [ ] 组件范式统一（卡片/表格/按钮/弹窗/Toast/抽屉/右键菜单）

## Flow Status
- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
