# M50: UX Bug 修复与交互打磨

## Context

M49 完成连接模型重构（v0.42.0）后，实际使用中发现一批 UX 问题和功能缺失，涵盖终端显示、资源管理、收藏功能、Agent/环境信息展示、S3 连接测试等方面。本里程碑集中修复这些问题，提升产品可用性。

版本类型：patch（bug 修复 + UX 打磨）
版本号：0.43.0

## 产品边界

**做什么**：
- 修复 xterm 终端底部裁剪和顶部 ^^^^ 字符问题
- 修复资源修改后侧栏不刷新
- 修复 SQLite 资源打开后内容为空
- 修复 logout 按钮无图标
- 添加收藏资源的 UI 入口
- 环境详情页展示 agent token
- 修复 S3 连接测试报错（config_json 传递问题）
- Agent 页提示获取 token 但无入口

**不做什么**：
- 不引入新协议或新功能模块
- 不改变现有 API 契约
- 不做大规模重构

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 前端：修复 xterm 终端底部行裁剪（overflow + 字体加载等待） | ✅ |
| 2 | 前端：修复 xterm 顶部 ^^^^ 字符（字体加载后重新 fit） | ✅ |
| 3 | 前端：修复资源修改后侧栏不刷新（add updateResource） | ✅ |
| 4 | 前端：修复 SQLite 资源打开后内容为空 | ⬜ |
| 5 | 前端：修复 logout 按钮图标（替换为 SVG icon） | ⬜ |
| 6 | 前端：添加收藏资源 UI 入口（右键菜单 + 资源属性） | ⬜ |
| 7 | 前端：环境详情页展示 agent token | ⬜ |
| 8 | 后端：修复 S3 连接测试（确保 config_json 正确传递） | ⬜ |
| 9 | 质量验证 | ⬜ |

## 子任务详细设计

### 1 前端：修复 xterm 终端底部行裁剪

- **功能目标**：修复 SSH 终端底部约 2 行内容被容器裁剪
- **问题根因**：`.tv-container` 设置了 `overflow: hidden`，且初始 fit 未等待字体加载完成，fallback 字体 cell height 偏小导致 rows 过多
- **文件结构**：
  - 修改：`packages/rex-console-web/src/features/terminal/TerminalView.vue`（CSS overflow）
  - 修改：`packages/rex-console-web/src/features/terminal/useTerminal.ts`（字体加载等待）
- **修复方案**：
  1. TerminalView.vue：`.tv-container` 改为 `overflow: visible` 或移除 overflow
  2. useTerminal.ts：`createTerminal` 中等待 `document.fonts.ready` 后再执行 `fit.fit()`
  3. 添加二次 fit：字体加载完成后延迟再 fit 一次
- **测试标准**：终端底部行完整显示，无裁剪
- **提交信息**：`fix(terminal): prevent bottom row clipping by waiting for font load`

### 2 前端：修复 xterm 顶部 ^^^^ 字符

- **功能目标**：修复 SSH 终端顶部出现 ^^^^ 字符
- **问题根因**：与字体加载/fit 计算相关，初始 fit 使用未加载字体的 metrics，导致 rows 计算错误，终端渲染异常
- **文件结构**：同任务 1
- **修复方案**：任务 1 的字体加载等待修复后验证。如仍存在，检查 xterm.js 的 `rows` 设置是否被错误覆盖
- **测试标准**：终端顶部无异常字符
- **提交信息**：与任务 1 合并提交

### 3 前端：修复资源修改后侧栏不刷新

- **功能目标**：修复通过 ResourceProperties 修改资源后，侧栏连接树不更新
- **问题根因**：`environments.ts` store 只有 `createResource` 和 `deleteResource`，缺少 `updateResource` 方法。ResourceProperties 保存后 `envResources` 不更新
- **文件结构**：
  - 修改：`packages/rex-console-web/src/stores/environments.ts`（添加 updateResource）
  - 修改：`packages/rex-console-web/src/features/workspace/ResourceProperties.vue`（保存后调用 updateResource）
- **修复方案**：
  1. environments.ts 添加 `updateResource(envId, id, data)` 方法
  2. ResourceProperties 保存成功后调用 updateResource 更新 envResources
- **测试标准**：修改资源名称/配置后侧栏立即刷新
- **提交信息**：`fix(store): add updateResource to refresh sidebar after edit`

### 4 前端：修复 SQLite 资源打开后内容为空

