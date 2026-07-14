# Step 5: Code Review Report — 0.87.0

## Summary

Reviewed all Rust and Vue changes across 4 commits (85fe37e through 64f54e8).

## Findings

### 🟢 ws.rs — perform_update retry logic
- **Retry bounds**: MAX_RETRIES=3, backoff 1s/2s — reasonable for network errors
- **Non-retryable exits**: version mismatch, SHA256 missing, checksum mismatch all `return` immediately — correct, no infinite retry risk
- **Stream error handling**: triggers `continue` to retry — correct
- **SHA256 required**: eliminates blind trust, proper security improvement
- **Version gating**: `if let Some(served)` borrows header str, no unnecessary allocation

### 🟢 agent_download.rs — version fallback
- Both `download_agent` and `serve_local` use `rex_common::version::VERSION` as fallback — ensures Hub and Agent versions always match when built from same source
- `version` query param is optional, tracing-only — no behavioral change

### 🟢 update.rs — Hub reliability fixes
- **GitHub repo fix**: `user/rex` → `JeeLin/REX` — was a latent bug causing download failures
- **SHA256 verify**: `Ok(false)` returns error and cleans up staged file; `Err(e)` logs warning and continues (matches pre-existing "not mandatory" intent, but now observable)
- **Staged binary check**: `apply_update` validates file exists before `exit(10)` — prevents supervisor from replacing with missing binary

### 🟢 UpdateSection.vue — SHA256 display
- Truncated hash display with `title` for full hash on hover — standard pattern
- `.version-sha` CSS follows existing style conventions

## Conclusion

🟢 **No 🔴 or 🟡 issues found.** All changes are correct, follow project conventions, and improve reliability/security over the previous implementation.
