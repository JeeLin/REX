use anyhow::{Context, Result};
use r2d2::{ManageConnection, Pool};
use rusqlite::Connection;
use std::path::Path;
use tempfile::TempDir;

pub type ConnPool = Pool<SqliteManager>;

pub struct SqliteManager {
    db_path: Option<String>,
}

impl SqliteManager {
    fn file(path: &Path) -> Self {
        Self {
            db_path: Some(path.to_string_lossy().into_owned()),
        }
    }
}

impl ManageConnection for SqliteManager {
    type Connection = Connection;
    type Error = rusqlite::Error;

    fn connect(&self) -> std::result::Result<Self::Connection, Self::Error> {
        let conn = if let Some(path) = &self.db_path {
            Connection::open(path)?
        } else {
            Connection::open_in_memory()?
        };
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")?;
        run_migrations(&conn).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        Ok(conn)
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> std::result::Result<(), Self::Error> {
        conn.execute_batch("PRAGMA foreign_keys=ON;")
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}

pub struct Database {
    pub pool: ConnPool,
    _tempdir: Option<TempDir>,
}

impl Database {
    pub fn new(db_path: &Path) -> Result<Self> {
        let manager = SqliteManager::file(db_path);
        let pool = Pool::builder()
            .build(manager)
            .context("failed to create connection pool")?;
        Ok(Self {
            pool,
            _tempdir: None,
        })
    }

    pub fn new_in_memory() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("db.sqlite");
        let manager = SqliteManager::file(&db_path);
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .context("failed to create connection pool")?;
        Ok(Self {
            pool,
            _tempdir: Some(temp_dir),
        })
    }

    pub fn get_resource_by_id(&self, id: &str) -> Result<Option<super::resource::Resource>> {
        let conn = self.pool.get().context("failed to get connection")?;
        let result = conn.query_row(
            "SELECT id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at FROM resources WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok(super::resource::Resource {
                id: row.get(0)?,
                environment_id: row.get(1)?,
                name: row.get(2)?,
                protocol: row.get(3)?,
                agent_id: row.get(4)?,
                config_json: row.get(5)?,
                status: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            }),
        );
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_sql_resources(
        &self,
        environment_id: &str,
    ) -> Result<Vec<super::resource::Resource>> {
        let conn = self.pool.get().context("failed to get connection")?;
        let mut stmt = conn.prepare(
            "SELECT id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at FROM resources WHERE environment_id = ?1 AND protocol IN ('mysql', 'postgresql')"
        )?;
        let rows = stmt.query_map(rusqlite::params![environment_id], |row| {
            Ok(super::resource::Resource {
                id: row.get(0)?,
                environment_id: row.get(1)?,
                name: row.get(2)?,
                protocol: row.get(3)?,
                agent_id: row.get(4)?,
                config_json: row.get(5)?,
                status: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let mut resources = Vec::new();
        for row in rows {
            resources.push(row?);
        }
        Ok(resources)
    }
}

fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("migrations.sql"))
        .context("failed to run database migrations")?;

    // Migration: remove connection_mode from resources (it belongs to environment, not resource)
    let _ = conn.execute_batch("ALTER TABLE resources DROP COLUMN connection_mode");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_in_memory_creates_tables() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.pool.get().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM environments", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn new_in_memory_creates_all_tables() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.pool.get().unwrap();
        let tables = [
            "environments",
            "agents",
            "resources",
            "audit_log",
            "settings",
        ];
        for table in &tables {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "table {table} should exist and be empty");
        }
    }

    #[test]
    fn new_in_memory_returns_empty_resource() {
        let db = Database::new_in_memory().unwrap();
        let result = db.get_resource_by_id("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn new_in_memory_insert_and_get_resource() {
        let db = Database::new_in_memory().unwrap();
        {
            let conn = db.pool.get().unwrap();
            // First create an environment (required by foreign key)
            conn.execute(
                "INSERT INTO environments (id, name, description, connection_mode, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params!["env_001", "test-env", "test", "direct", "2024-01-01", "2024-01-01"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO resources (id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    "res_001",
                    "env_001",
                    "test-resource",
                    "ssh",
                    None::<String>,
                    "{}",
                    "ready",
                    "2024-01-01",
                    "2024-01-01"
                ],
            )
            .unwrap();
        }
        let resource = db.get_resource_by_id("res_001").unwrap();
        assert!(resource.is_some());
        let resource = resource.unwrap();
        assert_eq!(resource.id, "res_001");
        assert_eq!(resource.name, "test-resource");
    }

    #[test]
    fn new_in_memory_insert_and_delete_resource() {
        let db = Database::new_in_memory().unwrap();
        {
            let conn = db.pool.get().unwrap();
            // First create an environment (required by foreign key)
            conn.execute(
                "INSERT INTO environments (id, name, description, connection_mode, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params!["env_001", "test-env", "test", "direct", "2024-01-01", "2024-01-01"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO resources (id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    "res_002",
                    "env_001",
                    "test-resource-2",
                    "mysql",
                    None::<String>,
                    "{}",
                    "ready",
                    "2024-01-01",
                    "2024-01-01"
                ],
            )
            .unwrap();
            let deleted = conn
                .execute(
                    "DELETE FROM resources WHERE id = ?1",
                    rusqlite::params!["res_002"],
                )
                .unwrap();
            assert_eq!(deleted, 1);
        }
        let resource = db.get_resource_by_id("res_002").unwrap();
        assert!(resource.is_none());
    }
}
