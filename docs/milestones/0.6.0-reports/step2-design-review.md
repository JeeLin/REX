# 0.6.0 设计核对报告

## 审查日期

2026-06-22

## 审查维度

### 1. 产品定位一致性 ✅

- 单用户、自托管定位符合
- 未引入多用户、RBAC、企业协作等概念
- 深色优先设计原则未受影响

### 2. 架构一致性 ✅

- 单二进制 + supervisor + worker 模型未改变
- Hub 和 Agent 保持相同的运行时模型
- TLS 实现不影响进程模型

### 3. 文件传输边界 ✅

- 二进制下载是后端直接提供文件流
- 文件数据不经过浏览器中转
- 符合 PRODUCT.md 中"文件传输数据不经过浏览器"原则

### 4. 通信协议 ✅

- PRODUCT.md 定义的协议层次：
  - 控制台 ↔ 服务端：HTTPS（REST API）+ WebSocket（终端数据流）
  - Agent ↔ 服务端：WebSocket (TLS)
- 0.6.0 实现 Hub 原生 HTTPS，符合协议设计
- Agent 已支持 wss:// 连接（reqwest + tokio-tungstenite 均有 rustls-tls）

### 5. 功能边界 ✅

- 未跳阶段实现（TLS 和下载端点是基础设施层功能）
- 未引入不该有的概念
- 未修改 PRODUCT.md

### 6. 设计合理性 ✅

- TLS 配置优先级（CLI > 环境变量 > 配置文件）合理
- Agent 下载端点需要认证（protected route）符合安全要求
- Agent 更新源配置（hub | github）保留灵活性
- Docker 镜像 TLS 支持通过环境变量和卷挂载实现

## 修正记录

审查中发现并修正了以下小问题：

1. **子任务2 文件结构路径错误**
   - 原：`docs/milestones/0.6.0-reports/agent-binaries/`
   - 修正为：`data/agent-binaries/`
   - 原因：Agent 二进制应放在数据目录，而非里程碑报告目录

2. **子任务2 未说明认证要求**
   - 补充：下载端点需要 Bearer token 认证（protected route）
   - 原因：防止未授权访问 Agent 二进制

3. **子任务3 表述不准确**
   - 原："替代 GitHub Releases 直接下载"
   - 修正为："新增选项，适用于内网环境"
   - 原因：Hub 下载是补充方案，不是替代方案

## 结论

✅ 里程碑文档设计合理，符合产品定位和架构原则，可以进入开发阶段。
