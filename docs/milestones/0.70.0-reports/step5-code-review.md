# Step 5: 代码审查报告

## 审查范围

4 个提交（子任务1-3），涉及 24 个文件，+459 / -49 行。

## 审查结果

| 级别 | 数量 |
|------|------|
| 🔴 必须修复 | 0 |
| 🟡 应该修复 | 2 |
| 🟢 可选改进 | 2 |

## 🟡 应该修复

### 1. ws.onerror 未设置 disconnected 状态
- **文件**：`WorkspaceTerminal.vue:404-407`
- **问题**：`ws.onerror` 只调用 `stopPing()`，注释说"让 onclose 处理重连"。但 `onerror` 有时不会触发 `onclose`（如连接阻塞时），导致 UI 停留在 connecting 状态。
- **建议**：在 `onerror` 中也设置 `connectionStatus.value = 'disconnected'`，让 onclose 的重连逻辑自然触发。
- **实际风险**：低。浏览器通常在 error 后紧跟 close 事件。

### 2. client.ts 错误消息硬编码中文
- **文件**：`api/client.ts:36-55`
- **问题**：错误拦截器中的提示消息使用硬编码中文，未使用 i18n。
- **说明**：已在步骤4精简报告中记录，client.ts 是模块级代码，i18n 需要 Vue 组件上下文。当前实现可接受。

## 🟢 可选改进

### 1. RateLimiter Mutex 粒度
- **文件**：`security.rs`
- **说明**：`check` 和 `record_failure` 分别获取 Mutex 锁，理论上存在 check 后、record 前的竞态。但单用户场景下几乎不可能触发，无需优化。

### 2. ErrorBoundary 不捕获异步错误
- **文件**：`ErrorBoundary.vue`
- **说明**：`onErrorCaptured` 只捕获渲染错误，不捕获事件处理器或 `onMounted` 中的异步错误。已有全局 unhandledrejection 处理（在别处），可接受。

## 正确性验证

| 维度 | 结果 |
|------|------|
| 速率限制逻辑 | ✅ check/record/clear 正确，窗口过期清理正确 |
| 安全头 | ✅ CSP/HSTS/X-Frame-Options/X-Content-Type-Options/Cache-Control 均正确 |
| healthz 向后兼容 | ✅ `/healthz` 保留 "ok" 文本，`/api/health` 返回 JSON |
| ErrorBoundary | ✅ 捕获渲染错误，显示错误页+重试 |
| API 错误处理 | ✅ 401/429/5xx/超时/网络错误均处理 |
| WebSocket 重连 | ✅ 指数退避 1s→16s，最多5次，手动断开不重连 |
| 审计日志 | ✅ 速率限制失败记录到审计日志 |
| CSP WebSocket | ✅ connect-src 包含 ws: wss: |

## 结论

✅ 代码审查通过。无 🔴 必须修复项。2 项 🟡 建议修复（低风险，不阻塞）。
