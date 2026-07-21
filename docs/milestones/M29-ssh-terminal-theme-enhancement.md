# M29: SSH 终端主题增强（背景图 + 透明度）

## Context

M28 完成了 Redis FormatViewer 高级格式解码。当前 SSH 终端已有 3 套主题预设（REX Default / Ubuntu / Solarized Dark），ResourceProperties.vue 中有主题选择器和透明度字段但未接线到 TerminalView。PRODUCT.md 3.6 要求终端支持背景图、透明度、ANSI 调色板。本里程碑将现有 UI 基础设施接通并新增背景图支持。

版本类型：minor（新功能），版本号 0.27.0 → 0.28.0。

## 产品边界

**本阶段做：**
- 接通 ResourceProperties → TerminalView 的数据流（theme/opacity/cursor/font）
- 实现终端背景透明度（CSS opacity + alpha channel）
- 新增终端背景图支持（可选预设 + 自定义 URL）
- 全局设置页新增终端主题/透明度/背景图控件
- 全局设置接通 useTerminal

**本阶段不做：**
- 新增更多主题预设（后续里程碑可扩展）
- 终端录制/回放
- 多窗口分离

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 接通 ResourceProperties → TerminalView 数据流 | ⬜ |
| 2 | 终端背景透明度 + 背景图支持 | ⬜ |
| 3 | 全局设置页终端主题控件 + 全局设置接通 useTerminal | ⬜ |

## 子任务详细设计

### 1 接通 ResourceProperties → TerminalView 数据流

**功能目标**

将 ResourceProperties.vue 中已有的主题选择器、透明度、光标样式、字体大小等配置传递到 TerminalView 组件，使终端实际应用这些设置。

**文件结构**

修改：
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — 将 tab 的 resourceProps 传递给 TerminalView
- `packages/rex-console-web/src/features/terminal/TerminalView.vue` — 接收新 props，传递给 useTerminal
- `packages/rex-console-web/src/features/terminal/useTerminal.ts` — createTerminal 使用传入的 options

**接口设计**

TerminalView 新增 props：
```typescript
const props = defineProps<{
  tabId: string
  resourceId?: string
  host?: string
  port?: number
  // 新增
  theme?: string          // 'default' | 'ubuntu' | 'solarized-dark'
  fontSize?: number       // 10-24
  opacity?: number        // 0-100
  cursorStyle?: string    // 'block' | 'underline' | 'bar'
  cursorBlink?: boolean
}>()
```

**实现流程**

1. WorkspacePage.vue：Tab 数据模型已有 `resourceProps`，取出 theme/fontSize/opacity/cursorStyle/cursorBlink 作为 props 传给 TerminalView
2. TerminalView.vue：接收 props，在 onMounted 中将 theme/fontSize/cursorStyle/cursorBlink 传给 createTerminal 的 options 参数
3. useTerminal.ts：createTerminal 已支持 `...options` spread，将 cursorStyle 映射为 xterm 的 `cursorStyle` 选项

**测试标准**

- 创建 SSH Tab → 打开 ResourceProperties → 修改主题 → 切回终端 → 主题已应用
- 修改光标样式（block/underline/bar）→ 终端光标立即变化
- 修改字体大小 → 终端字体大小变化
- type-check + build 通过

**提交信息**: `feat(terminal): wire ResourceProperties settings to TerminalView`

### 2 终端背景透明度 + 背景图支持

**功能目标**

实现终端背景透明度（终端内容下方透出背景图）和背景图功能。

**文件结构**

修改：
- `packages/rex-console-web/src/features/terminal/TerminalView.vue` — 背景图层 + 透明度 CSS
- `packages/rex-console-web/src/features/terminal/useTerminal.ts` — 新增 setBackground 方法
- `packages/rex-console-web/src/features/workspace/ResourceProperties.vue` — 新增背景图字段
- `packages/rex-console-web/src/api/settings.ts` — Settings 新增 terminal_bg_image

**交互设计**

