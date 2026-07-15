# M8: 基础设施层

## Context

M0–M7 完成了设计系统、组件库和 UI 壳，4 个功能模块（SQL/Redis/Files/Terminal）有真实 API 层但未嵌入工作区，6 个页面是纯假数据。M8 建立所有后续功能依赖的地基：数据库持久化、认证系统、路由框架、前端状态管理。

本里程碑版本类型：minor（新功能），版本号 0.8.0 → 0.9.0。

## 产品边界

**本阶段做：**
- SQLite 数据库 schema + 迁移 + 连接池
- 单用户密码认证（argon2 + JWT）
- axum 路由框架重构（统一 AppState + auth 中间件）
- 审计日志写入基础设施
- 前端 API 客户端封装（统一 fetch + auth header 注入）
- 前端 auth Pinia store + 路由守卫
- 登录页真实认证 + 首次设置密码

**本阶段不做：**
- 环境/资源 CRUD API（M9）
- 工作区 Tab 嵌入（M10）
- Agent 管理（M12）
- 凭据 AES 加密（M14，M8 先用明文占位）
- i18n 完整翻译（M14）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | SQLite schema + Database struct + 迁移 | ✅ |
| 2 | 认证系统 + AppState 重构 + 路由重组（合并，紧耦合） | ✅ |
| 3 | 现有 API 模块注入 auth header + WebSocket token 认证 | ✅ |
| 4 | 前端 API 客户端封装（client.ts） | ✅ |
| 5 | auth Pinia store | ⬜ |
| 6 | 路由守卫 + 登录页改造 | ⬜ |
| 7 | 首次设置密码页面 | ⬜ |

## 子任务详细设计

### 1 SQLite schema + Database struct + 迁移

**功能目标**

建立 SQLite 持久化层，为环境、资源、Agent、审计日志、设置提供存储。

**文件结构**

新建：
- `crates/rex-hub/src/db.rs` — Database struct + 连接池 + 迁移
- `crates/rex-hub/src/models.rs` — 所有数据模型 Rust struct（Serialize/Deserialize）

修改：
- `crates/rex-hub/Cargo.toml` — 添加 `rusqlite`（bundled）、`r2d2`、`r2d2_sqlite` 依赖
- 根 `Cargo.toml` — workspace.dependencies 中添加 `r2d2_sqlite = "0.2"`
- `crates/rex-hub/src/lib.rs` — 添加 `pub mod db; pub mod models;`

**数据库路径**

`REX_DATA_DIR/rex.db`（默认 `~/.rex/rex.db`），目录不存在时自动创建。

**Schema 设计**

对齐 `docs/reference/data-models.md`，调整如下：

```sql
-- 环境
CREATE TABLE IF NOT EXISTS environments (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL UNIQUE,
  description   TEXT DEFAULT '',
  connection_mode TEXT NOT NULL DEFAULT 'direct',  -- 'direct' | 'agent'
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL
);

-- 资源（M8 先建表，M9 完善 CRUD）
CREATE TABLE IF NOT EXISTS resources (
  id              TEXT PRIMARY KEY,
  environment_id  TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
  name            TEXT NOT NULL,
  protocol        TEXT NOT NULL,           -- ssh/mysql/postgresql/redis/sqlite/sftp/s3
  host            TEXT NOT NULL,
  port            INTEGER,
  username        TEXT DEFAULT '',
  config_json     TEXT NOT NULL DEFAULT '{}',  -- 协议特有配置（含密码，M14 加密）
  color           TEXT,
  sort_order      INTEGER DEFAULT 0,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);

-- Agent（M8 先建表，M12 完善）
CREATE TABLE IF NOT EXISTS agents (
  id              TEXT PRIMARY KEY,
  environment_id  TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
  name            TEXT NOT NULL,
  token_hash      TEXT NOT NULL,
  version         TEXT DEFAULT '',
  os              TEXT DEFAULT '',
  arch            TEXT DEFAULT '',
  hostname        TEXT DEFAULT '',
  ip              TEXT DEFAULT '',
  status          TEXT NOT NULL DEFAULT 'offline',
  last_seen_at    TEXT,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);

-- 审计日志
CREATE TABLE IF NOT EXISTS audit_log (
  id              TEXT PRIMARY KEY,
  time            TEXT NOT NULL,
  action          TEXT NOT NULL,
  target          TEXT,
  environment_id  TEXT,
  resource_id     TEXT,
  agent_id        TEXT,
  result          TEXT NOT NULL DEFAULT 'success',
  detail          TEXT DEFAULT ''
);

-- 设置（KV 存储）
CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

**Database struct**

```rust
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