- **功能目标**：修复新建 SQLite 资源指向 rex.db 但工作区显示空
- **问题根因**：SQLite 资源的 `file_path` 配置可能未正确传递到后端，或后端未正确使用 config_json 中的 file_path
- **文件结构**：
  - 检查：`packages/rex-console-web/src/features/sql/SqlPage.vue`（SQLite 连接逻辑）
  - 检查：`crates/rex-hub/src/sql_api.rs`（SQLite connect handler）
- **修复方案**：排查 SQLite 连接流程，确保 file_path 从 config_json 正确提取并传递给 rex-sqlite connector
- **测试标准**：新建 SQLite 资源指向 rex.db 后可正常打开并显示表数据
- **提交信息**：`fix(sqlite): ensure file_path from config_json is used for connection`

### 5 前端：修复 logout 按钮图标

- **功能目标**：修复 logout 按钮无图标显示
- **问题根因**：logout 按钮使用 Unicode 字符 `⏻`，在某些字体/浏览器下不渲染
- **文件结构**：
  - 修改：`packages/rex-console-web/src/layouts/AppLayout.vue`
- **修复方案**：将 `⏻` 替换为 SVG icon（power-off 图标），与其他图标风格一致
- **测试标准**：logout 按钮正确显示 power-off 图标
- **提交信息**：`fix(ui): replace logout Unicode char with SVG icon`

### 6 前端：添加收藏资源 UI 入口

- **功能目标**：在资源操作中添加收藏/取消收藏按钮
- **问题根因**：favorites store 已有 `toggleFavorite` / `isFavorite`，但无 UI 入口触发
- **文件结构**：
  - 修改：`packages/rex-console-web/src/features/workspace/ConnectionTree.vue`（右键菜单添加收藏项）
  - 修改：`packages/rex-console-web/src/features/workspace/ResourceProperties.vue`（添加收藏按钮）
- **修复方案**：
  1. ConnectionTree 右键菜单添加「收藏/取消收藏」项
  2. ResourceProperties 顶部添加收藏星标按钮
- **测试标准**：可通过右键菜单和属性弹窗收藏资源，收藏后侧栏收藏 tab 显示
- **提交信息**：`feat(favorites): add favorite toggle to context menu and properties`

### 7 前端：环境详情页展示 agent token

- **功能目标**：在环境详情页展示 agent 注册 token，方便用户复制部署命令
- **问题根因**：环境详情页只有 agent 状态显示，没有展示 token。Agents 页的部署弹窗中有 token，但环境详情页缺失
- **文件结构**：
  - 修改：`packages/rex-console-web/src/pages/EnvironmentDetailPage.vue`
- **修复方案**：在 Agent 面板中添加 token 展示（可复制），格式参考 Agents 页部署弹窗
- **测试标准**：环境详情页可查看并复制 agent token
- **提交信息**：`feat(env-detail): show agent registration token`

### 8 后端：修复 S3 连接测试

- **功能目标**：修复 S3 连接测试报错（日志 host= 为空）
- **问题根因**：S3 测试连接时 `body.host` 为空（S3 使用 endpoint 不是 host），tracing 日志显示 host= 为空。需确认 config_json 中的 endpoint 是否正确传递到 S3 client
- **文件结构**：
  - 检查：`crates/rex-hub/src/resource_api.rs`（S3 test_connection 分支）
  - 检查：`packages/rex-console-web/src/features/resource/WizardModal.vue`（S3 test connection 请求）
- **修复方案**：
  1. 确认 WizardModal 的 `buildConfig()` 正确包含 S3 endpoint
  2. 后端 S3 分支使用 config_json 中的 endpoint 构建 client
  3. 修复 tracing 日志：S3 协议时显示 endpoint 而非 host
- **测试标准**：S3 资源在 WizardModal 中测试连接成功
- **提交信息**：`fix(s3): pass endpoint correctly in test connection`

### 9 质量验证

- **功能目标**：确保所有改动通过质量门禁
- **文件结构**：无新文件
- **后端流程**：
  1. `cargo fmt --check`
  2. `cargo clippy --workspace --all-targets`
  3. `cargo test --workspace`
- **前端流程**：
  1. `bun run type-check`
  2. `bun run lint`
  3. `bun run build`
- **测试标准**：所有检查通过
- **提交信息**：`chore: quality gate verification for M50`

## 设计核对点

1. **终端体验**：xterm 底部行完整显示，顶部无异常字符，字体加载后 fit 正确
2. **资源管理**：修改资源后侧栏立即刷新，SQLite 连接正常
3. **收藏功能**：可通过右键菜单和属性弹窗收藏资源
4. **Agent/环境**：环境详情页可查看 token，Agent 页部署信息完整
5. **S3 连接**：测试连接端点正确传递

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
| | | |

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
| | | | | |
