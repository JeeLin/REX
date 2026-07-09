use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::helpers::{
    bad_request, err_resp, gen_id, not_found, now_iso, ApiResponse, ErrorResponse,
};
use crate::routes::AppState;

// ── 数据模型 ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Notebook {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub tags: String, // JSON 数组
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotebookWithBlocks {
    #[serde(flatten)]
    pub notebook: Notebook,
    pub blocks: Vec<NotebookBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotebookBlock {
    pub id: String,
    pub notebook_id: String,
    pub block_type: String,
    pub content: String,
    pub resource_id: Option<String>,
    pub protocol: Option<String>,
    pub order_index: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotebookExecution {
    pub id: String,
    pub block_id: String,
    pub status: String,
    pub output: String,
    pub duration_ms: Option<i64>,
    pub executed_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotebook {
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotebook {
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBlocks {
    pub blocks: Vec<BlockInput>,
}

#[derive(Debug, Deserialize)]
pub struct BlockInput {
    pub id: Option<String>,
    pub block_type: String,
    pub content: Option<String>,
    pub resource_id: Option<String>,
    pub protocol: Option<String>,
    pub order_index: i32,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteCommand {
    pub block_id: String,
}

// ── Notebook CRUD ──────────────────────────────────────────

pub async fn list_notebooks(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<Notebook>>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let notebooks = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "数据库连接失败"))?;
        let mut stmt = conn
            .prepare("SELECT id, title, description, tags, created_at, updated_at FROM notebooks ORDER BY updated_at DESC")
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Notebook {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    tags: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let notebooks: Vec<Notebook> = rows.filter_map(|r| r.ok()).collect();
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(notebooks)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: notebooks }))
}

pub async fn get_notebook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<NotebookWithBlocks>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "数据库连接失败"))?;

        // 获取 notebook
        let notebook = conn
            .query_row(
                "SELECT id, title, description, tags, created_at, updated_at FROM notebooks WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(Notebook {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        description: row.get(2)?,
                        tags: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            )
            .map_err(|_| not_found("Notebook 不存在"))?;

        // 获取 blocks
        let mut stmt = conn
            .prepare("SELECT id, notebook_id, block_type, content, resource_id, protocol, order_index, created_at, updated_at FROM notebook_blocks WHERE notebook_id = ?1 ORDER BY order_index ASC")
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let blocks: Vec<NotebookBlock> = stmt
            .query_map(rusqlite::params![id], |row| {
                Ok(NotebookBlock {
                    id: row.get(0)?,
                    notebook_id: row.get(1)?,
                    block_type: row.get(2)?,
                    content: row.get(3)?,
                    resource_id: row.get(4)?,
                    protocol: row.get(5)?,
                    order_index: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?
            .filter_map(|r| r.ok())
            .collect();

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(NotebookWithBlocks { notebook, blocks })
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: result }))
}

pub async fn create_notebook(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateNotebook>,
) -> Result<(StatusCode, Json<ApiResponse<Notebook>>), (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let title = input.title.unwrap_or_else(|| "Untitled".to_string());
    let description = input.description;
    let notebook = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "数据库连接失败"))?;
        let id = gen_id("nb");
        let now = now_iso();
        conn.execute(
            "INSERT INTO notebooks (id, title, description, tags, created_at, updated_at) VALUES (?1, ?2, ?3, '[]', ?4, ?5)",
            rusqlite::params![id, title, description, now.clone(), now.clone()],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(Notebook {
            id,
            title,
            description,
            tags: "[]".to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok((StatusCode::CREATED, Json(ApiResponse { data: notebook })))
}

pub async fn update_notebook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<UpdateNotebook>,
) -> Result<Json<ApiResponse<Notebook>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let notebook = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "数据库连接失败"))?;
        let now = now_iso();

        // 检查存在
        let exists: bool = conn
            .query_row("SELECT COUNT(*) FROM notebooks WHERE id = ?1", rusqlite::params![id], |row| row.get::<_, i64>(0))
            .map_err(|_| not_found("Notebook 不存在"))?
            > 0;
        if !exists {
            return Err(not_found("Notebook 不存在"));
        }

        // 更新字段
        if let Some(title) = &input.title {
            conn.execute("UPDATE notebooks SET title = ?1, updated_at = ?2 WHERE id = ?3", rusqlite::params![title, now, id])
                .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        }
        if let Some(desc) = &input.description {
            conn.execute("UPDATE notebooks SET description = ?1, updated_at = ?2 WHERE id = ?3", rusqlite::params![desc, now, id])
                .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        }

        // 返回更新后的记录
        let notebook = conn
            .query_row(
                "SELECT id, title, description, tags, created_at, updated_at FROM notebooks WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(Notebook {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        description: row.get(2)?,
                        tags: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            )
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(notebook)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: notebook }))
}

