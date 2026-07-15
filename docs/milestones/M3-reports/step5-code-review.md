# Step 5: Code Review — M3 SSH 终端

## 审查范围

| 文件 | 变更类型 |
|------|----------|
| `crates/rex-ssh/src/lib.rs` | 修改：SSH 连接/会话实现 |
| `crates/rex-hub/src/terminal_ws.rs` | 修改：WebSocket 终端桥接 |
| `packages/rex-console-web/src/features/terminal/useTerminal.ts` | 修改：终端连接 composable |
| `packages/rex-console-web/src/features/terminal/TerminalView.vue` | 修改：终端组件 |
| `packages/rex-console-web/src/features/terminal/TerminalSearch.vue` | 新增：查找栏 |
| `packages/rex-console-web/src/features/terminal/TerminalContextMenu.vue` | 新增：右键菜单 |
| `packages/rex-console-web/src/features/terminal/MobileTerminalBar.vue` | 新增：移动端工具栏 |
| `packages/rex-console-web/src/features/terminal/terminal-themes.ts` | 新增：主题预设 |

---

## 发现

### 🟡 应该修复

**1. SSH 锁持有时间过长，可能阻塞 WebSocket 读写任务**

文件：`crates/rex-hub/src/terminal_ws.rs:171-213`

ssh_task 在 `tokio::select!` 中持有 `session` 锁，直到整个 select 分支执行完毕（含 base64 编解码、channel send）才释放。`ws_read_task` 和 `ws_write_task` 依赖 `cmd_tx`/`data_rx` channel，但 channel 满时（cmd_tx 容量 64，data_tx 容量 512），发送方会 await，而接收方（ssh_task）持有锁期间可能无法及时消费，造成短暂背压。

当前容量足够缓冲，实际风险低。但如果未来增加高频数据场景，可能成为瓶颈。

**建议**：将 SSH 操作与 channel 操作分离，SSH 读写只在持锁期间完成，channel send 放在锁外。

---

**2. `pasteClipboard` 降级时发送空字符串**

文件：`packages/rex-console-web/src/features/terminal/TerminalContextMenu.vue:37-40`

```ts
} catch {
  // 降级：直接 paste
  props.terminal.paste('')
}
```

剪贴板读取失败时（权限拒绝），降级为粘贴空字符串，用户无任何反馈。应提示用户或使用 `terminal.paste()`（无参数版本让 xterm 自行处理）。

---

**3. `TerminalSearch` 自动聚焦使用 `document.querySelector('.ts-input')`**

文件：`packages/rex-console-web/src/features/terminal/TerminalSearch.vue:71-74`

```ts
setTimeout(() => {
  const input = document.querySelector('.ts-input') as HTMLInputElement
  input?.focus()
  input?.select()
}, 50)
```

使用全局选择器 `.ts-input` 定位输入框，若页面有多个 `.ts-input` 可能聚焦到错误元素。应使用 `ref` 直接引用模板中的 input 元素。

---

**4. 主题切换未持久化**

文件：`packages/rex-console-web/src/features/terminal/useTerminal.ts:56-70`

`applyTheme` 只修改当前 Terminal 实例的 `options.theme`，刷新或新建 Tab 后回到默认主题。里程碑文档明确说"M7 设置模块预留接口，本里程碑通过代码切换"，所以当前行为可接受。但应至少用 `localStorage` 临时持久化用户选择，避免每次刷新丢失。

---

### 🟢 可选改进

**5. `terminal_ws.rs` 中 JSON 控制消息的判断方式**

文件：`crates/rex-hub/src/terminal_ws.rs:220`

```rust
let msg = if data.starts_with('{') {
```

通过检查字符串首字符是否为 `{` 来区分 JSON 控制消息和 base64 数据。base64 字符集不含 `{`，所以逻辑正确，但属于隐式约定。可加注释说明。

---

**6. `useTerminal.ts` 中 `sendInput` 的类型断言**

文件：`packages/rex-console-web/src/features/terminal/useTerminal.ts:158-163`

```ts
const wsRef = options.ws as Ref<WebSocket | null>
```

通过 `as` 类型断言绕过 `readonly` 限制，虽然是有意设计（内部使用），但可考虑将 `ws` 从 `readonly` 改为普通 `Ref`，或提供内部写入方法。

---

**7. 会话 ID 格式**

文件：`crates/rex-hub/src/terminal_ws.rs:87`

```rust
let session_id = format!("sess_{}", &uuid::Uuid::new_v4().to_string()[..8]);
```

取 UUID 前 8 位作为会话 ID，碰撞概率极低但非零。对于调试用途足够，可保持现状。

---

## 总结

| 级别 | 数量 |
|------|------|
| 🔴 必须修复 | 0 |
| 🟡 应该修复 | 4 |
| 🟢 可选改进 | 3 |

**结论**：✅ 无 🔴 必须修复项，可以进入步骤 6。

🟡 项为可改善但不阻塞发布的问题，建议在后续迭代中修复。整体代码结构清晰，Rust 后端通道分离合理，前端 composable 封装得当，功能实现与里程碑文档一致。
