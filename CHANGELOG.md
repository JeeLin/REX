# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/).

## [Unreleased]

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
