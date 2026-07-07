# Step 2: 设计核对报告

## 审查维度

| 维度 | 结果 | 说明 |
|------|------|------|
| 产品定位 | ✅ | 单用户自托管，未引入多用户/RBAC |
| 架构一致性 | ✅ | 安全中间件集成在 axum Router 层，不改变 supervisor+worker 模型 |
| 文件传输 | ✅ | 不涉及文件传输逻辑 |
| 跳阶段实现 | ✅ | 安全加固和错误处理是合理的质量提升，不跳阶段 |
| 产品文档污染 | ✅ | 不修改 PRODUCT.md |
| 子任务拆分 | ✅ | 3 个子任务粒度合理，每个 1-2 个 commit |
| 测试覆盖 | ✅ | 每个子任务有明确测试标准 |
| 依赖规则 | ✅ | 不引入新的外部 crate 依赖 |
| CSP 兼容性 | ✅ | 允许 unsafe-inline 以兼容现有内联样式 |

## 详细审查

### 子任务 1：后端安全加固
- 登录限流用内存 HashMap，适合单用户场景，不引入 Redis ✅
- 安全头（CSP/HSTS/X-Frame-Options）是标准安全实践 ✅
- healthz 增强返回 JSON，向后兼容（原返回 "ok" 文本，需确认是否影响现有监控）— **小问题**：建议保持原 `/healthz` 返回 "ok" 文本，新增 `/api/health` 返回 JSON
- RateLimiter 需要考虑并发安全（`Arc<Mutex<HashMap>>` 或 `RwLock`）✅ 已在设计中提及

### 子任务 2：前端错误处理
- ErrorBoundary 是 Vue 3 标准错误处理模式 ✅
- API 拦截器增强不改变现有 401 处理逻辑 ✅
- Toast 提示使用现有 useToast composable ✅

### 子任务 3：离线检测与 WebSocket 重连
- useNetworkStatus 使用浏览器原生 API，无外部依赖 ✅
- 指数退避策略（1s→16s，最多 5 次）合理 ✅
- 保留现有手动重连 UI，自动重连是增强而非替代 ✅

## 小问题修正

healthz 端点设计需调整：原 `/healthz` 返回纯文本 "ok"，被 Docker/K8s 健康检查使用。改为 JSON 会破坏现有部署。建议：
- 保留 `/healthz` 返回 "ok"（向后兼容）
- 新增 `/api/health` 返回详细 JSON

已自动修正里程碑文档。

## 结论

✅ 设计核对通过。1 个小问题已修正。
