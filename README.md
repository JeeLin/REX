# REX Hub

个人自托管远程运维控制台 / Personal Self-Hosted Remote Operations Console

在一个深色优先的 Web 页面里，管理 SSH 终端、数据库查询、Redis 管理、文件传输、对象存储、SIP 电话等多类远程资源；并通过内网 Agent 隧道，无需 VPN 或开放入站端口，从任何有网络的地方访问你的公网与内网服务器。

> **单用户 · 自托管 · 数据自主掌控 · 深色优先**  
> **Single-user · Self-hosted · Data sovereignty · Dark-mode first**

---

## 名字含义 / Name Meaning

- **REX** = **R**emote **EX**change（远程交换）：在一个页面里交换、中转对各类远程资源的操作。  
  Remote Exchange: Exchange and forward operations for various remote resources on a single page.
- **Hub / Agent** 是两个二进制：Hub 是部署在用户侧的服务端（托管前端 + 聚合资源），Agent 是部署在目标内网的反向代理进程。**"Hub" 用于与 Agent 区分**，不是泛称"中枢"。  
  Hub/Agent are two binaries: Hub is deployed on the user side (hosts frontend + aggregates resources), Agent is deployed in the target internal network as a reverse proxy process. "Hub" is used to distinguish from Agent, not a generic "core".

---

## 核心承诺 / Core Promises

| 承诺 / Promise | 传统方式 / Traditional Way | REX Hub |
|--------------|--------------------------|---------|
| 一个平台管多种资源类型<br/>One platform for multiple resource types | SSH/iTerm2 + Navicat + FileZilla… 在多个客户端间切换<br/>Switching between multiple clients | **一个工作空间**搞定一切<br/>**One workspace** to rule them all |
| 一个服务管公网 + 内网<br/>One service for public + internal network | VPN / 端口转发 / 跳板机<br/>VPN / port forwarding / jump host | Agent 反向隧道，**零端口暴露**<br/>Agent reverse tunnel, **zero port exposure** |
| 数据自主掌控<br/>Data sovereignty | 在线 SaaS 工具，数据在第三方<br/>Online SaaS tools, data in third-party | 完全**自托管**，数据在你手中<br/>Completely **self-hosted**, data in your hands |

> 专为单用户设计，不涉及多用户、团队协作、RBAC 权限。  
> Designed for single-user only, no multi-user, team collaboration, or RBAC permissions.

---

## 设计方法 / Design Approach

每个协议模块的交互与布局对标其领域最成熟的桌面工具，用现代 Web 实现复刻其**操作逻辑**，而非外观：

Each protocol module's interaction and layout benchmarks against the most mature desktop tools in its field, using modern Web to replicate their **operational logic**, not just appearance:

| 模块 / Module | 对标产品 / Benchmark Product |
|--------------|----------------------------|
| 工作空间 / SSH 终端<br/>Workspace / SSH Terminal | **Xshell** |
| 数据库控制台<br/>Database Console | **Navicat** |
| Redis 控制台<br/>Redis Console | **ARDM** (Another Redis Desktop Manager) |
| 文件管理 / 对象存储<br/>File Management / Object Storage | **Xftp** |
| 管理模块（仪表盘/Agent/审计/设置）<br/>Management modules (Dashboard/Agent/Audit/Settings) | REX 自有设计系统<br/>REX's own design system |

**现代化**（Web 原生、深色优先、自定义细滚动条）、**极客化**（等宽字体、高信息密度、键盘优先）、**易用化**（对标用户熟悉的专业工具，一致的交互范式贯穿全站）。  
**Modern** (Web-native, dark-mode first, custom scrollbars), **Geeky** (monospace fonts, high information density, keyboard-oriented), **User-friendly** (benchmarked against familiar professional tools with consistent interaction patterns throughout the site).

---

## 协议支持 / Protocol Support

| 协议 / Protocol | 用途 / Purpose |
|----------------|---------------|
| SSH | 远程终端（内置 SFTP）<br/>Remote terminal (with built-in SFTP) |
| SFTP | 文件传输<br/>File transfer |
| MySQL / PostgreSQL | 数据库查询<br/>Database querying |
| Redis | 缓存管理<br/>Cache management |
| SQLite | 本地数据库<br/>Local database |
| S3 / MinIO | 对象存储<br/>Object storage |
| SIP 电话 | 语音通信<br/>Voice communication |

---

## 架构 / Architecture

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
  Single binary + supervisor + worker, Hub/Agent versions must match
- **技术栈** — Rust 后端（tokio async）+ Vue 3 前端（Vite + TypeScript）+ SQLite 本地存储  
  Rust backend (tokio async) + Vue 3 frontend (Vite + TypeScript) + SQLite local storage
- **文件传输** — 数据在 Hub/Agent/远端之间传输，**不经过浏览器**  
  Data transfers between Hub/Agent/remote endpoints, **not through the browser**

---

## 1.0 范围 / v1.0 Scope

1.0 包含两个阶段：  
v1.0 includes two phases:

- **阶段 1 — 单协议深度**：7 类协议各自做到可用、好用，交互对标成熟工具。  
  Phase 1 — Single-protocol depth: 7 protocol types each made usable and pleasant, interaction benchmarked against mature tools.
- **阶段 2 — Agent 代理体验**：内网隧道、注册、心跳、自更新、断线自愈、部署指南完整可用。  
  Phase 2 — Agent proxy experience: internal network tunneling, registration, heartbeat, auto-update, disconnect self-healing, deployment guide complete and usable.

访问形态：1.0 通过 **Web 控制台**访问（Web 是 1.0 的呈现形态，非产品固有属性）。跨协议联动与桌面端为后续演进方向，不在 1.0 范围内。  
Access form: v1.0 accessed via **Web console** (Web is v1.0's presentation form, not an intrinsic product attribute). Cross-protocol coordination and desktop evolution are post-v1.0 directions, not in v1.0 scope.

---

## 快速开始 / Quick Start

环境工具由 `mise` 管理：  
Environment tools managed by `mise`:

```bash
mise install        # 安装 rust/node/bun   Install rust/node/bun
mise x -- cargo build --workspace
cd packages/rex-console-web && bun install && bun run dev
```

更多部署方式（Docker / 二进制 / 配置文件）详见 [部署指南](docs/deployment/README.md)。  
More deployment methods (Docker / binary / config file) see [deployment guide](docs/deployment/README.md).

---

## 文档 / Documentation

- [产品文档](docs/PRODUCT.md) / Product Documentation
- [开发文档](docs/DEVELOPMENT.md) / Development Documentation
- [架构文档](docs/architecture/) / Architecture Documentation
- [API 参考](docs/reference/api-endpoints.md) / API Reference
- [环境变量](docs/reference/env-variables.md) / Environment Variables
- [部署指南](docs/deployment/README.md) / Deployment Guide
- [Agent 部署](docs/agent-readme.md) / Agent Deployment