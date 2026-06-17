# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目定位

REX Hub 是个人自托管远程资源统一管理平台，单用户、自托管、深色优先。不要引入多用户、RBAC、企业协作等概念。

产品文档在 `docs/PRODUCT.md`，实现细节和开发规划在 `docs/DEVELOPMENT.md`。新增功能前先确认产品文档中的功能边界，再把实现细节写入开发文档。

## 开发流程

开发必须按 `docs/DEVELOPMENT.md` 的里程碑顺序推进，不要跳阶段做大而全实现。

每个开发任务都必须串行执行以下流程，不能并行跳步骤：

```text
1. 使用 superpower 写开发文档
2. 和设计核对
3. 开发
4. 测试（100% 覆盖率）
5. simplify
6. code review
7. 和设计再确认
8. 下一项开发任务
```

### 1. 写开发文档

每个开发任务开始前，先写或更新开发文档。

产物：

```text
docs/DEVELOPMENT.md
```

大型功能可以拆成多个子功能点，每个子功能点单独进入流程。开发文档必须包含：

- 功能目标
- 产品边界
- 文件结构
- 接口设计
- 数据模型
- 状态流转
- 前端交互
- 后端流程
- 测试 / QA 标准
- 提交边界

如果功能较大，先拆成多个可独立提交的子功能点。

### 2. 和设计核对

写完开发文档后，先不开发，先核对设计。

核对重点：

- 是否符合 `docs/PRODUCT.md`
- 是否保持单用户、自托管定位
- Hub / Agent 是否仍是单二进制 + supervisor + worker
- 文件传输是否不经过浏览器
- Agent 是否尽量保持单文件
- 是否引入 RBAC、多用户、企业化概念
- 是否跳阶段实现
- 是否把实现细节写进了产品文档

如果不一致，先改开发文档，直到确认。

### 3. 开发

开发必须以已确认的开发文档为准。

原则：

- 文档为先。
- 不临时改产品方向。
- 不做文档外的大功能。
- 每个 commit 只包含一个完整子功能点。
- 不做一次性大提交。

### 4. 测试

开发完成后写测试，要求 100% 测试代码覆盖率。

测试原则：

- 所有公开函数和关键逻辑路径必须有测试。
- 测试代码依赖也必须使用 `workspace = true`，不单独声明版本。
- 测试只验证当前功能，不依赖外部服务或网络。
- 单元测试放在模块内 `#[cfg(test)] mod tests`。
- 集成测试放在 `tests/` 目录（如需）。

