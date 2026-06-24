# 设计再确认报告 — 0.15.0 跨连接文件传输

## 确认维度

### 子任务1：传输执行引擎
- ✅ `executor.rs` 实现了 `execute_transfer` 函数，签名与文档一致
- ✅ 状态流转：`Pending → Running → Completed/Failed`
- ✅ 全量读写模式（`source.read()` → `target.write()`）
- ✅ 测试覆盖：成功传输、源文件不存在、空文件、进度更新（4 个测试）

### 子任务2：连接器池 + 端点解析
- ✅ `resolve_connector` 实现了 local 和 sftp 两种类型
- ✅ `local` 类型使用 `LocalConnector::new("/")` 支持任意路径
- ✅ `sftp` 类型通过 `resource_id` 从数据库查找凭据并建立 SSH 连接
- ⚠️ 已知偏差：SFTP 未复用 WebSocket 连接（每次新建 SSH 连接），已在步骤5报告中记录

### 子任务3：集成到 API 路由
- ✅ `create_transfer` 中验证端点 → 解析 connector → 创建任务 → `tokio::spawn` 执行
- ✅ 无 API 变更
- ✅ 新增 JoinHandle panic 监控
- ✅ 测试覆盖：`create_local_to_local_transfer` 验证端到端传输

### 产品语义
- ✅ 传输数据不经过浏览器
- ✅ 单用户、无 RBAC
- ✅ 不引入新概念

## 偏差记录

| 偏差 | 影响 | 处理 |
|------|------|------|
| SFTP 新建连接而非复用 WebSocket | 每次传输多一次 SSH 握手 | 可接受，后续优化 |
| `LocalConnector` 使用 `/` 作为 base_path | 可访问任意本地路径 | 设计意图正确 |

## 结论

✅ 通过 — 实现与里程碑文档一致，产品语义正确
