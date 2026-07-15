# 0.6.0: M5 Redis 控制台

## Context
M0 骨架 → M1 设计系统 → M2 工作空间外壳 → M3 SSH 终端 → M4 数据库控制台。M5 在 M2 工作空间内接入 Redis 管理控制台，复用 M4 的 SQL 控制台架构模式（后端连接器 + REST API + 前端双面板）。

前序：M4 数据库控制台（SqlConnector trait 模式已验证，M5 复用相同架构思路到 Redis）。
后续：M6 文件管理。

版本类型：minor

## 产品边界
- **做**：后端 Redis 连接器（AUTH/SELECT/INFO/SCAN/各类型操作）、REST API、前端连接+键树、值查看器（FormatViewer + 集合表格编辑器）、CLI、Server Status、批量操作
- **不做**：导入导出向导（M6 或后续）、Redis Cluster 集群模式、Redis Sentinel

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 后端 Redis 连接器（连接/认证/DB 选择/SCAN/各类型 GET/SET/INFO） | ✅ |
| 2 | 前端连接+键树（虚拟滚动、命名空间分组、SCAN 分页、搜索、右键） | ✅ |
| 3 | 值查看器（FormatViewer + 集合表格编辑器：String/Hash/List/Set/ZSet/Stream） | ✅ |
| 4 | CLI（命令输入 + 结果日志） | ✅ |
| 5 | Server Status 卡片仪表盘 | ✅ |
| 6 | 批量操作（删除/TTL） | ✅ |
| 7 | 测试与收尾 | ✅ |

## 子任务详细设计

### 1 后端 Redis 连接器

- **功能目标**：rex-hub 提供统一 Redis 连接器，支持连接、认证、DB 选择、SCAN 遍历、各数据类型读写、INFO 获取
- **文件结构**：
  - `crates/rex-redis/Cargo.toml`（修改：添加 redis 依赖）
  - `crates/rex-redis/src/lib.rs`（实现：RedisConnector）
  - `crates/rex-common/src/redis.rs`（新增：RedisConnector trait + 共享类型）
  - `crates/rex-hub/src/redis_api.rs`（新增：REST 路由）
  - `crates/rex-hub/src/bin/rex-hub.rs`（修改：注册 `/api/redis/*` 路由）
- **接口设计**：
  ```rust
  #[async_trait]
  pub trait RedisConnector: Send + Sync {
      async fn info(&mut self) -> Result<RedisInfo>;
      async fn dbs(&mut self) -> Result<Vec<DbInfo>>;
      async fn select_db(&mut self, db: i32) -> Result<()>;
      async fn scan(&mut self, pattern: &str, count: u32) -> Result<Vec<KeyInfo>>;
      async fn get_type(&mut self, key: &str) -> Result<String>;
      async fn get_value(&mut self, key: &str) -> Result<RedisValue>;
      async fn set_value(&mut self, key: &str, value: &str) -> Result<()>;
      async fn del(&mut self, keys: &[String]) -> Result<u64>;
      async fn ttl(&mut self, key: &str) -> Result<i64>;
      async fn set_ttl(&mut self, key: &str, seconds: i64) -> Result<()>;
      async fn close(&mut self) -> Result<()>;
  }

  pub struct DbInfo { pub index: i32, pub keys: u64, pub expires: u64 }
  pub struct KeyInfo { pub key: String, pub type_name: String }
  pub enum RedisValue {
      String(String),
      List(Vec<String>),
      Set(Vec<String>),
      ZSet(Vec<(String, f64)>),
      Hash(Vec<(String, String)>),
  }
  ```
  REST API：
  ```http
  POST /api/redis/connect       # 建立连接
  POST /api/redis/disconnect     # 断开连接
  GET  /api/redis/databases      # DB 列表（含键数）
  POST /api/redis/select         # 切换 DB
  GET  /api/redis/scan           # SCAN 遍历键
  GET  /api/redis/key?type=xxx   # 获取键值
  POST /api/redis/set            # 设置键值
  POST /api/redis/del            # 删除键
  GET  /api/redis/ttl?key=xxx    # 获取 TTL
  POST /api/redis/ttl            # 设置 TTL
  GET  /api/redis/info           # Server INFO
  POST /api/redis/command        # 执行任意命令（CLI 用）
  ```
