# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.73.1 | 🟡 | Windows service install 不支持 | 用户反馈 | `rex-agent.exe service install` 输出 `automatic service management is not supported on this platform`，提示用 nssm 或 Task Scheduler。应支持 Windows 原生服务注册（sc create / NSSM / WinSW） |
| v0.73.1 | 🟡 | Agent 启动瞬间页面重连导致重复 SSH 连接 | 用户反馈 | Agent 启动后页面 WebSocket 重试机制触发两次连接请求，日志显示两个不同 request_id/channel_id 的 SSH 会话在 300ms 内相继建立。需在前端或 Hub 侧去重，避免重复连接 |
