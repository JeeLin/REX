# M0 设计核对报告

## 核对范围
M0 里程碑文档（`docs/milestones/M0-project-skeleton.md`）+ 已实现骨架 vs 产品文档（`docs/PRODUCT.md`）+ 开发文档（`docs/DEVELOPMENT.md`）

## 结论：✅ 通过（小问题已就地修正）

## 逐项核对

### 1. 产品定位与边界
- ✅ 单用户、自托管 — 未引入多用户/RBAC/企业协作
- ✅ M0 只做骨架，未提前实现业务功能（终端/SQL/Redis/传输后端逻辑均为占位）

### 2. 视觉语言（PRODUCT.md §0 / §6）
- ✅ 深色优先：tokens.css 定义 `--bg-deep #0D1117 / --bg-page #161B22 / --bg-surface #1C2128`，GitHub 暗色系
- ✅ 品牌色橙 `#E8912D`（`--accent`），与产品文档规范一致
- ✅ 协议色齐全：SSH 绿 / MySQL 蓝 / PostgreSQL+SFTP 紫 / Redis 红 / SQLite 橙 / S3 橙
- ✅ 字体：JetBrains Mono（代码/标题）+ Inter（正文），与规范一致
- ✅ 高信息密度、6px 细滚动条（global.css）
- ⚠️ 小问题：原 tokens.css 缺 `--bg-elevated` 在组件中的统一约定，已通过组件库统一使用 `var(--bg-elevated)` 修正

### 3. 架构一致性（PRODUCT.md §8 / DEVELOPMENT §2）
- ✅ Rust workspace 10 crate，supervisor+worker 入口符合进程模型
- ✅ Hub 整合前端静态资源（单端口代理），符合"文件传输不经过浏览器"原则
- ✅ 前端按功能域组织（features/ components/ layouts/ pages/ stores/ api/）

### 4. 对标复刻边界
- ✅ M0 不涉及具体对标模块（Xshell/Navicat/ARDM/Xftp），仅搭建设计基础，符合 M0 范围
- ✅ 设计系统预览页作为后续模块的可视化基线，正确

### 5. 子任务拆分粒度
- ✅ 5 个子任务，每个对应 1-2 个 commit，粒度合理

## 修正记录
- 组件库统一引用 `--bg-elevated` 变量（原部分组件用 `--bg-surface` 不一致），已对齐
- 版本号归零至 0.1.0（与 CHANGELOG Unreleased 一致）
- DEVELOPMENT.md M0 版本号从 1.0.0 → 0.1.0（与里程碑文档一致）
- DEVELOPMENT.md M7 移除硬编码版本号（由里程碑文档确定）
