use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::helpers::{err_resp, not_found, now_iso, ApiResponse, ErrorResponse};
use crate::routes::AppState;

// ── 日志存储 ─────────────────────────────────────────────

/// 一条日志条目，与 Agent 端 LogCollector 保持一致
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// Agent 日志内存存储（每个 Agent 最多 1000 条）
pub struct AgentLogStore {
    logs: tokio::sync::RwLock<std::collections::HashMap<String, std::collections::VecDeque<LogEntry>>>,
}

impl AgentLogStore {
    pub fn new() -> Self {
        Self {
            logs: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// 存储 Agent 上报的日志（增量追加）
    pub async fn append_logs(&self, agent_id: &str, logs: Vec<LogEntry>) {
        if logs.is_empty() {
            return;
        }
        let mut map = self.logs.write().await;
        let queue = map
            .entry(agent_id.to_string())
            .or_insert_with(|| std::collections::VecDeque::with_capacity(1000));
        for entry in logs {
            queue.push_back(entry);
        }
        // 保持每个 agent 最多 1000 条
        while queue.len() > 1000 {
            queue.pop_front();
        }
    }

    /// 获取指定 Agent 的日志
    pub async fn get_logs(&self, agent_id: &str) -> Vec<LogEntry> {
        let map = self.logs.read().await;
        match map.get(agent_id) {
            Some(queue) => queue.iter().cloned().collect(),
            None => vec![],
        }
    }

    /// 获取指定 Agent 的日志（since 过滤）
    pub async fn get_logs_since(&self, agent_id: &str, since_ts: &str) -> Vec<LogEntry> {
        let map = self.logs.read().await;
        match map.get(agent_id) {
            Some(queue) => queue
                .iter()
                .filter(|e| e.timestamp.as_str() > since_ts)
                .cloned()
                .collect(),
            None => vec![],
        }
    }
}

// ── 工具函数 ─────────────────────────────────────────────

/// SHA256 哈希令牌，用于 token 匹配
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ── 数据模型 ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub os: String,
    pub arch: String,
    pub hostname: Option<String>,
    pub os_version: Option<String>,
    pub status: String,
    pub last_seen_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── 注册 ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub id: String,
    pub token: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub arch: String,
    pub hostname: Option<String>,
    pub os_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: String,
    pub environment_id: String,
    pub status: String,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(input): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<ApiResponse<RegisterResponse>>), (StatusCode, Json<ErrorResponse>)> {
    if input.token.is_empty() {
        return Err(err_resp("INVALID_TOKEN", "注册令牌不能为空"));
    }

    // Hash token to find matching environment
    let token_hash = hash_token(&input.token);

    let db = state.db.clone();
    let id = input.id.clone();
    let name = input.name.clone();
    let version = input.version.clone();
    let sha256 = input.sha256.clone();
    let os = input.os.clone();
    let arch = input.arch.clone();
    let hostname = input.hostname.clone();
    let os_version = input.os_version.clone();

    let result = tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        // Find environment by token hash
        let env_id: Option<String> = conn
            .query_row(
                "SELECT id FROM environments WHERE agent_token_hash = ?1",
                rusqlite::params![token_hash],
                |row| row.get(0),
            )
            .ok();

        let environment_id =
            env_id.ok_or_else(|| err_resp("INVALID_TOKEN", "注册令牌无效"))?;

        let now = now_iso();

        // Upsert agent
        conn.execute(
            "INSERT INTO agents (id, environment_id, name, token_hash, version, sha256, os, arch, hostname, os_version, status, last_seen_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'online', ?11, ?12, ?12)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name, version = excluded.version, sha256 = excluded.sha256,
                os = excluded.os, arch = excluded.arch, hostname = excluded.hostname,
                os_version = excluded.os_version, status = 'online', last_seen_at = excluded.last_seen_at,
                updated_at = excluded.updated_at",
            rusqlite::params![
                id,
                environment_id,
                name,
                token_hash,
                version,
                sha256,
                os,
                arch,
                hostname,
                os_version,
                now,
                now
            ],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        Ok::<_, (StatusCode, Json<ErrorResponse>)>((environment_id, now))
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    Ok((
        StatusCode::OK,
        Json(ApiResponse {
            data: RegisterResponse {
                id: input.id,
                environment_id: result.0,
                status: "online".to_string(),
            },
        }),
    ))
}

// ── 重置令牌 ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ResetTokenResponse {
    pub token: String,
}

/// 为指定 Agent 所在环境生成新的注册令牌。
/// 返回明文 token，由前端展示给用户。
pub async fn reset_token(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Result<Json<ApiResponse<ResetTokenResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let id = agent_id.clone();

    let result = tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        // 查找 agent 所属 environment_id
        let environment_id: String = conn
            .query_row(
                "SELECT environment_id FROM agents WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .map_err(|_| not_found("AGENT_NOT_FOUND", "Agent 不存在"))?;

        // 生成新 token（16 字节随机 → 32 字符 hex）
        let mut buf = [0u8; 16];
        use rand_core::RngCore;
        rand_core::OsRng.fill_bytes(&mut buf);
        let token = hex::encode(buf);
        let token_hash = hash_token(&token);

        let now = now_iso();
        conn.execute(
            "UPDATE environments SET agent_token_hash = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![token_hash, now, environment_id],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(token)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    Ok(Json(ApiResponse {
        data: ResetTokenResponse { token: result },
    }))
}

