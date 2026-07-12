# REX Hub — AI Agent 上下文

## 项目

REX Hub 是个人自托管远程资源统一管理平台。单用户、自托管、深色优先。**永远不要引入多用户、RBAC、企业协作概念。**

- 产品文档：`docs/PRODUCT.md`
- 开发文档：`docs/DEVELOPMENT.md`
- 架构文档：`docs/architecture/`
- 里程碑：`docs/milestones/`

## 硬性约束

1. **前端命令一律用 `bun`**（`bun run dev`、`bun run build` 等），禁止 `npm run`。项目工具链由 `.mise.toml` 管理，bun 是前端包管理器。
2. **Hub/Agent 版本必须一致**，不存在跨版本兼容。
3. **文件传输数据不经过浏览器**，前端只创建任务、选择源/目标、展示进度、处理冲突。
4. 依赖声明在根 `Cargo.toml`，子 crate 用 `workspace = true`，不重复声明版本。

## 已知问题（截至 0.84.1）

这些 bug 待后续里程碑修复：

1. SSH 终端复制粘贴与浏览器 Ctrl+C/V 冲突

## 待规划功能

- 文件传输并发数量控制
- Redis 操作界面增强（参考 Another Redis Desktop Manager）
- SQL 操作界面增强（参考 Navicat）

## CI 优化

docker-agent 和 build-hub 两个 CI 流程各约 30 分钟，计划在 0.23.1 优化。方向：Docker layer caching、构建矩阵并行、增量编译复用、交叉编译优化。

## 架构

```text
rex-hub / rex-agent 启动（PID 1）
  → 父进程进入 supervisor 模式
  → 启动 worker 子进程
```

单二进制 + supervisor + worker。Agent 是内网反向代理，主动出站连接 Hub 建立 WebSocket 加密隧道。

## 开发流程

8 步串行：写里程碑文档 → 设计核对 → 开发 → simplify → code review → 测试 → 设计再确认 → 提交。

详细流程见 `CLAUDE.md`。

## 质量门禁

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
