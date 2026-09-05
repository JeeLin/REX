# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
| v0.73.1 | 🟡 | 设置页保存按钮需要滚动到底部才能看到 | 用户反馈 | 设置页内容较多，保存按钮固定在页面最底部，用户需要滚动到最下面才能点击保存。应在页面顶部或固定位置显示保存按钮，或改为 sticky 底部栏 |
| v0.73.1 | 🟡 | Docker 启动没有日志输出 | 用户反馈 | 使用 Docker 启动 Hub/Agent 时控制台没有日志输出，排查问题困难。需要确保 Docker 模式下日志能正常输出到 stdout/stderr |
| v0.73.1 | 🔴 | Agent 启动时持续重复触发更新检查 | 用户反馈 | Agent 启动后每 30 秒触发一次 `update started`（日志显示 7 次以上），直到更新完成。根因：Hub 每次心跳检测版本不匹配都推送更新命令，Agent 无防重入机制，每次都 spawn 新的 update task。需要加锁或状态标记，仅允许一个 update task 运行 |
| v0.73.1 | 🔴 | Windows Agent 更新失败：无法 rename 正在运行的 exe | 用户反馈 | Windows 下 supervisor 尝试 rename staged binary 替换当前 exe 时报 `拒绝访问 (os error 5)`。Windows 不允许 rename/delete 正在运行的可执行文件，需要先 rename 当前 exe 为 .old 再 rename staged 为当前，或用 `MoveFileEx` + `MOVEFILE_REPLACE_EXISTING` + `MOVEFILE_DELAY_UNTIL_REBOOT` |
| v0.73.1 | 🟡 | 页面 subtitle i18n key 缺失 | 用户反馈 | `agents.subtitle` 和 `auditLog.subtitle` 在 zh.json/en.json 中未定义，页面显示 fallback 英文文本 |