验证命令：

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
```

### 5. simplify

开发完成后先做 simplify，重点看：

- 是否有重复代码
- 是否有过度设计
- 是否提前实现第二阶段能力
- 是否符合 Rust workspace / Vue 功能域结构
- 是否把原型交互照搬成最终代码
- 是否可以把大文件拆小
- 是否符合 `workspace = true` 依赖规则

### 6. code review

simplify 后再 review。

review 维度：

- 正确性
- 安全性
- 架构一致性
- 测试覆盖
- 错误处理
- 配置和密钥处理
- 审计日志
- 与开发文档是否一致

如果 review 发现不符合设计：

```text
打回开发
```

不能直接进入确认。

### 7. 和设计再确认

review 通过后，再和设计确认一次。

如果确认发现：

- 实现和文档不一致
- 产品语义变了
- 阶段目标变了
- 用户可见行为变了

则打回开发，先改实现或改文档。

### 8. 提交

确认通过后提交。

提交规则：

```text
每个 commit 包含一个完整子功能点
```

例如：

```bash
git commit -m 'feat: add agent heartbeat model'
git commit -m 'feat: add agent list api'
git commit -m 'feat: add agent page cards'
```

不要：

```bash
git commit -m 'feat: add agent management'
```

如果一个大功能点拆了多个 commit，必须保证每个 commit 都是可理解、可独立存在的子功能点。

### 技能使用

后续每个开发任务按需要调用：

- `superpowers:writing-plans`：写开发计划 / 开发文档
- `superpowers:executing-plans`：按确认后的计划执行开发
- `superpowers:subagent-driven-development`：任务较大时拆分并行开发，但仍按阶段 review
- `simplify`：开发完成后先做代码简化
- `code-review`：simplify 后做 review
- `verify`：需要运行验证时调用
- `frontend-design`：涉及 UI / 视觉方向时调用
- `superpowers:verification-before-completion`：完成前做最终验证

当前仓库主要是文档和 HTML 原型，正式后端/前端工程尚未初始化。不要假设已有 Rust workspace、Vue 工程或测试命令。

## Rust 依赖规则

Rust 依赖必须声明在最外层 `Cargo.toml`，crate 内部使用 `workspace = true`。

后续 Rust workspace 初始化后，根 `Cargo.toml` 负责声明共享依赖：

```toml
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
tracing = "0.1"
```

crate 内部使用：

```toml
[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
```

不要在子 crate 里重复声明版本。

## 原型使用规则

原型只作为开发阶段前端参考。

开发前端时直接看：

```text
prototype/*.html
prototype/shared.js
prototype/css/*.css
```

不需要启动服务。

如果要看效果，可以启动静态服务：

```bash
python -m http.server 8000 --directory prototype
```

但原型不是最终代码，不应直接作为正式前端实现。

## 常用命令

环境工具由 `.mise.toml` 管理：

```bash
mise install
```

当前 `.mise.toml` 指定：

```toml
rust = "stable"
node = "latest"
npm:bun = "latest"
```

原型阶段没有构建脚本。可手动打开 `prototype/*.html` 查看交互，或用本地静态服务器预览：

```bash
python -m http.server 8000 --directory prototype
```

然后访问：

```text
http://localhost:8000
```

正式 Rust workspace 初始化后，常用命令应以实际 `Cargo.toml` 为准。开发文档中规划的目标命令是：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test -p <crate> <test_name>
```

正式 Vue 前端初始化后，常用命令应以 `packages/rex-console-web/package.json` 为准。开发文档中规划的技术栈是 Vue 3 + Vite。

## 仓库结构

```text
docs/
  PRODUCT.md       产品功能、架构决策、用户可见流程
  DEVELOPMENT.md   实现细节、里程碑、开发任务规划
prototype/
  *.html           产品交互原型
  css/             原型样式
  shared.js        原型共享逻辑
README.md          产品简介、部署命令、架构概览
.mise.toml         本地工具版本
```

当前还没有：

```text
Cargo.toml
crates/
packages/rex-console-web/
```

如果后续创建这些目录，应遵循 `docs/DEVELOPMENT.md` 中的 Rust workspace 和前端功能域结构。

## 架构原则

### Hub / Agent

Hub 和 Agent 都采用单二进制 + supervisor + worker 模型：

```text
rex-hub / rex-agent 启动（PID 1）
  ↓
父进程进入 supervisor 模式
  ↓
启动 worker 子进程
```

父进程就是 supervisor，不需要 s6-overlay、launcher 或 wrapper。第一阶段不自动更新时，父进程只做启动和监控；第二阶段增加更新检测、替换和回滚能力，进程模型不变。

### Agent

Agent 是内网反向代理进程，主动出站连接 Hub，建立加密隧道。内网服务器不开放入站端口。

Agent 页面应提供各平台二进制下载按钮，Hub 需要内置同版本 Agent 二进制。

### 版本兼容

Hub 和 Agent 版本必须一致，不存在跨版本兼容。每个 Hub 版本部署包内打包对应版本 Agent 二进制。

### 文件传输

文件传输数据不经过浏览器。前端只创建任务、选择源/目标、展示进度、处理冲突。实际传输由 Hub / Agent / 远端资源之间的后端传输层完成。

### 前端组织

前端未来应按功能域组织，而不是只按页面文件组织：

```text
features/<功能域>/
pages/              只做路由入口
components/         只放跨功能通用组件
api/                按接口域拆分
stores/             只放跨功能状态
```

`pages/` 不应承载大量业务逻辑。

## 原型文件

`prototype/` 是交互验证文件，不是最终产品代码。后续正式实现应迁移到 Vue 组件、stores、API client、主题和 i18n 模块。

保留原型中的关键交互：

- 标签右键菜单
- 全局连接菜单
- 多标签分屏
- SSH 终端内置 SFTP
- 移动端方向键和历史选择
- SQL 右键菜单
- 文件右键“发送到…”
- Agent 页面二进制下载按钮
- 设置页版本总览
- 深色/浅色/跟随系统主题
- 中文/英文 i18n

## 提交约定

按功能拆分提交。当前已按以下方向提交过：

- 项目元信息
- 产品/开发文档
- 原型共享 UI
- 管理页原型
- 工作区/操作页原型
- 登录页原型
- 开发路线图文档

新增 commit 应继续保持功能边界清晰。
