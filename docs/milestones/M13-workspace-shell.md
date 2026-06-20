# M13: 工作空间 — 多标签页、连接菜单、分屏布局

## Context

M0-M12 完成了所有管理页面（仪表盘、环境、Agent、审计日志、设置）和独立的全屏页面（Terminal、SQL、Files）。但产品核心——**工作空间**（§3.5）——尚未实现。当前 Terminal/SQL/Files 以独立路由页面存在，缺少统一的多标签管理、分屏布局和连接菜单。

M13 构建工作空间的**外壳框架**：标签栏、连接菜单、分屏布局切换、快捷键面板和键盘快捷键。面板内容（SSH 终端、SQL 控制台、SFTP 等）在后续里程碑嵌入。

## 产品边界

**做什么：**
- 工作空间页面（路由 `/workspace`），替代当前独立的全屏页面
- 标签栏：新建/关闭/切换/拖拽排序标签
- 连接菜单（Ctrl+N）：搜索资源、按环境分组、键盘导航
- 5 种分屏布局（单面板/左右/上下/四宫格/主+侧边）
- 快捷键面板（F1）
- 全局键盘快捷键（Ctrl+N/W/Tab/1~9, Alt+1~5, F1, F11）
- 标签右键菜单（关闭系列/复制/移动到面板/新建连接/全部断开）
- 空状态提示
- 侧边栏资源项点击跳转到工作空间

**不做什么：**
- 面板内具体内容（SSH 终端、SQL 控制台、SFTP 面板）——后续里程碑
- 面板间拖拽标签交换（后续增强）
- 移动端适配（后续优化）
- 连接状态实际管理（WebSocket 连接）

---

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 13.1 | 工作空间页面框架与路由 | 前端 | ✅ |
| 13.2 | 标签栏组件 | 前端 | ✅ |
| 13.3 | 连接菜单 | 前端 | ✅ |
| 13.4 | 分屏布局系统 | 前端 | ✅ |
| 13.5 | 快捷键面板与全局快捷键 | 前端 | ✅ |
| 13.6 | 标签右键菜单 | 前端 | ✅ |
| 13.7 | 侧边栏集成与路由更新 | 前端 | ✅ |

---

## 子任务 13.1：工作空间页面框架与路由

### 功能目标

创建 `/workspace` 路由的 Workspace 页面，包含整体布局结构：标签栏 + 内容区。支持空状态显示。

### 修改文件

```text
packages/rex-console-web/src/
├── pages/Workspace.vue          新增：工作空间页面
├── router.ts                    修改：添加 /workspace 路由
├── layouts/AppLayout.vue        修改：导航链接指向 /workspace
```

### 接口设计

```ts
// Workspace.vue 内部状态
interface Tab {
  id: string
  name: string          // 资源名称
  proto: Protocol       // 协议类型
  resourceId: string    // 资源 ID（后续连接用）
  panelIndex: number    // 所在面板索引
  status: 'online' | 'offline' | 'connecting'
}

type Layout = 'single' | 'left-right' | 'top-bottom' | 'quad' | 'sidebar-main'
```

### 交互设计

参考原型 `prototype/app.html` 第 331-353 行：

- 整体布局：左侧 AppLayout 侧边栏 + 右侧 ws-main
- ws-main 包含：ws-tabbar（标签栏）+ ws-content（内容区）
- 空状态：居中显示图标 + 提示文字 + 快捷键提示
- 内容区使用 CSS Grid 实现分屏

### 测试标准

- `/workspace` 路由正常渲染
- 空状态显示正确
- 侧边栏导航链接正确
- 页面布局与原型一致

### 提交信息

```
feat: add workspace page shell with routing
```

---

## 子任务 13.2：标签栏组件

### 功能目标

实现标签栏（TabBar）组件，支持标签的增删、切换、拖拽排序。

### 修改文件

