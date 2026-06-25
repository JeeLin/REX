# Step 5: 代码审查报告

## 审查范围

全局查询功能新增/修改的全部文件。

---

## 🔴 必须修复

### 1. 前端 EventSource 不支持 POST 请求 ✅ 已修复

**文件**：`useGlobalQuery.ts`

**修复**：改用 `fetch` + `ReadableStream` 手动解析 SSE 流，支持 POST 请求。

### 2. 前端 SSE 数据格式与后端序列化不匹配 ✅ 已修复

**修复**：
- 后端使用 `#[serde(tag = "type", content = "data")]`，前端 switch 使用大写 `Start`/`Result`/`Progress`/`Done`/`Error` 匹配
- 前端正确访问嵌套的 `event.data.connection_id`（camelCase/snake_case 均处理）
- 使用后端返回的 `columns` 字段（优先于 `Object.keys`）

### 3. `GlobalQueryModal.vue` 的 `onClose` prop 不安全 ✅ 已修复

**修复**：移除 `onClose` prop，改用 `defineEmits` 的 `update:visible` 模式。

### 4. `useGlobalQuery` 参数类型不匹配 ✅ 已修复

**修复**：`useGlobalQuery` 接受普通数组 `DatabaseInfo[]` 而非 `Ref<DatabaseInfo[]>`，参数接口改为 `SqlResource` 类型。

---

## 🟡 应该修复

### 5. `switch` 语句 `case 'start'` 缺少 `break` ✅ 已修复

**修复**：每个 `case` 后添加 `break`，移除无用的注释。

### 6. 取消功能未实现 ✅ 已修复

**修复**：使用 `AbortController` 支持取消 fetch 请求，取消时中止 HTTP 连接。

### 7. `send_event` 中 `.unwrap()` 可能 panic

**状态**：保留。当前所有 `GlobalQueryEvent` 变体都是可序列化的，panic 风险极低。可在后续迭代中改为 `.unwrap_or_default()`。

### 8. `global_query` 路由放置在认证中间件之外 ✅ 已修复

**修复**：改用 fetch POST + Bearer token 认证，不再使用 EventSource。

---

## 🟢 可选改进

### 9. SSE stream unfold 闭包中未使用的 `Some(Err(e))` 分支

**状态**：保留。`rx.recv()` 的 `RecvError` 在通道关闭时返回 `None`，实际不会触发 `Some(Err)` 分支。

### 10. `result` 对象中 columns 应该来自后端而非 Object.keys ✅ 已修复

**修复**：前端优先使用 `data.columns`，仅在无后端 columns 时 fallback 到 `Object.keys`。

---

## 额外修复

### 11. API 重新设计：资源 ID 替代原始连接详情 ✅ 已修复

**问题**：前端传数据库连接详情（含密码）不安全。

**修复**：
- 后端 `GlobalQueryRequest` 改为接受 `resource_ids: Vec<String>`
- 后端从数据库加载资源配置，解析连接参数
- 新增 `GET /api/resources/:resource_id/sql/peers` 端点获取同环境 SQL 资源列表
- 前端 modal 改为选择资源（resource），不涉及密码

### 12. `ResourceNew.vue` TypeScript 类型错误 ✅ 已修复

**修复**：修正 axios 泛型类型，正确匹配 `ApiResponse` 嵌套结构。

### 13. `isCompatible` 从 ref 改为函数 ✅ 已修复

**修复**：移除 `isCompatible` ref，模板直接使用 `checkCompatibility` 函数。

---

## 总结

| 严重度 | 数量 | 状态 |
|--------|------|------|
| 🔴 必须修复 | 4 | ✅ 全部修复 |
| 🟡 应该修复 | 4 | ✅ 3 项修复，1 项保留 |
| 🟢 可选改进 | 2 | ✅ 1 项修复，1 项保留 |

**结论**：🔴 必须修复项全部修复，**通过**。