pub async fn delete_notebook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "数据库连接失败"))?;
        let affected = conn
            .execute("DELETE FROM notebooks WHERE id = ?1", rusqlite::params![id])
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        if affected == 0 {
            return Err(not_found("Notebook 不存在"));
        }
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(StatusCode::NO_CONTENT)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?
}

// ── Blocks 批量更新 ────────────────────────────────────────

pub async fn update_blocks(
    State(state): State<Arc<AppState>>,
    Path(notebook_id): Path<String>,
    Json(input): Json<UpdateBlocks>,
) -> Result<Json<ApiResponse<Vec<NotebookBlock>>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let blocks = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "数据库连接失败"))?;
        let now = now_iso();

        // 检查 notebook 存在
        let exists: bool = conn
            .query_row("SELECT COUNT(*) FROM notebooks WHERE id = ?1", rusqlite::params![notebook_id], |row| row.get::<_, i64>(0))
            .map_err(|_| not_found("Notebook 不存在"))?
            > 0;
        if !exists {
            return Err(not_found("Notebook 不存在"));
        }

        // 删除旧 blocks
        conn.execute("DELETE FROM notebook_blocks WHERE notebook_id = ?1", rusqlite::params![notebook_id])
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        // 插入新 blocks
        let mut result_blocks = Vec::new();
        for block_input in &input.blocks {
            let id = block_input.id.clone().unwrap_or_else(|| gen_id("nbb"));
            conn.execute(
                "INSERT INTO notebook_blocks (id, notebook_id, block_type, content, resource_id, protocol, order_index, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    id,
                    notebook_id,
                    block_input.block_type,
                    block_input.content.as_deref().unwrap_or(""),
                    block_input.resource_id,
                    block_input.protocol,
                    block_input.order_index,
                    now,
                    now,
                ],
            )
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

            result_blocks.push(NotebookBlock {
                id,
                notebook_id: notebook_id.clone(),
                block_type: block_input.block_type.clone(),
                content: block_input.content.clone().unwrap_or_default(),
                resource_id: block_input.resource_id.clone(),
                protocol: block_input.protocol.clone(),
                order_index: block_input.order_index,
                created_at: now.clone(),
                updated_at: now.clone(),
            });
        }

        // 更新 notebook 的 updated_at
        conn.execute("UPDATE notebooks SET updated_at = ?1 WHERE id = ?2", rusqlite::params![now, notebook_id])
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(result_blocks)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: blocks }))
}

// ── 执行命令块 ────────────────────────────────────────────

pub async fn execute_command(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ExecuteCommand>,
) -> Result<Json<ApiResponse<NotebookExecution>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let execution = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "数据库连接失败"))?;

        // 获取 block 信息
        let block = conn
            .query_row(
                "SELECT id, notebook_id, block_type, content, resource_id, protocol FROM notebook_blocks WHERE id = ?1",
                rusqlite::params![input.block_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .map_err(|_| not_found("命令块不存在"))?;

        let (block_id, _notebook_id, block_type, content, resource_id, protocol) = block;

        if block_type != "command" {
            return Err(bad_request("只有命令块可以执行"));
        }

        let resource_id = resource_id.ok_or_else(|| bad_request("未绑定资源"))?;
        let protocol = protocol.ok_or_else(|| bad_request("未指定协议"))?;

        // 执行命令（简化版：记录执行状态，实际执行需要调用对应协议 crate）
        let id = gen_id("nbe");
        let now = now_iso();
        let output = format!("[模拟执行] {} 命令: {} (资源: {})", protocol, content, resource_id);

        conn.execute(
            "INSERT INTO notebook_executions (id, block_id, status, output, duration_ms, executed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, block_id, "success", output, 0i64, now.clone()],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(NotebookExecution {
            id,
            block_id,
            status: "success".to_string(),
            output,
            duration_ms: Some(0),
            executed_at: now,
        })
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: execution }))
}

