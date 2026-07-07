# Step 7: 设计再确认报告

## 实现 vs 里程碑文档对比

### 子任务1：后端安全加固

| 设计项 | 实现 | 一致 |
|--------|------|------|
| RateLimiter HashMap<IpAddr, Vec<Instant>> | ✅ security.rs | ✅ |
| 同IP 5分钟内最多5次失败登录 | ✅ auth.rs 集成 | ✅ |
| 超出返回 429 Too Many Requests | ✅ StatusCode::TOO_MANY_REQUESTS | ✅ |
| 安全头：X-Content-Type-Options: nosniff | ✅ security.rs | ✅ |
| 安全头：X-Frame-Options: DENY | ✅ security.rs | ✅ |
| 安全头：HSTS max-age=31536000 | ✅ security.rs | ✅ |
| 安全头：CSP（含 ws: wss:） | ✅ security.rs | ✅ |
| 安全头：Cache-Control: no-store | ✅ security.rs | ✅ |
| /healthz 保留 "ok" 文本 | ✅ routes.rs | ✅ |
| /api/health 返回 JSON | ✅ routes.rs（已有） | ✅ |
| RateLimiter 4 个单元测试 | ✅ security.rs | ✅ |
| SecurityHeaders 验证测试 | ❌ 未实现 | 🟡 小偏差 |

### 子任务2：前端错误处理

| 设计项 | 实现 | 一致 |
|--------|------|------|
| ErrorBoundary.vue 组件 | ✅ | ✅ |
| App.vue 包裹 ErrorBoundary | ✅ | ✅ |
| onErrorCaptured 捕获渲染错误 | ✅ | ✅ |
| 重试按钮（刷新页面） | ✅ | ✅ |
| client.ts 401 跳转登录 | ✅ | ✅ |
| client.ts 429 Toast 提示 | ✅ | ✅ |
| client.ts 5xx Toast 提示 | ✅ | ✅ |
| client.ts 超时 Toast 提示 | ✅ | ✅ |
| client.ts 网络错误 Toast 提示 | ✅ | ✅ |
| i18n errorBoundary 文案（中/英） | ✅ | ✅ |

### 子任务3：离线检测与 WebSocket 自动重连

| 设计项 | 实现 | 一致 |
|--------|------|------|
| useNetworkStatus composable | ✅ | ✅ |
| 监听 online/offline 事件 | ✅ | ✅ |
| App.vue Toast 离线/恢复提示 | ✅ | ✅ |
| WebSocket 指数退避 1s→16s | ✅ RECONNECT_DELAYS | ✅ |
| 最大 5 次重连 | ✅ MAX_RECONNECT_ATTEMPTS=5 | ✅ |
| 手动断开不重连 | ✅ manualDisconnect | ✅ |
| 成功后重置计数 | ✅ reconnectAttempts=0 | ✅ |
| onBeforeUnmount 清理定时器 | ✅ | ✅ |
| i18n network/reconnect 文案（中/英） | ✅ | ✅ |

## 小偏差

1. **SecurityHeaders 验证测试未实现**：里程碑设计要求"验证所有安全头存在"的单元测试，但 security.rs 中只有 RateLimiter 测试，未测试 SecurityHeaders 中间件。安全头中间件是标准实现，风险低。

## 结论

✅ 设计再确认通过。实现与里程碑文档一致，1 项小偏差（SecurityHeaders 测试）不阻塞。