impl Database {
    /// 打开或创建数据库，运行迁移
    pub fn open(path: &Path) -> Result<Self> {
        std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))?;
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::new(manager).map_err(|e| RExError::Message(e.to_string()))?;
        let db = Self { pool };
        db.run_migrations()?;
        Ok(db)
    }

    /// 获取连接
    pub fn conn(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool.get().map_err(|e| RExError::Message(e.to_string()))
    }

    /// 执行迁移
    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn()?;
        conn.execute_batch(include_str!("migrations.sql"))?;
        Ok(())
    }

    // --- 设置 ---
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> { ... }
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> { ... }

    // --- 审计日志 ---
    pub fn write_audit_log(&self, entry: &NewAuditEntry) -> Result<()> { ... }
    pub fn query_audit_log(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> { ... }

    // --- 环境（基础 CRUD，M9 完善）---
    pub fn list_environments(&self) -> Result<Vec<Environment>> { ... }
    pub fn get_environment(&self, id: &str) -> Result<Option<Environment>> { ... }
    pub fn create_environment(&self, env: &NewEnvironment) -> Result<Environment> { ... }
    pub fn update_environment(&self, id: &str, env: &UpdateEnvironment) -> Result<Environment> { ... }
    pub fn delete_environment(&self, id: &str) -> Result<()> { ... }

    // --- 资源（M8 仅建表，CRUD 在 M9 实现）---
    // --- Agent（M8 仅建表，CRUD 在 M12 实现）---
}
```

**models.rs 数据模型**

对齐参考文档的字段定义，所有 model derive `Debug, Clone, Serialize, Deserialize`：

```rust
pub struct Environment {
    pub id: String,
    pub name: String,
    pub description: String,
    pub connection_mode: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct Resource { /* 字段同 schema */ }
pub struct Agent { /* 字段同 schema */ }
pub struct AuditEntry { /* 字段同 schema */ }

pub struct NewAuditEntry {
    pub action: String,
    pub target: Option<String>,
    pub environment_id: Option<String>,
    pub resource_id: Option<String>,
    pub agent_id: Option<String>,
    pub result: String,
    pub detail: Option<String>,
}

pub struct AuditFilter {
    pub time_from: Option<String>,
    pub time_to: Option<String>,
    pub action: Option<String>,
    pub environment_id: Option<String>,
    pub result: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}
```

**测试标准**

- `Database::open()` 创建文件并运行迁移
- `get_setting` / `set_setting` 读写正常
- `write_audit_log` / `query_audit_log` 写入和查询正常
- `list_environments` / `create_environment` / `delete_environment` 基础 CRUD 正常

**审计日志写入模式**

```rust
// handler 中一行调用，审计失败不阻断主流程：
state.db.write_audit_log(&NewAuditEntry {
    action: "SQL_QUERY".into(),
    target: Some(body.session_id.clone()),
    result: "success".into(),
    ..Default::default()
}).ok();
```

**提交信息**

```
feat(db): add SQLite schema, migration runner, Database struct, and audit logging
```

---

### 2 认证系统 + AppState 重构 + 路由重组

> 注意：Tasks 2 和 3 紧耦合（auth 中间件需要 AppState，AppState 路由需要 auth 中间件），合并为一个任务在同一个 commit 中实现。

**功能目标**

实现单用户密码认证：首次设置密码 → 登录获取 JWT → 中间件验证 token。

**文件结构**

新建：
- `crates/rex-hub/src/auth.rs` — 认证逻辑（密码哈希、JWT 签发/验证、登录 handler）
- `crates/rex-hub/src/middleware.rs` — axum auth 中间件（extractor）

修改：
- `crates/rex-hub/src/lib.rs` — 添加 `pub mod auth; pub mod middleware;`
- `crates/rex-hub/Cargo.toml` — 确认 `jsonwebtoken`、`argon2`、`chrono` 依赖已存在（workspace）

**认证流程**

```text
首次启动
  settings 表无 password_hash
  → GET /api/auth/check → { requires_setup: true }
  → 前端引导到设置密码页
  → POST /api/auth/password { password } → hash 存入 settings → 返回 token

后续启动
  settings 表有 password_hash
  → GET /api/auth/check → { requires_setup: false }
  → 前端显示登录页
  → POST /api/auth/login { password } → argon2 验证 → JWT 签发 → 返回 token

请求认证
  前端 → Authorization: Bearer <token> header
  → auth 中间件验证 JWT → 有效则放行，无效返回 401
```

**auth.rs 核心结构**

```rust
pub struct AuthConfig {
    pub jwt_secret: Vec<u8>,
    pub db: Arc<Database>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,     // 固定 "admin"（单用户）
    pub exp: usize,      // 过期时间戳
    pub iat: usize,      // 签发时间戳
}

impl AuthConfig {
    pub fn new(db: Arc<Database>) -> Self {
        // JWT secret 从 settings 表读取（首次自动生成并存储）
    }

