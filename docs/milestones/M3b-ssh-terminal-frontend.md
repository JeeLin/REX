# M3b: SSH 终端前端

## Context

M3a 完成了 SSH 终端后端：`rex-ssh` crate、SSH 凭据加密、`TerminalSession` 模型、WebSocket 终端数据通道（`ws_terminal.rs`）和 REST API（`POST /api/ssh/sessions`、`DELETE /api/ssh/sessions/:id`、`GET /ws/terminal/:id`）。M3b 在此基础上实现前端 terminal 页面，集成 xterm.js，实现完整的 SSH 终端交互体验。

## 产品边界

**做什么：**
- Terminal 页面全屏布局（隐藏侧边栏）
- xterm.js 终端集成
- WebSocket 双向数据通道（base64 编码）
- 终端 resize 通知后端
- 断开连接确认弹窗
- 终端右键菜单（复制/粘贴/清屏）

**不做什么：**
- 多标签页系统（M3b 后续迭代）
- SFTP 内置面板（M4）
- 移动端浮动工具栏 + bash 历史（M3b 后续迭代）
- Agent SSH 代理通道（M3b 后续迭代）
- 后端审计 SSH 连接/断开（M3b 后续迭代）

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 3b.1 | 安装 xterm.js 依赖 + 类型定义 | 前端 | ✅ |
| 3b.2 | Terminal API 客户端 | 前端 | ✅ |
| 3b.3 | Terminal 页面全屏布局 | 前端 | ✅ |
| 3b.4 | xterm.js 集成 + WebSocket 桥接 | 前端 | ✅ |
| 3b.5 | 右键菜单 + 断开确认 | 前端 | ✅ |

---

## 子任务 3b.1：安装 xterm.js 依赖

### 功能目标

安装 xterm.js 及其 fit 插件，添加 TypeScript 类型定义。

### 文件结构

```text
packages/rex-console-web/
├── package.json          修改：添加 xterm.js、xterm-fit 依赖
└── src/
    └── env.d.ts          修改：添加 xterm 类型声明
```

### 接口设计

无需接口设计，纯依赖安装。

### 测试标准

- `bun run type-check` 通过
- `bun run build` 通过

### 提交信息

```
feat: add xterm.js dependencies
```

---

## 子任务 3b.2：Terminal API 客户端

### 功能目标

创建 `src/api/terminal.ts`，封装 terminal REST API 调用（创建会话、删除会话）。WebSocket 连接在组件内直接管理。

### 文件结构

```text
packages/rex-console-web/src/
└── api/
    └── terminal.ts       新增：terminal REST API
```

### 接口设计

```typescript
// src/api/terminal.ts
import client from './client'

export interface CreateSessionRequest {
  resource_id: string
  cols: number
  rows: number
}

export interface CreateSessionResponse {
  session_id: string
}

export function createSession(data: CreateSessionRequest) {
  return client.post<ApiResponse<CreateSessionResponse>>('/ssh/sessions', data)
}

export function deleteSession(sessionId: string) {
  return client.delete(`/ssh/sessions/${sessionId}`)
}
```

### 测试标准

- TypeScript 类型检查通过

### 提交信息

```
feat: add terminal REST API client
```

---

## 子任务 3b.3：Terminal 页面全屏布局

### 功能目标

创建 `src/pages/Terminal.vue`，实现全屏布局（隐藏侧边栏）。从 `EnvironmentDetail.vue` 资源列表点击 SSH 资源跳转到此页面。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/
│   └── Terminal.vue      新增：Terminal 页面
├── router.ts             修改：添加 terminal 路由
└── layouts/
    └── FullScreenLayout.vue  修改：实现全屏布局
```

### 接口设计

路由：`/terminal/:resourceId`（全屏，不使用 AppLayout）

```typescript
// router.ts 新增路由
{
  path: '/terminal/:resourceId',
  name: 'terminal',
  component: () => import('./pages/Terminal.vue'),
  meta: { layout: 'fullscreen' },
}
```

### 前端交互

1. 路由参数 `resourceId` 标识要连接的 SSH 资源
2. 页面全屏，隐藏侧边栏和顶栏
3. 页面加载时调用 `POST /api/ssh/sessions` 创建会话
4. 建立 WebSocket 连接到 `/ws/terminal/:sessionId`

### 测试标准

- 页面可正常渲染
- 从环境详情页点击 SSH 资源可跳转

### 提交信息

```
feat: add terminal page with fullscreen layout
```

---

## 子任务 3b.4：xterm.js 集成 + WebSocket 桥接

### 功能目标

在 Terminal.vue 中集成 xterm.js，实现 WebSocket 双向数据桥接。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/
│   └── Terminal.vue      修改：集成 xterm.js
└── composables/
    └── useWebSocket.ts   新增：WebSocket 连接管理
```

### 接口设计

```typescript
// WebSocket 消息协议（与后端 ws_terminal.rs 对应）
interface TerminalMessage {
  type: 'terminal.input' | 'terminal.resize'
  payload: { data?: string; cols?: number; rows?: number }
}

interface TerminalOutput {
  type: 'terminal.output'
  payload: { data: string } // base64 encoded
}

interface TerminalError {
  type: 'terminal.error'
  payload: { message: string }
}

interface TerminalClosed {
  type: 'terminal.closed'
  payload: { exit_status: number }
}
```

### 后端流程

1. 创建 xterm.js 实例，挂载到 DOM
2. 监听 `terminal.onData` → base64 编码 → WebSocket 发送 `terminal.input`
3. 监听 `terminal.onResize` → WebSocket 发送 `terminal.resize`
4. WebSocket 收到 `terminal.output` → base64 解码 → `terminal.write()`
5. WebSocket 收到 `terminal.error` → 显示错误提示
6. WebSocket 收到 `terminal.closed` → 显示断开提示

### 测试标准

- 终端可正常显示输出
- 键盘输入可发送到后端
- resize 后端可接收

### 提交信息

```
feat: integrate xterm.js with WebSocket terminal bridge
```

---

## 子任务 3b.5：右键菜单 + 断开确认

### 功能目标

终端区域右键菜单（复制/粘贴/清屏），断开连接确认弹窗。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/
│   └── Terminal.vue      修改：右键菜单、断开确认
└── components/
    └── ConfirmDialog.vue 新增：通用确认弹窗
```

### 接口设计

无需接口设计，纯 UI 交互。

### 前端交互

1. 右键菜单项：
   - 复制（`terminal.selection.selectAll()` + `navigator.clipboard.writeText()`）
   - 粘贴（`navigator.clipboard.readText()` → `terminal.paste()`）
   - 清屏（`terminal.clear()`）
   - 断开连接（触发确认弹窗）

2. 断开确认弹窗：
   - 标题："断开连接？"
   - 描述："断开后当前会话将终止，未保存的工作可能会丢失。"
   - 按钮：取消 / 断开（红色）

3. 断开操作：
   - 调用 `DELETE /api/ssh/sessions/:sessionId`
   - 关闭 WebSocket
   - 跳转回来源页面

### 测试标准

- 右键菜单正常弹出
- 复制/粘贴功能正常
- 断开确认弹窗正常

### 提交信息

```
feat: add terminal context menu and disconnect confirmation
```

## 设计核对点

- [ ] 页面全屏布局符合原型 `terminal.html` 的结构
- [ ] 深色主题与终端背景一致（`#0D1117`）
- [ ] 右键菜单交互与原型一致
- [ ] 断开确认弹窗与原型一致
- [ ] 响应式：移动端隐藏桌面端元素（保留基础终端功能）

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交
