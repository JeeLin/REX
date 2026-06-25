# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/).

## [Unreleased]

## [0.19.0] - 2026-06-25

### Added
- 系统指标采集和存储后端：新增 `metrics.rs` 模块，实现性能指标（延迟、吞吐量、错误、连接）的采集、存储和查询
- 系统健康检查 API：新增 `/api/health` 端点，返回系统运行状态、资源使用情况和数据库状态
- 指标聚合查询 API：新增 `/api/metrics/summary` 和 `/api/metrics/timeline` 端点，支持按时间段和资源聚合查询
- 数据库迁移：添加 `metrics` 表及相关索引
- 前端健康 API 调用：新增 `src/api/health.ts` 用于获取系统健康状态
- 前端设置页版本显示：修改 `src/pages/Settings.vue` 从 API 动态获取版本号

## [0.18.0] - 2026-06-25

### Added
- 配置导出 API：POST /api/backup/export，支持按环境筛选和可选密码加密（PBKDF2-SHA256 + AES-256-GCM）
- 配置导入 API：POST /api/backup/import，支持 skip_existing / overwrite 合并策略
- 导入预览 API：POST /api/backup/preview，导入前预览冲突和差异
- 设置页备份与恢复面板：导出配置、导入上传（拖拽/点击）、加密选项、导入预览弹窗、导入结果统计
- 导入操作事务保护：失败时自动回滚
- 备份文件大小限制（50MB）

## [0.17.0] - 2026-06-24

### Added
- 全局查询后端 API：POST /api/sql/global-query，SSE 流式响应，并行执行相同 SQL 于多个资源
- 同环境 SQL 资源发现 API：GET /api/resources/:resource_id/sql/peers
- 全局查询前端模态弹窗：Ctrl+Shift+Q 快捷键触发，资源多选、方言验证、进度条、结果标签页
- 前端 useGlobalQuery composable：fetch + ReadableStream 手动解析 SSE，AbortController 取消支持
- 后端并行查询：tokio::spawn + 30s 超时 + 部分失败独立处理
- 安全改进：API 接受 resource_id 而非原始连接详情，密码在服务端读取

## [0.16.0] - 2026-06-24

### Added
- SQL AI 助手后端：AI 配置存储（数据库加密）、SSE 代理 API
- SQL AI 助手前端面板：Ctrl+Shift+A 快捷键、抽屉式布局、流式聊天
- 快捷操作按钮：生成 SQL、分析慢查询、表关系
- SQL 代码块复制：AI 回复中 SQL 代码一键复制到编辑器

## [0.15.0] - 2026-06-24

### Added
- 传输执行引擎：`rex-transfer` crate 新增 `executor.rs`，支持 local↔local 文件传输
- 连接器解析器：`transfer.rs` 新增 `resolve_connector`，根据端点类型创建 LocalConnector/SftpConnector
- `create_transfer` API 现在自动启动异步传输执行（`tokio::spawn`）
- Executor panic 监控：避免任务永久卡在 Running 状态

### Fixed
- 移除未使用的测试辅助函数
- 修复 executor 中重复的 progress 更新

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
