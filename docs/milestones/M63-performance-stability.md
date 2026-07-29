# M63: Performance Optimization & Stability

## Context
M62 完成了健康监控和 WebSocket 增强。M0-M62 已实现所有核心功能。本里程碑聚焦性能优化和稳定性提升，为生产环境部署做好准备。

版本类型：minor（性能优化，向后兼容）

## 产品边界
本阶段优化系统性能和稳定性，不新增功能模块。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 性能基准测试与瓶颈分析 | ✅ |
| 2 | 数据库查询优化与索引 | ✅ |
| 3 | 内存使用优化 | ✅ |
| 4 | 并发处理优化 | ✅ |
| 5 | 错误处理与恢复机制增强 | ✅ |
| 6 | 测试覆盖率提升 | ✅ |

## 子任务详细设计

### 1 性能基准测试与瓶颈分析

- **功能目标**：建立性能基准，识别关键瓶颈
- **文件结构**（创建）：
  - `benches/api_benchmarks.rs` — API 性能基准测试
  - `benches/db_benchmarks.rs` — 数据库操作基准测试
  - `docs/milestones/M63-reports/performance-baseline.md` — 性能基线报告
- **接口设计**：使用 criterion.rs 进行 Rust 基准测试
- **测试标准**：建立响应时间、吞吐量、内存使用基线
- **提交信息**：`bench: add performance benchmarks and establish baseline`

### 2 数据库查询优化与索引

- **功能目标**：优化 SQLite 查询性能，添加必要索引
- **文件结构**（修改）：
  - `crates/rex-hub/src/db/migrations/` — 添加性能索引
  - `crates/rex-hub/src/db/queries.rs` — 优化查询逻辑
- **接口设计**：分析慢查询，添加复合索引
- **性能目标**：关键查询 P99 < 100ms
- **提交信息**：`perf(db): add indexes and optimize slow queries`

### 3 内存使用优化

- **功能目标**：减少内存占用，优化大数据集处理
- **文件结构**（修改）：
  - `crates/rex-hub/src/api/` — 优化响应序列化
  - `crates/rex-common/src/` — 优化数据结构
- **优化方向**：
  - 使用流式处理替代全量加载
  - 优化 JSON 序列化/反序列化
  - 减少不必要的内存分配
- **提交信息**：`perf: optimize memory usage with streaming and zero-copy`

### 4 并发处理优化

- **功能目标**：提升并发连接处理能力
- **文件结构**（修改）：
  - `crates/rex-hub/src/server.rs` — 优化连接池
  - `crates/rex-hub/src/ws/` — 优化 WebSocket 并发
- **优化方向**：
  - 数据库连接池调优
  - WebSocket 连接管理优化
  - 异步任务调度优化
- **性能目标**：支持 1000+ 并发连接
- **提交信息**：`perf: optimize concurrency and connection pooling`

### 5 错误处理与恢复机制增强

- **功能目标**：增强系统容错能力和自动恢复
- **文件结构**（修改）：
  - `crates/rex-common/src/error.rs` — 完善错误类型
  - `crates/rex-hub/src/` — 添加重试和恢复逻辑
- **增强内容**：
  - 网络错误自动重试
  - 数据库连接断线重连
  - 优雅降级处理
- **提交信息**：`feat(resilience): add retry and recovery mechanisms`

### 6 测试覆盖率提升

- **功能目标**：将测试覆盖率提升至 90%
- **文件结构**（修改）：
  - `crates/*/src/**/*.rs` — 补充单元测试
  - `tests/` — 补充集成测试
- **测试标准**：cargo llvm-cov 覆盖率 ≥ 90%
- **提交信息**：`test: improve coverage to 90%`

## 设计核对点

- 性能优化不改变功能行为
- 不引入新的外部依赖
- 所有优化都有基准测试验证
- 后端依赖使用 `workspace = true`

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
| [x] | 🟡 | CI build-hub 和 build-agent 可合并避免二次编译 | 用户反馈 | CI 中 build-hub 和 build-agent 分开编译导致重复编译共享 crate，可合并为一次编译 |