```text
packages/rex-console-web/src/
├── features/workspace/TabBar.vue       新增：标签栏组件
├── features/workspace/useTabs.ts       新增：标签状态管理 composable
├── pages/Workspace.vue                 修改：集成 TabBar
├── i18n/zh.ts, en.ts                   修改：添加 workspace 相关 i18n
```

### 交互设计

参考原型 `prototype/app.html` 第 15-66 行（CSS）+ 第 510-599 行（JS）：

**标签项结构：**
- 协议彩色图标（`$` SSH / `dB` MySQL / `📁` SFTP 等）
- 资源名称
- 连接状态点（🟢 在线 / ⚫ 离线 / 🟠 连接中）
- 关闭按钮（悬停显示）
- 拖拽排序支持

**标签行为：**
- 去重：相同名称+协议的资源不重复打开，跳转到已有标签
- 拖拽排序：`draggable` + `dragstart/dragover/drop`
- 双击标签：快速进入左右分屏

**右侧区域：**
- `+` 新建连接按钮
- 布局指示器（后续子任务实现）
- 全屏/快捷键按钮（后续子任务实现）

### useTabs.ts 接口

```ts
export function useTabs() {
  const tabs = ref<Tab[]>([])
  const activeTabId = ref<string | null>(null)
  const tabCounter = ref(0)

  function addTab(name: string, proto: Protocol, resourceId: string): void
  function closeTab(id: string): void
  function closeOtherTabs(id: string): void
  function closeTabsRight(id: string): void
  function closeTabsLeft(id: string): void
  function closeAllTabs(): void
  function activateTab(id: string): void
  function duplicateTab(id: string): void
  function moveTabToPanel(id: string, panelIndex: number): void

  return { tabs, activeTabId, addTab, closeTab, closeOtherTabs, closeTabsRight, closeTabsLeft, closeAllTabs, activateTab, duplicateTab, moveTabToPanel }
}
```

### 协议图标映射

```ts
const PROTOCOL_ICONS: Record<Protocol, string> = {
  ssh: '$', mysql: 'dB', postgresql: 'pg', sftp: '📁',
  redis: 'R', docker: '🐳', sqlite: 'S', s3: '☁',
}
const PROTOCOL_COLORS: Record<Protocol, string> = {
  ssh: 'var(--success)', mysql: 'var(--info)', postgresql: 'var(--accent-purple)',
  sftp: 'var(--accent-purple)', redis: 'var(--danger)', docker: 'var(--info)',
  sqlite: 'var(--warning)', s3: 'var(--accent)',
}
```

### 测试标准

- 点击 `+` 弹出连接菜单，选择资源后标签出现在标签栏
- 点击标签切换 active 状态
- 点击关闭按钮关闭标签
- 拖拽标签可调整顺序
- 去重逻辑正常
- 所有标签关闭后显示空状态

### 提交信息

```
feat: add workspace tab bar with drag-and-drop reorder
```

---

## 子任务 13.3：连接菜单

### 功能目标

实现全局连接菜单组件（Ctrl+N 触发），搜索资源并按环境分组展示。

### 修改文件

```text
packages/rex-console-web/src/
├── features/workspace/ConnectionMenu.vue  新增：连接菜单组件
├── features/workspace/useConnectionMenu.ts 新增：连接菜单状态管理
├── pages/Workspace.vue                    修改：集成 ConnectionMenu
├── i18n/zh.ts, en.ts                      修改：添加连接菜单 i18n
```

### 交互设计

参考原型 `prototype/app.html` 第 94-114 行（CSS）+ 第 447-508 行（JS）：

**触发方式：** `Ctrl+N` 或点击标签栏 `+` 按钮

**布局：**
- 半透明黑色遮罩 + 模糊背景
- 居中弹窗 420px 宽
- 搜索框：⌕ 图标 + placeholder「搜索资源...」+ Esc 快捷键标签
- 资源列表：按环境分组，组标题为环境名
- 每个资源项：协议彩色图标 + 名称 + 地址 + 协议标签
- 底栏：`↑↓ 选择 · ↵ 连接` / `Esc 关闭`