    pub fn generate_token(&self) -> Result<String> { ... }
    pub fn verify_token(&self, token: &str) -> Result<Claims> { ... }

    /// 检查是否需要设置密码
    pub fn requires_setup(&self) -> Result<bool> {
        let hash = self.db.get_setting("password_hash")?;
        Ok(hash.is_none())
    }

    /// 设置密码（首次）
    pub fn set_password(&self, password: &str) -> Result<()> {
        let hash = argon2::hash_encoded(password.as_bytes(), ...)?;
        self.db.set_setting("password_hash", &hash)?;
        Ok(())
    }

    /// 验证密码并返回 token
    pub fn login(&self, password: &str) -> Result<String> {
        let hash = self.db.get_setting("password_hash")?
            .ok_or_else(|| RExError::Message("no password set".into()))?;
        if !argon2::verify_encoded(&hash, password.as_bytes())? {
            return Err(RExError::Message("invalid password".into()));
        }
        self.generate_token()
    }
}

// --- axum handlers ---

/// GET /api/auth/check
pub async fn check_auth(State(state): State<AppState>) -> Json<serde_json::Value> {
    let requires_setup = state.auth.requires_setup().unwrap_or(true);
    Json(json!({ "requires_setup": requires_setup }))
}

/// POST /api/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    match state.auth.login(&body.password) {
        Ok(token) => {
            // 写审计日志
            state.db.write_audit_log(&NewAuditEntry {
                action: "AUTH_LOGIN".into(),
                result: "success".into(),
                ..Default::default()
            }).ok();
            let expires = chrono::Utc::now() + chrono::Duration::days(7);
            Ok(Json(json!({ "token": token, "expiresAt": expires.to_rfc3339() })))
        }
        Err(e) => {
            state.db.write_audit_log(&NewAuditEntry {
                action: "AUTH_LOGIN".into(),
                result: "failure".into(),
                detail: Some(e.to_string()),
                ..Default::default()
            }).ok();
            Err(error_response("AUTH_INVALID", "密码错误"))
        }
    }
}

