# M17: 工作区标签右键菜单增强

## Context

M16 完成了资源创建测试连接、部署指南改进和移动端修复。标签右键菜单中"移动到面板"子菜单和"全部断开"菜单项在 PRODUCT.md 3.5 节中有定义，但尚未实现。

## 产品边界

**做什么：**
- 标签右键菜单增加"移动到面板"子菜单（仅分屏模式可用）
- 标签右键菜单增加"全部断开"菜单项

**不做什么：**
- 不实现面板间拖拽（属于后续里程碑）
- 不实现标签拖拽到面板的视觉反馈

---

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 17.1 | 标签右键菜单增加"移动到面板"子菜单 | ✅ |
| 17.2 | 标签右键菜单增加"全部断开"菜单项 | ✅ |

---

## 子任务详细设计

### 17.1 标签右键菜单增加"移动到面板"子菜单

**功能目标：**
在分屏模式下，通过标签右键菜单将标签内容移动到指定面板。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/workspace/TabBar.vue          修改：增加菜单项
├── features/workspace/useTabs.ts          修改：增加 moveToPanel 方法
├── i18n/zh.ts, en.ts                      修改：添加 i18n
```

**交互设计（参考 PRODUCT.md 3.5 节）：**

```text
关闭
关闭其他
关闭右侧
关闭左侧
关闭全部
─── 分隔线 ───
复制标签
移动到面板 ▸  面板 1 / 面板 2 / 面板 3 / 面板 4
─── 分隔线 ───
新建连接
全部断开
```

**移动到面板子菜单：**
- 仅在分屏模式（panelCount > 1）下显示
- 列出所有面板编号，当前所在面板灰显
- 点击后面板编号切换，标签内容移动到目标面板

**useTabs.ts 接口：**
```ts
function moveToPanel(tabId: string, panelIndex: number): void
```

**测试标准：**
- 单面板模式下不显示"移动到面板"菜单项
- 分屏模式下显示所有面板编号
- 当前面板灰显不可选
- 移动后面板内容正确切换

**提交信息：**
```
feat: add move-to-panel submenu in workspace tab context menu
```

---

### 17.2 标签右键菜单增加"全部断开"菜单项

**功能目标：**
在标签右键菜单末尾增加"全部断开"菜单项，断开所有标签的连接。

**修改文件：**
```text
packages/rex-console-web/src/
├── features/workspace/TabBar.vue          修改：增加菜单项
├── features/workspace/useTabs.ts          修改：增加 disconnectAll 方法
├── i18n/zh.ts, en.ts                      修改：添加 i18n
```

**交互设计（参考 PRODUCT.md 3.5 节）：**
- "全部断开"放在菜单末尾，与"新建连接"同行或下方
- 点击后断开所有标签的连接状态（将所有标签 status 设为 offline）

**useTabs.ts 接口：**
```ts
function disconnectAll(): void
```

**测试标准：**
- 点击"全部断开"后所有标签状态变为 offline
- 标签不关闭，只是断开连接
- 操作可逆（重新连接后恢复 online）

**提交信息：**
```
feat: add disconnect-all option in workspace tab context menu
```

---

## 设计核对点

- [x] 移动到面板子菜单与 PRODUCT.md 3.5 节一致
- [x] 全部断开与 PRODUCT.md 3.5 节一致
- [x] 单面板模式下不显示移动到面板
- [x] 分屏模式下面板编号正确
- [x] i18n 覆盖所有新增文字

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
