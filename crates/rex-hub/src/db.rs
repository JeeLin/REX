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

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
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
        let existing = self.get_environment(id)?
            .ok_or_else(|| RExError::Message("environment not found".into()))?;
        let name = env.name.as_deref().unwrap_or(&existing.name);
        let desc = env.description.as_deref().unwrap_or(&existing.description);
        let mode = env.connection_mode.as_deref().unwrap_or(&existing.connection_mode);
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
        conn.execute("DELETE FROM environments WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| RExError::Message(e.to_string()))?;
        Ok(())
    }
}
