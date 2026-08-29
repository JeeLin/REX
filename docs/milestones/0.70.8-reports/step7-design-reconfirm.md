# 步骤7：设计再确认 — v0.70.8

> 审查对象：已实现代码（里程碑变更）vs 里程碑文档 `docs/milestones/0.70.8-hub-agent-service-cli.md`
> 维度来源：dev-flow 步骤7 再确认清单（实现与文档一致 / 产品语义未变 / 用户可见行为未变）

## 核对项

| # | 维度 | 结论 | 说明 |
|---|------|------|------|
| 1 | 子任务 1：共享 CLI 框架（run/version/service/stop） | ✅ | `rex-common::cli` 定义 `Cli`/`Commands`/`RunOpts`/`ServiceCmd`，`dispatch` 分发；hub/agent 复用 |
| 2 | 子任务 2：hub/agent 接入 CLI | ✅ | 两个二进制 `main` 调用 `cli::dispatch`，保留原有 supervisor+worker 启动路径 |
| 3 | 子任务 3：`service install` + 开机自启 | ✅ | `rex-common::service` 生成 systemd（Linux）/ launchd（macOS）单元，`open-boot` 启用；install 时落 `--data-dir` 确保与手动运行一致 |
| 4 | 子任务 4：`config.yaml` 解析 | ✅ | `rex-common::config` 支持 `config.yaml`，env 优先于文件 |
| 5 | 子任务 5：部署文档 | ✅ | PRODUCT.md §9 / agent-deploy.md / architecture/process-model.md 均已补充 service install 与开机自启 |
| 6 | 子任务 6：单实例互斥（同一 data_dir） | ✅ | `ensure_single_instance` 按 `data_dir` 校验，冲突给出 `rex-{kind} stop` 提示 |
| 7 | 子任务 7：更新相对 URL 修复 | ✅ | `updater.rs` 以 `REX_HUB_URL` 基地址解析相对下载 URL，单元测试覆盖 |
| 8 | 产品语义 | ✅ | 未引入账户/多用户/RBAC/协作等概念；仅本机运维增强 |
| 9 | 用户可见行为 | ✅ | 退出码（42/10）、pidfile 路径、日志路径等行为保持不变；新增 `service`/`stop`/`version` 为增量能力 |

## 汇总

- **通过维度**：9 / 9
- **结论**：✅ 通过，步骤7 勾选

## 发现的问题

无