/// POST /api/auth/password（首次设置密码）
pub async fn set_password(
    State(state): State<AppState>,
    Json(body): Json<PasswordRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    state.auth.set_password(&body.password)?;
    let token = state.auth.generate_token()?;
    let expires = chrono::Utc::now() + chrono::Duration::days(7);
    Ok(Json(json!({ "token": token, "expiresAt": expires.to_rfc3339() })))
}
```

**middleware.rs — AuthUser extractor**

```rust
pub struct AuthUser(pub Claims);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for AuthUser {
    type Rejection = (StatusCode, Json<ErrorBody>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = parts.extensions.get::<AppState>()
            .ok_or_else(|| error_response("AUTH_REQUIRED", "missing state"))?;
        let header = parts.headers.get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| error_response("AUTH_REQUIRED", "missing token"))?;
        let token = header.strip_prefix("Bearer ")
            .ok_or_else(|| error_response("AUTH_REQUIRED", "invalid format"))?;
        let claims = state.auth.verify_token(token)
            .map_err(|_| error_response("AUTH_INVALID", "token expired or invalid"))?;
        Ok(AuthUser(claims))
    }
}
```

**路由白名单**

以下路径不需要认证：
- `POST /api/auth/login`
- `POST /api/auth/password`（仅首次，已设置密码后返回 409）
- `GET /api/auth/check`
- `GET /healthz`
- 静态文件（SPA fallback）

**WebSocket 认证**

浏览器 WebSocket API 不支持自定义 HTTP header。终端 WebSocket 采用 **query param** 方案：

```
ws://host/ws/terminal?token=<jwt_token>
```

`terminal_ws.rs` 的 `ws_handler` 从 query param 提取 token 并验证。前端 `useTerminal.ts` 构建 WebSocket URL 时附加 token。

**`error_response` 适配**

现有 `error_response` 固定返回 400。auth 中间件需要返回 401。方案：新增 `auth_error_response(code, message, status)` 函数，auth 相关 handler 使用此函数返回正确的 HTTP 状态码（401 Unauthorized）。

**`set_password` 保护**

`POST /api/auth/password` handler 增加检查：若密码已设置，返回 409 Conflict，防止重复设置。

```rust
pub async fn set_password(...) -> Result<...> {
    if !state.auth.requires_setup()? {
        return Err(error_response("PASSWORD_ALREADY_SET", "密码已设置"));
    }
    state.auth.set_password(&body.password)?;
    // ...
}
```

**子任务依赖**

Tasks 1→2→3 顺序执行（2 和 3 紧耦合：auth 中间件需要 AppState，AppState 路由需要 auth 中间件，必须在同一个 commit 中实现）。Task 4 依赖 Task 1（使用 db.rs 的审计方法）。Tasks 5–8 为前端，其中 Task 7 依赖 Task 6（路由守卫需要 auth store）。

**测试标准**

- 首次调用 `/api/auth/check` 返回 `requires_setup: true`
- `POST /api/auth/password` 设置密码后返回 token
- `/api/auth/check` 变为 `requires_setup: false`
- `POST /api/auth/login` 正确密码返回 token，错误密码返回 401
- 带有效 token 的请求通过中间件
- 无效/过期 token 返回 401

**提交信息**

```
feat(auth): add JWT authentication with argon2 password hashing
```

**AppState 设计**

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub auth: Arc<AuthConfig>,
    pub sql_pool: SqlState,     // Arc<Mutex<SqlConnectionPool>>
    pub redis_pool: RedisState,  // Arc<Mutex<RedisConnectionPool>>
    pub file_pool: FileState,    // Arc<Mutex<FileConnectionPool>>
}
```

**路由重组**

```rust
pub fn build_router(state: AppState, static_dir: PathBuf) -> Router {
    // 公开路由（无需认证）
    let public_routes = Router::new()
        .route("/api/auth/check", get(auth::check_auth))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/password", post(auth::set_password));

    // 受保护路由（需要认证）
    let protected_routes = Router::new()
        .nest("/api/sql", sql_api::sql_routes())
        .nest("/api/redis", redis_api::redis_routes())
        .nest("/api/files", file_api::file_routes())
        .route("/ws/terminal", get(terminal_ws::ws_handler))
        .layer(axum::middleware::from_extractor_with_state::<_, AuthUser>(
            state.clone(),
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
        .fallback_service(
            ServeDir::new(&static_dir)
                .append_index_html_on_directories(true)
                .not_found_service(ServeFile::new(static_dir.join("index.html")))
        )
}

// worker_main 改造：
// 1. 打开 Database
// 2. 创建 AuthConfig
// 3. 构建 AppState
// 4. 调用 build_router(state, static_dir)
```

**现有 API 模块适配**

