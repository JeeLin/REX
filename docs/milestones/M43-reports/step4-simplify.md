# M43 Step 4: Code Simplification Report

## Changed Files (M43 milestone)

| File | Changes |
|------|---------|
| `features/resource-panel/ResourcePanel.vue` | Removed dead code |
| `features/terminal/useTerminal.ts` | Clean (no issues) |
| `layouts/AppLayout.vue` | Removed unused import |
| `pages/WorkspacePage.vue` | Pre-existing issues (not introduced by M43) |
| `stores/environments.ts` | Clean (no issues) |
| `stores/workspace.ts` | Clean (no issues) |

## Findings & Fixes

### ResourcePanel.vue — Fixed
- **Unused import**: Removed `import type { Environment }` (never referenced)
- **Dead variable**: Removed `expandedEnvIds` ref (declared but never read/written)
- **Dead emit**: Removed `defineEmits<{ openResource: ... }>()` (emit never called; `wsStore.openResource()` used instead)
- **Type error fix**: `store.envResources.value` → `store.envResources` (Pinia auto-unwraps ref state)
- **Type error fix**: `res.environmentId` → `res.environment_id` (snake_case API field)
- **Type error fix**: `res.port` → `res.port ?? undefined`, `res.color` → `res.color ?? undefined` (null→undefined for optional params)

### AppLayout.vue — Fixed
- **Unused import**: Removed `watch` from vue import (was used by collapsed state watcher, now removed)

### WorkspacePage.vue — Not Fixed (pre-existing)
- `getBroadcastTargets()` and `onBroadcastInput()` dead code — pre-existing, not from M43
- `closePane(idx)` unused parameter — pre-existing
- Nested ternary readability — pre-existing
- `onPropsSave(data: any)` type safety — pre-existing

## Quality Gates

| Check | Result |
|-------|--------|
| `bun run type-check` | ✅ 0 errors |
| `bun run lint` | ✅ 0 errors, 55 warnings (all pre-existing) |

## Conclusion

Simplification is complete. All M43-introduced dead code and type errors resolved. Pre-existing issues in WorkspacePage.vue left untouched (not in scope).