// ── Agent 列表 ───────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AgentListItem {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub hostname: Option<String>,
    pub os_version: Option<String>,
    pub status: String,
    pub last_seen_at: Option<String>,
}

pub async fn list_agents(
    State(state): State<Arc<AppState>>,
    Path(env_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<AgentListItem>>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let agents = tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        // Check environment exists
        let env_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM environments WHERE id = ?1",
                rusqlite::params![env_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        if !env_exists {
            return Err(not_found("ENVIRONMENT_NOT_FOUND", "环境不存在"));
        }

        let mut stmt = conn
            .prepare(
                "SELECT id, environment_id, name, version, os, arch, hostname, os_version, status, last_seen_at
                 FROM agents WHERE environment_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        let agents = stmt
            .query_map(rusqlite::params![env_id], |row| {
                Ok(AgentListItem {
                    id: row.get(0)?,
                    environment_id: row.get(1)?,
                    name: row.get(2)?,
                    version: row.get(3)?,
                    os: row.get(4)?,
                    arch: row.get(5)?,
                    hostname: row.get(6)?,
                    os_version: row.get(7)?,
                    status: row.get(8)?,
                    last_seen_at: row.get(9)?,
                })
            })
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?
            .filter_map(|r| r.ok())
            .collect();

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(agents)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    Ok(Json(ApiResponse { data: agents }))
}

// ── 更新心跳 ─────────────────────────────────────────────

pub fn update_heartbeat(db: &crate::db::Database, agent_id: &str, version: &str, sha256: &str) {
    let now = now_iso();
    if let Ok(conn) = db.pool.get() {
        let _ = conn.execute(
            "UPDATE agents SET version = ?1, sha256 = ?2, last_seen_at = ?3, status = 'online', updated_at = ?3 WHERE id = ?4",
            rusqlite::params![version, sha256, now, agent_id],
        );
    }
}

pub fn update_heartbeat_with_config(
    db: &crate::db::Database,
    agent_id: &str,
    version: &str,
    sha256: &str,
    config_json: &str,
) {
    let now = now_iso();
    if let Ok(conn) = db.pool.get() {
        let _ = conn.execute(
            "UPDATE agents SET version = ?1, sha256 = ?2, last_seen_at = ?3, status = 'online', config_json = ?4, updated_at = ?3 WHERE id = ?5",
            rusqlite::params![version, sha256, now, config_json, agent_id],
        );
    }
}

/// 获取 Agent 配置（从 config_json 列解析）
pub fn get_agent_config(db: &crate::db::Database, agent_id: &str) -> Option<serde_json::Value> {
    let conn = db.pool.get().ok()?;
    let config_json: String = conn
        .query_row(
            "SELECT config_json FROM agents WHERE id = ?1",
            rusqlite::params![agent_id],
            |row| row.get(0),
        )
        .ok()?;
    serde_json::from_str(&config_json).ok()
}

/// 更新 Agent 配置（写入 config_json 列）
pub fn update_agent_config(
    db: &crate::db::Database,
    agent_id: &str,
    config: &serde_json::Value,
) -> Result<(), String> {
    let now = now_iso();
    let config_json = serde_json::to_string(config).map_err(|e| e.to_string())?;
    let conn = db.pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE agents SET config_json = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![config_json, now, agent_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_agent_status(db: &crate::db::Database, agent_id: &str, status: &str) {
    let now = now_iso();
    if let Ok(conn) = db.pool.get() {
        let _ = conn.execute(
            "UPDATE agents SET status = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![status, now, agent_id],
        );
    }
}

pub fn find_env_by_token_hash(db: &crate::db::Database, token_hash: &str) -> Option<String> {
    let conn = db.pool.get().ok()?;
    conn.query_row(
        "SELECT id FROM environments WHERE agent_token_hash = ?1",
        rusqlite::params![token_hash],
        |row| row.get(0),
    )
    .ok()
}

#[allow(clippy::too_many_arguments)]
pub fn upsert_agent(
    db: &crate::db::Database,
    agent_id: &str,
    environment_id: &str,
    name: &str,
    token_hash: &str,
    version: &str,
    sha256: &str,
    os: &str,
    arch: &str,
    hostname: Option<&str>,
    os_version: Option<&str>,
) {
    let now = now_iso();
    if let Ok(conn) = db.pool.get() {
        let _ = conn.execute(
            "INSERT INTO agents (id, environment_id, name, token_hash, version, sha256, os, arch, hostname, os_version, status, last_seen_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'online', ?11, ?11, ?11)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name, version = excluded.version, sha256 = excluded.sha256,
                os = excluded.os, arch = excluded.arch, hostname = excluded.hostname,
                os_version = excluded.os_version, status = 'online', last_seen_at = excluded.last_seen_at,
                updated_at = excluded.updated_at",
            rusqlite::params![
                agent_id, environment_id, name, token_hash, version, sha256,
                os, arch, hostname, os_version, now
            ],
        );
    }
}

// ── Agent 配置 API ─────────────────────────────────────

/// GET /api/agents/:agent_id/config
pub async fn get_agent_config_handler(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let aid = agent_id.clone();

    let config = tokio::task::spawn_blocking(move || get_agent_config(&db, &aid))
        .await
        .map_err(|_| err_resp("INTERNAL_ERROR", "internal error"))?
        .unwrap_or_else(|| serde_json::json!({ "auto_update": true }));

    Ok(Json(ApiResponse { data: config }))
}

/// PATCH /api/agents/:agent_id/config
pub async fn update_agent_config_handler(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
    Json(input): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let aid = agent_id.clone();

    // 合并现有配置与新配置
    let existing = {
        let db2 = state.db.clone();
        let aid2 = aid.clone();
        tokio::task::spawn_blocking(move || get_agent_config(&db2, &aid2))
            .await
            .map_err(|_| err_resp("INTERNAL_ERROR", "internal error"))?
            .unwrap_or_else(|| serde_json::json!({}))
    };

    let mut merged = existing;
    if let Some(obj) = input.as_object() {
        if let Some(merged_obj) = merged.as_object_mut() {
            for (k, v) in obj {
                merged_obj.insert(k.clone(), v.clone());
            }
        }
    }

    let final_config = merged.clone();
    tokio::task::spawn_blocking(move || update_agent_config(&db, &aid, &final_config))
        .await
        .map_err(|_| err_resp("INTERNAL_ERROR", "internal error"))?
        .map_err(|e| err_resp("UPDATE_FAILED", &e))?;

    Ok(Json(ApiResponse { data: merged }))
}

// ── 日志查询 API ─────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LogQueryParams {
    pub since: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogResponse {
    pub logs: Vec<LogEntry>,
    pub total: usize,
}

/// GET /api/agents/:agent_id/logs
pub async fn get_agent_logs(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<LogQueryParams>,
) -> Result<Json<ApiResponse<LogResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let logs = match &params.since {
        Some(since) => state.agent_log_store.get_logs_since(&agent_id, since).await,
        None => state.agent_log_store.get_logs(&agent_id).await,
    };
    let total = logs.len();
    Ok(Json(ApiResponse {
        data: LogResponse { logs, total },
    }))
}

// ── Agent 远程重启 API ──────────────────────────────

#[derive(Debug, Serialize)]
pub struct RestartResponse {
    pub message: String,
}

/// POST /api/agents/:agent_id/restart
/// 通过 WebSocket 向 Agent 发送重启指令
pub async fn restart_agent(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Result<Json<ApiResponse<RestartResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let connections = state.connections.read().await;
    match connections.get(&agent_id) {
        Some(conn) => {
            if conn.cmd_tx.send("restart".to_string()).await.is_err() {
                Err(err_resp("AGENT_UNREACHABLE", "Agent 连接已断开，无法发送重启指令"))
            } else {
                Ok(Json(ApiResponse {
                    data: RestartResponse {
                        message: "restart command sent".to_string(),
                    },
                }))
            }
        }
        None => Err(not_found("AGENT_OFFLINE", "Agent 不在线或不存在")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn test_db() -> Arc<crate::db::Database> {
        Arc::new(Database::new_in_memory().unwrap())
    }

    fn setup_env(db: &Database, env_id: &str, token_hash: &str) {
        let conn = db.pool.get().unwrap();
        conn.execute(
            "INSERT INTO environments (id, name, connection_mode, agent_token_hash, created_at, updated_at)
             VALUES (?1, 'test', 'agent_proxy', ?2, '20260101', '20260101')",
            rusqlite::params![env_id, token_hash],
        )
        .unwrap();
    }

    #[test]
    fn find_env_by_token_hash_found() {
        let db = test_db();
        let hash = "abc123";
        setup_env(&db, "env_test1", hash);
        assert_eq!(
            find_env_by_token_hash(&db, hash),
            Some("env_test1".to_string())
        );
    }

    #[test]
    fn find_env_by_token_hash_not_found() {
        let db = test_db();
        assert_eq!(find_env_by_token_hash(&db, "nonexistent"), None);
    }

    #[test]
    fn upsert_agent_inserts() {
        let db = test_db();
        let hash = "abc123";
        setup_env(&db, "env_test1", hash);
        upsert_agent(
            &db,
            "agt_001",
            "env_test1",
            "agent1",
            hash,
            "0.1.0",
            "",
            "linux",
            "amd64",
            None,
            None,
        );
        let agents = list_agents_sync(&db, "env_test1");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, "agt_001");
    }

    #[test]
    fn upsert_agent_updates_existing() {
        let db = test_db();
        let hash = "abc123";
        setup_env(&db, "env_test1", hash);
        upsert_agent(
            &db,
            "agt_001",
            "env_test1",
            "agent1",
            hash,
            "0.1.0",
            "",
            "linux",
            "amd64",
            None,
            None,
        );
        upsert_agent(
            &db,
            "agt_001",
            "env_test1",
            "agent1-updated",
            hash,
            "0.2.0",
            "",
            "linux",
            "arm64",
            None,
            None,
        );
        let agents = list_agents_sync(&db, "env_test1");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].version, "0.2.0");
    }

    #[test]
    fn set_agent_status_works() {
        let db = test_db();
        let hash = "abc123";
        setup_env(&db, "env_test1", hash);
        upsert_agent(
            &db,
            "agt_001",
            "env_test1",
            "agent1",
            hash,
            "0.1.0",
            "",
            "linux",
            "amd64",
            None,
            None,
        );
        set_agent_status(&db, "agt_001", "offline");
        let agents = list_agents_sync(&db, "env_test1");
        assert_eq!(agents[0].status, "offline");
    }

    fn list_agents_sync(db: &Database, env_id: &str) -> Vec<AgentListItem> {
        let conn = db.pool.get().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, environment_id, name, version, os, arch, hostname, os_version, status, last_seen_at
                 FROM agents WHERE environment_id = ?1",
            )
            .unwrap();
        stmt.query_map(rusqlite::params![env_id], |row| {
            Ok(AgentListItem {
                id: row.get(0)?,
                environment_id: row.get(1)?,
                name: row.get(2)?,
                version: row.get(3)?,
                os: row.get(4)?,
                arch: row.get(5)?,
                hostname: row.get(6)?,
                os_version: row.get(7)?,
                status: row.get(8)?,
                last_seen_at: row.get(9)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    #[test]
    fn hash_token_produces_consistent_hash() {
        let hash1 = hash_token("test_token");
        let hash2 = hash_token("test_token");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex string
    }

    #[test]
    fn hash_token_different_for_different_inputs() {
        let hash1 = hash_token("token1");
        let hash2 = hash_token("token2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn update_heartbeat_works() {
        let db = test_db();
        let hash = "abc123";
        setup_env(&db, "env_test1", hash);
        upsert_agent(
            &db,
            "agt_001",
            "env_test1",
            "agent1",
            hash,
            "0.1.0",
            "",
            "linux",
            "amd64",
            None,
            None,
        );
        update_heartbeat(&db, "agt_001", "0.2.0", "newsha256");
        let agents = list_agents_sync(&db, "env_test1");
        assert_eq!(agents[0].version, "0.2.0");
    }

    #[test]
    fn agent_struct_serializes() {
        let agent = Agent {
            id: "agt_123".to_string(),
            environment_id: "env_456".to_string(),
            name: "test-agent".to_string(),
            version: "0.1.0".to_string(),
            sha256: "abc123".to_string(),
            os: "linux".to_string(),
            arch: "amd64".to_string(),
            hostname: Some("host1".to_string()),
            os_version: Some("22.04".to_string()),
            status: "online".to_string(),
            last_seen_at: Some("2024-01-01".to_string()),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        };
        let json = serde_json::to_string(&agent).unwrap();
        assert!(json.contains("agt_123"));
        assert!(json.contains("linux"));
    }

    #[test]
    fn register_request_deserializes() {
        let json = r#"{
            "id": "agt_123",
            "token": "mytoken",
            "name": "agent1",
            "version": "0.1.0",
            "os": "linux",
            "arch": "amd64"
        }"#;
        let req: RegisterRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, "agt_123");
        assert_eq!(req.token, "mytoken");
        assert_eq!(req.name, "agent1");
    }

    #[test]
    fn agent_list_item_serializes() {
        let item = AgentListItem {
            id: "agt_123".to_string(),
            environment_id: "env_456".to_string(),
            name: "test-agent".to_string(),
            version: "0.1.0".to_string(),
            os: "linux".to_string(),
            arch: "amd64".to_string(),
            hostname: Some("host1".to_string()),
            os_version: Some("22.04".to_string()),
            status: "online".to_string(),
            last_seen_at: Some("2024-01-01".to_string()),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("agt_123"));
        assert!(json.contains("online"));
    }

    #[test]
    fn register_response_serializes() {
        let resp = RegisterResponse {
            id: "agt_123".to_string(),
            environment_id: "env_456".to_string(),
            status: "online".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("agt_123"));
        assert!(json.contains("online"));
    }
}

// ── HTTP Handler Tests ─────────────────────────────────────

#[cfg(test)]
mod handler_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, post};
    use axum::Router;
    use tower::ServiceExt;

    use crate::routes::AppState;
    use rex_transfer::task::TransferManager;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            db: Arc::new(crate::db::Database::new_in_memory().unwrap()),
            secret_key: "test-secret".to_string(),
            connections: Arc::new(crate::ws::new_connections()),
            sessions: Arc::new(crate::terminal::SessionManager::new(900)),
            transfer: Some(Arc::new(crate::transfer::TransferState {
                manager: Arc::new(TransferManager::new()),
            })),
            update_cache: tokio::sync::RwLock::new(crate::routes::UpdateCache::new()),
            data_dir: std::env::temp_dir(),
            metrics: Arc::new(crate::metrics::MetricsCollector::new(
                Arc::new(crate::db::Database::new_in_memory().unwrap()),
                3600,
            )),
            agent_log_store: Arc::new(crate::agent::AgentLogStore::new()),
        })
    }

    fn auth_header() -> axum::http::header::HeaderValue {
        use jsonwebtoken::{encode, EncodingKey, Header};
        let exp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
            + 3600;
        let claims = crate::auth::Claims {
            sub: "admin".to_string(),
            exp,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("test-secret".as_bytes()),
        )
        .unwrap();
        axum::http::header::HeaderValue::from_str(&format!("Bearer {token}")).unwrap()
    }

    fn setup_env(state: &AppState, env_id: &str, token_hash: &str) {
        let conn = state.db.pool.get().unwrap();
        conn.execute(
            "INSERT INTO environments (id, name, connection_mode, agent_token_hash, created_at, updated_at) VALUES (?1, 'test', 'agent_proxy', ?2, '2024-01-01', '2024-01-01')",
            rusqlite::params![env_id, token_hash],
        )
        .unwrap();
    }

    #[tokio::test]
    async fn register_rejects_empty_token() {
        let state = test_state();
        let app = Router::new()
            .route("/api/agents/register", post(register))
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents/register")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"id":"agt_001","token":"","name":"agent1","version":"0.1.0"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn register_rejects_invalid_token() {
        let state = test_state();
        let app = Router::new()
            .route("/api/agents/register", post(register))
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents/register")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"agt_001","token":"invalid_token","name":"agent1","version":"0.1.0"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn list_agents_returns_empty_for_new_env() {
        let state = test_state();
        setup_env(&state, "env_test", "hash123");
        let app = Router::new()
            .route("/api/environments/:env_id/agents", get(list_agents))
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/environments/env_test/agents")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["data"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_agents_returns_not_found_for_unknown_env() {
        let state = test_state();
        let app = Router::new()
            .route("/api/environments/:env_id/agents", get(list_agents))
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/environments/nonexistent/agents")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_agent_config_returns_default_when_no_config() {
        let state = test_state();
        let token_hash = hash_token("test-token");
        setup_env(&state, "env_001", &token_hash);
        // Register an agent
        let conn = state.db.pool.get().unwrap();
        conn.execute(
            "INSERT INTO agents (id, environment_id, name, token_hash, version, sha256, os, arch, status, config_json, created_at, updated_at) VALUES (?1, ?2, 'test-agent', ?3, '0.20.0', '', 'linux', 'amd64', 'online', '{}', '2024-01-01', '2024-01-01')",
            rusqlite::params!["agt_001", "env_001", token_hash],
        ).unwrap();

        let app = Router::new()
            .route(
                "/api/agents/:agent_id/config",
                get(get_agent_config_handler),
            )
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents/agt_001/config")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["auto_update"], true);
    }

    #[tokio::test]
    async fn update_agent_config_persists_auto_update() {
        let state = test_state();
        let token_hash = hash_token("test-token");
        setup_env(&state, "env_001", &token_hash);
        let conn = state.db.pool.get().unwrap();
        conn.execute(
            "INSERT INTO agents (id, environment_id, name, token_hash, version, sha256, os, arch, status, config_json, created_at, updated_at) VALUES (?1, ?2, 'test-agent', ?3, '0.20.0', '', 'linux', 'amd64', 'online', '{}', '2024-01-01', '2024-01-01')",
            rusqlite::params!["agt_002", "env_001", token_hash],
        ).unwrap();
        drop(conn);

        let app = Router::new()
            .route(
                "/api/agents/:agent_id/config",
                get(get_agent_config_handler).patch(update_agent_config_handler),
            )
            .with_state(state);

        // PATCH auto_update to false
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/agents/agt_002/config")
                    .header("authorization", auth_header())
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"auto_update": false}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["auto_update"], false);
    }

    #[tokio::test]
    async fn get_agent_config_returns_default_for_unknown_agent() {
        let state = test_state();
        let app = Router::new()
            .route(
                "/api/agents/:agent_id/config",
                get(get_agent_config_handler),
            )
            .with_state(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents/nonexistent/config")
                    .header("authorization", auth_header())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["data"]["auto_update"], true);
    }
}
