use anyhow::{Context, Result};
use r2d2::{ManageConnection, Pool};
use rusqlite::Connection;
use std::path::Path;

pub type ConnPool = Pool<SqliteManager>;

pub struct SqliteManager {
    db_path: Option<String>,
}

impl SqliteManager {
    fn memory() -> Self {
        Self { db_path: None }
    }

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
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
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
}

impl Database {
    pub fn new(db_path: &Path) -> Result<Self> {
        let manager = SqliteManager::file(db_path);
        let pool = Pool::builder()
            .build(manager)
            .context("failed to create connection pool")?;
        Ok(Self { pool })
    }

    pub fn new_in_memory() -> Result<Self> {
        let manager = SqliteManager::memory();
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .context("failed to create connection pool")?;
        Ok(Self { pool })
    }
}

fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("migrations.sql"))
        .context("failed to run database migrations")?;
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
}
