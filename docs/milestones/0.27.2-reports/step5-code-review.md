# 步骤5：代码审查报告

## 审查范围

0.27.2 里程碑全部 5 个子任务的代码变更（18 个文件，+947/-69 行）。

## 审查维度

### 正确性

- ✅ `tls_client.rs`：`InsecureServerNameVerifier` 正确实现 `ServerCertVerifier` 的所有必需方法（`verify_tls12_signature`、`verify_tls13_signature`、`supported_verify_schemes`）
- ✅ `acme.rs` 重试逻辑正确：最多 3 次，每次重建 state，5 秒间隔
- ✅ `settings.rs` ACME 状态读取使用 `read().await`，不会阻塞写入
- ✅ 步骤4 已修复 `EventOk` 枚举匹配（原 `GotCertificate` 字符串匹配永远不会命中）
- ✅ `config.rs` 配置优先级正确：CLI > 环境变量 > 配置文件

### 安全性

- ✅ `--insecure` 使用 `danger_accept_invalid_certs` + 自定义 `ServerCertVerifier`，仅影响 TLS 层
- ✅ `--insecure` 启动时输出 `tracing::warn!` 警告
- ✅ CA 证书文件不存在时返回明确错误（`failed to read CA certificate`）
- ✅ ACME 状态不泄露敏感信息（仅 status/error 字符串）

### 架构一致性

- ✅ `AgentTlsConfig` 作为 `AgentConfig` 的子结构，遵循现有配置嵌套模式
- ✅ `SharedAcmeStatus` 使用 `Arc<RwLock<>>` 模式，与项目其他共享状态一致
- ✅ TLS 状态 API 字段使用 `skip_serializing_if = "Option::is_none"` 避免 JSON 冗余

### 错误处理

- ✅ 所有 `build_*` 函数返回 `Result<T>` 并附带 `anyhow::Context`
- ✅ ACME 驱动失败时更新共享状态而非静默吞错
- ✅ HTTP-01 challenge server 绑定失败不阻止 Hub 启动（`tracing::error` + continue）

### 测试覆盖

- ✅ `tls_client.rs`：12 个测试覆盖 default/insecure/CA/empty PEM 场景
- ✅ `config.rs`：新增 5 个测试覆盖 http_port 的 default/env/CLI/invalid 场景
- ✅ `acme.rs`：新增 3 个测试覆盖 AcmeStatus 默认/序列化/错误
- ✅ `settings.rs`：新增 6 个测试覆盖 ACME 状态序列化/shared_acme_status 读写
- 🟡 `build_reqwest_client_with_valid_ca` 测试生成了证书但未实际测试 CA 信任链（可改进）

### 与里程碑文档一致性

- ✅ 子任务 1-5 全部实现，功能与详细设计一致
- ✅ CLI 参数（`--ca-cert`、`--insecure`、`--acme-http-port`）与设计匹配
- ✅ 环境变量（`REX_CA_CERT`、`REX_INSECURE`、`REX_ACME_HTTP_PORT`）与设计匹配

## 发现

### 🟡 应该修复

无。

### 🟢 可选改进

1. **`build_reqwest_client_with_valid_ca` 测试**（`tls_client.rs`）：生成了 CA 证书但未用它构建客户端验证信任链。可改用 `build_reqwest_client(Some(&cert_path), false)` 加载自签名 CA 并验证构建成功。

2. **`run_agent` 中 HTTP client 重建**（`bin/rex-agent.rs`）：`HubClient` 消费了 `http_client`，之后又 `build_reqwest_client` 重建一个用于 WS。可考虑传工厂函数避免重复构建，但当前实现简洁明了，不阻塞。

3. **`ws_connector: Option<Connector>`**（`ws.rs`）：`AgentWs::new()` 传 `None`，`with_tls()` 传 `Some`。`Option` 表示"可能没有自定义 TLS"，语义合理，但也可考虑拆分为两个类型。当前方式简洁，不阻塞。

## 结论

✅ **无 🔴 必须修复项。** 所有发现均为 🟢 可选改进，不影响功能正确性和安全性。代码审查通过。
