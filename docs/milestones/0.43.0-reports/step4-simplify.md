# 0.43.0 Step 4: Code Simplification

## Analysis Date: 2026-07-03

## Changes Analyzed

- `crates/rex-common/src/sql.rs` — ViewInfo struct, list_views trait method
- `crates/rex-mysql/src/connector.rs` — list_views implementation
- `crates/rex-postgresql/src/connector.rs` — list_views implementation
- `crates/rex-hub/src/sql.rs` — list_views API handler
- `crates/rex-hub/src/routes.rs` — route registration
- `packages/rex-console-web/src/api/sql.ts` — ViewInfo type, listViews function
- `packages/rex-console-web/src/features/sql/SqlSidebar.vue` — view nodes, context menu
- `packages/rex-console-web/src/i18n/zh.ts` — i18n keys
- `packages/rex-console-web/src/i18n/en.ts` — i18n keys

## Simplification Checks

### 1. Duplicate Code

- `loadColumnsForView` vs `loadColumnsForTable`: Same logic but different state variables. Only 2 call sites each, extraction adds complexity. **No action needed.**
- `toggleView` vs `toggleTable`: Similar structure but different behavior (table emits `select-table`, view doesn't). **No action needed.**
- `tree-col-item` template duplicated in table and view sections: Vue template extraction requires prop definitions, not worth it for 6 lines. **No action needed.**

### 2. Over-engineering

- View state (`views`, `viewColumns`, `viewExpanded`) is separate from table state, which is correct for independent expansion/collapse behavior.
- No premature optimization or future capability implementation.

### 3. Large File Split

- `SqlSidebar.vue` is now ~570 lines, which is within acceptable range for a single-file component.
- No need to split.

### 4. Dependency Rules

- All Rust crates use `workspace = true` correctly.
- No version duplication.

### 5. Project Style

- Code follows existing patterns (ref, computed, watch, async functions).
- CSS uses CSS variables consistently.
- i18n keys follow naming convention (`sql.tree.ctx.*`, `sql.viewLabel`, `sql.toast.*`).

## Conclusion

Code is already clean and consistent. No simplification changes needed.
