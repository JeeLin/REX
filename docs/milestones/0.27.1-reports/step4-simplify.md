# Step 4: Code Simplification Report

## Milestone: 0.27.1 UI 设计优化与无障碍

## Reviewed Files

Changed files (6 commits, 0.27.1 scope):

| Category | Files |
|----------|-------|
| Core components | `ConfirmDialog.vue`, `ContextMenu.vue`, `CommandPalette.vue`, `ToastProvider.vue` |
| New composables | `useId.ts` |
| Layout | `AppLayout.vue` |
| Pages | `Dashboard.vue`, `Environments.vue`, `EnvironmentDetail.vue`, `Agents.vue`, `Terminal.vue`, `Files.vue`, `Settings.vue`, `Workspace.vue` |
| Workspace panels | `WorkspaceTerminal.vue`, `WorkspaceFiles.vue` |
| Feature components | `AiAssistantPanel.vue`, `AiMessage.vue`, `FileList.vue`, `TransferQueuePanel.vue`, `ContainerList.vue`, `SettingsSection.vue` |
| i18n | `zh.ts`, `en.ts` |

## Findings

### 1. Duplicate i18n Key Structure: `ws.workspace` vs `ws.empty/shortcuts/nav/actions`

🟡 **Should fix (pre-existing, out of scope)**

The `ws` namespace contains both top-level keys (`ws.empty`, `ws.shortcuts`, `ws.nav`, `ws.actions`, `ws.fullscreen`) and a nested `ws.workspace` object with identical sub-keys. The `Workspace.vue` page uses `ws.workspace.*` while other components may reference the top-level versions. This duplication predates 0.27.1.

**Decision**: Out of scope for this milestone — pre-existing issue.

### 2. Unused i18n Keys

🟡 **Should fix (pre-existing, out of scope)**

- `resource.env` / `resource.envPlaceholder` — not referenced in any component (only `resource.env` is used as plain text in a different context)
- `redis.host/port/password/db/name` — the Redis creation form uses `resource.redis.*` instead

**Decision**: Out of scope — pre-existing unused keys.

### 3. Minor Duplicate Keys

🟢 **Optional improvement**

- `files.error` and `files.loadError` both mean "Failed to load" — could consolidate to one key
- `dashboard.environments` and `env.title` both mean "Environments" — acceptable as they serve different UI contexts (section heading vs page title)

### 4. ARIA Implementation Quality

✅ **Clean**

- `ConfirmDialog.vue`: Focus trap is minimal and correct (only 2 buttons, Tab cycle is sufficient). No over-engineering.
- `ContextMenu.vue`: Keyboard navigation correctly uses `actionableItems` computed to skip separators. `keyboardIdx` tracking is straightforward.
- `CommandPalette.vue`: `activeDescendantId` computed correctly maps flat index to option ID. `useId` provides stable IDs.
- `ToastProvider.vue`: Simple `role="status"` + `aria-live="polite"` — appropriate for non-critical notifications.

### 5. `useId` Composable

✅ **Clean**

- 17 lines, single responsibility
- Uses Vue 3.5+ `useId()` with try/catch fallback
- No over-engineering — the fallback counter is sufficient for non-setup contexts

### 6. CSS Variable Replacements

✅ **Clean**

- Hardcoded colors (`#3FB950`, `#F85149`, etc.) replaced with CSS variables (`--success`, `--danger`)
- `background: #fff` → `var(--bg-surface)` in SettingsSection
- All replacements are mechanical 1:1 substitutions, no behavioral change

### 7. i18n Key Addition Quality

✅ **Clean**

- New keys follow existing naming conventions (`component.subcomponent.key`)
- zh.ts and en.ts are in sync — same key structure
- `useI18n` imports added consistently where `t()` is used

## Simplification Actions

No code changes required. The 0.27.1 changes are mechanical and well-structured:

- ARIA additions are minimal and correct
- i18n replacements are consistent
- CSS variable substitutions are clean
- No new abstractions or over-engineering introduced

## Conclusion

**No simplification needed.** The milestone changes are clean, mechanical transformations that don't introduce new abstractions, duplicated logic, or over-engineering. Pre-existing issues (duplicate `ws.workspace` keys, unused `resource.env` keys) are documented but out of scope.
