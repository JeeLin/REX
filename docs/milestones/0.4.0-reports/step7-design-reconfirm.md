# 0.4.0 步骤7：设计再确认报告

## 确认范围

已实现代码 vs 里程碑文档 `0.4.0-settings-connect-behavior.md`

## 逐项核对

### 子任务 1：终端设置连接实际行为

| 检查项 | 里程碑文档 | 实际实现 | 结果 |
|--------|-----------|---------|------|
| 字体大小 → xterm | watch terminalSettings.fontSize | ✅ Terminal.vue + WorkspaceTerminal.vue | ✅ |
| 字体族 → xterm | watch terminalSettings.fontFamily | ✅ 同上 | ✅ |
| 光标闪烁 → xterm | watch terminalSettings.cursorBlink | ✅ 同上 | ✅ |
| 保活间隔存储 | localStorage 持久化 | ✅ stores/settings.ts | ✅ |
| 共享 store | reactive store 单一数据源 | ✅ stores/settings.ts | ✅ |

### 子任务 2：安全设置连接实际行为

| 检查项 | 里程碑文档 | 实际实现 | 结果 |
|--------|-----------|---------|------|
| 会话超时 | useSessionTimeout composable | ✅ 监听 mousemove/keydown/touchstart | ✅ |
| 自动登出 | 超时后清除 token 跳转 /login | ✅ | ✅ |
| 审计日志开关 | 控制侧边栏入口显示 | ✅ securitySettings.auditEnabled → AppLayout computed | ✅ |
| 配置加密开关 | 明确标注不实现 | ✅ 用 pointer-events: none 禁用 | ✅ |

## 产品边界检查

| 检查项 | 结果 |
|--------|------|
| 单用户、自托管 | ✅ |
| 不引入新依赖 | ✅ |
| 不引入多用户/RBAC | ✅ |

## 结论

✅ 确认通过。