- **测试标准**：`cargo build`、`cargo clippy` 通过
- **提交**：`feat(redis): add Redis connector trait and implementation`

### 2 前端连接+键树

- **功能目标**：Redis 控制台左侧键树面板，支持虚拟滚动、命名空间分组、SCAN 分页、搜索、多选、右键菜单
- **文件结构**：
  - `src/features/redis/RedisPage.vue`（新增：双面板布局）
  - `src/features/redis/RedisKeyTree.vue`（新增：键树组件）
  - `src/features/redis/useRedisKeyTree.ts`（新增：键树数据 composable）
  - `src/api/redis.ts`（新增：Redis API 封装）
- **交互设计**：
  - 连接列表：显示连接状态、DB 选择器（每个 DB 显示键数）
  - 键树：按分隔符（默认 `:`）分命名空间文件夹，文件夹显示 `(count)` 徽章
  - 虚拟滚动：支持大量键
  - SCAN 流式分页：滚动到底自动加载下一批
  - 搜索：精确匹配过滤
  - 多选：Shift 范围选择
  - 右键：复制/删除/新 Tab 打开/内存分析
- **测试标准**：`bun run type-check && bun run lint && bun run build` 通过
- **提交**：`feat(web): add Redis key tree with virtual scroll and namespace grouping`

### 3 值查看器

- **功能目标**：右侧固定值查看器，通用 FormatViewer + 集合类型表格编辑器
- **文件结构**：
  - `src/features/redis/RedisValueViewer.vue`（新增）
  - `src/features/redis/RedisStringView.vue`（新增：String 查看/编辑）
  - `src/features/redis/RedisHashView.vue`（新增：Hash 表格）
  - `src/features/redis/RedisListView.vue`（新增：List 表格）
  - `src/features/redis/RedisSetView.vue`（新增：Set 表格）
  - `src/features/redis/RedisZSetView.vue`（新增：ZSet 表格）
- **交互设计**：
  - 键名 + TTL + 删除 + 刷新按钮
  - String：文本查看器 + 保存
  - Hash：# / 字段 / 值 表格 + 增改删除
  - List：# / 值 表格
  - Set：# / 值 表格
  - ZSet：# / 分数 / 成员 表格
  - 每行操作：复制/编辑/删除
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add Redis value viewer with type-specific editors`

### 4 CLI

- **功能目标**：Redis 命令行界面，支持输入命令并查看结果
- **文件结构**：
  - `src/features/redis/RedisCli.vue`（新增）
- **交互设计**：
  - 命令输入行 + 历史上/下
  - 结果区域：只读日志，JSON 格式化显示
  - Ctrl+L 清屏
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add Redis CLI with command history`

### 5 Server Status 仪表盘

- **功能目标**：Redis 服务器状态卡片仪表盘
- **文件结构**：
  - `src/features/redis/RedisStatus.vue`（新增）
- **内容**：
  - Server 卡片（版本/OS/PID）
  - Memory 卡片（used/peak）
  - Stats 卡片（连接数/命令数）
  - 键统计表格（每 DB 行：库/键数/过期）
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add Redis server status dashboard`

### 6 批量操作

- **功能目标**：批量删除和批量 TTL 设置
- **文件结构**：
  - `src/features/redis/RedisBatchOps.vue`（新增）
- **交互设计**：
  - 选中多个键 → 工具栏显示批量操作按钮
  - 批量删除：确认对话框
  - 批量 TTL：输入 TTL 值
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add Redis batch delete and TTL operations`

### 7 测试与收尾

- **功能目标**：验证全部功能，修复问题
- **测试标准**：type-check + lint + build + cargo build + cargo clippy 全通过
- **提交**：`fix(web): Redis console polish and fixes`

## 设计核对点
- [ ] 后端 RedisConnector trait 统一连接/扫描/读写，REST API 可操作
- [ ] 前端键树正确展示命名空间分组，SCAN 分页流畅
- [ ] 值查看器正确区分 5 种数据类型并展示
- [ ] CLI 可执行任意 Redis 命令并显示结果
- [ ] Server Status 卡片展示关键指标
- [ ] 批量删除/TTL 功能正常
- [ ] 各面板切换后状态保留

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
