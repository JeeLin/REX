# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目定位

REX Hub 是个人自托管远程资源统一管理平台，单用户、自托管、深色优先。不要引入多用户、RBAC、企业协作等概念。

- 产品文档：`docs/PRODUCT.md`
- 开发文档：`docs/DEVELOPMENT.md`

新增功能前先确认产品文档中的功能边界，再把实现细节写入开发文档。

---

## 开发流程

开发按里程碑（M0 → M9）顺序推进，不跳阶段。每个里程碑串行执行以下 8 步：

```text
1. 写开发文档（创建里程碑文档）
2. 和设计核对
3. 开发（前后端一起）
4. simplify
5. code review
6. 测试（100% 覆盖率）
7. 和设计再确认
8. 提交 + 完成里程碑
```

### 1. 写开发文档

每个里程碑开始前，在 `docs/milestones/` 创建该阶段的里程碑文档（如 `M0-project-skeleton.md`）。

**大功能可以拆成多个子里程碑**，每个子里程碑独立进入 8 步流程。例如 M3（SSH 终端）可以拆为 M3a（后端 SSH crate）和 M3b（前端 terminal 页面）。

**里程碑文档必须包含：**

```markdown
# M{N}: {标题}

## Context
前序阶段完成了什么，本阶段在整体产品中的位置。

## 产品边界
本阶段做什么、不做什么。

## 子任务清单
| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| N.1 | ... | 后端 | ⬜ |
| N.2 | ... | 前端 | ⬜ |

## 子任务详细设计（每个子任务包含）
- 功能目标
- 文件结构（创建/修改哪些文件）
- 接口设计（API 端点 / 组件 props / 数据模型）
- 前端交互（参考 prototype 原型）
- 后端流程
- 测试标准
- 提交信息（commit message）

## 设计核对点
```

里程碑文档完成后保留，作为历史记录和后续阶段回溯参考。

### 2. 和设计核对

写完开发文档后，对照 `docs/PRODUCT.md` 检查：

- 是否符合产品定位（单用户、自托管）
- 架构是否一致（单二进制 + supervisor + worker）
- 文件传输是否不经过浏览器
- 是否引入了不该有的概念（RBAC、多用户）
- 是否跳阶段实现
- 是否把实现细节写进了产品文档

**不通过 → 改文档，直到确认。**

### 3. 开发

按确认的里程碑文档逐个子任务实现，前后端一起做。

原则：
- 文档为先，不临时改方向
- 不做文档外的大功能
- 每个 commit 只包含一个完整子功能点
- 不做一次性大提交

### 4. simplify

开发完成后精简代码：
- 是否有重复代码
- 是否有过度设计
- 是否提前实现了下一阶段能力
- 是否符合 Rust workspace / Vue 功能域结构
- 是否把原型交互照搬成最终代码
- 是否可以把大文件拆小
- 是否符合 `workspace = true` 依赖规则

### 5. code review

审查维度：
- 正确性、安全性、架构一致性
- 测试覆盖、错误处理
- 配置和密钥处理
- 审计日志
- 与里程碑文档是否一致

**review 发现不符合设计 → 打回开发，不能直接进入测试。**

### 6. 测试

在 simplify + review 后的最终代码上跑测试。

验证命令：
```bash
# Rust
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace

# 前端
npm run type-check
npm run lint
npm run build
```

测试原则：
- 所有公开函数和关键逻辑路径必须有测试
- Rust 测试代码依赖使用 `workspace = true`
- 单元测试放在模块内 `#[cfg(test)] mod tests`
- 前端测试只验证当前功能，不依赖外部服务

### 7. 和设计再确认

再核对一次，确保：
- 实现和里程碑文档一致
- 产品语义没变
- 用户可见行为没变

**不一致 → 打回开发。**

### 8. 提交

确认通过后提交。每个 commit 只包含一个子功能点：

```bash
# ✅ 好的
git commit -m 'feat: add agent heartbeat model'
git commit -m 'feat: add agent list api'

# ❌ 不好
git commit -m 'feat: add agent management'
```

里程碑完成后：
1. 保留里程碑文档（`docs/milestones/M{N}*.md`）
2. 删除 `docs/DEVELOPMENT.md` 中对应阶段的内容

---

## 仓库结构

```text
docs/
  PRODUCT.md              产品功能、架构决策、用户可见流程
  DEVELOPMENT.md          实现细节、里程碑、开发任务规划
  milestones/
    M{N}-{name}.md        里程碑开发文档（完成后保留）
prototype/
  *.html                  产品交互原型
  css/                    原型样式
  shared.js               原型共享逻辑
README.md                 产品简介
CLAUDE.md                 本文件
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

## 原型使用规则

`prototype/` 只作为前端开发参考，不是最终代码。

参考时直接看：
```text
prototype/*.html
prototype/shared.js
prototype/css/*.css
```

保留原型中的关键交互：
- 标签右键菜单、全局连接菜单、多标签分屏
- SSH 终端内置 SFTP、移动端方向键和历史选择
- SQL 右键菜单、文件右键"发送到…"
- Agent 页面二进制下载按钮、设置页版本总览
- 深色/浅色/跟随系统主题、中文/英文 i18n

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

Rust：
```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

前端（`packages/rex-console-web/`）：
```bash
npm run dev          # 开发服务器
npm run build        # 构建
npm run type-check   # 类型检查
npm run lint         # lint
```

---

## 提交约定

按功能拆分提交，每个 commit 只包含一个子功能点。里程碑完成后保留里程碑文档。

新增 commit 继续保持功能边界清晰。