sql_api.rs / redis_api.rs / file_api.rs 中的 handler 需要从 `AppState` 中取出各自的 pool。有两种方式：
- 方案 A：每个 handler 从 `State(state): State<AppState>` 取出 `state.sql_pool`
- 方案 B：嵌套路由继续用 `.with_state(state.sql_pool)` — 但 auth 中间件需要访问 AppState

采用**方案 A**：所有 handler 统一接收 `AppState`，从其中取出需要的 pool。现有 handler 签名从 `State(pool): State<SqlState>` 改为 `State(state): State<AppState>`，然后 `let mut pool = state.sql_pool.lock().await;`。

**测试标准**

- 所有现有 API（SQL/Redis/Files）在认证后正常工作
- 未认证请求返回 401
- 静态文件和登录路由不需要认证

**提交信息**

```
feat(auth): add JWT authentication, AppState, and restructure router
```

---

### 3 现有 API 模块注入 auth header + WebSocket token 认证

**功能目标**

M8 完成后所有后端 API 和 WebSocket 都需要认证。确保现有前端功能（SQL/Redis/Files/Terminal）在认证后不中断。

**文件结构**

修改：
- `packages/rex-console-web/src/api/sql.ts` — 注入 auth header
- `packages/rex-console-web/src/api/redis.ts` — 注入 auth header
- `packages/rex-console-web/src/api/files.ts` — 注入 auth header
- `packages/rex-console-web/src/features/terminal/useTerminal.ts` — WebSocket URL 附加 token query param

**前端 API 模块适配（最小改动）**

```typescript
// 在每个 API 模块顶部添加：
function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('rex-token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}

// 每个 fetch 调用添加 headers：
const res = await fetch(url, {
  headers: { ...authHeaders(), 'Content-Type': 'application/json' },
})
```

**WebSocket 认证**

`useTerminal.ts` 构建 WebSocket URL 时附加 token：

```typescript
const token = localStorage.getItem('rex-token')
const url = `ws://${location.host}/ws/terminal?token=${token}`
ws = new WebSocket(url)
```

后端 `terminal_ws.rs` 的 `ws_handler` 从 query param 提取并验证 token，无效则拒绝升级。

**测试标准**

- SQL/Redis/Files 页面在登录后正常调用 API（不 401）
- 终端 WebSocket 连接正常建立（token query param 验证通过）
- 未登录时终端 WebSocket 被拒绝

**提交信息**

```
fix(web): inject auth headers in existing API modules and WebSocket token auth
```

---

### 4 前端 API 客户端封装（client.ts）

**功能目标**

统一所有前端 API 调用的 HTTP 客户端，自动注入 auth header、处理 401、统一错误格式。

**文件结构**

新建：
- `packages/rex-console-web/src/api/client.ts`

**设计**

```typescript
export class AuthError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'AuthError'
  }
}

export class ApiError extends Error {
  code: string
  constructor(code: string, message: string) {
    super(message)
    this.name = 'ApiError'
    this.code = code
  }
}

class ApiClient {
  private baseUrl = '/api'

  private getHeaders(isFormData = false): Record<string, string> {
    const headers: Record<string, string> = {}
    if (!isFormData) {
      headers['Content-Type'] = 'application/json'
    }
    const token = localStorage.getItem('rex-token')
    if (token) {
      headers['Authorization'] = `Bearer ${token}`
    }
    return headers
  }

  async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const res = await fetch(`${this.baseUrl}${path}`, {
      ...options,
      headers: {
        ...this.getHeaders(),
        ...options.headers as Record<string, string>,
      },
    })

    if (res.status === 401) {
      localStorage.removeItem('rex-token')
      throw new AuthError('认证已过期')
    }

    if (!res.ok) {
      const body = await res.json().catch(() => null)
      throw new ApiError(
        body?.error?.code || 'UNKNOWN',
        body?.error?.message || res.statusText
      )
    }

