use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::helpers::{
    bad_request, err_resp, gen_id, not_found, now_iso, ApiResponse, ErrorResponse,
};
use crate::routes::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTag {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTag {
    pub name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetResourceTags {
    pub tag_ids: Vec<String>,
}

// ── Tag CRUD ─────────────────────────────────────────────

pub async fn list_tags(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<Tag>>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let tags = tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let mut stmt = conn
            .prepare("SELECT id, name, color, created_at FROM tags ORDER BY name ASC")
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let tags: Vec<Tag> = rows.filter_map(|r| r.ok()).collect();
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(tags)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: tags }))
}

pub async fn get_tag(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Tag>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let tag = tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        conn.query_row(
            "SELECT id, name, color, created_at FROM tags WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
        .map_err(|_| not_found("TAG_NOT_FOUND", "标签不存在"))
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: tag }))
}

pub async fn create_tag(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateTag>,
) -> Result<(StatusCode, Json<ApiResponse<Tag>>), (StatusCode, Json<ErrorResponse>)> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(bad_request("标签名称不能为空"));
    }
    let color = input.color.unwrap_or_else(|| "#58A6FF".to_string());

    let db = state.db.clone();
    let name_clone = name.clone();
    let color_clone = color.clone();

    let tag = tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        // Check duplicate name
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM tags WHERE name = ?1",
                rusqlite::params![name_clone],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        if exists {
            return Err(bad_request("标签名称已存在"));
        }

        let id = gen_id("tag");
        let now = now_iso();
        conn.execute(
            "INSERT INTO tags (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, name_clone, color_clone, now.clone()],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(Tag {
            id,
            name: name_clone,
            color: color_clone,
            created_at: now,
        })
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    Ok((StatusCode::CREATED, Json(ApiResponse { data: tag })))
}

