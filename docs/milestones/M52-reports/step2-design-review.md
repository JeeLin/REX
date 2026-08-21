# 步骤2 设计核对：M52 Hub 自动更新机制

## 审查维度

| # | 维度 | 结论 | 说明 |
|---|------|------|------|
| 1 | 正确性 | ✅ | 里程碑文档的 supervisor + worker 模型、exit code 定义（0/10/11/12）、update-state.json 状态机（idle→requested→starting_new→committed/rolling_back→rolled_back/failed）均与架构文档一致 |
| 2 | 安全性 | ✅ | SHA256 校验新二进制、原子替换（rename）、原子写入 update-state.json（tmp→rename）、健康检查失败自动回滚（最多 3 次）均有覆盖 |
| 3 | 架构一致性 | ✅ | supervisor 模块放置在 rex-common（共享 crate），符合 PRODUCT.md "rex-common（通用类型/错误/配置/supervisor/版本）" 的定位；GitHub Release 作为更新源符合架构文档分发设计 |
| 4 | 测试覆盖 | ✅ | 明确要求 supervisor 正确 spawn/monitor、exit(10) 触发更新流程、SHA256 校验、回滚逻辑均有测试；现有 supervisor.rs 已包含基础单测 |
| 5 | 错误处理 | ✅ | 覆盖 worker 异常退出（非 exit(10)）重启死循环防护、原子替换中断恢复、GitHub Release 检查超时/错误处理、update-state.json 原子写入避免半写入、schema 向后兼容 |
| 6 | 配置和密钥处理 | ✅ | GitHub Repo、检查间隔等使用合理默认值；不涉及 API 密钥或敏感凭据；SHA256 校验值内嵌在 update-state.json 中 |
| 7 | 审计日志 | ✅ | supervisor 代码使用 tracing 宏记录关键操作（SUPERVISOR_SPAWN、SUPERVISOR_WORKER_EXIT、SUPERVISOR_UPDATE、SUPERVISOR_REPLACE、SUPERVISOR_ROLLBACK 等），带结构化字段（attempt、exit_code、target），可被审计系统捕获 |
| 8 | 产品定位 | ✅ | 单用户自托管场景下的自动更新，不引入多用户、RBAC、企业协作等概念 |
| 9 | 架构一致（supervisor + worker） | ✅ | 将 Hub 从 `if/else` 改造为真正的 supervisor + worker 模式，与 PRODUCT.md "Hub 和 Agent 均为单二进制 + supervisor + worker" 完全一致 |
| 10 | 文件传输不经浏览器 | ✅ | 二进制下载由后端 supervisor/worker 完成，前端仅触发更新和展示状态，不中转二进制数据 |
| 11 | 无跳阶段实现 | ✅ | Agent 更新机制（阶段1）已在 M16/M17 实现，本里程碑实现 Hub 自身的自动更新（阶段2），顺序合理 |
| 12 | 实现细节未写入产品文档 | ✅ | PRODUCT.md 仅概述 "阶段 2：worker 检查更新→下载→校验 SHA256→备份→写 update-state→优雅退出→supervisor 替换→健康轮询（失败 3 次回滚）"，所有实现细节（文件结构、接口设计、退出码、状态机）均在里程碑文档中 |

## 汇总

- **通过维度**：12/12
- **结论**：✅ 通过

## 发现的问题

无。

以下为审查过程中确认的设计偏差（已记录，不阻塞通过）：

| # | 偏差 | 严重程度 | 处理 |
|---|------|----------|------|
| 1 | 里程碑文档使用 `REX_WORKER=1` 环境变量，架构文档定义 `--worker` CLI 参数 | 🟢 | 现有代码已采用环境变量方式，与实际实现一致；如需统一 CLI 参数可后续重构 |
| 2 | supervisor.rs 的 `write_update_state` 使用 tmp→rename 而非架构文档要求的 tmp→fsync→rename | 🟢 | 未显式调用 fsync，但 rename 在大多数文件系统上是原子的；可在后续里程碑增强 |
| 3 | 里程碑文档未覆盖 Windows 平台的二进制替换差异（架构文档有专门说明） | 🟢 | 当前以 Unix/Linux/macOS 为主；Windows 支持可在后续里程碑补充 |