    return res.json()
  }

  get<T>(path: string, params?: Record<string, string>): Promise<T> {
    const url = params
      ? `${path}?${new URLSearchParams(params).toString()}`
      : path
    return this.request<T>(url)
  }

  post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>(path, {
      method: 'POST',
      body: body ? JSON.stringify(body) : undefined,
    })
  }

  put<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>(path, {
      method: 'PUT',
      body: body ? JSON.stringify(body) : undefined,
    })
  }

  del<T>(path: string): Promise<T> {
    return this.request<T>(path, { method: 'DELETE' })
  }

  async upload<T>(path: string, formData: FormData): Promise<T> {
    return this.request<T>(path, {
      method: 'POST',
      body: formData,
      headers: this.getHeaders(true),
    })
  }
}

export const api = new ApiClient()
```

**现有 API 模块适配**

M8 完成后所有 `/api/*` 和 `/ws/*` 路由都需要认证。现有 sql.ts、redis.ts、files.ts 使用原始 `fetch()` 且不携带 `Authorization` header，**必须在 M8 中修复**否则这三个模块会全部 401。

修复方案（最小改动）：在每个现有 API 模块的 `fetch` 调用中注入 auth header：

```typescript
// 在每个 API 模块顶部添加：
function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('rex-token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}

// 在每个 fetch 调用中添加 headers：
const res = await fetch(url, { headers: { ...authHeaders(), ... } })
```

完整迁移到 `client.ts` 延迟到 M10，M8 只做 header 注入。

**测试标准**

- `api.get/post/put/del` 正确发送请求
- 自动携带 Authorization header
- 401 响应抛出 AuthError
- 非 200 响应抛出 ApiError
- FormData 上传不设置 Content-Type

**提交信息**

```
feat(web): add centralized API client with auth header injection
```

---

### 5 auth Pinia store

**功能目标**

管理前端认证状态：token 持久化、登录/登出、认证检查。

**文件结构**

新建：
- `packages/rex-console-web/src/stores/auth.ts`

**设计**

```typescript
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api, AuthError } from '@/api/client'

interface AuthCheckResponse {
  requires_setup: boolean
}

interface LoginResponse {
  token: string
  expiresAt: string
}

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem('rex-token'))
  const requiresSetup = ref(false)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const isAuthenticated = computed(() => !!token.value)

  /** 检查认证状态（页面加载时调用） */
  async function checkAuth() {
    try {
      const res = await api.request<AuthCheckResponse>('/auth/check')
      requiresSetup.value = res.requires_setup
    } catch {
      // 无法连接后端时假设需要设置
      requiresSetup.value = true
    }
  }

  /** 首次设置密码 */
  async function setupPassword(password: string) {
    loading.value = true
    error.value = null
    try {
      const res = await api.post<LoginResponse>('/auth/password', { password })
      token.value = res.token
      localStorage.setItem('rex-token', res.token)
      requiresSetup.value = false
    } catch (e: any) {
      error.value = e.message
      throw e
    } finally {
      loading.value = false
    }
  }

  /** 登录 */
  async function login(password: string) {
    loading.value = true
    error.value = null
    try {
      const res = await api.post<LoginResponse>('/auth/login', { password })
      token.value = res.token
      localStorage.setItem('rex-token', res.token)
    } catch (e: any) {
      error.value = e.message
      throw e
    } finally {
      loading.value = false
    }
  }

  /** 登出 */
  function logout() {
    token.value = null
    localStorage.removeItem('rex-token')
  }

  return {
    token, requiresSetup, loading, error, isAuthenticated,
    checkAuth, setupPassword, login, logout,
  }
})
```

**测试标准**

- `checkAuth` 正确读取后端状态
- `login` 成功后 token 存入 localStorage
- `logout` 清除 token
- `isAuthenticated` 计算属性正确

**提交信息**

```
feat(web): add auth Pinia store with login/logout token management
```

---

### 6 路由守卫 + 登录页改造

**功能目标**

前端路由加守卫：未登录跳转登录页，已登录时访问登录页跳转工作区。登录页对接真实 API。

**文件结构**

修改：
- `packages/rex-console-web/src/router/index.ts` — 添加 beforeEach 守卫
- `packages/rex-console-web/src/pages/LoginPage.vue` — 对接真实认证 API

**路由守卫**

```typescript
// router/index.ts
router.beforeEach(async (to) => {
  const auth = useAuthStore()

  // 首次访问时检查认证状态
  if (!auth.isAuthenticated && to.name !== 'login' && to.name !== 'setup') {
    await auth.checkAuth()
  }

  // 需要设置密码 → setup 页面
  if (auth.requiresSetup && to.name !== 'setup' && to.name !== 'login') {
    return { name: 'setup' }
  }

  // 未登录 → 登录页
  if (!auth.isAuthenticated && to.name !== 'login' && to.name !== 'setup') {
    return { name: 'login', query: { redirect: to.fullPath } }
  }

  // 已登录访问登录页 → 工作区
  if (auth.isAuthenticated && to.name === 'login') {
    return { name: 'workspace' }
  }
})
```

**登录页改造**

```vue
<script setup lang="ts">
import { useAuthStore } from '@/stores/auth'
import { useRouter, useRoute } from 'vue-router'

