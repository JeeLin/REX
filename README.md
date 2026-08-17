# REX Hub

**个人自托管远程运维控制台**

在一个深色优先的 Web 页面里，管理 SSH 终端、数据库查询、Redis 管理、文件传输、对象存储、SIP 电话等多类远程资源；并通过内网 Agent 隧道，无需 VPN 或开放入站端口，从任何有网络的地方访问你的公网与内网服务器。

> **单用户 · 自托管 · 数据自主掌控 · 深色优先**

---

## 名字含义

- **REX** = **R**emote **E**xchange（远程交换）：在一个页面里交换、中转对各类远程资源的操作。
- **Hub / Agent** 是两个二进制：Hub 是部署在用户侧的服务端（托管前端 + 聚合资源），Agent 是部署在目标内网的反向代理进程。**"Hub" 用于与 Agent 区分**，不是泛称"中枢"。

---

## 核心承诺

| 承诺 | 传统方式 | REX Hub |
|------|----------|---------|
| 一个平台管多种资源类型 | SSH/iTerm2 + Navicat + FileZilla… 在多个客户端间切换 | **一个工作空间**搞定一切 |
| 一个服务管公网 + 内网 | VPN / 端口转发 / 跳板机 | Agent 反向隧道，**零端口暴露** |
| 数据自主掌控 | 在线 SaaS 工具，数据在第三方 | 完全**自托管**，数据在你手中 |

> 专为单用户设计，不涉及多用户、团队协作、RBAC 权限。

---

## 设计方法

每个协议模块的交互与布局对标其领域最成熟的桌面工具，用现代 Web 实现复刻其**操作逻辑**，而非外观：

| 模块 | 对标产品 |
|------|----------|
| 工作空间 / SSH 终端 | **Xshell** |
| 数据库控制台 | **Navicat** |
| Redis 控制台 | **ARDM** |
| 文件管理 / 对象存储 | **Xftp** |
| 管理模块（仪表盘/Agent/审计/设置） | REX 自有设计系统 |

**现代化**（Web 原生、深色优先、自定义细滚动条）、**极客化**（等宽字体、高信息密度、键盘优先）、**易用化**（对标用户熟悉的专业工具，一致的交互范式贯穿全站）。

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
| SIP 电话 | 语音通信 |

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

## 1.0 范围

1.0 包含两个阶段：

- **阶段 1 — 单协议深度**：7 类协议各自做到可用、好用，交互对标成熟工具。
- **阶段 2 — Agent 代理体验**：内网隧道、注册、心跳、自更新、断线自愈、部署指南完整可用。

访问形态：1.0 通过 **Web 控制台**访问（Web 是 1.0 的呈现形态，非产品固有属性）。跨协议联动与桌面端为后续演进方向，不在 1.0 范围内。

---

## 快速开始

环境工具由 `mise` 管理：

```bash
mise install        # 安装 rust/node/bun
mise x -- cargo build --workspace
cd packages/rex-console-web && bun install && bun run dev
```

更多部署方式（Docker / 二进制 / 配置文件）详见 [部署指南](docs/deployment/README.md)。

---

## 文档

- [产品文档](docs/PRODUCT.md)
- [开发文档](docs/DEVELOPMENT.md)
- [架构文档](docs/architecture/)
- [API 参考](docs/reference/api-endpoints.md)
- [环境变量](docs/reference/env-variables.md)
- [部署指南](docs/deployment/README.md)
- [Agent 部署](docs/agent-readme.md)
