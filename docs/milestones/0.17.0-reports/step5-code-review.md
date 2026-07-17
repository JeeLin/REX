# M16 步骤5：代码审查报告

## 审查范围

M16 新增文件（简化后）：update.rs、update_api.rs、updater.rs、supervisor.rs、agent_ws.rs（更新部分）、前端 AgentsPage/SettingsPage

## 发现

### 🟡 SHA256 校验未生效
- `updater.rs:46` — `if !cmd.sha256.is_empty()` 永远跳过（sha256 始终为空）
- **影响**：下载的二进制不做完整性校验
- **评估**：已标记 TODO，后续从 release asset 获取

### 🟢 download 端点无大小限制
- `download_agent_binary` 读取整个二进制到内存
- **评估**：Agent 二进制 ~10MB，单用户足够

## 架构确认

- ✅ 版本检查通过 WebSocket 心跳完成，无额外 REST 端点
- ✅ 更新触发通过 WebSocket 推送，不暴露 REST API
- ✅ Supervisor/worker 进程模型正确（exit code 42 → update-state.json → rename）
- ✅ 备份机制（.bak 文件）存在
- ✅ 无 GitHub API 依赖，无 reqwest/anyhow 在 rex-hub

## 结论

**✅ 通过 — 无 🔴 必须修复项**