**键盘导航：**
- `↑↓` 移动选中项
- `Enter` 连接选中资源
- `Esc` 关闭菜单

**搜索：**
- 按名称、地址、协议模糊匹配
- 无匹配时显示「没有匹配的资源」

### useConnectionMenu.ts 接口

```ts
export function useConnectionMenu(onConnect: (resource: Resource) => void) {
  const visible = ref(false)
  const searchQuery = ref('')
  const selectedIndex = ref(0)
  const resources = computed(() => filterResources(searchQuery.value))
  const groupedResources = computed(() => groupByEnv(resources.value))

  function show(): void
  function hide(): void
  function selectNext(): void
  function selectPrev(): void
  function connectSelected(): void

  return { visible, searchQuery, selectedIndex, groupedResources, show, hide, selectNext, selectPrev, connectSelected }
}
```

### Mock 数据

当前无后端 API，使用 mock 资源列表：

```ts
const MOCK_RESOURCES = [
  { id: 'r1', name: '云服务器', addr: 'root@192.168.1.100:22', proto: 'ssh', env: '阿里云' },
  { id: 'r2', name: '主数据库', addr: 'db.internal:3306', proto: 'mysql', env: '阿里云' },
  { id: 'r3', name: '分析数据库', addr: 'analytics.internal:5432', proto: 'postgresql', env: '阿里云' },
  { id: 'r4', name: '开发服务器', addr: 'root@10.0.1.15:22', proto: 'ssh', env: '树莓派集群' },
  { id: 'r5', name: '测试数据库', addr: '10.0.1.20:3306', proto: 'mysql', env: '树莓派集群' },
  { id: 'r6', name: 'NAS 主机', addr: '192.168.0.100:22', proto: 'ssh', env: '家庭 NAS' },
  { id: 'r7', name: 'NAS 文件', addr: '192.168.0.100', proto: 'sftp', env: '家庭 NAS' },
]
```

### 测试标准

- `Ctrl+N` 打开连接菜单
- 搜索框输入内容实时过滤
- `↑↓` 键盘导航正常
- `Enter` 连接资源并关闭菜单
- `Esc` 关闭菜单
- 点击遮罩关闭菜单
- 选择资源后在标签栏新增标签

### 提交信息

```
feat: add connection menu with search and keyboard navigation
```

---

## 子任务 13.4：分屏布局系统

### 功能目标

实现 5 种分屏布局模式，通过 Alt+1~5 或布局指示器切换。

### 修改文件

```text
packages/rex-console-web/src/
├── features/workspace/LayoutEngine.vue    新增：布局引擎组件
├── features/workspace/useLayout.ts        新增：布局状态管理 composable
├── pages/Workspace.vue                    修改：集成 LayoutEngine
├── i18n/zh.ts, en.ts                      修改：添加布局 i18n
```

### 交互设计

参考原型 `prototype/app.html` 第 68-93 行（CSS）：

**5 种布局：**

| 布局 | 快捷键 | CSS 类 | Grid 定义 |
|------|--------|--------|-----------|
| 单面板 | Alt+1 | `layout-single` | 无 Grid |
| 左右分屏 | Alt+2 | `layout-left-right` | `grid-template-columns: 1fr 1fr` |
| 上下分屏 | Alt+3 | `layout-top-bottom` | `grid-template-rows: 1fr 1fr` |
| 四宫格 | Alt+4 | `layout-quad` | `grid-template-columns: 1fr 1fr; grid-template-rows: 1fr 1fr` |
| 主+侧边 | Alt+5 | `layout-sidebar-main` | `grid-template-columns: 2fr 1fr` |

**布局切换：**
- 按标签顺序自动填充面板
- 布局切换时保持已有标签内容
- 空面板显示虚线边框占位

**布局指示器（标签栏右侧）：**
- 显示当前布局图标和名称
- 点击切换到下一个布局
- hover 显示 tooltip

### useLayout.ts 接口

