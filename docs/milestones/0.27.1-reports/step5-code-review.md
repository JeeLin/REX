# Step 5: Code Review Report

## Milestone: 0.27.1 UI 设计优化与无障碍

## Review Scope

6 commits: `de62f11` → `2639737` (0.27.1 scope)

## Findings

### 🔴 Must Fix

#### 1. WorkspaceFiles.vue: 15 hardcoded Chinese strings not converted to i18n

**File**: `packages/rex-console-web/src/features/workspace/panels/WorkspaceFiles.vue`

This file was modified in 0.27.1 (ConfirmDialog replacement, Toast additions, `useI18n` import added), but ~15 user-visible Chinese strings in the template were NOT converted to i18n keys:

| Line | Hardcoded | Should be |
|------|-----------|-----------|
| 12 | `📁 新建` | `t('files.newFolder')` |
| 13 | `📄 新建文件` | `t('files.newFile')` |
| 20 | `🗑 删除` | `t('files.delete')` |
| 23 | `项` | `t('files.items', { count: entries.length })` |
| 44 | `加载中...` | `t('files.loading')` |
| 51 | `新建文件夹` | `t('files.newFolder')` |
| 56 | `文件夹名称` | `t('files.folderName')` |
| 60 | `取消` | `t('common.cancel')` |
| 61 | `创建` | `t('files.createBtn')` |
| 69 | `新建文件` | `t('files.newFile')` |
| 74 | `文件名称` | `t('files.fileName')` |
| 78 | `取消` | `t('common.cancel')` |
| 79 | `创建` | `t('files.createBtn')` |
| 106 | `打开` | `t('files.open')` |
| 112 | `复制路径` | `t('files.copyPath')` |
| 119 | `删除` | `t('files.delete')` |
| 124 | `新建文件夹` | `t('files.newFolder')` |
| 127 | `新建文件` | `t('files.newFile')` |
| 131 | `刷新` | `t('files.refresh')` |

The i18n keys already exist in `zh.ts` and `en.ts` — the strings just need to be replaced.

### 🟡 Should Fix

*(none found in 0.27.1 scope)*

### 🟢 Optional Improvements

*(none found in 0.27.1 scope)*

## ARIA & Accessibility Review

| Component | Verdict |
|-----------|---------|
| ConfirmDialog | ✅ Focus trap correct (2 buttons, Tab cycle), ARIA attributes correct |
| ContextMenu | ✅ Keyboard navigation correct, `actionableItems` skips separators |
| CommandPalette | ✅ ARIA combobox pattern correct, `activeDescendantId` mapping correct |
| ToastProvider | ✅ `role="status"` + `aria-live="polite"` appropriate |
| AppLayout | ✅ Skip-to-content, landmarks, nav labels all correct |
| useId | ✅ Minimal, correct fallback |

## i18n Review

- zh.ts and en.ts are in sync ✅
- New keys follow existing naming conventions ✅
- `useI18n` imports added consistently ✅
- **Exception**: `WorkspaceFiles.vue` (see 🔴 #1 above)

## CSS Variable Review

- All hardcoded colors in Docker/ContainerList replaced with CSS vars ✅
- `SettingsSection.vue` `background: #fff` → `var(--bg-surface)` ✅
- Remaining hardcoded colors (RedisConsole, S3Console, BackupSection) are pre-existing and out of scope

## Error Handling Review

- All catch blocks in modified pages have user feedback (Toast or ErrorState) ✅
- WorkspaceTerminal error messages use i18n ✅
- TransferQueuePanel empty state uses i18n ✅

## Conclusion

**1 🔴 was found and fixed**: `WorkspaceFiles.vue` had ~15 hardcoded Chinese strings. Fixed in commit `e311c2e`. All i18n keys now properly used.

**No remaining 🔴 issues.**
