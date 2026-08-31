# M39 设计核对报告

## 审查维度

### 1. 产品定位一致性

| 检查项 | 结果 |
|--------|------|
| 单用户模型 | ✅ session timeout 为单用户闲置检测，不涉及多用户/会话管理 |
| 自托管 | ✅ 不引入外部服务依赖 |
| 深色优先 | ✅ CommandPalette 和设置页遵循深色主题 |
| 不引入 RBAC/多用户 | ✅ 无权限/角色概念 |

### 2. 架构一致性

| 检查项 | 结果 |
|--------|------|
| 单二进制 + supervisor + worker | ✅ 不改变进程模型 |
| 前端 bun 命令 | ✅ 所有前端命令使用 bun |
| workspace 依赖规则 | ✅ 不涉及新 Rust 依赖 |
| 文件传输不经过浏览器 | ✅ 本里程碑不涉及文件传输 |

### 3. 功能边界

| 检查项 | 结果 |
|--------|------|
| 不做文档外大功能 | ✅ 4 个子任务均在 milestone 文档范围内 |
| CommandPalette 真实数据 | ✅ 使用 environments store，不引入新数据源 |
| 后端 settings API 已存在 | ✅ 不需要新增后端 API 端点 |
| session timeout 为前端逻辑 | ✅ 前端 idle 检测 + 自动登出，不需要后端新端点 |

### 4. 子任务拆分粒度

| 子任务 | 粒度评估 |
|--------|----------|
| 1. CommandPalette 对接真实数据 | ✅ 合理，1-2 个 commit |
| 2. session timeout composable | ✅ 合理，独立可测试模块 |
| 3. session timeout 集成 | ✅ 合理，路由守卫 + 设置页 |
| 4. 前端设置同步后端 | ✅ 合理，1 个新 store + API 模块 |

### 5. 与前序里程碑一致性

- M38（测试覆盖）后进行集成完善 → 符合「先测试后优化」的开发节奏
- CommandPalette 在 M2 创建时为 mock 数据，M39 对接真实数据 → 合理的技术债务清理
- Settings localStorage 持久化在 M14 实现，M39 补充后端同步 → 合理的渐进增强

### 6. 潜在风险

| 风险 | 评估 |
|------|------|
| session timeout 与 WebSocket 终端冲突 | 🟡 用户在终端操作时鼠标/键盘事件会重置 idle 计时器，不会误触发。安全。 |
| settings API 后端不可用时的降级 | ✅ 设计中已考虑 localStorage fallback |
| CommandPalette 大量资源时的性能 | 🟡 资源数量通常 <100，computed + filter 足够，无需虚拟滚动 |

## 结论

✅ 设计合理，与产品文档和架构一致，无大问题。子任务拆分粒度适当，无跨里程碑功能泄露。
