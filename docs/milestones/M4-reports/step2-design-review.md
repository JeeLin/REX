# Step 2: Design Review — M4 SQL 控制台

## 审查维度

### 1. 产品定位一致性
- ✅ 单用户、自托管，无多用户/RBAC 概念引入
- ✅ M4 数据库控制台与 PRODUCT.md §3.7 SQL 控制台定义一致
- ✅ 对标 Navicat 的两栏布局、导航树、查询编辑器、结果网格均在设计中体现

### 2. 架构一致性
- ✅ 复用 Rust workspace crate 结构（rex-mysql/rex-postgresql/rex-sqlite/rex-common）
- ✅ `SqlConnector` trait 放在 rex-common，三库 crate 各自实现，符合共享 crate 架构
- ✅ REST + WebSocket 端点在 rex-hub 注册，与现有 `/ws/terminal` 路由模式一致
- ✅ 连接池 `HashMap<sessionId, Box<dyn SqlConnector>>` 符合单用户场景

### 3. API 设计规范
- ✅ REST 端点命名遵循 `/api/sql/*` 前缀规范
- ✅ 错误响应格式与 api-design.md 统一（含 code/message/position）
- ✅ WebSocket 消息协议使用 JSON + type tag，与 M3 terminal 协议风格一致

### 4. 产品边界
- ✅ "做"范围合理：后端协议 + 前端基础查询控制台
- ✅ "不做"明确：内联编辑、表设计器、DDL 抽屉、AI 助手归入 M4b
- ✅ 未引入超出 PRODUCT.md 定义的功能

### 5. 子任务拆分
- ✅ 6 个子任务，每个可独立完成并提交
- ✅ 后端（子任务1）+ 前端（子任务2-5）+ 收尾（子任务6），边界清晰
- ✅ 子任务间依赖合理：1→2→3→4→5 顺序执行

### 6. 版本号
- ✅ 0.5.0（minor 递增，从 0.4.0），与新功能里程碑一致

## 小问题（已直接修正）

无。

## 结论

✅ 里程碑文档设计合理，与产品文档和架构规范一致，可以进入步骤 3（开发）。
