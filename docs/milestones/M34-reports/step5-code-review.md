# M34 Step 5: Code Review Report

## 审查范围

M34 文件编辑器 + 连接导入导出 + SSH Keepalive 全部变更。

## 审查维度

### 1. 正确性

| 检查项 | 结论 | 说明 |
|--------|------|------|
| read_for_edit / save_from_edit | ✅ | SFTP/S3 实现正确，5MB 限制生效 |
| base64 编码/解码 | ✅ | 前端 atob/btoa + unescape/encodeURIComponent 正确处理 UTF-8 |
| CodeMirror 语言检测 | ✅ | LANG_MAP 覆盖常用语言，fallback 到纯文本 |
| CodeMirror 清理 | ✅ | watch(visible) + onBeforeUnmount 双重清理 |
| export_environments | ✅ | 正确导出所有环境和资源，config_json 保留加密内容 |
| import_environments 去重 | ✅ | 按环境名去重，已存在则 skip |
| SSH keepalive | ✅ | 在 client::Config 设置 keepalive_interval，连接前生效 |

### 2. 安全性

| 检查项 | 结论 | 说明 |
|--------|------|------|
| 导出密码处理 | ✅ | config_json 已加密，不泄露明文密码 |
| 文件大小限制 | ✅ | read_for_edit 限制 5MB |
| import 无注入风险 | ✅ | 使用参数化查询（rusqlite） |

### 3. 错误处理

| 检查项 | 结论 | 说明 |
|--------|------|------|
| import 资源创建失败 | 🟡 | `let _ = spawn_blocking(...)` 静默丢弃错误，环境被标记为 imported 但资源可能缺失 |
| export 数据库错误 | ✅ | 使用 `map_err` 正确传播为 500 |
| CodeMirror 初始化错误 | ✅ | loadFile 有 try/catch |

### 4. 性能

| 检查项 | 结论 | 说明 |
|--------|------|------|
| export 循环 spawn_blocking | 🟡 | 每个环境单独 spawn_blocking，大批量导出时效率不高 |

## 发现汇总

| 严重程度 | 问题 | 说明 |
|----------|------|------|
| 🟡 | import_environments 静默忽略资源创建失败 | `let _ =` 丢弃错误，用户无法知道哪些资源导入失败 |
| 🟡 | export_environments 循环 spawn_blocking | 大批量导出时每个环境单独 blocking call，可合并为一个 |
| 🟢 | 里程碑文档写 Monaco Editor，实际用 CodeMirror | 设计偏差，CodeMirror 更轻量，可接受 |
| 🟢 | atob/btoa UTF-8 workaround | `unescape(encodeURIComponent(...))` 是已知的 UTF-8 兼容方案，可用 TextDecoder 替代 |

## 修复项

### 🟡 import 资源创建失败应记录错误

```rust
// 当前（静默丢弃）:
let _ = tokio::task::spawn_blocking(move || db.create_resource(&env_id, &new_res)).await;

// 建议: 至少 log warning
let result = tokio::task::spawn_blocking(move || db.create_resource(&env_id, &new_res)).await;
if let Err(e) = result { tracing::warn!("import resource failed: {e}"); }
```

## 结论

无 🔴 必须修复项。2 个 🟡 应该修复，2 个 🟢 可选改进。

**结论：✅ 通过**（无 🔴 阻断项）
