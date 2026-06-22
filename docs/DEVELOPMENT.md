# REX Hub — 开发文档

本文档是开发索引。产品功能和设计决策请参阅 [PRODUCT.md](PRODUCT.md)。架构决策、数据模型、API 设计等详细参考信息见各子文档。

---

## 1. 技术栈

| 层 | 技术 |
|---|---|
| 后端 | Rust（tokio async） |
| 前端 | Vue 3 + Vite + TypeScript |
| 终端 | xterm.js |
| 通信 | WebSocket + HTTPS |
| 加密 | TLS 1.3, AES-256-GCM, ECDHE-X25519 |
| 数据库 | SQLite（Hub 本地） |

---

## 2. Rust crate 结构

```text
crates/
├── rex-common        通用类型、错误定义、CLI、supervisor、版本、更新状态
├── rex-ssh           SSH/SFTP 协议实现
├── rex-mysql         MySQL 协议实现
├── rex-postgresql    PostgreSQL 协议实现
├── rex-redis         Redis 协议实现
├── rex-docker        Docker 协议实现
├── rex-sqlite        SQLite 协议实现
├── rex-s3            S3/MinIO 协议实现
├── rex-transfer      文件传输引擎（FileConnector trait）
├── rex-tunnel        WebSocket 隧道
├── rex-supervisor    进程 supervisor（启动、监控、状态判断、替换、回滚）
├── rex-hub           Hub 二进制入口（整合所有 crate + 前端静态资源）
└── rex-agent         Agent 二进制入口（整合所有 crate）
```

Hub 和 Agent 共享所有协议 crate，区别在于：

```text
rex-hub   = 所有 crate + 前端静态资源（embedded）
rex-agent = 所有 crate（无前端）
```

---

## 3. 里程碑总览

| 里程碑 | 标题 | 状态 |
|--------|------|------|
| M0 | 项目骨架 | ✅ 完成 |
| M1 | Hub 基础服务 | ✅ 完成 |
| M2 | Agent 连接 | ✅ 完成 |
| M3a | SSH 后端 | ✅ 完成 |
| M3b | SSH 终端前端 | ✅ 完成 |
| M4a | 文件传输后端 | ✅ 完成 |
| M4b | 文件管理前端 | ✅ 完成 |
| M5a | SQL 后端 | ✅ 完成 |
| M5b | SQL 前端 | ✅ 完成 |
| M6 | 更新检测 | ✅ 完成 |
| M7 | 自动更新 | ✅ 完成 |
| M8 | 前端精化 | ✅ 完成 |
| M9 | 打包发布 | ✅ 完成 |
| M10 | 审计日志页面 + 设置页 | ✅ 完成 |
| M11 | 右键上下文菜单系统 | ✅ 完成 |
| M12 | Agent 管理增强 | ✅ 完成 |
| M13 | 工作空间多标签 + 分屏 | ✅ 完成 |
| M14 | 工作空间面板集成 | ✅ 完成 |
| M15 | Agent 部署指南 | ✅ 完成 |
| M16 | 资源创建向导 + 测试连接 | ✅ 完成 |
| M17 | 工作区标签右键菜单 | ✅ 完成 |
| M18 | 终端内置 SFTP + SQL 右键菜单 | ✅ 完成 |
| M19 | SQL 库表结构树 + 查询标签菜单 | ✅ 完成 |
| M20 | 工作空间面板拖拽 | ✅ 完成 |
| M21 | Terminal 移动端 + 状态栏 | ✅ 完成 |
| M22 | 侧边栏收藏 + 最近使用 | ✅ 完成 |
| M23 | 文件上传下载 + 跨连接传输 | ✅ 完成 |
| M24 | 开发文档重构 | ✅ 完成 |
| 0.2.0 | 终端 SFTP 面板 + 编辑器补全 | ✅ 完成 |
| 0.3.0 | UI 打磨与 i18n 补全 | ✅ 完成 |

---

## 4. 架构文档

详细架构决策和设计原理：

| 文档 | 内容 |
|------|------|
| [进程模型](architecture/process-model.md) | supervisor + worker、退出码语义、Windows 差异 |
| [更新机制](architecture/update-mechanism.md) | update-state.json、状态流转、原子替换、回滚 |
| [文件传输](architecture/file-transfer.md) | FileConnector trait、传输路径、冲突处理 |
| [连接通道](architecture/connection-channels.md) | 直连资源、Agent 代理、WebSocket 隧道协议 |
| [Docker](architecture/docker.md) | 信号处理、Dockerfile、部署限制 |

---

## 5. 参考文档

开发时查阅的参考信息：

| 文档 | 内容 |
|------|------|
| [数据模型](reference/data-models.md) | SQLite 表结构、凭据加密、资源配置 |
| [API 设计](reference/api-design.md) | 认证、错误格式、分页、WebSocket 消息 |
| [前端工程](reference/frontend-architecture.md) | 页面路由、功能域组织、组件规范 |
| [配置约定](reference/config-conventions.md) | Hub/Agent 配置、目录结构、后端工程结构 |

---

## 6. 里程碑详细文档

每个里程碑的详细设计和实现记录：

```text
docs/milestones/
├── M0-project-skeleton.md
├── M1-hub-management.md
├── M2-agent-connection.md
├── M3a-ssh-backend.md
├── M3b-ssh-terminal-frontend.md
├── M4a-file-transfer-backend.md
├── M4b-file-management-frontend.md
├── M5a-sql-backend.md
├── M5b-sql-frontend.md
├── M6-update-detection.md
├── M7-auto-update.md
├── M8-frontend-refinement.md
├── M9-release.md
├── M10-audit-settings.md
├── M11-context-menus.md
├── M12-agent-management-enhancements.md
├── M13-workspace-shell.md
├── M14-workspace-panel-integration.md
├── M15-agent-deploy-guide.md
├── M16-wizard-and-tabmenu.md
├── M17-workspace-tabmenu.md
├── M18-terminal-sftp-sql-contextmenu.md
├── M19-sql-tree-tab-contextmenu.md
├── M20-workspace-panel-drag.md
├── M21-terminal-mobile-statusbar.md
├── M22-sidebar-favorites-recent.md
├── M23-file-upload-download.md
├── M24-docs-restructure.md
├── M25-sql-query-files.md
└── M26-sql-history-and-polish.md
```
