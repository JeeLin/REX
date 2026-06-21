# M22: 侧边栏收藏 + 最近使用 + 右键菜单增强

## Context

M21 完成了 Terminal 移动端浮动工具栏、状态栏增强、工具栏 i18n。侧边栏（AppLayout.vue）已有基本导航和环境资源树，但缺少收藏、最近使用、侧边栏右键菜单等 PRODUCT.md §4 描述的功能。

## 产品边界

**做什么：**
- 收藏功能：侧边栏显示收藏资源列表，支持添加/取消收藏
- 最近使用：侧边栏显示最近连接的资源，支持清除
- 侧边栏右键菜单：资源项、环境节点的右键操作

**不做什么：**
- 不实现全局搜索命令面板（Ctrl+K）
- 不实现侧边栏拖拽排序
- 不实现环境树的展开/折叠动画

---

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 22.1 | 收藏功能 | ✅ |
| 22.2 | 最近使用 | ✅ |
| 22.3 | 侧边栏右键菜单 | ✅ |

---

## 子任务详细设计

### 22.1 收藏功能

**功能目标：**
在侧边栏显示用户收藏的资源列表，支持快速访问。

**修改文件：**
```text
packages/rex-console-web/src/
├── composables/
│   └── useSidebar.ts                  修改：添加收藏相关方法
├── layouts/
│   └── AppLayout.vue                  修改：添加收藏区域
├── i18n/zh.ts, en.ts                 修改：添加收藏 i18n 键
```

**交互设计（参考 PRODUCT.md §4）：**

```text
侧边栏收藏区域（位于环境树下方）：

┌────────────────────────────┐
│  收藏                        │  ← 可折叠
│  ★ 云服务器                │
│  ★ 主数据库                  │
│  ★ Web文件服务器             │
└────────────────────────────┘

添加收藏：
- 资源项右键菜单 → 添加到收藏
- Dashboard 快速连接项右键 → 添加到收藏

取消收藏：
- 收藏项右键菜单 → 取消收藏
- 收藏项点击 X 按钮
```

**useSidebar.ts 修改：**
- 新增 `favorites` ref（从 localStorage 读取）
- `addFavorite(resourceId: string)` — 添加收藏
- `removeFavorite(resourceId: string)` — 取消收藏
- `isFavorite(resourceId: string)` — 检查是否收藏

**AppLayout.vue 修改：**
- 在环境树和 footer 之间添加收藏区域
- 收藏项点击连接到资源
- 收藏项右键菜单（取消收藏、在工作区打开）

**i18n 键：**
| 键 | 中文 | English |
|---|------|---------|
| `sidebar.favorites` | 收藏 | Favorites |
| `sidebar.favoritesEmpty` | 暂无收藏 | No favorites yet |
| `sidebar.addFavorite` | 添加到收藏 | Add to Favorites |
| `sidebar.removeFavorite` | 取消收藏 | Remove from Favorites |

**测试标准：**
- 收藏区域显示在侧边栏正确位置
- 添加/取消收藏后列表实时更新
- 收藏数据持久化到 localStorage
- 空收藏时显示提示文字
- 收藏项点击连接到资源

**提交信息：**
```
feat: add favorites section to sidebar
```

---

### 22.2 最近使用

**功能目标：**
在侧边栏显示最近连接的资源列表，帮助用户快速重新连接。

**修改文件：**
```text
packages/rex-console-web/src/
├── composables/
│   └── useSidebar.ts                  修改：添加最近使用方法
├── layouts/
│   └── AppLayout.vue                  修改：添加最近使用区域
├── i18n/zh.ts, en.ts                 修改：添加最近使用 i18n 键
```

**交互设计（参考 PRODUCT.md §4）：**

```text
侧边栏最近使用区域（位于收藏下方）：

┌────────────────────────────┐
│  最近使用                    │  ← 可折叠
│  ● 云服务器     5分钟前    │
│  ● 主数据库       1小时前    │
│  ● Web文件服务器   昨天      │
│  ● 分析数据库     2天前      │
└────────────────────────────┘

添加到最近使用：
- 连接资源时自动添加
- 已有的资源更新时间戳

清除：
- 右键菜单 → 从最近使用中移除
- 右键菜单 → 清空最近使用
```

