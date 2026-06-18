use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::helpers::{err_resp, gen_id, now_iso, ApiResponse, ErrorResponse};
use crate::routes::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub time: String,
    pub user: String,
    pub ip: Option<String>,
    pub environment_id: Option<String>,
    pub resource_id: Option<String>,
    pub agent_id: Option<String>,
    #[serde(rename = "type")]
    pub log_type: String,
    pub result: String,
    pub summary: String,
    pub detail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    #[serde(rename = "type")]
    pub log_type: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogList {
    pub items: Vec<AuditLogEntry>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

pub fn write_audit_log(
    db: &crate::db::Database,
    log_type: &str,
    result: &str,
    summary: &str,
    environment_id: Option<&str>,
    resource_id: Option<&str>,
    agent_id: Option<&str>,
    detail: Option<&str>,
    ip: Option<&str>,
) {
    let id = gen_id("log");
    let time = now_iso();

    if let Ok(conn) = db.pool.get() {
        let _ = conn.execute(
            "INSERT INTO audit_log (id, time, user, ip, environment_id, resource_id, agent_id, type, result, summary, detail_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![id, time, "admin", ip, environment_id, resource_id, agent_id, log_type, result, summary, detail],
        );
    }
}

pub async fn list_audit_log(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<ApiResponse<AuditLogList>>, (StatusCode, Json<ErrorResponse>)> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).min(100);
    let offset = (page - 1) * page_size;

    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        let mut where_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref from) = query.from {
            where_clauses.push(format!("time >= ?{}", params.len() + 1));
            params.push(Box::new(from.clone()));
        }
        if let Some(ref to) = query.to {
            where_clauses.push(format!("time <= ?{}", params.len() + 1));
            params.push(Box::new(to.clone()));
        }
        if let Some(ref log_type) = query.log_type {
            where_clauses.push(format!("type = ?{}", params.len() + 1));
            params.push(Box::new(log_type.clone()));
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM audit_log {where_sql}");
        let total: i64 = conn.query_row(&count_sql, rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| row.get(0))
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        let query_sql = format!(
            "SELECT id, time, user, ip, environment_id, resource_id, agent_id, type, result, summary, detail_json FROM audit_log {where_sql} ORDER BY time DESC LIMIT ?{} OFFSET ?{}",
            params.len() + 1,
            params.len() + 2
        );

        params.push(Box::new(page_size as i64));
        params.push(Box::new(offset as i64));

        let mut stmt = conn.prepare(&query_sql)
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        let items: Vec<AuditLogEntry> = stmt.query_map(rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
            Ok(AuditLogEntry {
                id: row.get(0)?,
                time: row.get(1)?,
                user: row.get(2)?,
                ip: row.get(3)?,
                environment_id: row.get(4)?,
                resource_id: row.get(5)?,
                agent_id: row.get(6)?,
                log_type: row.get(7)?,
                result: row.get(8)?,
                summary: row.get(9)?,
                detail: row.get(10)?,
            })
        })
        .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?
        .filter_map(|r| r.ok())
        .collect();

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(AuditLogList { items, total, page, page_size })
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    Ok(Json(ApiResponse { data: result }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn write_audit_log_works() {
        let db = Database::new_in_memory().unwrap();
        write_audit_log(
            &db,
            "login",
            "success",
            "登录成功",
            None,
            None,
            None,
            None,
            Some("127.0.0.1"),
        );

        let count: i64 = db
            .pool
            .get()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // 验证 IP 字段
        let ip: Option<String> = db
            .pool
            .get()
            .unwrap()
            .query_row("SELECT ip FROM audit_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(ip, Some("127.0.0.1".to_string()));
    }

    #[test]
    fn write_audit_log_with_refs() {
        let db = Database::new_in_memory().unwrap();
        write_audit_log(
            &db,
            "environment_create",
            "success",
            "创建环境「测试」",
            Some("env_12345678"),
            None,
            None,
            None,
            None,
        );

        let count: i64 = db
            .pool
            .get()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn write_audit_log_without_ip() {
        let db = Database::new_in_memory().unwrap();
        write_audit_log(&db, "test", "success", "test", None, None, None, None, None);

        let ip: Option<String> = db
            .pool
            .get()
            .unwrap()
            .query_row("SELECT ip FROM audit_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(ip, None);
    }
}
