# M49 Step 5: Code Review Report

**Review Date**: 2026-07-28
**Reviewer**: CodeReview Agent
**Files Reviewed**: 23 (19 frontend + 4 backend Rust)
**Milestone**: M49 — Connection Model Redesign

## Change Overview

This milestone refactors all protocol connect handlers (SQL, Redis, Files) from accepting raw connection parameters (host, port, password, etc.) from the frontend to accepting only a `resource_id`. A new shared helper `resource_conn.rs` loads resource records from the DB and decrypts `config_json` server-side. Frontend components were updated to pass `resource_id` instead of connection details.

### Changed Files

| Category | Files | Description |
|----------|-------|-------------|
| Backend | `resource_conn.rs` | New shared helper for loading resource configs from DB |
| Backend | `sql_api.rs` | SQL connect handler uses `load_resource_config` |
| Backend | `redis_api.rs` | Redis connect handler uses `load_resource_config` |
| Backend | `file_api.rs` | Files connect handler uses `load_resource_config` |
| Frontend API | `sql.ts`, `redis.ts`, `files.ts`, `settings.ts` | API functions now send `resource_id` only |
| Frontend Pages | `SqlPage.vue`, `RedisPage.vue`, `FilesPage.vue`, `FilesDrawer.vue` | Auto-connect via `resourceId` prop |
| Frontend Terminal | `TerminalView.vue`, `useTerminal.ts`, `MobileTerminalBar.vue` | Alt-screen guard, fit debounce, mobile arrow keys |
| Frontend Workspace | `WorkspacePage.vue`, `ResourceProperties.vue`, `WizardModal.vue` | Tab model simplified, S3 wizard fix |
| Frontend Other | `AgentsPage.vue`, `AuditLogPage.vue`, `SettingsPage.vue`, `AppLayout.vue` | Deployment guide, layout fixes |
| i18n | `en.json`, `zh.json` | Agent deployment guide translations |

---

## Security Review

### ✅ Sensitive Information Properly Hidden

All protocol handlers now accept only `resource_id` — no password, private_key, or config_json is sent from the frontend:

- **SQL**: `ConnectBody { type, resource_id }` → `load_resource_config()` → `ConnectRequest`
- **Redis**: `ConnectBody { resource_id }` → `load_resource_config()` → `RedisConnectRequest`
- **Files**: `ConnectBody { resource_id }` → `load_resource_config()` → SFTP/S3 connector

`resource_conn.rs` loads the full Resource record from DB and decrypts `config_json` server-side. Each handler extracts protocol-specific fields (password, private_key, database_name, access_key, etc.) from the decrypted config. **No sensitive data leaks to the frontend.**

### ✅ No Sensitive Data in Error Messages

Error responses from `load_resource_config` include the `resource_id` (UUID) for debugging but do not expose host, port, or credential values.

---

## Findings

### 🟡 MEDIUM-1: SQL connect handler trusts frontend `db_type` instead of DB record

**File**: `crates/rex-hub/src/sql_api.rs` (lines 143–173)
**Confidence**: 0.92

The SQL connect handler uses `body.db_type` (supplied by the frontend) to determine which connector to instantiate (MySQL, PostgreSQL, or SQLite). However, `res.protocol` from the DB record — loaded via `load_resource_config` — is the authoritative source of the protocol type. The frontend-derived `db_type` could theoretically mismatch the DB record, leading to confusing connection errors.

The `body.db_type` is used in two places:
1. **Line 146**: `match db_type.to_lowercase().as_str()` — builds the `ConnectRequest`
2. **Line 173**: `match db_type.to_lowercase().as_str()` — selects the connector

**Suggestion**: Derive `db_type` from `res.protocol`:
```rust
let db_type = res.protocol.clone();
```

---

### 🟡 MEDIUM-2: SQL connect handler missing audit log (inconsistent with Redis/Files)

**File**: `crates/rex-hub/src/sql_api.rs` (lines 196–207)
**Confidence**: 0.95

Redis connect writes `REDIS_CONNECT` audit log (redis_api.rs:218). Files connect writes `FILE_CONNECT` audit log (file_api.rs:219). SQL connect does **not** write any audit log on success — it only emits `tracing::info!`. SQL connections are invisible in the Audit Log page, breaking the audit trail consistency established across the other two protocol handlers.

**Suggestion**: Add audit log write after successful SQL connect, matching the Redis/Files pattern:
```rust
let audit_db = state.db.clone();
let audit_target = body.resource_id.clone();
let _ = tokio::task::spawn_blocking(move || {
    audit_db.write_audit_log(&crate::models::NewAuditEntry {
        action: "SQL_CONNECT".into(),
        target: Some(audit_target),
        result: "success".into(),
        ..Default::default()
    })
}).await;
```

---

### 🟡 MEDIUM-3: RedisPage.vue retains dead connection management code

**File**: `packages/rex-console-web/src/features/redis/RedisPage.vue` (lines 28–44)
**Confidence**: 0.97

The following are dead code from the old direct-connect model:
- `Connection` interface (with `host`, `port`, `password` fields)
- `connections` ref (local-only array)
- `showEditConnection`, `editingConnection`, `showDeleteConnection`, `deletingConnection` refs
- Functions: `editConnection`, `saveConnection`, `deleteConnection`, `confirmDeleteConnection`, `copyConnection`
- Edit/Delete modals in template (lines 726–767)
- `redis-conn-item--active` hardcoded to `false` (line 756)

This dead code operates on local-only state that never persists to the DB or affects backend connections. It adds confusion about the actual connection model and maintenance burden.

---

### 🟡 MEDIUM-4: FilesDrawer.vue `loadPanel` silently swallows errors

**File**: `packages/rex-console-web/src/features/files/FilesDrawer.vue` (lines 68–72)
**Confidence**: 0.90

The `loadPanel` function catches errors with `catch { p.entries = [] }` without setting any error state. When file listing fails (network error, permission denied, etc.), the user sees an empty file list with no error indication. The `doConnect` function in the same component properly shows errors via `error.value`, and the analogous `loadDir` in `FilesPage.vue` properly propagates errors, making this inconsistency noticeable.

**Suggestion**: Add error state to `loadPanel`:
```typescript
catch (e: unknown) { p.entries = []; error.value = e instanceof Error ? e.message : String(e) }
```

---

## Summary

| Severity | Count | Issues |
|----------|-------|--------|
| 🔴 Must Fix | 0 | — |
| 🟡 Should Fix | 4 | SQL db_type trust, SQL audit log, RedisPage dead code, FilesDrawer error swallowing |
| 🟢 Nice to Have | 0 | — |

## Conclusion

### ✅ PASS (0 🔴)

The core objective of M49 — **preventing sensitive connection parameters (password, private_key, config_json) from reaching the frontend** — is correctly implemented across all protocols. The `resource_conn.rs` helper provides a clean, unified pattern for loading and decrypting resource configs server-side.

The 4 🟡 findings are non-blocking:
- **MEDIUM-1** (SQL db_type trust) is a correctness hardening that prevents confusing errors on protocol mismatch
- **MEDIUM-2** (SQL audit log) is an inconsistency that should be fixed for audit trail completeness
- **MEDIUM-3** (RedisPage dead code) is technical debt from the migration
- **MEDIUM-4** (FilesDrawer error swallowing) is a minor UX gap

All findings are recommended for the next cycle but none block the M49 release.
