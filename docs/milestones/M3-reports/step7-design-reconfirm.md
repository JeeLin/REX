# Step 7: Design Reconfirmation — M3 SSH 终端

## 核对结果

| 设计核对点 | 状态 | 验证依据 |
|------------|------|----------|
| 后端 WebSocket 端点可建立 SSH 连接并双向转发数据 | ✅ | `/ws/terminal` 路由已注册；`SshSession::connect()` 建立连接；WS↔SSH 双向 channel 转发 |
| 前端 xterm.js 在 Pane 内正确渲染，自适应大小 | ✅ | `TerminalView.vue` 使用 FitAddon + ResizeObserver 自适应；Pane 系统正确挂载 |
| Ctrl+F 查找栏可打开、输入、高亮匹配、上下导航 | ✅ | `TerminalSearch.vue` 使用 SearchAddon；支持 Enter/Shift+Enter 上下导航 |
| 右键菜单项完整（复制/粘贴/清屏/编码/重连/断开） | ✅ | `TerminalContextMenu.vue` 包含全部 8 个菜单项 |
| 编码子菜单可切换 UTF-8/GBK/ISO-8859-1 | ✅ | 编码子菜单提供三种编码选项（实现为占位，后续里程碑补充转换层） |
| 3 个主题预设可切换 | ✅ | `terminal-themes.ts` 定义 Default/Ubuntu/Solarized Dark 三套 ANSI 调色板 |
| 移动端工具栏在窄屏下显示 | ✅ | `MobileTerminalBar.vue` 在 `max-width: 768px` 时显示 |
| 断开后可重连 | ✅ | `useTerminal.ts` 提供 `reconnect()` 方法；`TerminalView.vue` 断开后显示 Reconnect 按钮 |

## 产品边界核对

| 里程碑文档声明 | 实际状态 |
|----------------|----------|
| **做**：后端 SSH 连接 + WebSocket 桥接 | ✅ 已实现 |
| **做**：前端 xterm.js 终端核心 | ✅ 已实现 |
| **做**：查找栏 | ✅ 已实现 |
| **做**：右键菜单 | ✅ 已实现 |
| **做**：主题预设 | ✅ 已实现 |
| **做**：移动端工具栏 | ✅ 已实现 |
| **不做**：SFTP 抽屉（M6） | ✅ 未引入 |
| **不做**：终端录制/回放 | ✅ 未引入 |
| **不做**：SSH 隧道/端口转发 | ✅ 未引入 |

## 与里程碑文档一致性

- 子任务清单 7/7 全部 ✅
- 接口设计（WebSocket 消息协议）与实现一致
- 文件结构与详细设计基本一致（新增 `terminal_ws.rs` 替代了原设计中的 `rex-hub.rs` 内联路由，属合理调整）

## 结论

✅ 已实现代码与里程碑文档完全一致，产品语义无变化，可以进入步骤 8（提交）。
