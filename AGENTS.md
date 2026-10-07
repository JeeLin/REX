# AGENTS.md

## 项目定位

REX Hub 是个人自托管远程资源统一管理平台，单用户、自托管、深色优先。不要引入多用户、RBAC、企业协作等概念。

- 产品文档：`docs/PRODUCT.md`
- 开发文档：`docs/DEVELOPMENT.md`（索引）
- 架构文档：`docs/architecture/`
- 参考文档：`docs/reference/`
- 里程碑文档：`.mdflow/milestones/`

新增功能前先确认产品文档中的功能边界，再把实现细节写入里程碑文档。

---

## 硬性约束

1. **前端命令一律用 `bun`**（`bun run dev`、`bun run build` 等），禁止 `npm run`。项目工具链由 `.mise.toml` 管理，bun 是前端包管理器。
2. **Hub/Agent 版本必须一致**，不存在跨版本兼容。
3. **文件传输数据不经过浏览器**，前端只创建任务、选择源/目标、展示进度、处理冲突。
4. 依赖声明在根 `Cargo.toml`，子 crate 用 `workspace = true`，不重复声明版本。

## 设计对标

2.0 重设计，交互布局对标成熟专业工具：

- 工作空间 / SSH 终端 → Xshell
- 数据库控制台 → Navicat
- Redis 控制台 → Another Redis Desktop Manager (ARDM)
- 文件管理 / 对象存储 → Xftp

详见 `docs/PRODUCT.md` 第 0 节「设计基调」与第 10 节「设计核对基线」。

## 设计审查配置

- 人工复核：可选（自动设计审查通过后，有争议时才进入人工复核）

## 质量门禁

Rust：
```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

前端（`packages/rex-console-web/`）：
```bash
bun run type-check
bun run lint
bun run build
```

## 仓库结构

```text
docs/
  PRODUCT.md              产品功能、架构决策、用户可见流程
  DEVELOPMENT.md          开发索引（技术栈、crate 结构、里程碑总览）
  architecture/           架构文档（进程模型、更新机制、文件传输、连接通道、Docker）
  reference/              参考文档（数据模型、API 设计、前端工程、配置约定）
  BUGS.md                 缺陷池
.mdflow/                  dev-flow 产出物
  milestones/
    M{N}-{name}.md        里程碑开发文档（完成后保留）
    M{N}-reports/         里程碑报告（步骤 2/4/5/6/7）
README.md                 产品简介
AGENTS.md                 本文件
.mise.toml                本地工具版本
Cargo.toml                Rust workspace 根配置
crates/
  rex-common/             通用类型、错误、配置解析
  rex-hub/                Hub 二进制（HTTP server + 前端托管）
  rex-agent/              Agent 二进制（反向代理）
  rex-ssh/                SSH/SFTP 协议
  rex-transfer/           文件传输引擎
packages/
  rex-console-web/        Vue 3 前端工程
```

---

## Rust 依赖规则

依赖声明在根 `Cargo.toml`，crate 内使用 `workspace = true`：

```toml
# 根 Cargo.toml
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

```toml
# crates/rex-hub/Cargo.toml
[dependencies]
serde = { workspace = true }
tokio = { workspace = true }
anyhow = { workspace = true }
```

子 crate 不重复声明版本。

---

## 前端组织

按功能域组织，不只按页面：

```text
packages/rex-console-web/src/
├── pages/          只做路由入口
├── features/       按功能域组织组件
│   ├── terminal/
│   ├── sql/
│   ├── files/
│   └── agents/
├── components/     跨功能通用组件
├── api/            按接口域拆分
├── stores/         跨功能状态
├── layouts/        布局组件
├── styles/         主题和全局样式
└── i18n/           国际化
```

---

## 架构原则

### Hub / Agent 进程模型

单二进制 + supervisor + worker：

```text
rex-hub / rex-agent 启动（PID 1）
  ↓
父进程进入 supervisor 模式
  ↓
启动 worker 子进程
```

父进程就是 supervisor，不需要 s6-overlay。第一阶段只做启动和监控；第二阶段增加更新检测、替换和回滚。

### Agent

内网反向代理进程，主动出站连接 Hub，建立 WebSocket 加密隧道。内网服务器不开放入站端口。

### 版本兼容

Hub 和 Agent 版本必须一致，不存在跨版本兼容。

### 文件传输

文件传输数据不经过浏览器。前端只创建任务、选择源/目标、展示进度、处理冲突。实际传输由后端完成。

---

## 常用命令

环境工具由 `.mise.toml` 管理：`mise install`

```toml
rust = "stable"
node = "latest"
npm:bun = "latest"
```

开发服务器（`packages/rex-console-web/`）：
```bash
bun run dev
```

门检命令见 [质量门禁](#质量门禁)。
