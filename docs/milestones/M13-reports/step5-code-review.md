# M13 步骤5：代码审查报告

## 审查范围

M13 工作空间相关文件：
- `packages/rex-console-web/src/pages/Workspace.vue`
- `packages/rex-console-web/src/features/workspace/TabBar.vue`
- `packages/rex-console-web/src/features/workspace/useTabs.ts`
- `packages/rex-console-web/src/composables/useSidebar.ts`

## 发现

### 🔴 必须修复（已修复）

| # | 文件 | 问题 | 修复 |
|---|------|------|------|
| 1 | Workspace.vue:226 | 搜索时 selectedResourceIdx 未重置，键盘导航可能越界 | 添加 `watch(connSearchQuery, () => selectedResourceIdx.value = 0)` |
| 2 | TabBar.vue:87-88 | 拖拽排序直接 splice tabs 数组，绕过 composable 封装 | 添加 `reorderTab()` 方法到 useTabs.ts，TabBar 改用该方法 |

### 🟡 应该修复（已修复）

| # | 文件 | 问题 | 修复 |
|---|------|------|------|
| 3 | useSidebar.ts | CustomEvent 在 router.push 之前 dispatch，从非 AppLayout 页面导航时 Workspace 未挂载，事件丢失 | 改用路由 query 参数传递资源信息，Workspace 在 onMounted 中读取 |
| 4 | useTabs.ts:23 | 标签去重仅用 name+proto，不同环境同名资源会误合并 | 改用 resourceId 去重 |

### 🟡 已知问题（未修复，非阻塞）

| # | 文件 | 问题 | 说明 |
|---|------|------|------|
| 5 | Workspace.vue | 快捷键面板和连接菜单缺少 focus trap | 可访问性改进，后续优化 |
| 6 | Workspace.vue | ~15 个用户可见字符串硬编码中文 | 快捷键面板内容为静态帮助文本，暂可接受 |
| 7 | Workspace.vue | requestFullscreen() 错误未处理 | 边缘情况，后续增强 |

### 🟢 可选改进（未修复）

| # | 文件 | 问题 | 说明 |
|---|------|------|------|
| 8 | TabBar.vue:91-94 | onTabDblclick 为空函数 | 双击进入分屏未实现，可后续添加 |
| 9 | useTabs.ts:109-113 | moveTabToPanel 未被 UI 调用 | 右键菜单「移动到面板」子菜单未实现 |
| 10 | useSidebar.ts:28 | localStorage JSON.parse 无校验 | 损坏数据静默重置，影响极小 |

## 结论

🔴 必须修复项 2 个，均已修复。无遗留 🔴 项。审查通过。
