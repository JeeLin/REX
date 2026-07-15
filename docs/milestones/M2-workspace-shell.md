# 0.3.0: M2 工作空间外壳

## Context
M0 完成项目骨架，M1 完成设计系统和组件库。M2 构建 Xshell 风格的工作空间外壳，是后续 SSH 终端、SQL 控制台、Redis 控制台的统一宿主。

前序：M1 设计系统与组件库。
后续：M3 SSH 终端。

版本类型：minor

## 产品边界
- **做**：工作空间外壳完整交互（连接树侧栏、Tab 管理、分屏、状态栏、快捷键面板、Quick Connect、资源属性对话框）
- **不做**：终端/SQL/Redis 实际功能（M3+），文件传输，后端 API

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 连接树侧栏（可折叠、搜索、环境→资源分组、颜色标签） | ⬜ |
| 2 | Tab 管理增强（滚动、右键菜单、设色/重命名） | ⬜ |
| 3 | 分屏增强（5 种布局预设、Tab 跨 Pane 拖动） | ⬜ |
| 4 | Quick Connect 栏 + 快捷键面板（F1） | ⬜ |
| 5 | 资源属性对话框 | ⬜ |
| 6 | 测试与收尾 | ⬜ |

## 子任务详细设计

### 1 连接树侧栏

- **功能目标**：工作区内嵌可折叠连接树，环境→资源分组展示
- **文件结构**：`src/features/workspace/ConnectionTree.vue`（新增）
- **接口设计**：
  ```ts
  interface TreeNode {
    id: string
    type: 'group' | 'resource'
    name: string
    protocol?: Protocol
    host?: string
    status?: StatusDotStatus
    color?: string
    children?: TreeNode[]
  }
  ```
- **交互设计**：
  - 侧栏宽度可拖拽调整（min 200px, max 400px）
  - 环境分组可折叠，显示资源数量徽章
  - 搜索框实时过滤（按名称/主机）
  - 资源项显示：协议色标 + 名称 + 主机 + 状态点
  - 颜色标签（8 色循环，用户可标记资源）
  - 双击资源 → 在工作区打开 Tab
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add workspace connection tree sidebar`

### 2 Tab 管理增强

- **功能目标**：完善 Tab 交互，对标 Xshell
- **文件结构**：修改 `src/pages/WorkspacePage.vue`
- **增强内容**：
  - Tab 栏溢出时左右滚动箭头
  - 右键菜单（关闭/关闭其他/关闭右侧/重命名/设色）
  - Tab 设色（8 色圆点选择器）
  - Tab 重命名（双击 Tab 标签进入编辑态）
  - Tab 拖拽排序
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): enhance tab management with context menu, color, rename`

### 3 分屏增强

- **功能目标**：完善分屏布局，对标 Xshell 5 种布局
- **文件结构**：修改 `src/pages/WorkspacePage.vue`
- **布局预设**：
  | 快捷键 | 布局 | 说明 |
  |--------|------|------|
  | Alt+1 | 单面板 | 当前 |
  | Alt+2 | 左右分屏 | 50/50 |
  | Alt+3 | 上下分屏 | 50/50 |
  | Alt+4 | 四宫格 | 25×4 |
  | Alt+5 | 主+侧 | 70/30 |
- **交互设计**：
  - 布局切换时按 Tab 顺序自动填充面板
  - Tab 可拖拽到目标 Pane
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add workspace layout presets and tab-to-pane drag`

### 4 Quick Connect + 快捷键面板

- **功能目标**：顶部快速连接栏 + F1 快捷键帮助
- **文件结构**：`src/features/workspace/QuickConnect.vue`（新增）、`src/features/workspace/ShortcutPanel.vue`（新增）
- **Quick Connect**：
  - 协议下拉 + 主机 + 端口 + 用户名 + 连接按钮
  - 回车触发连接（M3+ 实际连接逻辑）
- **Shortcut Panel**：
  - F1 打开/关闭
  - 分组展示所有快捷键（工作空间/Tab/分屏/终端）
  - 半透明遮罩层
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add Quick Connect bar and F1 shortcut panel`

### 5 资源属性对话框

- **功能目标**：查看和编辑资源连接属性
- **文件结构**：`src/features/workspace/ResourceProperties.vue`（新增）
- **交互设计**：
  - Modal 弹窗，分类 Tab（连接/认证/终端/外观）
  - 连接：主机、端口、协议
  - 认证：用户名、密码/密钥
  - 终端：编码、保活
  - 外观：颜色标签、备注
  - 右键资源 → 属性 打开
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): add resource properties dialog`

### 6 测试与收尾

- **功能目标**：验证全部交互，修复问题
- **测试标准**：type-check + lint + build 全通过
- **提交**：`fix(web): workspace shell polish and fixes`

## 设计核对点
- [ ] 连接树侧栏可折叠、可搜索、环境分组
- [ ] Tab 管理：右键菜单、设色、重命名
- [ ] 5 种布局预设可通过快捷键切换
- [ ] Quick Connect 栏可输入并触发连接
- [ ] F1 快捷键面板可打开查看
- [ ] 资源属性对话框可查看分类配置

## Flow Status

- [ ] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
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