const auth = useAuthStore()
const router = useRouter()
const route = useRoute()

const password = ref('')
const loading = ref(false)
const errorMsg = ref('')

async function handleLogin() {
  if (!password.value) return
  loading.value = true
  errorMsg.value = ''
  try {
    await auth.login(password.value)
    const redirect = (route.query.redirect as string) || '/workspace'
    router.push(redirect)
  } catch (e: any) {
    errorMsg.value = e.message || '登录失败'
  } finally {
    loading.value = false
  }
}
</script>
```

改造要点：
- 删除 `setTimeout` 假延迟
- 调用 `authStore.login(password)`
- 错误时显示真实错误信息
- 成功后 `router.push(redirect || '/workspace')`
- 单用户模型，不需要用户名字段（或保留但忽略）

**测试标准**

- 未登录访问 `/workspace` → 自动跳转 `/login`
- 输入密码登录 → 跳转 `/workspace`
- 错误密码 → 显示错误信息，不跳转
- 已登录访问 `/login` → 自动跳转 `/workspace`
- 刷新页面后 token 恢复，不需要重新登录

**提交信息**

```
feat(web): add route guard and real authentication to login page
```

---

### 7 首次设置密码页面

**功能目标**

首次使用时引导用户设置密码。

**文件结构**

新建：
- `packages/rex-console-web/src/pages/SetupPage.vue`

修改：
- `packages/rex-console-web/src/router/index.ts` — 添加 `/setup` 路由

**交互设计**

```text
┌──────────────────────────────┐
│         REX Hub              │
│                              │
│   首次使用，请设置密码        │
│                              │
│   密码：[____________]       │
│   确认：[____________]       │
│                              │
│   [设置密码]                 │
│                              │
│   密码用于登录和 API 认证，   │
│   请牢记。                    │
└──────────────────────────────┘
```

- 纯居中卡片布局
- 密码最少 6 位
- 两次输入不一致 → 提示错误
- 设置成功 → 自动登录，跳转工作区
- 调用 `authStore.setupPassword(password)`

**路由**

```typescript
{
  path: '/setup',
  name: 'setup',
  component: () => import('../pages/SetupPage.vue'),
  meta: { fullscreen: true },
}
```

**测试标准**

- 首次访问任何页面 → 自动跳转 `/setup`
- 设置密码后跳转工作区
- 再次访问 `/setup` → 跳转工作区（密码已设置）
- 密码少于 6 位 → 表单验证提示

**提交信息**

```
feat(web): add password setup page for first-time use
```

---

## 设计核对点

| 检查项 | 说明 |
|--------|------|
| 单用户模型 | 认证系统无多用户概念，无 RBAC |
| 自托管 | 无外部认证依赖，SQLite 本地存储 |
| 深色优先 | SetupPage 和 LoginPage 使用现有 token 系统的深色主题 |
| 路由架构 | 遵循现有 `AppLayout` → `RouterView` 结构 |
| 错误格式 | 对齐 `docs/reference/api-design.md` 的统一错误响应格式 |
| 数据模型 | 对齐 `docs/reference/data-models.md` 的表结构 |
| 认证协议 | 对齐 `docs/reference/api-design.md` 的 Bearer token 方案 |

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
