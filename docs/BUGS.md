# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.87.2 | ✅ | SSH 终端 vim 编辑 xterm.js 崩溃 | 用户反馈 | 已修复：降级 xterm.js 6.0→5.5，6.0 的 requestMode 有 ReferenceError bug |
| v0.87.2 | ✅ | SSH 终端粘贴功能异常 | 用户反馈 | 已修复：粘贴内容用 bracketed paste 模式包裹（ESC[200~...ESC[201~） |
| v0.87.2 | ✅ | Windows 服务启动超时错误 1053 | 用户反馈 | 已修复：实现 Windows SCM 协议（StartServiceCtrlDispatcher / SERVICE_RUNNING 状态上报） |
| v0.87.4 | 🟢 | 亮色主题过亮，优化长时间使用体验 | 用户反馈 | 亮色主题太亮，不适合长时间使用，希望后续版本降低亮度/对比度 |
| v0.87.4 | 🟡 | SSH 终端打开对应 SFTP 报错（并发 session） | 用户反馈 | failed to open session: Disconnected，提示 SSH server 可能不支持并发 session（MaxSessions≥2 或先断终端）；修复需动 SSH/SFTP 协议层会话管理，经用户确认归入缺陷池 |