**useSidebar.ts 修改：**
- 新增 `recentConnections` ref（从 localStorage 读取）
- `addRecent(resourceId: string, name: string)` — 添加/更新最近使用
- `removeRecent(resourceId: string)` — 移除
- `clearRecent()` — 清空所有
- 最多保留 10 条，按时间倒序

**AppLayout.vue 修改：**
- 在收藏区域下方添加最近使用区域
- 显示时间差（5分钟前、1小时前、昨天等）
- 右键菜单（打开、移除、清空）

**i18n 键：**
| 键 | 中文 | English |
|---|------|---------|
| `sidebar.recent` | 最近使用 | Recent |
| `sidebar.recentEmpty` | 暂无记录 | No recent items |
| `sidebar.removeRecent` | 从最近使用中移除 | Remove from Recent |
| `sidebar.clearRecent` | 清空最近使用 | Clear Recent |

**测试标准：**
- 最近使用区域显示在侧边栏正确位置
- 连接资源后自动添加到最近使用
- 时间差显示正确（刚连接、5分钟前、1小时前、昨天等）
- 清空功能正常工作
- 最多保留 10 条记录
- 数据持久化到 localStorage

**提交信息：**
```
feat: add recent connections section to sidebar
```

---

### 22.3 侧边栏右键菜单

**功能目标：**
为侧边栏资源项和环境节点添加完整的右键菜单操作。

**修改文件：**
```text
packages/rex-console-web/src/
├── layouts/
│   └── AppLayout.vue                  修改：完善右键菜单功能
├── i18n/zh.ts, en.ts                 修改：添加菜单 i18n 键
```

**交互设计（参考 PRODUCT.md §4）：**

资源项右键菜单：
```text
连接
在新标签中连接
─── 分隔线 ───
编辑资源
删除资源
─── 分隔线 ───
复制地址
添加到收藏
```

环境节点右键菜单：
```text
打开环境详情
─── 分隔线 ───
在工作区打开所有资源
新建资源
─── 分隔线 ───
编辑环境
删除环境
```

**AppLayout.vue 修改：**
- 完善 `onResourceItemCtx`：添加编辑、删除、复制地址、添加到收藏
- 完善 `onEnvGroupCtx`：添加打开详情、打开所有资源、新建资源、编辑、删除
- 连接操作调用 `connectToResource`
- 新标签连接：在工作空间中打开新标签
- 复制地址：复制资源名称到剪贴板
- 添加到收藏：调用 `addFavorite`
- 删除操作显示确认弹窗

**i18n 键：**
| 键 | 中文 | English |
|---|------|---------|
| `ctx.connect` | 连接 | Connect |
| `ctx.connectNewTab` | 在新标签中连接 | Open in New Tab |
| `ctx.editResource` | 编辑资源 | Edit Resource |
| `ctx.deleteResource` | 删除资源 | Delete Resource |
| `ctx.copyAddress` | 复制地址 | Copy Address |
| `ctx.addToFavorite` | 添加到收藏 | Add to Favorites |
| `ctx.openDetail` | 打开环境详情 | Open Environment Detail |
| `ctx.openAllWorkspace` | 在工作区打开所有资源 | Open All in Workspace |
| `ctx.newResource` | 新建资源 | New Resource |
| `ctx.editEnv` | 编辑环境 | Edit Environment |
| `ctx.deleteEnv` | 删除环境 | Delete Environment |

**测试标准：**
- 资源项右键显示完整菜单
- 环境节点右键显示完整菜单
- 所有菜单项功能正常（连接、编辑、删除、复制、收藏）
- 删除操作显示确认弹窗
- 收藏操作实时更新收藏列表

**提交信息：**
```
feat: enhance sidebar context menus with full actions
```

---

## 设计核对点

- [ ] 收藏区域与 PRODUCT.md §4 一致
- [ ] 最近使用区域与 PRODUCT.md §4 一致
- [ ] 右键菜单与 PRODUCT.md §4 一致
- [ ] 数据持久化到 localStorage
- [ ] 收藏和最近使用最多保留 10 条
- [ ] 时间差显示格式正确

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
