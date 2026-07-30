# M66: 移动端适配与交互增强

## Context
M65 完成认证体验改进和环境变量文档。本里程碑聚焦移动端体验优化，响应产品文档中「移动端底部导航」的设计要求。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段做什么：
- 移动端底部导航栏（产品文档 3.4 节：5 图标导航）
- 触摸手势支持（滑动关闭 Drawer/Pan）
- 移动端键盘适配（虚拟键盘弹出时布局调整）
- 移动端文件管理优化

本阶段不做什么：
- 不修改桌面端布局
- 不新增后端 API
- 不改变核心功能逻辑

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 移动端底部导航栏 | ✅ |
| 2 | 触摸手势支持（滑动关闭） | ✅ |
| 3 | 移动端键盘适配 | ✅ |
| 4 | 移动端文件管理优化 | ⬜ |

## 子任务详细设计

### 1 移动端底部导航栏

- **功能目标**：实现产品文档 3.4 节的移动端底部导航
- **文件结构**（修改）：
  - `packages/rex-console-web/src/layouts/AppLayout.vue` — 添加移动端底部导航组件
- **交互设计**：
  - 桌面端（≥768px）：底部导航隐藏，使用侧栏
  - 移动端（<768px）：底部导航显示 5 图标（仪表盘 / 环境 / + / Agent / 设置）
  - 中间「+」按钮：快速创建环境
  - 当前页面高亮显示
- **实现要点**：
  - 使用 CSS 媒体查询 `@media (max-width: 768px)`
  - 底部导航固定在 viewport 底部
  - 安全区域适配（iPhone 刘海/底部指示条）：`env(safe-area-inset-bottom)`
- **提交信息**：`feat(mobile): add bottom navigation bar for mobile view`

### 2 触摸手势支持

- **功能目标**：移动端滑动手势操作
- **文件结构**（创建）：
  - `packages/rex-console-web/src/composables/useSwipeGesture.ts` — 滑动手势 composable
- **交互设计**：
  - 从左边缘右滑 → 打开侧栏抽屉
  - 从右边缘左滑 → 关闭 Drawer/Pan
  - 下拉 → 刷新当前页面数据
- **实现要点**：
  - 使用 touchstart/touchmove/touchend 事件
  - 设置滑动阈值（50px）和方向检测
  - 复用现有 `mobileMenuOpen` 状态控制侧栏
- **测试标准**：
  - iOS Safari / Android Chrome 触摸手势正常
  - 不干扰页面内滚动
- **提交信息**：`feat(mobile): add swipe gestures for sidebar and drawer`

### 3 移动端键盘适配

- **功能目标**：虚拟键盘弹出时调整布局，避免遮挡输入框
- **文件结构**（修改）：
  - `packages/rex-console-web/src/layouts/AppLayout.vue` — 键盘弹出时隐藏底部导航
  - `packages/rex-console-web/src/styles/mobile.css` — 键盘适配样式
- **交互设计**：
  - 虚拟键盘弹出时（focus input/textarea）：
    - 隐藏底部导航栏
    - 调整内容区高度
  - 虚拟键盘收起时：
    - 恢复底部导航栏
    - 恢复内容区高度
- **实现要点**：
  - 监听 `visualViewport` resize 事件检测键盘弹出
  - 或使用 CSS `env(keyboard-inset-height)`（iOS 15+）
- **提交信息**：`feat(mobile): adapt layout for virtual keyboard`

### 4 移动端文件管理优化

- **功能目标**：SFTP 文件管理在移动端的交互优化
- **文件结构**（修改）：
  - `packages/rex-console-web/src/features/files/MobileFilesBar.vue` — 增强移动端文件操作
- **交互设计**：
  - 长按文件 → 显示操作菜单（下载/删除/重命名）
  - 文件列表显示文件大小和修改时间
  - 上传按钮更显眼
- **测试标准**：
  - 触摸操作流畅
  - 操作菜单响应式布局
- **提交信息**：`feat(mobile): enhance file management for touch devices`

## 设计核对点

- 单用户模式：不引入多用户概念
- 自托管优先：移动端优化不增加部署复杂度
- 响应式设计：桌面端功能不受影响
- 后端依赖使用 `workspace = true`
- 前端按功能域组织

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
