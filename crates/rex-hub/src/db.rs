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

    // --- Audit Log ---

    pub fn write_audit_log(&self, entry: &NewAuditEntry) -> Result<()> {
        let conn = self.conn()?;
        let id = uuid::Uuid::new_v4().to_string();
        let time = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO audit_log (id, time, action, target, environment_id, resource_id, agent_id, result, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                id, time, entry.action, entry.target,
                entry.environment_id, entry.resource_id, entry.agent_id,
                entry.result, entry.detail,
            ],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(())
    }

    pub fn query_audit_log(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> {
        let conn = self.conn()?;
        let mut sql = String::from(
            "SELECT id, time, action, target, environment_id, resource_id, agent_id, result, detail
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

        sql.push_str(" ORDER BY time DESC");

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
        let mut sql = String::from("SELECT COUNT(*) AS total FROM audit_log WHERE 1=1");
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

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();

        let total: i64 = conn
            .query_row(&sql, param_refs.as_slice(), |row| row.get(0))
            .map_err(|e| RExError::Message(e.to_string()))?;

        // Reuse same WHERE for success/failure counts
        let mut sql_success = sql.replace("COUNT(*) AS total", "COUNT(*) AS sc");
        let mut sql_failure = sql.replace("COUNT(*) AS total", "COUNT(*) AS fc");
        sql_success.push_str(" AND result = 'success'");
        sql_failure.push_str(" AND result = 'failure'");

        let success_count: i64 = conn
            .query_row(&sql_success, param_refs.as_slice(), |row| row.get(0))
            .map_err(|e| RExError::Message(e.to_string()))?;
        let failure_count: i64 = conn
            .query_row(&sql_failure, param_refs.as_slice(), |row| row.get(0))
            .map_err(|e| RExError::Message(e.to_string()))?;

        Ok(AuditStats {
            total,
            success_count,
            failure_count,
        })
    }

    // --- Environments ---

    pub fn list_environments(&self) -> Result<Vec<Environment>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, connection_mode, created_at, updated_at FROM environments ORDER BY name")
            .map_err(|e| RExError::Message(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    connection_mode: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
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
            .prepare("SELECT id, name, description, connection_mode, created_at, updated_at FROM environments WHERE id = ?1")
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![id], |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    connection_mode: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
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
            .prepare("SELECT id, name, description, connection_mode, created_at, updated_at FROM environments WHERE name = ?1")
            .map_err(|e| RExError::Message(e.to_string()))?;
        let mut rows = stmt
            .query_map(rusqlite::params![name], |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    connection_mode: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
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
        let now = chrono::Utc::now().to_rfc3339();
        let desc = env.description.as_deref().unwrap_or("");
        let mode = env.connection_mode.as_deref().unwrap_or("direct");
        conn.execute(
            "INSERT INTO environments (id, name, description, connection_mode, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, env.name, desc, mode, now, now],
        )
        .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(Environment {
            id,
            name: env.name.clone(),
            description: desc.to_string(),
            connection_mode: mode.to_string(),
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
                "SELECT e.id, e.name, e.description, e.connection_mode, e.created_at, e.updated_at,
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
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    },
                    resource_count: row.get(6)?,
                    agent_status: row.get(7)?,
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
                "SELECT e.id, e.name, e.description, e.connection_mode, e.created_at, e.updated_at,
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
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    },
                    resource_count: row.get(6)?,
                    agent_status: row.get(7)?,
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