// ── 获取执行历史 ──────────────────────────────────────────

pub async fn list_executions(
    State(state): State<Arc<AppState>>,
    Path(block_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<NotebookExecution>>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let executions = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "数据库连接失败"))?;
        let mut stmt = conn
            .prepare("SELECT id, block_id, status, output, duration_ms, executed_at FROM notebook_executions WHERE block_id = ?1 ORDER BY executed_at DESC")
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let rows = stmt
            .query_map(rusqlite::params![block_id], |row| {
                Ok(NotebookExecution {
                    id: row.get(0)?,
                    block_id: row.get(1)?,
                    status: row.get(2)?,
                    output: row.get(3)?,
                    duration_ms: row.get(4)?,
                    executed_at: row.get(5)?,
                })
            })
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let executions: Vec<NotebookExecution> = rows.filter_map(|r| r.ok()).collect();
        Ok::<_, (StatusCode, Json<ErrorResponse>)>(executions)
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: executions }))
}

// ── 导入/导出 ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct NotebookExport {
    #[serde(rename = "rex-notebook")]
    pub version: String,
    pub title: String,
    pub description: Option<String>,
    pub blocks: Vec<BlockExport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockExport {
    #[serde(rename = "type")]
    pub block_type: String,
    pub content: Option<String>,
    pub protocol: Option<String>,
    pub resource_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NotebookImport {
    #[serde(rename = "rex-notebook")]
    pub version: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub blocks: Option<Vec<BlockExport>>,
}

pub async fn export_notebook(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<NotebookExport>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let export = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "数据库连接失败"))?;

        let notebook = conn
            .query_row(
                "SELECT id, title, description FROM notebooks WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .map_err(|_| not_found("Notebook 不存在"))?;

        let mut stmt = conn
            .prepare("SELECT block_type, content, protocol FROM notebook_blocks WHERE notebook_id = ?1 ORDER BY order_index ASC")
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
        let blocks: Vec<BlockExport> = stmt
            .query_map(rusqlite::params![id], |row| {
                Ok(BlockExport {
                    block_type: row.get(0)?,
                    content: row.get(1)?,
                    protocol: row.get(2)?,
                    resource_name: None,
                })
            })
            .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?
            .filter_map(|r| r.ok())
            .collect();

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(NotebookExport {
            version: "1.0".to_string(),
            title: notebook.1,
            description: notebook.2,
            blocks,
        })
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok(Json(ApiResponse { data: export }))
}

pub async fn import_notebook(
    State(state): State<Arc<AppState>>,
    Json(input): Json<NotebookImport>,
) -> Result<(StatusCode, Json<ApiResponse<Notebook>>), (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.clone();
    let notebook = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "数据库连接失败"))?;
        let id = gen_id("nb");
        let now = now_iso();
        let title = input.title.unwrap_or_else(|| "Imported Notebook".to_string());
        let description = input.description;

        conn.execute(
            "INSERT INTO notebooks (id, title, description, tags, created_at, updated_at) VALUES (?1, ?2, ?3, '[]', ?4, ?5)",
            rusqlite::params![id, title, description, now.clone(), now.clone()],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

        // 插入 blocks
        if let Some(blocks) = &input.blocks {
            for (i, block) in blocks.iter().enumerate() {
                let block_id = gen_id("nbb");
                conn.execute(
                    "INSERT INTO notebook_blocks (id, notebook_id, block_type, content, resource_id, protocol, order_index, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        block_id,
                        id,
                        block.block_type,
                        block.content.as_deref().unwrap_or(""),
                        block.protocol,
                        i as i32,
                        now,
                        now,
                    ],
                )
                .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;
            }
        }

        Ok::<_, (StatusCode, Json<ErrorResponse>)>(Notebook {
            id,
            title,
            description,
            tags: "[]".to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))??;
    Ok((StatusCode::CREATED, Json(ApiResponse { data: notebook })))
}
