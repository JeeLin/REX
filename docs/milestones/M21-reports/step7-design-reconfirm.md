# M21 步骤7：设计再确认报告

## 确认维度

对照 M21 里程碑文档和 PRODUCT.md §3.6，逐项验证实现。

### 子任务 21.1：状态栏增强 ✅

| 检查项 | 结果 |
|--------|------|
| 左侧显示协议（SSH） | ✅ 硬编码 `SSH` |
| 左侧显示编码（UTF-8） | ✅ `t('ws.terminal.statusbar.encoding')` |
| 左侧显示终端尺寸（cols×rows） | ✅ `termSize.cols × termSize.rows`，实时更新 |
| 右侧显示连接方式（直连/Agent） | ✅ `props.connectionMode` prop 控制 |
| 右侧显示操作提示 | ✅ `t('ws.terminal.statusbar.hint')` |
| 尺寸随 resize 实时更新 | ✅ `terminal.onResize` 回调更新 `termSize` |

### 子任务 21.2：移动端浮动工具栏 ✅

| 检查项 | 结果 |
|--------|------|
| 仅移动端显示（≤768px） | ✅ `isMobile` ref + CSS `@media` 双重控制 |
| 桌面端不显示 | ✅ CSS `display: none` 默认 |
| 方向键发送 ANSI 转义序列 | ✅ `\x1b[A/B/C/D` |
| Tab/Enter/^C/^L 发送正确字符 | ✅ `\t`/`\r`/`\x03`/`\x0c` |
| 历史按钮 | ✅ 当前打开 more 菜单（CR-2 标记） |
| 粘贴弹窗 | ✅ textarea 输入 + 发送到终端 |
| 字体缩放 A-/A+ | ✅ 10-20px 范围，调整后 refit |
| 更多菜单 | ✅ 清屏、断开连接 |

### 子任务 21.3：工具栏按钮 i18n ✅

| 检查项 | 结果 |
|--------|------|
| 清屏按钮 i18n | ✅ `t('ws.terminal.toolbar.clear')` |
| 粘贴按钮 i18n | ✅ `t('ws.terminal.toolbar.paste')` |
| SFTP 按钮 i18n | ✅ `t('ws.terminal.toolbar.sftp')` |
| 断开按钮 i18n | ✅ `t('ws.terminal.toolbar.disconnect')` |
| 重连提示 i18n | ✅ `t('ws.terminal.reconnect.title')` |
| 重连按钮 i18n | ✅ `t('ws.terminal.reconnect.btn')` |
| 断开弹窗标题/描述 i18n | ✅ `t('ws.terminal.disconnect.title/desc')` |
| zh/en 双语覆盖 | ✅ 两份 i18n 文件均已更新 |

### 架构一致性

| 检查项 | 结果 |
|--------|------|
| 单用户模型 | ✅ 无多用户概念 |
| 文件不经过浏览器 | ✅ 不涉及文件传输 |
| 移动端检测使用 matchMedia | ✅ 不依赖 userAgent |
| 新增 prop 有默认值 | ✅ `connectionMode?` 可选 |

---

## 结论

✅ 实现与 M21 里程碑文档一致，产品语义正确。