- ResourceProperties → Appearance tab：新增「Background Image」字段（URL 输入 + 预设下拉）
- 预设背景图：None（默认）、Grid（网格线）、Dots（圆点矩阵）、Gradient（渐变）— 使用纯 CSS 实现，无需实际图片文件
- 自定义背景图：输入 URL 或 data URI
- 透明度滑块（0-100%）控制背景图的显示强度

**实现流程**

1. TerminalView.vue：
   - `.tv-container` 移除硬编码 `background: #0d1117`
   - 新增背景图层 `<div class="tv-bg" />`（position: absolute, z-index: 0）
   - 终端内容层 `<div class="tv-terminal" />`（position: relative, z-index: 1）
   - 背景图层根据 props.backgroundImage 应用 CSS background
   - 透明度：终端背景色使用 RGBA，alpha 由 props.opacity 控制

2. useTerminal.ts：
   - xterm theme 的 background 颜色附加 alpha 通道：`#0D1117` + opacity → `rgba(13,17,23,${opacity/100})`
   - 新增 `setBackground(image: string | null)` 方法

3. ResourceProperties.vue：
   - Appearance tab 新增 Background Image 下拉（None/Grid/Dots/Gradient + Custom URL）
   - 保存到 resourceProps.backgroundImage

**CSS 背景预设（纯 CSS，无图片文件）**

```css
/* Grid: 细线网格 */
background-image:
  linear-gradient(rgba(255,255,255,0.03) 1px, transparent 1px),
  linear-gradient(90deg, rgba(255,255,255,0.03) 1px, transparent 1px);
background-size: 20px 20px;

/* Dots: 圆点矩阵 */
background-image: radial-gradient(circle, rgba(255,255,255,0.05) 1px, transparent 1px);
background-size: 24px 24px;

/* Gradient: 对角渐变 */
background: linear-gradient(135deg, #0D1117 0%, #161B22 50%, #0D1117 100%);
```

**测试标准**

- 透明度 50% → 终端背景变半透明
- 选择 Grid 预设 → 背景显示网格
- 选择自定义 URL → 背景显示图片
- None → 无背景图，纯色
- type-check + build 通过

**提交信息**: `feat(terminal): add background image presets and opacity support`

### 3 全局设置页终端主题控件 + 全局设置接通 useTerminal

**功能目标**

全局设置页新增终端主题/透明度/背景图控件，作为所有终端 Tab 的默认值。useTerminal 读取全局设置作为 fallback。

**文件结构**

修改：
- `packages/rex-console-web/src/api/settings.ts` — Settings 新增 terminal_theme/terminal_opacity/terminal_bg_image
- `packages/rex-console-web/src/pages/SettingsPage.vue` — 新增终端主题/透明度/背景图控件
- `packages/rex-console-web/src/features/terminal/useTerminal.ts` — 读取全局设置作为默认值

**实现流程**

1. settings.ts：
   ```typescript
   export interface Settings {
     theme: string
     language: string
     terminal_font: string
     terminal_font_size: string
     // 新增
     terminal_theme: string       // 'default' | 'ubuntu' | 'solarized-dark'
     terminal_opacity: number     // 0-100, default 100
     terminal_bg_image: string    // 'none' | 'grid' | 'dots' | 'gradient' | URL
   }
   ```

2. SettingsPage.vue：
   - Terminal 区块新增：主题下拉（3 个预设）、透明度滑块（0-100%）、背景图下拉（None/Grid/Dots/Gradient/Custom）
   - 保存到后端 settings API

3. useTerminal.ts：
   - createTerminal 时：如果没有传入 theme/opacity 等 prop，从全局设置读取
   - 使用 `localStorage` 缓存设置（与现有 auth store 模式一致）

**测试标准**

- 全局设置修改终端主题 → 新开 SSH Tab 使用新主题
- 全局设置修改透明度 → 新开 SSH Tab 使用新透明度
- ResourceProperties 单独设置优先于全局设置
- type-check + build 通过

**提交信息**: `feat(terminal): add terminal theme controls to global settings page`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ 不引入新概念（纯 UI 增强）
- ✅ 不跳阶段实现
- ✅ 实现细节不污染产品文档
- ✅ 与 Xshell 对标（终端主题/透明度/背景图）

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

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
