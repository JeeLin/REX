# 0.87.0 步骤2：设计核对报告

## 核对结论：✅

## 核对维度（里程碑文档 vs 产品文档/AGENTS.md）

### 1. 产品边界一致性
- ✅ **单用户约束**：本里程碑不引入多用户/RBAC/企业协作，符合 `AGENTS.md` 硬性约束。
- ✅ **Hub 仅 Docker 部署**：子任务 1 的 systemd 模板明确定位为「可选自托管方式补充」，Hub 主路径仍为 Docker（与用户澄清一致）。Agent 作为内网反向代理，systemd 模板对其裸机/NAS 部署合理。
- ✅ **文件传输数据不经过浏览器**：本里程碑不涉及文件传输数据路径变更。

### 2. 架构一致性
- ✅ **supervisor + worker 模型**：`AGENTS.md` 架构段明确「单二进制 + supervisor + worker」「父进程进入 supervisor 模式 → 启动 worker 子进程」。子任务 1 将 Docker/compose 改为 supervisor 模式（去掉 `--worker` 直启），使 `exit 10`/崩溃能被 in-container supervisor 捕获，与既有架构完全一致。
- ✅ **Hub/Agent 版本一致**：硬约束要求「版本必须一致，不存在跨版本兼容」。子任务 2 的「版本门禁」拒绝 Agent 拉取未随当前 Hub 部署的版本（GitHub 兜底仅作最后手段且需匹配），直接支撑该约束。
- ✅ **Agent 元信息/设备信息**：`PRODUCT.md` 2.2 规定 Agent 展示版本号、OS/架构（影响二进制下载）。子任务 3 在前端呈现更新中/已回滚/版本不一致状态，是该信息模型的自然扩展，仅新增可选字段，向后兼容。

### 3. 子任务拆分粒度
- ✅ 5 个子任务，每个 1-2 个 commit，符合「一子功能点一 commit」原则。
- ✅ 前后端覆盖：子任务 2/3 同时涉及后端（ws.rs/agent_download.rs/api）与前端（UpdateSection.vue/api），避免前后端分离。

### 4. 版本类型判断
- ✅ minor：新增部署模式 + Agent 自动更新可靠性增强 + 前端状态呈现，均向后兼容，无破坏性变更。版本号 0.87.0 正确。

### 5. 风险与缺口确认
- ✅ **核心缺口成立**：经代码核查（`.github/workflows/ci.yml`、`Dockerfile.hub`、`docker-compose.hub.yaml`、`bin/rex-hub.rs`、`bin/rex-agent.rs`），当前 `CMD ["/app/rex-hub", "--worker"]` 与 compose 直接 `--worker` 启动，supervisor 分支不运行。Docker 下 `exit 10` 与进程崩溃无法被捕获做二进制替换/重启——子任务 1 修复此缺口，方向正确。
- ✅ **CI 打包链路**：`Dockerfile.hub` 已 `COPY dist/agents/ /app/data/agent-binaries/`，子任务 4 确保 CI 产出该目录，使内网无外网仍可分发，与既有 `agent_download.rs` 本地优先查找一致。

## 无需打回

设计方向、产品边界、子任务粒度均合理，无 🔴/🟡 级问题。直接通过。
