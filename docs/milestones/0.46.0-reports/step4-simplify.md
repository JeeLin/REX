# 0.46.0 Step 4: Simplify

## Simplify Date: 2026-07-03

## Changes Reviewed

| File | Lines Added | Assessment |
|---|---|---|
| ws_terminal.rs | +10 | Ping/pong handler — minimal, no redundancy |
| WorkspaceTerminal.vue | +74 | Latency state, ping logic, template, CSS — clean additions |
| Terminal.vue | ~5 lines changed | i18n replacement only |
| en.ts / zh.ts | +2 each | pasteHint + openSftpNewTab keys |

## Checks

- **Duplicate code**: None. Two latency CSS classes (`.ws-term-latency` and `.ws-term-latency-status`) serve different contexts (dark toolbar vs orange status bar).
- **Over-engineering**: None. Ping interval is a simple setInterval; no unnecessary abstractions.
- **Premature optimization**: None.
- **File splitting**: No need — all changes are cohesive within existing files.
- **Workspace = true**: No new Rust dependencies added.

## Conclusion

All additions are minimal and focused. No simplification needed.

**结论: ✅ 无需精简**