```ts
export function useLayout() {
  const currentLayout = ref<Layout>('single')
  const panelCount = computed(() => LAYOUT_PANELS[currentLayout.value])

  function setLayout(layout: Layout): void
  function cycleLayout(): void
  function nextLayout(): void

  return { currentLayout, panelCount, setLayout, cycleLayout, nextLayout }
}

const LAYOUT_PANELS: Record<Layout, number> = {
  'single': 1, 'left-right': 2, 'top-bottom': 2,
  'quad': 4, 'sidebar-main': 2,
}
const LAYOUT_ORDER: Layout[] = ['single', 'left-right', 'top-bottom', 'quad', 'sidebar-main']
```

### LayoutEngine.vue 设计

```vue
<!-- 根据 currentLayout 动态生成面板网格 -->
<div class="ws-content" :class="layoutClass">
  <div v-for="i in panelCount" :key="i"
       class="ws-panel" :class="{ active: isPanelActive(i - 1) }">
    <slot :name="'panel-' + (i - 1)">
      <div class="panel-placeholder">面板 {{ i }}</div>
    </slot>
  </div>
</div>
```

### 测试标准

- 5 种布局模式正确切换
- Alt+1~5 快捷键生效
- 布局指示器显示正确
- 面板按标签顺序自动填充
- 切换布局时不丢失已有标签

### 提交信息

```
feat: add workspace split layout engine with 5 modes
```

---

## 子任务 13.5：快捷键面板与全局快捷键

### 功能目标

实现快捷键帮助面板（F1）和全局键盘快捷键处理。

### 修改文件

```text
packages/rex-console-web/src/
├── features/workspace/ShortcutsPanel.vue  新增：快捷键面板组件
├── composables/useShortcuts.ts            新增：全局快捷键 composable
├── pages/Workspace.vue                    修改：注册全局快捷键
├── i18n/zh.ts, en.ts                      修改：添加快捷键 i18n
```

### 交互设计

参考原型 `prototype/app.html` 第 116-128 行（CSS）+ 第 372-398 行（HTML）：

**快捷键面板：**
- 模态弹窗，半透明遮罩 + 模糊背景
- 分组展示：标签页 / 布局 / 其他
- 每行：功能描述 + `kbd` 标签展示按键
- ESC 或点击遮罩关闭

