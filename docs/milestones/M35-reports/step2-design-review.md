# M35 Step 2: Design Review Report

## Review against PRODUCT.md

### Subtask 1: Shortcuts Panel (F1)

| Check | Result | Notes |
|-------|--------|-------|
| PRODUCT.md 3.5 mentions F1 | ✅ | "快捷键面板（F1）：分组展示所有快捷键" |
| Shortcut list matches §5 | ✅ | All shortcuts from §5 included in 4 groups |
| Single-user, self-hosted | ✅ | No multi-user concepts |
| Scope appropriate | ✅ | Display-only panel, no complex logic |

### Subtask 2: Terminal Encoding Submenu

| Check | Result | Notes |
|-------|--------|-------|
| PRODUCT.md 3.6 mentions encoding | ✅ | "编码 ▸ 子菜单（UTF-8 / GBK / ISO-8859-1 一键切换乱码时用）" |
| Encoding options match | ✅ | UTF-8, GBK, ISO-8859-1 as specified |
| Scope appropriate | ✅ | Frontend TextDecoder/TextEncoder approach, no backend changes needed |
| Not over-scoped | ✅ | Only encoding, not terminal search (deferred) |

### Subtask 3: Status Bar Enhancement

| Check | Result | Notes |
|-------|--------|-------|
| PRODUCT.md 3.5 mentions status bar | ✅ | "状态栏（底部，紧凑可点击）：协议+主机+端口、连接状态、终端尺寸、编码、广播/锁状态" |
| New indicators appropriate | ✅ | Encoding + broadcast are mentioned in spec |
| Not over-scoped | ✅ | Incremental enhancement, not rewrite |

### Subtask 4: Quick Connect Protocol Completion

| Check | Result | Notes |
|-------|--------|-------|
| PRODUCT.md 3.5 mentions Quick Connect | ✅ | "Quick Connect 栏：协议下拉 + 主机 + 端口 + 用户名 + 连接" |
| Auto-port is sensible | ✅ | Standard UX pattern, matches Xshell behavior |
| S3 extra fields | ✅ | S3 needs endpoint/access_key/secret_key/bucket |

## Scope Check

- ✅ All 4 subtasks are within PRODUCT.md feature boundaries
- ✅ No multi-user/RBAC concepts introduced
- ✅ Deferred items (terminal search, SFTP drawer) are clearly noted
- ✅ Each subtask is 1 commit granularity

## Conclusion

**✅ Pass** — All design items align with PRODUCT.md, scope is appropriate.
