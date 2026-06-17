# REX Hub

**个人自托管远程资源统一管理平台**

一个 Web 页面管理所有远程资源。无论公网服务器还是内网机器，REX Hub 让你不再在多个客户端之间切换。

---

## 核心理念

| 问题 | 传统方式 | REX Hub |
|------|----------|---------|
| 多种资源需要多个工具 | SSH/iTerm2 + Navicat + FileZilla… | **一个工作空间**搞定一切 |
| 内网资源无法从外部访问 | VPN / 端口转发 / 跳板机 | Agent 反向隧道，**零端口暴露** |
| 数据在第三方平台 | 在线 SaaS 工具 | 完全**自托管**，数据在你手中 |

> **个人工具** — 专为单用户设计，无需多用户管理，简单够用

---

## 核心功能

### 💻 统一工作空间

多标签 + 5 种分屏布局，同时管理所有远程连接：

- **SSH 终端** — 完整 ANSI 仿真，内置 SFTP 文件管理，xterm.js 驱动
- **SQL 控制台** — MySQL / PostgreSQL，语法高亮，AI 助手，全局跨库查询
- **SFTP 文件管理** — 双面板布局，传输队列，拖放上传
- **跨连接文件传输** — SSH、SFTP、S3 之间互相传送文件
- **8 种协议** — SSH、SFTP、MySQL、PostgreSQL、Redis、Docker、SQLite、S3/MinIO

### 🌐 内网穿透

- **Agent 代理** — 轻量级守护进程从内网主动连接，建立 TLS 加密隧道
- **公网直连** — 同样支持公网资源直接连接
- **零暴露** — 内网服务器无需开放任何入站端口

### 🎛️ 面板拖拽

- 标签拖拽到不同面板位置，自由组合分屏
- 面板间内容交换
- 5 种布局：单面板 / 左右分屏 / 上下分屏 / 四宫格 / 主+侧边

### 🖱️ 右键菜单

每个交互组件都有上下文菜单：

- **标签** — 关闭、关闭其他、复制标签、移动到面板
- **SSH 终端** — 复制/粘贴、清屏、打开 SFTP、重连
- **SQL 编辑器** — 执行选中、格式化、注释切换、历史记录
- **SQL 结果表** — 复制行/列、排序、生成 UPDATE/DELETE
- **库表结构树** — SELECT *、查看结构、导出数据
- **SFTP 文件** — 打开、下载、复制路径、发送到…（跨连接传输）
- **侧边栏资源** — 连接、编辑、删除、复制地址
- **环境卡片** — 打开详情、新建资源、编辑、删除
- **Agent 卡片** — 查看日志、配置、重启、重置令牌

### 🎨 主题 & 国际化

- **深色 / 浅色 / 跟随系统** — CSS 变量驱动，一键切换
- **中文 / English** — 完整双语支持，即时切换无刷新

---

## 协议支持

| 协议 | 图标 | 用途 | 色值 |
|------|------|------|------|
| SSH | `$` | 远程终端（内置 SFTP） | `#3FB950` 绿色 |
| SFTP | 📁 | 独立文件传输 | `#8B5CF6` 紫色 |
| MySQL | `dB` | 数据库查询 | `#58A6FF` 蓝色 |
| PostgreSQL | `pg` | 数据库查询 | `#8B5CF6` 紫色 |
| Redis | `R` | 缓存控制台 | `#F85149` 红色 |
| Docker | 🐳 | 容器管理 | `#58A6FF` 蓝色 |
| SQLite | `S` | 本地数据库 | `#D29922` 橙色 |
| S3 / MinIO | ☁ | 对象存储 | `#E8912D` 主色 |

---

## 环境管理

按网络或用途分组资源：

```
阿里云 (直连)
├── 云服务器     SSH
├── 主数据库       MySQL
├── 分析数据库     PostgreSQL
└── Web 文件服务器  SFTP

树莓派集群 (Agent 代理)
├── 开发服务器     SSH
├── 测试数据库     MySQL
└── 缓存服务       Redis

家庭 NAS (直连)
├── NAS 主机       SSH
└── NAS 文件       SFTP
```

每个环境拥有独立的 Agent 和注册令牌，Agent 状态实时监控。

---

## 快捷键

### 工作空间

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+N` | 新建连接 |
| `Ctrl+W` | 关闭标签 |
| `Ctrl+Tab` | 切换标签 |
| `Alt+1~5` | 切换布局 |
| `F11` | 全屏 |
| `F1` | 快捷键帮助 |

### SQL 控制台

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Enter` | 执行 SQL |
| `Ctrl+S` | 保存查询 |
| `Ctrl+Shift+F` | 格式化 SQL |
| `Ctrl+Shift+Q` | 全局查询 |
| `Ctrl+Shift+A` | AI 助手 |

### SSH 终端

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Shift+C` | 复制 |
| `Ctrl+Shift+V` | 粘贴 |
| `Ctrl+L` | 清屏 |

---

## 部署

### 控制台（Docker）

```bash
docker run -d \
  --name rex-hub \
  -p 3000:3000 \
  -v ./data:/app/data \
  -e REX_SECRET_KEY=your-secret-key \
  rexhub/console:latest
```

访问 `http://localhost:3000` 开始使用。

### Agent（Docker）

```bash
docker run -d \
  --name rex-agent \
  --restart unless-stopped \
  -e REX_SERVER=https://your-hub.com \
  -e REX_TOKEN=<环境注册令牌> \
  -v /var/run/docker.sock:/var/run/docker.sock \
  rexhub/agent:latest
```

### Agent（二进制）

```bash
# 下载
curl -fsSL https://get.rexhub.dev/agent/linux-amd64 -o rex-agent
chmod +x rex-agent

# 启动
./rex-agent \
  --server https://your-hub.com \
  --token <环境注册令牌> \
  --name "内网 Agent"
```

更多部署方式（Docker Compose、配置文件）详见 [产品文档](docs/PRODUCT.md)。

---

## 架构

```
浏览器 (REX Hub 控制台)
    │ WebSocket + HTTPS
    ▼
REX Hub 服务端
    │ TLS 加密隧道
    ▼
REX Agent (内网节点)
    │ 协议代理
    ▼
SSH / 数据库 / 文件系统 / Docker
```

---

## 许可证

自托管 · 开源
