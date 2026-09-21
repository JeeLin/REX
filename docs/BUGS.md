# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.87.2 | 🔴 | SSH 终端 vim 编辑 xterm.js 崩溃 | 用户反馈 | SSH 终端进入 vim 时 xterm.js 报 `Uncaught ReferenceError: i is not defined`（`La.requestMode`），可能导致终端功能异常 |
| v0.87.2 | 🔴 | SSH 终端粘贴功能异常 | 用户反馈 | 复制操作正常，但所有粘贴操作都有问题（无法粘贴或粘贴内容异常），可能与 xterm.js bracketed paste 模式处理有关 |
| v0.87.2 | ✅ | Windows 服务启动超时错误 1053 | 用户反馈 | 已修复：实现 Windows SCM 协议（StartServiceCtrlDispatcher / SERVICE_RUNNING 状态上报） |
