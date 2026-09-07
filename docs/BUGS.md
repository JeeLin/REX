# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
| v0.73.1 | 🟡 | 设置页保存按钮需要滚动到底部才能看到 | 用户反馈 | 设置页内容较多，保存按钮固定在页面最底部，用户需要滚动到最下面才能点击保存。应在页面顶部或固定位置显示保存按钮，或改为 sticky 底部栏 |
| v0.73.1 | 🟡 | Docker 启动没有日志输出 | 用户反馈 | 使用 Docker 启动 Hub/Agent 时控制台没有日志输出，排查问题困难。需要确保 Docker 模式下日志能正常输出到 stdout/stderr |
| v0.73.1 | 🟡 | 页面 subtitle i18n key 缺失 | 用户反馈 | `agents.subtitle` 和 `auditLog.subtitle` 在 zh.json/en.json 中未定义，页面显示 fallback 英文文本 |
| v0.73.1 | 🟡 | Windows service install 不支持 | 用户反馈 | `rex-agent.exe service install` 输出 `automatic service management is not supported on this platform`，提示用 nssm 或 Task Scheduler。应支持 Windows 原生服务注册（sc create / NSSM / WinSW） |
| v0.73.1 | 🟡 | version 子命令不显示 git hash | 用户反馈 | `rex-agent.exe version` 输出 `rex-agent 0.72.0` 而非 `rex-agent 0.73.1 (a1b2c3d)`。build.rs 通过 `REX_GIT_HASH` 注入 hash，但 `option_env!` 在 CI 构建时可能为空。需检查 CI workflow 是否设置 `GITHUB_SHA` 或 `cargo:rustc-env` |
| v0.73.1 | 🟡 | Agent 启动瞬间页面重连导致重复 SSH 连接 | 用户反馈 | Agent 启动后页面 WebSocket 重试机制触发两次连接请求，日志显示两个不同 request_id/channel_id 的 SSH 会话在 300ms 内相继建立。需在前端或 Hub 侧去重，避免重复连接 |
