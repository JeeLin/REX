# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.74.1里程碑 | 🔴 | SSH终端vim打开文件后卡死 | v0.74.1里程碑 | 通过SSH终端使用vim打开文件后界面卡死无响应，可能是终端resize/数据流处理或xterm与SSH通道交互问题。需深入排查Agent SSH session的终端I/O流 |
| v0.74.1里程碑 | 🔴 | Agent连接SQL报unrecognized dialect | v0.74.1里程碑 | 同一环境SSH能通过Agent连接，但SQL连接失败：AGENT_CONNECT_FAILED: unrecognized dialect。Agent有SSH日志但无SQL日志，需改善detect_dialect错误恢复和日志 |
| v0.74.1里程碑 | 🔴 | 通过Agent连接jump server失败 | v0.74.1里程碑 | Agent SSH连接报错 AGENT_SSH_FAILED: SSH connection failed。Agent不支持SSH ProxyJump/跳板机连接，需实现ProxyJump功能 |
| v0.74.1里程碑 | 🟡 | Windows service install 不支持 | v0.74.1里程碑 | rex-agent.exe service install 输出 automatic service management is not supported on this platform。应支持Windows原生服务注册 |
| v0.74.1里程碑 | 🟡 | Agent启动瞬间页面重连导致重复SSH连接 | v0.74.1里程碑 | Agent启动后WebSocket重试触发两次连接请求，需在前端或Hub侧去重 |
| v0.74.1里程碑 | 🟡 | 刷新页面会跳出文件下载 | v0.74.1里程碑 | 浏览器刷新页面时触发文件下载弹窗，应为正常页面重新加载 |
| v0.74.1里程碑 | 🟡 | 无法访问IPv6服务 | v0.74.1里程碑 | 服务器无IPv6地址时无法连接IPv6目标。需支持双栈或IPv4→IPv6回退 |
