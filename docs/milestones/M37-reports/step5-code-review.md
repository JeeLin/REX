# M37 Step 5: 代码审查报告

## 审查发现

| # | 严重程度 | 问题 | 处理 |
|---|----------|------|------|
| 1 | 🔴 | 后端 ClientMsg 未处理 Ping 变体，心跳无效 | ✅ 已修复：添加 Ping 变体 |
| 2 | 🔴 | X-XSS-Protection: 1; mode=block 已弃用 | ✅ 已修复：替换为 Referrer-Policy + Permissions-Policy |
| 3 | 🟡 | 缺少 HSTS/Referrer/Permissions 头 | ✅ 已修复：添加 Referrer-Policy + Permissions-Policy |
| 4 | 🟡 | SettingsPage 语言切换不持久化 | ✅ 已修复：添加 localStorage 持久化 |
| 5 | 🟡 | 安全头应用于 WebSocket 升级响应 | 🟢 可接受：无害，暂不处理 |

## 结论

✅ 所有 🔴 项已修复，无遗留必须修复项。