**全局快捷键列表：**

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+N` | 新建连接（打开连接菜单） |
| `Ctrl+W` | 关闭当前标签 |
| `Ctrl+Tab` | 切换下一个标签 |
| `Ctrl+Shift+Tab` | 切换上一个标签 |
| `Ctrl+1~9` | 跳转到指定标签 |
| `Alt+1` | 单面板布局 |
| `Alt+2` | 左右分屏 |
| `Alt+3` | 上下分屏 |
| `Alt+4` | 四宫格 |
| `Alt+5` | 主+侧边 |
| `F11` | 全屏切换 |
| `F1` | 快捷键帮助 |

### useShortcuts.ts 设计

```ts
export function useShortcuts(handlers: {
  onNewConnection: () => void
  onCloseTab: () => void
  onNextTab: () => void
  onPrevTab: () => void
  onSwitchTab: (index: number) => void
  onLayoutChange: (layout: Layout) => void
  onToggleFullscreen: () => void
  onToggleShortcuts: () => void
}) {
  // 注册 keydown 事件监听
  // Ctrl+N → onNewConnection
  // Ctrl+W → onCloseTab
  // Ctrl+Tab → onNextTab (需阻止浏览器默认行为)
  // Ctrl+Shift+Tab → onPrevTab
  // Ctrl+1~9 → onSwitchTab
  // Alt+1~5 → onLayoutChange
  // F11 → onToggleFullscreen
  // F1 → onToggleShortcuts
}
```

### 测试标准

- F1 打开/关闭快捷键面板
- 所有快捷键正确触发对应功能
- 不干扰浏览器默认行为（如 Ctrl+Tab）
- 快捷键面板内容完整

### 提交信息

```
feat: add keyboard shortcuts panel and global shortcut handling
```

---

## 子任务 13.6：标签右键菜单

### 功能目标

标签右键点击弹出上下文菜单，支持关闭、复制、移动面板等操作。

### 修改文件

```text
packages/rex-console-web/src/
├── features/workspace/TabBar.vue    修改：添加右键菜单事件
├── i18n/zh.ts, en.ts               修改：添加标签菜单 i18n
```

### 交互设计

参考原型 `prototype/app.html` 第 531-554 行：

**菜单项：**

| 菜单项 | 图标 | 说明 |
|--------|------|------|
| 关闭 | ✕ | 关闭当前标签 |
| 关闭其他 | | 关闭除当前外的所有标签 |
| 关闭右侧 | | 关闭右侧标签 |
| 关闭左侧 | | 关闭左侧标签 |
| 关闭全部 | | 关闭所有标签 |
| ─── 分隔线 ─── | | |
| 复制标签 | ⧉ | 复制当前连接 |
| 移动到面板 ▸ | ◧ | 子菜单：面板 1/2/3/4（仅分屏模式） |
| ─── 分隔线 ─── | | |
| 新建连接 | + | 打开连接菜单 |
| 全部断开 | ⚡ | 断开所有（danger 样式） |

**行为规则：**
- 使用 M11 创建的全局 ContextMenu 组件
- 「移动到面板」子菜单仅在分屏模式（panelCount > 1）时显示
- 「关闭其他/右侧/左侧」在只有 1 个标签时不显示

### 测试标准

- 右键标签弹出菜单
- 各菜单项功能正确
- 分屏模式下「移动到面板」子菜单可用
- 单面板模式下隐藏「移动到面板」
- 菜单项与原型一致

### 提交信息

```
feat: add tab context menu with close and panel move options
```

---

## 子任务 13.7：侧边栏集成与路由更新

### 功能目标

更新侧边栏，使资源项点击后在工作空间中打开（而非跳转到独立页面）。更新路由配置。

### 修改文件

```text
packages/rex-console-web/src/
├── layouts/AppLayout.vue        修改：导航链接更新
├── router.ts                    修改：添加 /workspace 路由
├── composables/useSidebar.ts    修改：资源点击行为
├── pages/Dashboard.vue          修改：快速连接跳转到 /workspace
├── pages/Environments.vue       修改：资源行点击跳转到 /workspace
├── pages/EnvironmentDetail.vue  修改：资源行点击跳转到 /workspace
```

### 交互设计

**导航链接更新：**
- 侧边栏「工作区」链接 → `/workspace`
- 侧边栏资源项点击 → 在工作空间打开新标签（不跳路由，通过事件）
- 仪表盘快速连接 → 跳转到 `/workspace` 并打开对应资源
- 环境列表/详情的资源行 → 跳转到 `/workspace` 并打开对应资源

**实现方式：**
- 侧边栏资源项 click 事件发送 `CustomEvent('open-in-workspace', { detail: { resource } })`
- Workspace.vue 监听该事件，调用 `addTab()`
- 仪表盘等页面通过 router.push('/workspace') + query 参数传递资源信息

### 测试标准

- 侧边栏「工作区」链接跳转到 `/workspace`
- 侧边栏资源项点击在工作空间打开标签
- 仪表盘快速连接跳转到工作空间
- 环境详情资源行点击跳转到工作空间
- 无标签时显示空状态

### 提交信息

```
feat: integrate sidebar resource links with workspace
```

---

## 设计核对点

- [ ] 工作空间页面布局与原型 app.html 一致
- [ ] 标签栏样式与原型一致（协议图标/状态点/关闭按钮/拖拽）
- [ ] 连接菜单与原型一致（搜索/环境分组/键盘导航）
- [ ] 5 种分屏布局与产品文档 §3.5 一致
- [ ] 快捷键面板与原型一致
- [ ] 标签右键菜单与产品文档 §3.5 一致
- [ ] 所有快捷键与产品文档 §5 一致
- [ ] i18n 覆盖所有新增文字
- [ ] 侧边栏集成不影响现有页面导航

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [ ] 步骤8：提交
