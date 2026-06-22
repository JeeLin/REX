# 0.6.0 代码精简报告

## 审查日期

2026-06-22

## 变更文件

- `crates/rex-hub/src/tls.rs` — TLS 证书加载 + HTTPS 服务器
- `crates/rex-hub/src/agent_download.rs` — Agent 二进制下载端点
- `crates/rex-hub/src/config.rs` — HubConfig 增加 TLS 字段
- `crates/rex-hub/src/bin/rex-hub.rs` — 条件 TLS 启动
- `crates/rex-common/src/cli.rs` — CLI 增加 --tls-cert/--tls-key
- `crates/rex-common/src/updater.rs` — Hub 下载源 + download_from_hub
- `crates/rex-agent/src/config.rs` — update_source 配置
- `Dockerfile.hub` — 证书目录 + 443 端口
- `docker-compose.hub.yaml` — TLS 环境变量和卷挂载

## 精简检查

### 1. 重复代码 ✅ 无需修改

- `agent_download.rs` 的 os/arch 验证模式相似，但只有两处调用，提取辅助函数会增加间接层
- `updater.rs` 的 `download_from_hub` 和 `download_update` 逻辑不同（GitHub API vs Hub 直接下载），不适合合并

### 2. 过度设计 ✅ 无

- TLS 模块职责单一：证书加载 + 服务器运行
- Agent 下载端点职责单一：参数验证 + 文件服务
- 配置结构清晰：TlsConfig 嵌套在 HubConfig 中，UpdateConfig 嵌套在 AgentConfig 中

### 3. 提前实现 ✅ 无

- 所有功能严格在里程碑文档范围内
- 未引入 ACME/自动证书（已确认放到下个版本）
- 未引入证书热加载

### 4. 代码组织 ✅ 符合项目风格

- 新文件遵循现有模块结构
- 测试放在 `#[cfg(test)] mod tests` 中
- 依赖通过 workspace = true 管理

### 5. 防御性编程

- `agent_download.rs` 的路径遍历检查（`starts_with`）技术上冗余（文件名从已验证输入构造），但作为安全最佳实践保留

## 结论

代码已经足够精简，无需修改。功能行为不变。