pub async fn update_tag(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<UpdateTag>,
) -> Result<Json<ApiResponse<Tag>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let id_clone = id.clone();

    let tag = tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let existing: Tag = conn
            .query_row(
                "SELECT id, name, color, created_at FROM tags WHERE id = ?1",
                rusqlite::params![id_clone],
                |row| {
                    Ok(Tag {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        color: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                },
            )
            .map_err(|_| not_found("TAG_NOT_FOUND", "标签不存在"))?;

        let orig_name = existing.name.clone();
        let name = input
            .name
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or(orig_name.clone());
        let color = input.color.unwrap_or(existing.color);

        // Check duplicate name if changed
        if name != orig_name {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM tags WHERE name = ?1 AND id != ?2",
                    rusqlite::params![name, id_clone],
                    |row| row.get::<_, i64>(0),
                )
                .map(|count| count > 0)
                .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
            if exists {
                return Err(bad_request("标签名称已存在"));
            }
        }

        conn.execute(
            "UPDATE tags SET name = ?1, color = ?2 WHERE id = ?3",
            rusqlite::params![name, color, id_clone],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(Tag {
            id: existing.id,
            name,
            color,
            created_at: existing.created_at,
        })
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    Ok(Json(ApiResponse { data: tag }))
}

pub async fn delete_tag(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let id_clone = id.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let affected = conn
            .execute(
                "DELETE FROM tags WHERE id = ?1",
                rusqlite::params![id_clone],
            )
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        if affected == 0 {
            return Err(not_found("TAG_NOT_FOUND", "标签不存在"));
        }
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(StatusCode::NO_CONTENT)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?
}

// ── Resource-Tag Association ─────────────────────────────

pub async fn get_resource_tags(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Tag>>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let rid = resource_id.clone();

    let tags = tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.name, t.color, t.created_at FROM tags t
             INNER JOIN resource_tags rt ON t.id = rt.tag_id
             WHERE rt.resource_id = ?1
             ORDER BY t.name ASC",
            )
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let rows = stmt
            .query_map(rusqlite::params![rid], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let tags: Vec<Tag> = rows.filter_map(|r| r.ok()).collect();
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(tags)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    Ok(Json(ApiResponse { data: tags }))
}

pub async fn set_resource_tags(
    State(state): State<Arc<AppState>>,
    Path(resource_id): Path<String>,
    Json(input): Json<SetResourceTags>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let rid = resource_id.clone();
    let tag_ids = input.tag_ids;

    tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        // Verify resource exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM resources WHERE id = ?1",
                rusqlite::params![rid],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        if !exists {
            return Err(not_found("RESOURCE_NOT_FOUND", "资源不存在"));
        }

        // Replace all tags for this resource
        conn.execute(
            "DELETE FROM resource_tags WHERE resource_id = ?1",
            rusqlite::params![rid],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        for tag_id in &tag_ids {
            conn.execute(
                "INSERT INTO resource_tags (resource_id, tag_id) VALUES (?1, ?2)",
                rusqlite::params![rid, tag_id],
            )
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        }

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(StatusCode::NO_CONTENT)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?
}

// ── Get resources by tag (for filtering) ─────────────────

#[derive(Debug, Serialize)]
pub struct ResourceTagInfo {
    pub resource_id: String,
    pub tags: Vec<Tag>,
}

pub async fn list_resources_by_tag(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<ResourceTagInfo>>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();

    let result = tokio::task::spawn_blocking(move || {
        let conn = db
            .pool
            .get()
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        // Get all resources with their tags
        let mut stmt = conn
            .prepare(
                "SELECT r.id, t.id, t.name, t.color, t.created_at
             FROM resources r
             LEFT JOIN resource_tags rt ON r.id = rt.resource_id
             LEFT JOIN tags t ON rt.tag_id = t.id
             ORDER BY r.name ASC",
            )
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        let rows = stmt
            .query_map([], |row| {
                let res_id: String = row.get(0)?;
                let tag_id: Option<String> = row.get(1)?;
                let tag_name: Option<String> = row.get(2)?;
                let tag_color: Option<String> = row.get(3)?;
                let tag_created: Option<String> = row.get(4)?;
                Ok((res_id, tag_id, tag_name, tag_color, tag_created))
            })
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        // Group tags by resource
        let mut map: std::collections::HashMap<String, Vec<Tag>> = std::collections::HashMap::new();
        for row in rows.filter_map(|r| r.ok()) {
            if let (res_id, Some(tag_id), Some(name), Some(color), Some(created)) = row {
                map.entry(res_id).or_default().push(Tag {
                    id: tag_id,
                    name,
                    color,
                    created_at: created,
                });
            }
        }

        let result: Vec<ResourceTagInfo> = map
            .into_iter()
            .map(|(resource_id, tags)| ResourceTagInfo { resource_id, tags })
            .collect();

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(result)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;

    Ok(Json(ApiResponse { data: result }))
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn test_db() -> Arc<Database> {
        Arc::new(Database::new_in_memory().unwrap())
    }

    fn insert_tag(conn: &rusqlite::Connection, id: &str, name: &str, color: &str) {
        conn.execute(
            "INSERT INTO tags (id, name, color, created_at) VALUES (?1, ?2, ?3, '2024-01-01T00:00:00Z')",
            rusqlite::params![id, name, color],
        ).unwrap();
    }

    fn insert_resource(conn: &rusqlite::Connection, id: &str, name: &str) {
        // Ensure environment exists (FK constraint)
        let _ = conn.execute(
            "INSERT OR IGNORE INTO environments (id, name, connection_mode, created_at, updated_at) VALUES ('env_1', 'test-env', 'direct', '2024-01-01', '2024-01-01')",
            [],
        );
        conn.execute(
            "INSERT INTO resources (id, environment_id, name, protocol, config_json, status, created_at, updated_at) VALUES (?1, 'env_1', ?2, 'ssh', '{}', 'active', '2024-01-01', '2024-01-01')",
            rusqlite::params![id, name],
        ).unwrap();
    }

    fn insert_resource_tag(conn: &rusqlite::Connection, resource_id: &str, tag_id: &str) {
        conn.execute(
            "INSERT INTO resource_tags (resource_id, tag_id) VALUES (?1, ?2)",
            rusqlite::params![resource_id, tag_id],
        )
        .unwrap();
    }

    #[test]
    fn tag_struct_serializes() {
        let tag = Tag {
            id: "tag_123".to_string(),
            name: "production".to_string(),
            color: "#3FB950".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&tag).unwrap();
        assert!(json.contains("tag_123"));
        assert!(json.contains("production"));
    }

    #[test]
    fn db_tables_created() {
        let db = test_db();
        let conn = db.pool.get().unwrap();

        let result = conn.execute("INSERT INTO tags (id, name, color, created_at) VALUES ('tag_1', 'test', '#58A6FF', '2024-01-01')", []);
        assert!(result.is_ok());

        // resource_tags requires valid FKs
        insert_resource(&conn, "res_1", "test-resource");
        let result = conn.execute(
            "INSERT INTO resource_tags (resource_id, tag_id) VALUES ('res_1', 'tag_1')",
            [],
        );
        assert!(result.is_ok());

        let _ = conn.execute("DELETE FROM resource_tags", []);
        let _ = conn.execute("DELETE FROM resources", []);
        let _ = conn.execute("DELETE FROM tags", []);
    }

    #[test]
    fn tag_name_uniqueness() {
        let db = test_db();
        let conn = db.pool.get().unwrap();

        insert_tag(&conn, "tag_1", "production", "#58A6FF");

        let result = conn.execute(
            "INSERT INTO tags (id, name, color, created_at) VALUES ('tag_2', 'production', '#3FB950', '2024-01-01')",
            [],
        );
        assert!(result.is_err());
        let _ = conn.execute("DELETE FROM tags", []);
    }

    #[test]
    fn resource_tag_cascade_on_tag_delete() {
        let db = test_db();
        let conn = db.pool.get().unwrap();

        insert_resource(&conn, "res_1", "web-server");
        insert_tag(&conn, "tag_1", "production", "#58A6FF");
        insert_resource_tag(&conn, "res_1", "tag_1");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM resource_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        conn.execute("DELETE FROM tags WHERE id = 'tag_1'", [])
            .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM resource_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);

        let _ = conn.execute("DELETE FROM resources", []);
    }

    #[test]
    fn resource_tag_cascade_on_resource_delete() {
        let db = test_db();
        let conn = db.pool.get().unwrap();

        insert_resource(&conn, "res_1", "web-server");
        insert_tag(&conn, "tag_1", "production", "#58A6FF");
        insert_resource_tag(&conn, "res_1", "tag_1");

        conn.execute("DELETE FROM resources WHERE id = 'res_1'", [])
            .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM resource_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);

        // Tag itself should still exist
        let exists: bool = conn
            .query_row("SELECT COUNT(*) FROM tags WHERE id = 'tag_1'", [], |r| {
                r.get::<_, i64>(0)
            })
            .map(|c| c > 0)
            .unwrap();
        assert!(exists);

        let _ = conn.execute("DELETE FROM tags", []);
    }

    #[test]
    fn resource_multiple_tags() {
        let db = test_db();
        let conn = db.pool.get().unwrap();

        insert_resource(&conn, "res_1", "web-server");
        insert_tag(&conn, "tag_1", "production", "#58A6FF");
        insert_tag(&conn, "tag_2", "critical", "#F85149");
        insert_resource_tag(&conn, "res_1", "tag_1");
        insert_resource_tag(&conn, "res_1", "tag_2");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM resource_tags WHERE resource_id = 'res_1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);

        let _ = conn.execute("DELETE FROM resource_tags", []);
        let _ = conn.execute("DELETE FROM tags", []);
        let _ = conn.execute("DELETE FROM resources", []);
    }

    #[test]
    fn tag_update_name_and_color() {
        let db = test_db();
        let conn = db.pool.get().unwrap();

        insert_tag(&conn, "tag_1", "prod", "#58A6FF");

        conn.execute(
            "UPDATE tags SET name = 'production', color = '#3FB950' WHERE id = 'tag_1'",
            [],
        )
        .unwrap();

        let (name, color): (String, String) = conn
            .query_row("SELECT name, color FROM tags WHERE id = 'tag_1'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(name, "production");
        assert_eq!(color, "#3FB950");

        let _ = conn.execute("DELETE FROM tags", []);
    }
}
