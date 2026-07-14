# REX Hub

**个人自托管远程资源统一管理平台**

一个 Web 页面管理所有远程资源：SSH 终端、数据库查询、Redis 管理、文件传输、对象存储。公网服务器和内网机器统一管理，不再在多个客户端之间切换。

---

## 设计基调（2.0 重设计）

2.0 是一次从 0 开始的全面重设计。**视觉语言是 REX 自己的，交互布局与操作逻辑对标成熟专业工具**，降低学习成本、提升专业感：

| 模块 | 对标产品 |
|------|----------|
| 工作空间 / SSH 终端 | **Xshell** |
| 数据库控制台 | **Navicat** |
| Redis 控制台 | **Another Redis Desktop Manager (ARDM)** |
| 文件管理 / 对象存储 | **Xftp** |
| 管理模块（仪表盘/Agent/审计/设置） | REX 自有设计系统 |

三要素：**现代化**（Web 原生、深色优先、自定义细滚动条）、**极客化**（等宽字体、高信息密度、键盘优先）、**易用化**（对标用户熟悉的专业工具，一致的交互范式贯穿全站）。

> 复刻的是操作方式与布局逻辑，不是 Windows 桌面软件的外观。

---

## 核心理念

| 问题 | 传统方式 | REX Hub |
|------|----------|---------|
| 多种资源需要多个工具 | SSH/iTerm2 + Navicat + FileZilla… | **一个工作空间**搞定一切 |
| 内网资源无法从外部访问 | VPN / 端口转发 / 跳板机 | Agent 反向隧道，**零端口暴露** |
| 数据在第三方平台 | 在线 SaaS 工具 | 完全**自托管**，数据在你手中 |

> **个人工具** — 专为单用户设计，不涉及多用户、团队协作、RBAC 权限。

---

## 协议支持

| 协议 | 用途 |
|------|------|
| SSH | 远程终端（内置 SFTP） |
| SFTP | 文件传输 |
| MySQL / PostgreSQL | 数据库查询 |
| Redis | 缓存管理 |
| SQLite | 本地数据库 |
| S3 / MinIO | 对象存储 |

---

## 架构

```text
浏览器 (REX Hub 控制台)
    │ HTTPS + WebSocket
    ▼
REX Hub 服务端 (Rust, single binary)
    │ TLS 加密隧道
    ▼
REX Agent (内网节点, 主动出站)
    │ 协议代理
    ▼
SSH / 数据库 / 文件系统 / 对象存储
```

- **进程模型** — 单二进制 + supervisor + worker，Hub/Agent 版本一致
- **技术栈** — Rust 后端（tokio async）+ Vue 3 前端（Vite + TypeScript）+ SQLite 本地存储
- **文件传输** — 数据在 Hub/Agent/远端之间传输，**不经过浏览器**

---

## 快速开始

环境工具由 `mise` 管理：

```bash
mise install        # 安装 rust/node/bun
mise x -- cargo build --workspace
cd packages/rex-console-web && bun install && bun run dev
```

更多部署方式（Docker / 二进制 / 配置文件）详见 [产品文档](docs/PRODUCT.md)。

---

## 文档

- 产品文档：`docs/PRODUCT.md`
- 开发文档：`docs/DEVELOPMENT.md`
- 架构文档：`docs/architecture/`
- 里程碑：`docs/milestones/`
