# REX Hub Decision Tree

Based on the codebase patterns and project structure, I'll create a comprehensive decision tree to help navigate this project.

## Project Context
REX Hub is a self-hosted remote resource unified management platform for personal developers and operations engineers. It's single-user, self-hosted, and dark-mode first.

## Navigation Patterns

### 1. Code Organization Decision Tree
**Where is code located?**
- If it's **Rust backend logic**: `crates/`
  - Common utilities: `rex-common/`
  - Hub-specific: `rex-hub/`
  - Agent-specific: `rex-agent/`
  - Protocol implementations: `rex-ssh/`, `rex-mysql/`, etc.
  - File transfer: `rex-transfer/`

- If it's **Vue frontend**: `packages/rex-console-web/`
  - Page-level components: `src/pages/`
  - Feature-specific components: `src/features/`
    - Terminal features: `src/features/terminal/`
    - SQL features: `src/features/sql/`
    - File features: `src/features/files/`
  - Shared components: `src/components/`
  - API calls: `src/api/`
  - State management: `src/stores/`
  - i18n: `src/i18n/`

### 2. Development Workflow Decision Tree
**What phase of development?**
1. **Planning**: Start with `docs/PRODUCT.md` and `docs/DEVELOPMENT.md`
2. **Implementation**: Follow the 8-step milestone process in `CLAUDE.md`
3. **Testing**: Use `cargo test --workspace` for Rust, `npm run test` for frontend
4. **Deployment**: See `docs/architecture/` for Docker and deployment patterns

### 3. Feature Implementation Decision Tree
**Building a new feature?**
1. Check `docs/PRODUCT.md` for feature boundaries
2. Create milestone document in `docs/milestones/`
3. Follow 8-step process:
   - Write development document
   - Design review
   - Development (frontend + backend together)
   - Simplify
   - Code review
   - Testing (100% coverage)
   - Design reconfirmation
   - Commit + complete milestone

### 4. Architecture Decision Tree
**Where does functionality belong?**
- **Hub processes**: All user-facing features, API endpoints, WebSocket handlers
- **Agent processes**: Reverse proxy, resource scanning, heartbeat management
- **Shared crates**: Protocol implementations, file transfer, common utilities

### 5. Technology Stack Decision Tree
**What technology is used for...?**
- **Backend**: Rust with tokio async runtime
- **Frontend**: Vue 3 + Vite + TypeScript
- **Terminal**: xterm.js
- **Communication**: WebSocket + HTTPS
- **Encryption**: TLS 1.3, AES-256-GCM, ECDHE-X25519
- **Database**: SQLite (local to Hub)

## Key Files Reference

| Purpose | File Path |
|---------|-----------|
| Project instructions | `CLAUDE.md` |
| Product documentation | `docs/PRODUCT.md` |
| Development index | `docs/DEVELOPMENT.md` |
| Architecture docs | `docs/architecture/` |
| Reference docs | `docs/reference/` |
| Milestone docs | `docs/milestones/` |
| Prototype HTML | `prototype/` |
| Rust workspace config | `Cargo.toml` |
| Frontend package | `packages/rex-console-web/package.json` |
