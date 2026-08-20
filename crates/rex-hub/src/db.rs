//! SQLite 访问层：基于 r2d2 连接池封装 CRUD 与审计日志写入。
//! 数据目录下的 `hub.db` 为本地唯一持久化存储。

use std::path::Path;

use rex_common::RExError;
use rusqlite::Connection;

use crate::models::*;

pub type Result<T> = std::result::Result<T, RExError>;

// --- Custom r2d2 ConnectionManager for rusqlite ---

struct SqliteConnectionManager {
    path: String,
}

impl r2d2::ManageConnection for SqliteConnectionManager {
    type Connection = Connection;
    type Error = rusqlite::Error;

    fn connect(&self) -> std::result::Result<Connection, Self::Error> {
        Connection::open(&self.path)
    }

    fn is_valid(&self, conn: &mut Connection) -> std::result::Result<(), Self::Error> {
        conn.execute_batch("SELECT 1").map(|_| ())
    }

    fn has_broken(&self, _: &mut Connection) -> bool {
        false
    }
}

// --- Database ---

pub struct Database {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl Database {
    /// Open or create database at the given path, run migrations.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RExError::Message(format!("failed to create data dir: {e}")))?;
        }
        let path_str = path.to_string_lossy().to_string();
        let manager = SqliteConnectionManager { path: path_str };
        let pool = r2d2::Pool::builder()
            .max_size(4)
            .build(manager)
            .map_err(|e| RExError::Message(format!("failed to create pool: {e}")))?;
        let db = Self { pool };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn()?;
        conn.execute_batch(include_str!("migrations.sql"))
            .map_err(|e| RExError::Message(format!("migration failed: {e}")))?;
        Ok(())
    }

    fn conn(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool
            .get()
            .map_err(|e| RExError::Message(format!("failed to get connection: {e}")))
    }

    // --- Settings ---

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT value FROM settings WHERE key = ?1")
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![key], |row| row.get::<_, String>(0))
            .map_err(|e| RExError::Message(e.to_string()))?;
        match rows.next() {
            Some(Ok(val)) => Ok(Some(val)),
            Some(Err(e)) => Err(RExError::Message(e.to_string())),
            None => Ok(None),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> Result<std::collections::HashMap<String, String>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| RExError::Message(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row.map_err(|e| RExError::Message(e.to_string()))?;
            map.insert(k, v);
        }
        Ok(map)
    }

    // --- Saved SQL Queries ---
    //
    // 命名 SQL 查询以 JSON 数组存放于 settings 表的 "saved_queries" 键下（单用户，无需独立表）。

    const SAVED_QUERIES_KEY: &'static str = "saved_queries";

    pub fn list_saved_queries(&self) -> Result<Vec<SavedQuery>> {
        let raw = match self.get_setting(Self::SAVED_QUERIES_KEY)? {
            Some(v) if !v.is_empty() => v,
            _ => return Ok(Vec::new()),
        };
        let mut list: Vec<SavedQuery> = serde_json::from_str(&raw)
            .map_err(|e| RExError::Message(format!("failed to parse saved_queries: {e}")))?;
        // 按更新时间降序（无时间的排末尾）
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(list)
    }

    pub fn upsert_saved_query(&self, q: &SavedQuery) -> Result<SavedQuery> {
        let mut list = self.list_saved_queries()?;
        let now = chrono::Utc::now().to_rfc3339();
        let id = if q.id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            q.id.clone()
        };
        let stored = SavedQuery {
            id: id.clone(),
            name: q.name.clone(),
            sql: q.sql.clone(),
            db_type: q.db_type.clone(),
            updated_at: Some(now),
        };
        if let Some(pos) = list.iter().position(|x| x.id == id) {
            list[pos] = stored.clone();
        } else {
            list.push(stored.clone());
        }
        let json = serde_json::to_string(&list)
            .map_err(|e| RExError::Message(format!("failed to serialize saved_queries: {e}")))?;
        self.set_setting(Self::SAVED_QUERIES_KEY, &json)?;
        Ok(stored)
    }

    pub fn delete_saved_query(&self, id: &str) -> Result<()> {
        let mut list = self.list_saved_queries()?;
        list.retain(|x| x.id != id);
        let json = serde_json::to_string(&list)
            .map_err(|e| RExError::Message(format!("failed to serialize saved_queries: {e}")))?;
        self.set_setting(Self::SAVED_QUERIES_KEY, &json)?;
        Ok(())
    }

    // --- Audit Log ---

    pub fn write_audit_log(&self, entry: &NewAuditEntry) -> Result<()> {
        let conn = self.conn()?;
        let id = uuid::Uuid::new_v4().to_string();
        let time = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO audit_log (id, time, action, target, environment_id, resource_id, agent_id, result, detail, ip)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                id, time, entry.action, entry.target,
                entry.environment_id, entry.resource_id, entry.agent_id,
                entry.result, entry.detail, entry.ip,
            ],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(())
    }

    pub fn query_audit_log(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> {
        let conn = self.conn()?;
        let mut sql = String::from(
            "SELECT id, time, action, target, environment_id, resource_id, agent_id, result, detail, ip
             FROM audit_log WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(ref t) = filter.time_from {
            sql.push_str(&format!(" AND time >= ?{idx}"));
            params.push(Box::new(t.clone()));
            idx += 1;
        }
        if let Some(ref t) = filter.time_to {
            sql.push_str(&format!(" AND time <= ?{idx}"));
            params.push(Box::new(t.clone()));
            idx += 1;
        }
        if let Some(ref a) = filter.action {
            sql.push_str(&format!(" AND action = ?{idx}"));
            params.push(Box::new(a.clone()));
            idx += 1;
        }
        if let Some(ref env) = filter.environment_id {
            sql.push_str(&format!(" AND environment_id = ?{idx}"));
            params.push(Box::new(env.clone()));
            idx += 1;
        }
        if let Some(ref aid) = filter.agent_id {
            sql.push_str(&format!(" AND agent_id = ?{idx}"));
            params.push(Box::new(aid.clone()));
            idx += 1;
        }
        if let Some(ref r) = filter.result {
            sql.push_str(&format!(" AND result = ?{idx}"));
            params.push(Box::new(r.clone()));
        }

        sql.push_str(" ORDER BY time DESC, id DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        if let Some(offset) = filter.offset {
            sql.push_str(&format!(" OFFSET {offset}"));
        }

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| RExError::Message(e.to_string()))?;
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(AuditEntry {
                    id: row.get(0)?,
                    time: row.get(1)?,
                    action: row.get(2)?,
                    target: row.get(3)?,
                    environment_id: row.get(4)?,
                    resource_id: row.get(5)?,
                    agent_id: row.get(6)?,
                    result: row.get(7)?,
                    detail: row.get(8)?,
                    ip: row.get(9)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| RExError::Message(e.to_string()))?);
        }
        Ok(entries)
    }

    pub fn query_audit_stats(&self, filter: &AuditFilter) -> Result<AuditStats> {
        let conn = self.conn()?;
        let mut sql = String::from(
            "SELECT COUNT(*) AS total,
                    SUM(CASE WHEN result = 'success' THEN 1 ELSE 0 END) AS success_count,
                    SUM(CASE WHEN result = 'failure' THEN 1 ELSE 0 END) AS failure_count
             FROM audit_log WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(ref t) = filter.time_from {
            sql.push_str(&format!(" AND time >= ?{idx}"));
            params.push(Box::new(t.clone()));
            idx += 1;
        }
        if let Some(ref t) = filter.time_to {
            sql.push_str(&format!(" AND time <= ?{idx}"));
            params.push(Box::new(t.clone()));
            idx += 1;
        }
        if let Some(ref a) = filter.action {
            sql.push_str(&format!(" AND action = ?{idx}"));
            params.push(Box::new(a.clone()));
            idx += 1;
        }
        if let Some(ref env) = filter.environment_id {
            sql.push_str(&format!(" AND environment_id = ?{idx}"));
            params.push(Box::new(env.clone()));
        }
        if let Some(ref r) = filter.result {
            sql.push_str(&format!(" AND result = ?{idx}"));
            params.push(Box::new(r.clone()));
        }

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();

        let stats = conn
            .query_row(&sql, param_refs.as_slice(), |row| {
                Ok(AuditStats {
                    total: row.get(0)?,
                    success_count: row.get(1)?,
                    failure_count: row.get(2)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;

        Ok(stats)
    }

    // --- Environments ---

    pub fn list_environments(&self) -> Result<Vec<Environment>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, connection_mode, registration_token, created_at, updated_at FROM environments ORDER BY name")
            .map_err(|e| RExError::Message(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    connection_mode: row.get(3)?,
                    registration_token: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut envs = Vec::new();
        for row in rows {
            envs.push(row.map_err(|e| RExError::Message(e.to_string()))?);
        }
        Ok(envs)
    }

    pub fn get_environment(&self, id: &str) -> Result<Option<Environment>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, connection_mode, registration_token, created_at, updated_at FROM environments WHERE id = ?1")
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![id], |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    connection_mode: row.get(3)?,
                    registration_token: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        match rows.next() {
            Some(Ok(env)) => Ok(Some(env)),
            Some(Err(e)) => Err(RExError::Message(e.to_string())),
            None => Ok(None),
        }
    }

    pub fn get_environment_by_name(&self, name: &str) -> Result<Option<Environment>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, connection_mode, registration_token, created_at, updated_at FROM environments WHERE name = ?1")
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![name], |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    connection_mode: row.get(3)?,
                    registration_token: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        match rows.next() {
            Some(Ok(env)) => Ok(Some(env)),
            Some(Err(e)) => Err(RExError::Message(e.to_string())),
            None => Ok(None),
        }
    }

    pub fn create_environment(&self, env: &NewEnvironment) -> Result<Environment> {
        let conn = self.conn()?;
        let id = uuid::Uuid::new_v4().to_string();
        let token = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let desc = env.description.as_deref().unwrap_or("");
        let mode = env.connection_mode.as_deref().unwrap_or("direct");
        conn.execute(
            "INSERT INTO environments (id, name, description, connection_mode, registration_token, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, env.name, desc, mode, token, now, now],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(Environment {
            id,
            name: env.name.clone(),
            description: desc.to_string(),
            connection_mode: mode.to_string(),
            registration_token: token,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn update_environment(&self, id: &str, env: &UpdateEnvironment) -> Result<Environment> {
        let existing = self
            .get_environment(id)?
            .ok_or_else(|| RExError::Message("environment not found".into()))?;
        let name = env.name.as_deref().unwrap_or(&existing.name);
        let desc = env.description.as_deref().unwrap_or(&existing.description);
        let mode = env
            .connection_mode
            .as_deref()
            .unwrap_or(&existing.connection_mode);
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE environments SET name = ?1, description = ?2, connection_mode = ?3, updated_at = ?4 WHERE id = ?5",
            rusqlite::params![name, desc, mode, now, id],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        self.get_environment(id)?
            .ok_or_else(|| RExError::Message("environment not found after update".into()))
    }

    pub fn delete_environment(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM environments WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(())
    }

    // --- Environments with stats ---

    pub fn list_environments_with_stats(&self) -> Result<Vec<EnvironmentDetail>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.name, e.description, e.connection_mode, e.registration_token, e.created_at, e.updated_at,
                        COALESCE(r.res_count, 0) AS resource_count,
                        (SELECT a.status FROM agents a WHERE a.environment_id = e.id LIMIT 1) AS agent_status
                 FROM environments e
                 LEFT JOIN (SELECT environment_id, COUNT(*) AS res_count FROM resources GROUP BY environment_id) r ON r.environment_id = e.id
                 ORDER BY e.name",
            )
            .map_err(|e| RExError::Message(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EnvironmentDetail {
                    environment: Environment {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        connection_mode: row.get(3)?,
                        registration_token: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    },
                    resource_count: row.get(7)?,
                    agent_status: row.get(8)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut envs = Vec::new();
        for row in rows {
            envs.push(row.map_err(|e| RExError::Message(e.to_string()))?);
        }
        Ok(envs)
    }

    pub fn get_environment_with_stats(&self, id: &str) -> Result<Option<EnvironmentDetail>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.name, e.description, e.connection_mode, e.registration_token, e.created_at, e.updated_at,
                        COALESCE(r.res_count, 0) AS resource_count,
                        (SELECT a.status FROM agents a WHERE a.environment_id = e.id LIMIT 1) AS agent_status
                 FROM environments e
                 LEFT JOIN (SELECT environment_id, COUNT(*) AS res_count FROM resources GROUP BY environment_id) r ON r.environment_id = e.id
                 WHERE e.id = ?1",
            )
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![id], |row| {
                Ok(EnvironmentDetail {
                    environment: Environment {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        connection_mode: row.get(3)?,
                        registration_token: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    },
                    resource_count: row.get(7)?,
                    agent_status: row.get(8)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        match rows.next() {
            Some(Ok(env)) => Ok(Some(env)),
            Some(Err(e)) => Err(RExError::Message(e.to_string())),
            None => Ok(None),
        }
    }

    // --- Resources ---

    pub fn list_resources_by_env(&self, env_id: &str) -> Result<Vec<Resource>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, environment_id, name, protocol, host, port, username, config_json, color, sort_order, created_at, updated_at
                 FROM resources WHERE environment_id = ?1 ORDER BY sort_order, name",
            )
            .map_err(|e| RExError::Message(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![env_id], |row| {
                Ok(Resource {
                    id: row.get(0)?,
                    environment_id: row.get(1)?,
                    name: row.get(2)?,
                    protocol: row.get(3)?,
                    host: row.get(4)?,
                    port: row.get(5)?,
                    username: row.get(6)?,
                    config_json: row.get(7)?,
                    color: row.get(8)?,
                    sort_order: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut resources = Vec::new();
        for row in rows {
            resources.push(row.map_err(|e| RExError::Message(e.to_string()))?);
        }
        Ok(resources)
    }

    pub fn get_resource(&self, id: &str) -> Result<Option<Resource>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, environment_id, name, protocol, host, port, username, config_json, color, sort_order, created_at, updated_at
                 FROM resources WHERE id = ?1",
            )
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![id], |row| {
                Ok(Resource {
                    id: row.get(0)?,
                    environment_id: row.get(1)?,
                    name: row.get(2)?,
                    protocol: row.get(3)?,
                    host: row.get(4)?,
                    port: row.get(5)?,
                    username: row.get(6)?,
                    config_json: row.get(7)?,
                    color: row.get(8)?,
                    sort_order: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        match rows.next() {
            Some(Ok(r)) => Ok(Some(r)),
            Some(Err(e)) => Err(RExError::Message(e.to_string())),
            None => Ok(None),
        }
    }

    pub fn create_resource(&self, env_id: &str, res: &NewResource) -> Result<Resource> {
        let conn = self.conn()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let config = res.config_json.as_deref().unwrap_or("{}");
        let color = res.color.as_deref();
        let sort = res.sort_order.unwrap_or(0);
        let port = res.port.map(|p| p as i64);
        let username = res.username.as_deref().unwrap_or("");
        conn.execute(
            "INSERT INTO resources (id, environment_id, name, protocol, host, port, username, config_json, color, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![id, env_id, res.name, res.protocol, res.host, port, username, config, color, sort, now, now],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(Resource {
            id,
            environment_id: env_id.to_string(),
            name: res.name.clone(),
            protocol: res.protocol.clone(),
            host: res.host.clone(),
            port: res.port,
            username: username.to_string(),
            config_json: config.to_string(),
            color: color.map(|s| s.to_string()),
            sort_order: sort,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn update_resource(&self, env_id: &str, id: &str, res: &NewResource) -> Result<Resource> {
        let existing = self
            .get_resource(id)?
            .ok_or_else(|| RExError::Message("resource not found".into()))?;
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        let config = res.config_json.as_deref().unwrap_or(&existing.config_json);
        let color = res.color.as_deref().or(existing.color.as_deref());
        let sort = res.sort_order.unwrap_or(existing.sort_order);
        let port = res
            .port
            .map(|p| p as i64)
            .or_else(|| existing.port.map(|p| p as i64));
        let username = res.username.as_deref().unwrap_or(&existing.username);
        conn.execute(
            "UPDATE resources SET name = ?1, protocol = ?2, host = ?3, port = ?4, username = ?5, config_json = ?6, color = ?7, sort_order = ?8, updated_at = ?9
             WHERE environment_id = ?10 AND id = ?11",
            rusqlite::params![res.name, res.protocol, res.host, port, username, config, color, sort, now, env_id, id],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        self.get_resource(id)?
            .ok_or_else(|| RExError::Message("resource not found after update".into()))
    }

    /// 仅改写 SIP 资源的生效账户（`config_json.activeAccount`），其余字段不动。
    /// `account_id` 必须属于该资源的 `SipProfile.accounts`，否则拒绝。
    /// `config_json` 以密文存储，本函数负责解密-改-加密写回。
    pub fn set_resource_active_account(
        &self,
        crypto: &crate::crypto::CredentialCrypto,
        env_id: &str,
        id: &str,
        account_id: &str,
    ) -> Result<Resource> {
        let mut resource = self
            .get_resource(id)?
            .ok_or_else(|| RExError::Message("resource not found".into()))?;
        if resource.config_json.is_empty() || resource.config_json == "{}" {
            return Err(RExError::Message("resource has no config_json".into()));
        }
        let decrypted = crypto
            .decrypt(&resource.config_json)
            .map_err(|e| RExError::Message(format!("decrypt failed: {e}")))?;
        let mut profile: serde_json::Value = serde_json::from_str(&decrypted)
            .map_err(|e| RExError::Message(format!("invalid config_json: {e}")))?;
        let accounts = profile
            .get("accounts")
            .and_then(|a| a.as_array())
            .ok_or_else(|| RExError::Message("config_json missing accounts".into()))?;
        if !accounts
            .iter()
            .any(|a| a.get("id").and_then(|v| v.as_str()) == Some(account_id))
        {
            return Err(RExError::Message(format!(
                "account {account_id} not found in resource"
            )));
        }
        profile["activeAccount"] = serde_json::Value::String(account_id.to_string());
        let updated = serde_json::to_string(&profile)
            .map_err(|e| RExError::Message(format!("serialize failed: {e}")))?;
        let encrypted = crypto
            .encrypt(&updated)
            .map_err(|e| RExError::Message(format!("encrypt failed: {e}")))?;

        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE resources SET config_json = ?1, updated_at = ?2 WHERE environment_id = ?3 AND id = ?4",
            rusqlite::params![encrypted, now, env_id, id],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        resource.config_json = updated;
        resource.updated_at = now;
        Ok(resource)
    }

    pub fn delete_resource(&self, env_id: &str, id: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM resources WHERE environment_id = ?1 AND id = ?2",
            rusqlite::params![env_id, id],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(())
    }

    // --- Agents ---

    /// 通过 token 查找 agent，返回 agent ID
    pub fn find_agent_by_token(&self, token: &str) -> Result<Option<String>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id FROM agents WHERE token_hash = ?1")
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![token], |row| row.get::<_, String>(0))
            .map_err(|e| RExError::Message(e.to_string()))?;
        match rows.next() {
            Some(Ok(id)) => Ok(Some(id)),
            Some(Err(e)) => Err(RExError::Message(e.to_string())),
            None => Ok(None),
        }
    }

    pub fn find_environment_by_registration_token(
        &self,
        token: &str,
    ) -> Result<Option<Environment>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, connection_mode, registration_token, created_at, updated_at FROM environments WHERE registration_token = ?1")
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![token], |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    connection_mode: row.get(3)?,
                    registration_token: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        match rows.next() {
            Some(Ok(env)) => Ok(Some(env)),
            Some(Err(e)) => Err(RExError::Message(e.to_string())),
            None => Ok(None),
        }
    }

    pub fn find_agent_by_env_id(&self, env_id: &str) -> Result<Option<String>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id FROM agents WHERE environment_id = ?1 LIMIT 1")
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![env_id], |row| row.get::<_, String>(0))
            .map_err(|e| RExError::Message(e.to_string()))?;
        match rows.next() {
            Some(Ok(id)) => Ok(Some(id)),
            Some(Err(e)) => Err(RExError::Message(e.to_string())),
            None => Ok(None),
        }
    }

    pub fn list_all_agents(&self) -> Result<Vec<Agent>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, environment_id, name, version, os, arch, hostname, ip, status, last_seen_at, created_at, updated_at
                 FROM agents ORDER BY name",
            )
            .map_err(|e| RExError::Message(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Agent {
                    id: row.get(0)?,
                    environment_id: row.get(1)?,
                    name: row.get(2)?,
                    version: row.get(3)?,
                    os: row.get(4)?,
                    arch: row.get(5)?,
                    hostname: row.get(6)?,
                    ip: row.get(7)?,
                    status: row.get(8)?,
                    last_seen_at: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut agents = Vec::new();
        for row in rows {
            agents.push(row.map_err(|e| RExError::Message(e.to_string()))?);
        }
        Ok(agents)
    }

    pub fn list_agents_by_env(&self, env_id: &str) -> Result<Vec<Agent>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, environment_id, name, version, os, arch, hostname, ip, status, last_seen_at, created_at, updated_at
                 FROM agents WHERE environment_id = ?1 ORDER BY name",
            )
            .map_err(|e| RExError::Message(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![env_id], |row| {
                Ok(Agent {
                    id: row.get(0)?,
                    environment_id: row.get(1)?,
                    name: row.get(2)?,
                    version: row.get(3)?,
                    os: row.get(4)?,
                    arch: row.get(5)?,
                    hostname: row.get(6)?,
                    ip: row.get(7)?,
                    status: row.get(8)?,
                    last_seen_at: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut agents = Vec::new();
        for row in rows {
            agents.push(row.map_err(|e| RExError::Message(e.to_string()))?);
        }
        Ok(agents)
    }

    pub fn get_agent(&self, id: &str) -> Result<Option<Agent>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, environment_id, name, version, os, arch, hostname, ip, status, last_seen_at, created_at, updated_at
                 FROM agents WHERE id = ?1",
            )
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![id], |row| {
                Ok(Agent {
                    id: row.get(0)?,
                    environment_id: row.get(1)?,
                    name: row.get(2)?,
                    version: row.get(3)?,
                    os: row.get(4)?,
                    arch: row.get(5)?,
                    hostname: row.get(6)?,
                    ip: row.get(7)?,
                    status: row.get(8)?,
                    last_seen_at: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        match rows.next() {
            Some(Ok(a)) => Ok(Some(a)),
            Some(Err(e)) => Err(RExError::Message(e.to_string())),
            None => Ok(None),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_agent(
        &self,
        env_id: &str,
        name: &str,
        token_hash: &str,
        version: &str,
        os: &str,
        arch: &str,
        hostname: &str,
    ) -> Result<Agent> {
        let conn = self.conn()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agents (id, environment_id, name, token_hash, version, os, arch, hostname, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'online', ?9, ?9)",
            rusqlite::params![id, env_id, name, token_hash, version, os, arch, hostname, now],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(Agent {
            id,
            environment_id: env_id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            os: os.to_string(),
            arch: arch.to_string(),
            hostname: hostname.to_string(),
            ip: String::new(),
            status: "online".to_string(),
            last_seen_at: Some(now.clone()),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn update_agent_heartbeat(&self, id: &str, version: &str, ip: &str) -> Result<()> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agents SET version = ?1, ip = ?2, status = 'online', last_seen_at = ?3, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![version, ip, now, id],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(())
    }

    pub fn set_agent_offline(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agents SET status = 'offline', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(())
    }

    pub fn reset_agent_token(&self, id: &str, new_token_hash: &str) -> Result<()> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agents SET token_hash = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![new_token_hash, now, id],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(())
    }

    // --- SIP CDR ---

    /// Upsert 一通 CDR（按 id 插入或更新；通话进行中多次状态变更复用同 id）。
    pub fn upsert_cdr(&self, cdr: &NewCdr) -> Result<()> {
        let conn = self.conn()?;
        let duration = if cdr.duration_sec > 0 {
            cdr.duration_sec
        } else {
            0
        };
        conn.execute(
            "INSERT INTO cdr (id, resource_id, peer, call_id, start_time, end_time, duration_sec, direction, state, recording_url, pcap_url)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
             ON CONFLICT(id) DO UPDATE SET
               peer=excluded.peer, call_id=excluded.call_id, end_time=excluded.end_time,
               duration_sec=excluded.duration_sec, direction=excluded.direction,
               state=excluded.state, recording_url=excluded.recording_url, pcap_url=excluded.pcap_url",
            rusqlite::params![
                cdr.id, cdr.resource_id, cdr.peer, cdr.call_id, cdr.start_time,
                cdr.end_time, duration, cdr.direction, cdr.state,
                cdr.recording_url, cdr.pcap_url,
            ],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(())
    }

    /// 查询 CDR 列表，支持 resource/direction/state/时间范围过滤 + 排序 + 分页。
    pub fn query_cdr(&self, filter: &CdrFilter) -> Result<Vec<CdrRecord>> {
        let conn = self.conn()?;
        let mut sql = String::from(
            "SELECT id, resource_id, peer, call_id, start_time, end_time, duration_sec, direction, state, recording_url, pcap_url
             FROM cdr WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;
        if let Some(ref r) = filter.resource_id {
            sql.push_str(&format!(" AND resource_id = ?{idx}"));
            params.push(Box::new(r.clone()));
            idx += 1;
        }
        if let Some(ref d) = filter.direction {
            sql.push_str(&format!(" AND direction = ?{idx}"));
            params.push(Box::new(d.clone()));
            idx += 1;
        }
        if let Some(ref s) = filter.state {
            sql.push_str(&format!(" AND state = ?{idx}"));
            params.push(Box::new(s.clone()));
            idx += 1;
        }
        if let Some(ref f) = filter.from {
            sql.push_str(&format!(" AND start_time >= ?{idx}"));
            params.push(Box::new(f.clone()));
            idx += 1;
        }
        if let Some(ref t) = filter.to {
            sql.push_str(&format!(" AND start_time <= ?{idx}"));
            params.push(Box::new(t.clone()));
        }
        // 稳定排序：start_time DESC, id DESC（与审计日志一致）。
        match filter.sort.as_deref() {
            Some("start_asc") => sql.push_str(" ORDER BY start_time ASC, id ASC"),
            _ => sql.push_str(" ORDER BY start_time DESC, id DESC"),
        }
        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        if let Some(offset) = filter.offset {
            sql.push_str(&format!(" OFFSET {offset}"));
        }
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| RExError::Message(e.to_string()))?;
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(CdrRecord {
                    id: row.get(0)?,
                    resource_id: row.get(1)?,
                    peer: row.get(2)?,
                    call_id: row.get(3)?,
                    start_time: row.get(4)?,
                    end_time: row.get(5)?,
                    duration_sec: row.get(6)?,
                    direction: row.get(7)?,
                    state: row.get(8)?,
                    recording_url: row.get(9)?,
                    pcap_url: row.get(10)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| RExError::Message(e.to_string()))?);
        }
        Ok(out)
    }

    /// CDR 总数（供分页；与 query_cdr 同样过滤条件，但不分页/排序）。
    pub fn count_cdr(&self, filter: &CdrFilter) -> Result<i64> {
        let conn = self.conn()?;
        let mut sql = String::from("SELECT COUNT(*) FROM cdr WHERE 1=1");
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;
        if let Some(ref r) = filter.resource_id {
            sql.push_str(&format!(" AND resource_id = ?{idx}"));
            params.push(Box::new(r.clone()));
            idx += 1;
        }
        if let Some(ref d) = filter.direction {
            sql.push_str(&format!(" AND direction = ?{idx}"));
            params.push(Box::new(d.clone()));
            idx += 1;
        }
        if let Some(ref s) = filter.state {
            sql.push_str(&format!(" AND state = ?{idx}"));
            params.push(Box::new(s.clone()));
            idx += 1;
        }
        if let Some(ref f) = filter.from {
            sql.push_str(&format!(" AND start_time >= ?{idx}"));
            params.push(Box::new(f.clone()));
            idx += 1;
        }
        if let Some(ref t) = filter.to {
            sql.push_str(&format!(" AND start_time <= ?{idx}"));
            params.push(Box::new(t.clone()));
        }
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let n = conn
            .query_row(&sql, param_refs.as_slice(), |row| row.get::<_, i64>(0))
            .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(n)
    }

    /// 按 id 取单条 CDR（详情抽屉用）。
    pub fn get_cdr(&self, id: &str) -> Result<Option<CdrRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, resource_id, peer, call_id, start_time, end_time, duration_sec, direction, state, recording_url, pcap_url
                 FROM cdr WHERE id = ?1",
            )
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![id], |row| {
                Ok(CdrRecord {
                    id: row.get(0)?,
                    resource_id: row.get(1)?,
                    peer: row.get(2)?,
                    call_id: row.get(3)?,
                    start_time: row.get(4)?,
                    end_time: row.get(5)?,
                    duration_sec: row.get(6)?,
                    direction: row.get(7)?,
                    state: row.get(8)?,
                    recording_url: row.get(9)?,
                    pcap_url: row.get(10)?,
                })
            })
            .map_err(|e| RExError::Message(e.to_string()))?;
        match rows.next() {
            Some(Ok(r)) => Ok(Some(r)),
            Some(Err(e)) => Err(RExError::Message(e.to_string())),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_db() -> (tempfile::TempDir, Database) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        (dir, db)
    }

    // --- Settings ---

    #[test]
    fn test_get_set_setting() {
        let (_dir, db) = test_db();
        assert_eq!(db.get_setting("key1").unwrap(), None);
        db.set_setting("key1", "value1").unwrap();
        assert_eq!(db.get_setting("key1").unwrap(), Some("value1".into()));
        db.set_setting("key1", "value2").unwrap();
        assert_eq!(db.get_setting("key1").unwrap(), Some("value2".into()));
    }

    // --- Saved SQL Queries ---

    #[test]
    fn test_saved_queries_crud() {
        let (_dir, db) = test_db();
        // 初始为空
        assert!(db.list_saved_queries().unwrap().is_empty());

        // 新建
        let created = db
            .upsert_saved_query(&SavedQuery {
                id: String::new(),
                name: "q1".into(),
                sql: "SELECT 1".into(),
                db_type: Some("mysql".into()),
                updated_at: None,
            })
            .unwrap();
        assert!(!created.id.is_empty());
        assert!(created.updated_at.is_some());

        // 列表返回一个
        let list = db.list_saved_queries().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "q1");

        // 覆盖更新（同 id）
        let created2 = db
            .upsert_saved_query(&SavedQuery {
                id: created.id.clone(),
                name: "q1-renamed".into(),
                sql: "SELECT 2".into(),
                db_type: None,
                updated_at: None,
            })
            .unwrap();
        assert_eq!(created.id, created2.id);
        let list = db.list_saved_queries().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "q1-renamed");

        // 删除
        db.delete_saved_query(&created.id).unwrap();
        assert!(db.list_saved_queries().unwrap().is_empty());
    }

    // --- Audit Log ---

    #[test]
    fn test_write_and_query_audit_log() {
        let (_dir, db) = test_db();
        db.write_audit_log(&NewAuditEntry {
            action: "SSH_CONNECT".into(),
            result: "success".into(),
            ..Default::default()
        })
        .unwrap();
        let entries = db.query_audit_log(&AuditFilter::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "SSH_CONNECT");
    }

    #[test]
    fn test_audit_log_filter_by_action() {
        let (_dir, db) = test_db();
        db.write_audit_log(&NewAuditEntry {
            action: "SSH_CONNECT".into(),
            result: "success".into(),
            ..Default::default()
        })
        .unwrap();
        db.write_audit_log(&NewAuditEntry {
            action: "SQL_QUERY".into(),
            result: "success".into(),
            ..Default::default()
        })
        .unwrap();
        let filter = AuditFilter {
            action: Some("SSH_CONNECT".into()),
            ..Default::default()
        };
        let entries = db.query_audit_log(&filter).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "SSH_CONNECT");
    }

    #[test]
    fn test_audit_log_pagination_stable_order() {
        let (_dir, db) = test_db();
        // 写入 5 条，时间单调递增（time DESC 即最新在前）
        for i in 0..5 {
            db.write_audit_log(&NewAuditEntry {
                action: format!("ACTION_{i}"),
                result: "success".into(),
                ..Default::default()
            })
            .unwrap();
        }

        // 全量（limit 10）→ 期望 5 条，按 time DESC 顺序排列
        let all = db
            .query_audit_log(&AuditFilter {
                limit: Some(10),
                offset: Some(0),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(all.len(), 5);
        let all_ids: Vec<&str> = all.iter().map(|e| e.id.as_str()).collect();

        // 分页：第一页 3 条，第二页 2 条；两页拼接应等于全量顺序且互不重复
        let page1 = db
            .query_audit_log(&AuditFilter {
                limit: Some(3),
                offset: Some(0),
                ..Default::default()
            })
            .unwrap();
        let page2 = db
            .query_audit_log(&AuditFilter {
                limit: Some(3),
                offset: Some(3),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page1.len(), 3);
        assert_eq!(page2.len(), 2);

        let page1_ids: Vec<&str> = page1.iter().map(|e| e.id.as_str()).collect();
        let page2_ids: Vec<&str> = page2.iter().map(|e| e.id.as_str()).collect();
        let mut combined = page1_ids.clone();
        combined.extend(page2_ids.iter().copied());
        assert_eq!(combined, all_ids, "分页拼接必须与全量排序一致");

        // 稳定二级排序：time 相同也能靠 id 稳定区分，这里验证无重复 id
        let mut sorted = combined.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 5, "分页不应出现重复记录");
    }

    // --- SIP CDR ---

    fn sample_cdr(id: &str, peer: &str) -> NewCdr {
        NewCdr {
            id: id.into(),
            resource_id: "res-1".into(),
            peer: peer.into(),
            call_id: format!("call-{id}"),
            start_time: "2026-08-18T10:00:00Z".into(),
            end_time: Some("2026-08-18T10:05:00Z".into()),
            duration_sec: 300,
            direction: "out".into(),
            state: "ended".into(),
            recording_url: String::new(),
            pcap_url: String::new(),
        }
    }

    #[test]
    fn test_upsert_and_get_cdr() {
        let (_dir, db) = test_db();
        assert!(db.get_cdr("c1").unwrap().is_none());
        db.upsert_cdr(&sample_cdr("c1", "sip:bob@x")).unwrap();
        let rec = db.get_cdr("c1").unwrap().expect("CDR 应存在");
        assert_eq!(rec.resource_id, "res-1");
        assert_eq!(rec.peer, "sip:bob@x");
        assert_eq!(rec.direction, "out");
        assert_eq!(rec.state, "ended");
        assert_eq!(rec.duration_sec, 300);
    }

    #[test]
    fn test_cdr_upsert_is_idempotent_on_same_id() {
        let (_dir, db) = test_db();
        db.upsert_cdr(&sample_cdr("c1", "sip:bob@x")).unwrap();
        // 同 id 更新：修改 peer / state，不应新增行
        let mut updated = sample_cdr("c1", "sip:alice@y");
        updated.state = "missed".into();
        db.upsert_cdr(&updated).unwrap();
        let all = db.query_cdr(&CdrFilter::default()).unwrap();
        assert_eq!(all.len(), 1, "同 id upsert 不新增行");
        assert_eq!(all[0].peer, "sip:alice@y");
        assert_eq!(all[0].state, "missed");
    }

    #[test]
    fn test_cdr_filter_and_pagination() {
        let (_dir, db) = test_db();
        db.upsert_cdr(&sample_cdr("c1", "sip:bob@x")).unwrap();
        let mut c2 = sample_cdr("c2", "sip:carol@z");
        c2.direction = "in".into();
        db.upsert_cdr(&c2).unwrap();
        db.upsert_cdr(&sample_cdr("c3", "sip:dave@w")).unwrap();

        // 过滤 direction=in
        let inbound = db
            .query_cdr(&CdrFilter {
                direction: Some("in".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(inbound.len(), 1);
        assert_eq!(inbound[0].peer, "sip:carol@z");
        assert_eq!(
            db.count_cdr(&CdrFilter {
                direction: Some("in".into()),
                ..Default::default()
            })
            .unwrap(),
            1
        );

        // 分页 + 稳定排序（start_time DESC, id DESC）
        let page1 = db
            .query_cdr(&CdrFilter {
                limit: Some(2),
                offset: Some(0),
                ..Default::default()
            })
            .unwrap();
        let page2 = db
            .query_cdr(&CdrFilter {
                limit: Some(2),
                offset: Some(2),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 1);
        let p1: Vec<&str> = page1.iter().map(|r| r.id.as_str()).collect();
        let p2: Vec<&str> = page2.iter().map(|r| r.id.as_str()).collect();
        assert!(!p1.contains(&p2[0]), "分页不应重叠");

        // start_asc 排序反向前两条，应与 start_desc 的逆向一致（start_time 相同，稳定二级排序按 id）。
        let asc = db
            .query_cdr(&CdrFilter {
                sort: Some("start_asc".into()),
                limit: Some(2),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(asc.len(), 2);
        let desc = db
            .query_cdr(&CdrFilter {
                limit: Some(2),
                ..Default::default()
            })
            .unwrap();
        // start_time 相同，稳定二级排序按 id：
        //   asc 取 id 最小在前 → asc[0] == "c1"；
        //   desc 取 id 最大在前 → 末条 == "c1"（即 id 最小者落在第二页末）。
        assert_eq!(asc[0].id, "c1", "asc 首条应为 id 最小者");
        assert_eq!(desc[0].id, "c3", "desc 首条应为 id 最大者");
        assert_eq!(desc[desc.len() - 1].id, "c2");
    }

    // --- Environments ---

    #[test]
    fn test_create_and_get_environment() {
        let (_dir, db) = test_db();
        let env = db
            .create_environment(&NewEnvironment {
                name: "test".into(),
                description: Some("desc".into()),
                connection_mode: Some("direct".into()),
            })
            .unwrap();
        assert_eq!(env.name, "test");
        assert_eq!(env.connection_mode, "direct");

        let got = db.get_environment(&env.id).unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().name, "test");
    }

    #[test]
    fn test_list_environments() {
        let (_dir, db) = test_db();
        db.create_environment(&NewEnvironment {
            name: "env1".into(),
            description: None,
            connection_mode: None,
        })
        .unwrap();
        db.create_environment(&NewEnvironment {
            name: "env2".into(),
            description: None,
            connection_mode: None,
        })
        .unwrap();
        let envs = db.list_environments().unwrap();
        assert_eq!(envs.len(), 2);
    }

    #[test]
    fn test_update_environment() {
        let (_dir, db) = test_db();
        let env = db
            .create_environment(&NewEnvironment {
                name: "old".into(),
                description: None,
                connection_mode: None,
            })
            .unwrap();
        let updated = db
            .update_environment(
                &env.id,
                &UpdateEnvironment {
                    name: Some("new".into()),
                    description: None,
                    connection_mode: None,
                },
            )
            .unwrap();
        assert_eq!(updated.name, "new");
    }

    #[test]
    fn test_delete_environment() {
        let (_dir, db) = test_db();
        let env = db
            .create_environment(&NewEnvironment {
                name: "test".into(),
                description: None,
                connection_mode: None,
            })
            .unwrap();
        db.delete_environment(&env.id).unwrap();
        assert!(db.get_environment(&env.id).unwrap().is_none());
    }

    // --- Resources ---

    #[test]
    fn test_create_and_list_resources() {
        let (_dir, db) = test_db();
        let env = db
            .create_environment(&NewEnvironment {
                name: "env".into(),
                description: None,
                connection_mode: None,
            })
            .unwrap();
        db.create_resource(
            &env.id,
            &NewResource {
                name: "res1".into(),
                protocol: "ssh".into(),
                host: "192.168.1.1".into(),
                port: None,
                username: None,
                config_json: None,
                color: None,
                sort_order: None,
            },
        )
        .unwrap();
        let resources = db.list_resources_by_env(&env.id).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "res1");
    }

    #[test]
    fn test_get_resource() {
        let (_dir, db) = test_db();
        let env = db
            .create_environment(&NewEnvironment {
                name: "env".into(),
                description: None,
                connection_mode: None,
            })
            .unwrap();
        let res = db
            .create_resource(
                &env.id,
                &NewResource {
                    name: "res1".into(),
                    protocol: "ssh".into(),
                    host: "192.168.1.1".into(),
                    port: None,
                    username: None,
                    config_json: None,
                    color: None,
                    sort_order: None,
                },
            )
            .unwrap();
        let got = db.get_resource(&res.id).unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().name, "res1");
    }

    #[test]
    fn test_delete_resource() {
        let (_dir, db) = test_db();
        let env = db
            .create_environment(&NewEnvironment {
                name: "env".into(),
                description: None,
                connection_mode: None,
            })
            .unwrap();
        let res = db
            .create_resource(
                &env.id,
                &NewResource {
                    name: "res1".into(),
                    protocol: "ssh".into(),
                    host: "192.168.1.1".into(),
                    port: None,
                    username: None,
                    config_json: None,
                    color: None,
                    sort_order: None,
                },
            )
            .unwrap();
        db.delete_resource(&env.id, &res.id).unwrap();
        assert!(db.get_resource(&res.id).unwrap().is_none());
    }

    // --- set_resource_active_account ---

    #[test]
    fn test_set_resource_active_account() {
        use crate::crypto::CredentialCrypto;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        // 同一数据目录派生密钥：加解密一致（与运行时 AppState.crypto 同源）。
        let crypto = CredentialCrypto::from_data_dir(dir.path()).unwrap();

        let env = db
            .create_environment(&NewEnvironment {
                name: "env".into(),
                description: None,
                connection_mode: None,
            })
            .unwrap();

        let profile = serde_json::json!({
            "accounts": [
                { "id": "a1", "server": "sip1.example.com", "port": 5060, "transport": "udp", "username": "u1", "displayName": "A1" },
                { "id": "a2", "server": "sip2.example.com", "port": 5060, "transport": "udp", "username": "u2", "displayName": "A2" },
            ],
            "activeAccount": "a1",
        });
        let encrypted = crypto.encrypt(&profile.to_string()).unwrap();

        let res = db
            .create_resource(
                &env.id,
                &NewResource {
                    name: "sip-res".into(),
                    protocol: "sip".into(),
                    host: "sip.example.com".into(),
                    port: None,
                    username: None,
                    config_json: Some(encrypted),
                    color: None,
                    sort_order: None,
                },
            )
            .unwrap();

        // 有效 account_id：写回成功，activeAccount 改变。
        // 返回值的 config_json 为明文（与 get_resource handler 解密后返回给前端一致）。
        let updated = db
            .set_resource_active_account(&crypto, &env.id, &res.id, "a2")
            .unwrap();
        let decrypted: serde_json::Value = serde_json::from_str(&updated.config_json).unwrap();
        assert_eq!(decrypted["activeAccount"], "a2");

        // 持久化验证：重新读取仍为 a2。
        let reread = db.get_resource(&res.id).unwrap().unwrap();
        let reread_dec: serde_json::Value =
            serde_json::from_str(&crypto.decrypt(&reread.config_json).unwrap()).unwrap();
        assert_eq!(reread_dec["activeAccount"], "a2");

        // 无效 account_id：拒绝。
        let err = db
            .set_resource_active_account(&crypto, &env.id, &res.id, "nope")
            .unwrap_err();
        assert!(err.to_string().contains("not found in resource"));

        // 不存在的资源：拒绝。
        let missing = db
            .set_resource_active_account(&crypto, &env.id, "ghost", "a1")
            .unwrap_err();
        assert!(missing.to_string().contains("resource not found"));
    }

    #[test]
    fn test_set_resource_active_account_rejects_empty_config() {
        use crate::crypto::CredentialCrypto;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let crypto = CredentialCrypto::from_data_dir(dir.path()).unwrap();

        let env = db
            .create_environment(&NewEnvironment {
                name: "env".into(),
                description: None,
                connection_mode: None,
            })
            .unwrap();
        let res = db
            .create_resource(
                &env.id,
                &NewResource {
                    name: "ssh-res".into(),
                    protocol: "ssh".into(),
                    host: "192.168.1.1".into(),
                    port: None,
                    username: None,
                    config_json: None,
                    color: None,
                    sort_order: None,
                },
            )
            .unwrap();

        // config_json 为空：拒绝（非 SIP 资源不应被误改）。
        let err = db
            .set_resource_active_account(&crypto, &env.id, &res.id, "a1")
            .unwrap_err();
        assert!(err.to_string().contains("no config_json"));
    }

    // --- Agents ---

    #[test]
    fn test_create_and_get_agent() {
        let (_dir, db) = test_db();
        let env = db
            .create_environment(&NewEnvironment {
                name: "env".into(),
                description: None,
                connection_mode: None,
            })
            .unwrap();
        let agent = db
            .create_agent(
                &env.id,
                "agent1",
                "token_hash",
                "1.0.0",
                "linux",
                "amd64",
                "host1",
            )
            .unwrap();
        assert_eq!(agent.name, "agent1");
        assert_eq!(agent.status, "online");

        let got = db.get_agent(&agent.id).unwrap();
        assert!(got.is_some());
    }

    #[test]
    fn test_agent_heartbeat() {
        let (_dir, db) = test_db();
        let env = db
            .create_environment(&NewEnvironment {
                name: "env".into(),
                description: None,
                connection_mode: None,
            })
            .unwrap();
        let agent = db
            .create_agent(
                &env.id,
                "agent1",
                "token_hash",
                "1.0.0",
                "linux",
                "amd64",
                "host1",
            )
            .unwrap();
        db.update_agent_heartbeat(&agent.id, "1.1.0", "10.0.0.1")
            .unwrap();
        let updated = db.get_agent(&agent.id).unwrap().unwrap();
        assert_eq!(updated.version, "1.1.0");
        assert_eq!(updated.ip, "10.0.0.1");
    }

    #[test]
    fn test_agent_offline() {
        let (_dir, db) = test_db();
        let env = db
            .create_environment(&NewEnvironment {
                name: "env".into(),
                description: None,
                connection_mode: None,
            })
            .unwrap();
        let agent = db
            .create_agent(
                &env.id,
                "agent1",
                "token_hash",
                "1.0.0",
                "linux",
                "amd64",
                "host1",
            )
            .unwrap();
        db.set_agent_offline(&agent.id).unwrap();
        let updated = db.get_agent(&agent.id).unwrap().unwrap();
        assert_eq!(updated.status, "offline");
    }

    #[test]
    fn test_list_agents_by_env() {
        let (_dir, db) = test_db();
        let env = db
            .create_environment(&NewEnvironment {
                name: "env".into(),
                description: None,
                connection_mode: None,
            })
            .unwrap();
        db.create_agent(
            &env.id,
            "agent1",
            "token_hash",
            "1.0.0",
            "linux",
            "amd64",
            "host1",
        )
        .unwrap();
        let agents = db.list_agents_by_env(&env.id).unwrap();
        assert_eq!(agents.len(), 1);
    }
}
