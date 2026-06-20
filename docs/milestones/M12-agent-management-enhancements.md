# M12: Agent 管理增强 — 配置弹窗、日志查看器、令牌重置

## Context

M0-M11 完成了核心功能和右键菜单。Agent 管理页（§3.9）的 Agent 卡片已有基本展示和右键菜单，但配置弹窗、日志查看器、令牌重置三个功能尚未实现。M12 补充这三个功能。

## 产品边界

**做什么：**
- Agent 配置弹窗（查看基本信息、连接信息、注册令牌复制、自动更新开关）
- Agent 日志查看器（按级别筛选、自动滚动、终端风格展示）
- 令牌重置确认弹窗（警告说明、确认操作）

**不做什么：**
- Agent 重启指令（需后端接口，后续阶段）
- Agent 配置文件编辑（后续阶段）
- Agent 日志实时流（后续阶段，当前使用 mock 数据）

---

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 12.1 | Agent 配置弹窗 | 前端 | ✅ |
| 12.2 | Agent 日志查看器 | 前端 | ✅ |
| 12.3 | 令牌重置确认弹窗 | 前端 | ✅ |

---

## 子任务 12.1：Agent 配置弹窗

### 功能目标

点击 Agent 卡片的「配置」按钮或右键菜单「配置」，弹出模态面板显示 Agent 配置信息。

### 修改文件

```text
packages/rex-console-web/src/
├── features/agents/AgentConfigModal.vue   新增：配置弹窗组件
├── features/agents/AgentCard.vue          修改：添加配置按钮点击事件
├── i18n/zh.ts, en.ts                      修改：添加配置弹窗 i18n keys
```

### 交互设计

参考原型 `prototype/agents.html` 第 779-810 行：

1. **基本信息区块**：所属环境（只读）、Agent ID（只读）、版本（只读）
2. **连接区块**：服务器地址（只读）、注册令牌（点击复制，带复制反馈）
3. **更新区块**：自动更新开关（toggle）、当前版本（只读）

### Props

```ts
defineProps<{
  agent: Agent | null
  visible: boolean
}>()

defineEmits<{
  close: []
}>()
```

### 测试标准

- 点击「配置」按钮弹出弹窗
- 显示正确的 Agent 信息
- 点击注册令牌可复制到剪贴板
- 点击遮罩或 × 关闭弹窗
- ESC 关闭弹窗

### 提交信息

```
feat: add agent config modal
```

---

## 子任务 12.2：Agent 日志查看器

### 功能目标

点击 Agent 卡片的「日志」按钮或右键菜单「查看日志」，弹出日志查看器模态面板。

### 修改文件

```text
packages/rex-console-web/src/
├── features/agents/AgentLogModal.vue      新增：日志查看器组件
├── features/agents/AgentCard.vue          修改：添加日志按钮点击事件
├── i18n/zh.ts, en.ts                      修改：添加日志查看器 i18n keys
```

### 交互设计

参考原型 `prototype/agents.html` 第 813-838 行：

1. **工具栏**：级别筛选按钮（全部/INFO/WARN/ERROR/DEBUG）+ 自动滚动开关
2. **日志内容区**：终端风格深色背景，每行格式 `时间 [LEVEL] 消息`
3. **颜色编码**：INFO 蓝色、WARN 橙色、ERROR 红色、DEBUG 灰色
4. **底栏**：日志行数 · 实时更新 · 最近 1 小时

### Mock 数据

当前无后端日志 API，使用 mock 数据展示：

```ts
const mockLogs = [
  { time: '16:51:30', level: 'info', message: 'TLS 握手完成' },
  { time: '16:51:28', level: 'info', message: 'Token 验证成功' },
  { time: '16:50:15', level: 'info', message: 'SSH 会话建立 · root@192.168.1.100' },
  { time: '16:49:01', level: 'info', message: 'MySQL 代理隧道建立 · db.internal:3306' },
  { time: '16:48:22', level: 'warn', message: '延迟告警: 125ms (阈值 100ms)' },
  { time: '16:47:50', level: 'debug', message: '心跳发送 · latency=12ms' },
  { time: '16:47:00', level: 'info', message: '资源扫描完成 · 发现 4 个资源' },
  { time: '16:45:13', level: 'error', message: 'SSH 连接失败: Connection refused (port 22)' },
]
```

### 测试标准

- 点击「日志」按钮弹出查看器
- 级别筛选按钮切换生效
- 自动滚动开关正常
- 日志行颜色编码正确
- ESC 或点击遮罩关闭

### 提交信息

```
feat: add agent log viewer modal
```

---

## 子任务 12.3：令牌重置确认弹窗

### 功能目标

点击 Agent 卡片右键菜单「重置令牌」，弹出确认弹窗。

### 修改文件

```text
packages/rex-console-web/src/
├── features/agents/AgentResetTokenModal.vue  新增：令牌重置确认弹窗
├── features/agents/AgentCard.vue             修改：右键菜单接入重置令牌
├── i18n/zh.ts, en.ts                         修改：添加重置令牌 i18n keys
```

### 交互设计

参考原型 `prototype/agents.html` 第 842-858 行：

1. **标题**：「重置注册令牌？」
2. **警告说明**：「确定要重置 [Agent名] 的注册令牌吗？重置后该 Agent 将立即断开连接且无法重新连接。请在 Agent 所在机器上使用新令牌重新启动。」
3. **确认复选框**：「重置令牌并关闭所有该环境的连接」
4. **操作按钮**：取消（ghost）、确认重置（红色 danger）

### 测试标准

- 右键菜单点击「重置令牌」弹出确认弹窗
- 显示正确的 Agent 名称
- 取消按钮关闭弹窗
- 确认重置按钮为红色 danger 样式
- ESC 或点击遮罩关闭

### 提交信息

```
feat: add agent token reset confirmation modal
```

---

## 设计核对点

- [ ] 配置弹窗与原型 agents.html 一致（基本信息/连接/更新三个区块）
- [ ] 日志查看器与原型一致（级别筛选/自动滚动/颜色编码）
- [ ] 令牌重置弹窗与原型一致（警告说明/确认复选框/danger 按钮）
- [ ] 所有弹窗支持 ESC 和点击遮罩关闭
- [ ] i18n 覆盖所有新增文字

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交
