# 0.2.0: M1 设计系统与组件库

## Context
M0 完成了项目骨架（Rust workspace + Vue 3 前端 + 12 个基础组件 + 设计 token 初版）。M1 在此基础上深化设计系统，补全组件库，为 M2 工作空间外壳和后续模块提供统一的视觉和交互基础。

前序：M0 项目骨架重建。
后续：M2 工作空间外壳（Xshell 模式）。

版本类型：minor

## 产品边界
- **做**：token 体系完善（组件令牌、动画、亮色主题补全）、现有组件增强（更多变体/状态）、新增缺失组件（Input/Select/Scrollbar 等）、设计预览页更新、组件文档
- **不做**：业务功能（终端/SQL/Redis/文件传输）、新页面、新路由

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Token 完善（组件令牌 + 动画 + 亮色主题补全） | ⬜ |
| 2 | 组件增强（Button/Input/Select/Badge/Card/Table/Modal/Drawer/Toast） | ⬜ |
| 3 | 新增组件（Scrollbar/Checkbox/Radio/Switch/Avatar/Alert/ToggleGroup） | ⬜ |
| 4 | 设计预览页更新 + 组件文档 | ⬜ |

## 子任务详细设计

### 1 Token 完善

- **功能目标**：补全组件级 CSS 变量、动画令牌、亮色主题完整映射
- **文件结构**：`src/styles/tokens.css`（扩展）
- **接口设计**：
  ```css
  /* 组件令牌 */
  --btn-height-sm/md/lg
  --input-height-sm/md/lg
  --badge-radius
  --card-radius
  --table-row-hover

  /* 动画 */
  --duration-fast: 0.1s
  --duration-normal: 0.2s
  --duration-slow: 0.3s
  --ease-in/out/inOut
  --scale-in: scale(0.95)
  --slide-up: translateY(4px)

  /* 亮色主题补全 */
  :root[data-theme='light'] { ... 完整映射 }
  ```
- **测试标准**：`bun run build` 通过
- **提交**：`feat(web): enhance design tokens with component vars and animations`

### 2 组件增强

- **功能目标**：为现有组件增加常用变体和状态，提升可用性
- **文件结构**：`src/components/ui/*.vue`（修改现有）
- **增强清单**：
  | 组件 | 增强内容 |
  |------|----------|
  | Button | `loading` 状态、`icon` slot、`block` 全宽变体 |
  | Input | `placeholder`、`clearable`、`disabled`、`error` 状态、`prefix/suffix` slot |
  | Select | 基础单选下拉（选项列表 + v-model） |
  | Badge | `size` 变体（sm/md）、`dot` 模式 |
  | Card | `hoverable` 状态、`footer` slot |
  | Table | 空状态 slot、斑马纹 `striped`、紧凑 `compact` |
  | Modal | `esc` 关闭、`max-width` prop |
  | Drawer | `placement`（left/right）、`width` prop |
  | Toast | `duration` prop、`position`（top-right/top-center） |
- **测试标准**：`bun run type-check` + `bun run build` 通过
- **提交**：`feat(web): enhance existing components with variants and states`

### 3 新增组件

- **功能目标**：补全表单和辅助组件
- **文件结构**：`src/components/ui/`（新增）
- **组件清单**：
  | 组件 | Props | 用途 |
  |------|-------|------|
  | Scrollbar | `height`、`horizontal` | 自定义 6px 细滚动条容器 |
  | Checkbox | `modelValue`、`label`、`disabled` | 复选框 |
  | Radio | `modelValue`、`options`、`disabled` | 单选按钮组 |
  | Switch | `modelValue`、`size` | 开关切换 |
  | Avatar | `src`、`size`、`fallback` | 头像/图标占位 |
  | Alert | `type`（info/success/warning/error）、`title`、`closable` | 提示信息 |
  | ToggleGroup | `modelValue`、`options` | 按钮组切换（用于工具栏） |
- **测试标准**：`bun run type-check` + `bun run build` 通过
- **提交**：`feat(web): add form and utility components (Scrollbar/Checkbox/Radio/Switch/Avatar/Alert/ToggleGroup)`

### 4 设计预览页更新 + 文档

- **功能目标**：更新设计预览页展示所有新增/增强组件，添加使用说明
- **文件结构**：`src/features/design-preview/DesignPreview.vue`（修改）
- **交互设计**：
  - 新增组件分区（表单组件、反馈组件、数据展示组件）
  - 每个组件展示所有变体（size/variant/state）
  - 亮色/暗色切换全局预览
  - 复制代码片段按钮
- **测试标准**：`bun run build` 通过，dev server 可访问
- **提交**：`feat(web): update design preview with all components and docs`

## 设计核对点
- [ ] Token 体系完整（组件级 + 动画 + 亮色主题）
- [ ] 组件风格统一（圆角/间距/颜色引用 token）
- [ ] 新增组件符合 REX 极客美学（深色优先、等宽字体、高对比）
- [ ] 所有组件支持 focus-visible 焦点环
- [ ] 设计预览页展示全部组件变体
- [ ] 亮色主题可正常切换

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
