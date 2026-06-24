# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/).

## [Unreleased]

## [0.14.0] - 2026-06-24

### Added
- 仪表盘环境卡片资源类型统计徽章（按协议分组计数，协议色半透明背景）

### Already Existing (confirmed in 0.14.0 review)
- SQL 查询结果导出（JSON/CSV）— 确认已在 SqlResults.vue 实现
- SQL 结果表格右键菜单（复制行/单元格/列、排序、生成 SQL）— 确认已实现
- 全局连接菜单 Ctrl+N（搜索 + 环境分组 + 键盘导航）— 确认已实现

## [0.13.0] - 2026-06-24

## [0.12.0] - 2026-06-24

### Fixed
- 修复前端 ESLint 警告（146 → 0）
  - 移除未使用变量
  - 修复 vue/attributes-order
  - 修复 vue/multiline-html-element-content-newline
  - 修复 vue/v-on-event-hyphenation

### Changed
- 提升后端测试覆盖率 63.43% → 69.45%
  - 新增 audit 模块单元测试（审计日志写入、查询、统计）
  - 新增 files 模块错误处理测试

## [0.11.0] - 2026-06-22

### Added
- S3/MinIO 协议支持：新增 `rex-s3` crate，实现 S3 对象存储连接和 S3Connector trait
- Hub S3 WebSocket 会话管理（`/ws/s3/:resource_id`）
- 前端 S3 控制台（Bucket 列表、Object 浏览、面包屑导航、右键菜单、属性弹窗）
- S3 文件上传/下载（通过 WebSocket base64 编码传输）
- S3 资源创建向导（Endpoint、Access Key、Secret Key、Region、Bucket、Path Style）
- 工作空间 S3 面板集成

### Fixed
- 修复 ObjectBrowser.vue 全局事件监听器未清理的内存泄漏
- 移除 rex-s3 未使用的依赖（rex-common、tracing、reqwest、hmac、sha2、hex、time、base64）

## [0.10.0] - 2026-06-22

### Added
- SQLite 协议支持：新增 `rex-sqlite` crate，实现 SQLite 数据库连接和 SqliteConnector trait
- Hub SQLite WebSocket 会话管理（`/ws/sqlite/:resource_id`）
- 前端 SQLite 控制台（SQL 编辑器、表列表侧边栏、结果表格）
- SQLite 资源创建向导（数据库文件路径、实例名称）
- 工作空间 SQLite 面板集成

## [0.9.0] - 2026-06-22

### Added
- Docker 容器管理：新增 `rex-docker` crate，实现 Docker Engine REST API 通信和 DockerConnector trait
- Hub Docker WebSocket 会话管理（`/ws/docker/:resource_id`）
- 前端 Docker 容器控制台（容器列表、搜索过滤、状态操作、日志查看、inspect 详情）
- Docker 资源创建向导（Unix Socket / TCP 连接模式、实例名称）
- 工作空间 Docker 面板集成

## [0.8.0] - 2026-06-22

### Added
- Redis 协议支持：新增 `rex-redis` crate，实现 RESP 协议解析和 RedisConnector trait
- Hub Redis WebSocket 会话管理（`/ws/redis/:resource_id`）
- 前端 Redis 命令控制台（命令输入、结果展示、历史记录）
- Redis 资源创建向导（host/port/password/db/name 表单）
- 工作空间 Redis 面板集成

## [0.7.0] - 2026-06-22

### Added
- ACME 自动证书：支持 Let's Encrypt 域名证书（HTTP-01）和 IP 证书（TLS-ALPN-01）
- 自签名证书生成（手动启用，适用于内网）
- TLS 模式优先级选择：manual > acme > none
- 前端设置页 TLS 状态面板（模式、证书状态、颁发者）
- Docker 镜像 ACME 支持（80 端口映射）
- `GET /api/settings/tls` API

### Changed
- CLI 新增 `--acme-domain`、`--acme-email`、`--acme-staging` 参数
- HubConfig 新增 `acme` 配置段
- 无 TLS 配置时默认 HTTP（不再自动生成自签名证书）

## [0.6.0] - 2026-06-22

### Added
- Hub TLS/HTTPS 支持：通过配置文件、环境变量或 CLI 参数指定证书和私钥
- Agent 二进制下载端点 `GET /api/agent/download?os={os}&arch={arch}`
- Agent 更新流程支持从 Hub 下载二进制（`update.source: hub`）
- Docker 镜像 TLS 配置支持（443 端口、证书目录、环境变量）

### Changed
- CLI 新增 `--tls-cert` 和 `--tls-key` 参数
- HubConfig 新增 `tls` 配置段
- AgentConfig 新增 `update.source` 配置项

## [0.5.0] - 2026-06-22

### Added
- Hub 和 Agent docker-compose 配置文件
- Agent 专用 README（用于 Docker Hub 页面）
- CHANGELOG.md

### Changed
- CI Release 使用 CHANGELOG.md 生成 Release Notes
- CI Agent Docker 构建引用 agent-readme

## [0.4.0] - 2026-06-21

### Added
- 终端设置（字体大小、字体族、光标闪烁）实时生效
- 会话超时自动登出（useSessionTimeout composable）
- 审计日志开关控制侧边栏入口显示
- 共享设置 store（stores/settings.ts）单一数据源

### Changed
- 重构设置存储为 reactive store

## [0.3.0] - 2026-06-21

### Added
- 审计日志 API 对接和分页
- i18n 补全（Profile、Agent 部署指南）
- 共享错误处理辅助函数（getErrorMessage）
- TypeScript `any` 类型修复

### Fixed
- 审计日志分页使用 API 总数而非过滤后数组长度

## [0.2.0] - 2026-06-20

### Added
- 终端内置 SFTP 面板
- 编辑器自动补全
- SQL 查询文件保存/加载
- SQL 结果 CSV 和 JSON 导出
