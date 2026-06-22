# 0.6.0 设计再确认报告

## 确认日期

2026-06-22

## 确认维度

### 1. 子任务实现完整性 ✅

| 子任务 | 设计要求 | 实现状态 |
|--------|----------|----------|
| 1 Hub TLS | HubConfig + TLS 字段 | ✅ |
| 1 Hub TLS | --tls-cert / --tls-key CLI | ✅ |
| 1 Hub TLS | REX_TLS_CERT / REX_TLS_KEY 环境变量 | ✅ |
| 1 Hub TLS | 优先级 CLI > env > config | ✅ |
| 1 Hub TLS | 证书加载 + HTTPS 服务器 | ✅ |
| 1 Hub TLS | 未配置时保持 HTTP | ✅ |
| 2 下载端点 | GET /api/agent/download?os=&arch= | ✅ |
| 2 下载端点 | Bearer token 认证 | ✅ |
| 2 下载端点 | os/arch 参数验证 | ✅ |
| 2 下载端点 | 二进制流 + 响应头 | ✅ |
| 2 下载端点 | 404 平台不存在 | ✅ |
| 2 下载端点 | 路径遍历防护 | ✅ |
| 3 Agent 更新 | UpdateSource 枚举 | ✅ |
| 3 Agent 更新 | update.source 配置 | ✅ |
| 3 Agent 更新 | REX_UPDATE_SOURCE 环境变量 | ✅ |
| 3 Agent 更新 | download_from_hub + SHA256 校验 | ✅ |
| 4 Docker | 443 端口 + 证书目录 | ✅ |
| 4 Docker | TLS 环境变量和卷挂载 | ✅ |

### 2. 产品边界一致性 ✅

- 单用户、自托管定位未改变
- 未引入多用户、RBAC、企业协作等概念
- 文件传输数据不经过浏览器（二进制下载是后端直传）
- TLS 实现符合单用户场景（无需 ACME/自动证书）

### 3. 架构一致性 ✅

- 单二进制 + supervisor + worker 模型未改变
- Hub 和 Agent 保持相同的运行时模型
- TLS 实现不影响进程模型
- 新模块遵循现有 crate 结构

### 4. 不做什么核对 ✅

- 不实现 ACME/自动证书 → 未引入
- 不实现证书热加载 → 未引入
- 不实现 Hub 自动从 GitHub 下载 Agent 二进制 → 未引入
- 不修改 WebSocket 连接协议 → 未修改

## 结论

✅ 实现与里程碑文档完全一致，产品语义未变，用户可见行为符合设计。
