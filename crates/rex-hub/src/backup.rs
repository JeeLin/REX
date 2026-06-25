use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::{Multipart, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::Json;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::Database;
use crate::helpers::{ErrorBody, ErrorResponse};
use crate::routes::AppState;

/// 备份文件格式
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupFile {
    pub version: String,
    pub created_at: String,
    pub hub_version: String,
    pub encrypted: bool,
    pub salt: Option<String>,
    pub nonce: Option<String>,
    pub data: Option<String>,
}

/// 备份数据（明文）
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupData {
    pub environments: Vec<BackupEnvironment>,
    pub resources: Vec<BackupResource>,
    pub settings: Vec<BackupSetting>,
}

/// 环境信息
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupEnvironment {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub connection_mode: String,
    pub agent_token_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 资源信息
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupResource {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub protocol: String,
    pub agent_id: Option<String>,
    pub config_json: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 设置项
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupSetting {
    pub key: String,
    pub value: String,
}

/// 导入统计
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub environments: ImportCounts,
    pub resources: ImportCounts,
    pub settings: ImportCounts,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportCounts {
    pub created: usize,
    pub skipped: usize,
    pub updated: usize,
}

/// 预览结果
#[derive(Debug, Serialize, Deserialize)]
pub struct PreviewResult {
    pub hub_version: String,
    pub created_at: String,
    pub encrypted: bool,
    pub environments: Vec<PreviewItem>,
    pub resources: Vec<PreviewItem>,
    pub settings_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreviewItem {
    pub id: String,
    pub name: String,
    pub exists: bool,
    pub extra: Option<String>,
}

/// 从数据库导出配置
pub fn export_backup(
    db: &Database,
    env_ids: Option<&str>,
    password: Option<&str>,
) -> Result<BackupFile> {
    let conn = db.pool.get()?;

    // 查询环境
    let environments: Vec<BackupEnvironment> = if let Some(ids) = env_ids {
        let id_list: Vec<&str> = ids.split(',').map(|s| s.trim()).collect();
        let mut envs = Vec::new();
        for id in &id_list {
            if let Ok(env) = conn.query_row(
                "SELECT id, name, description, connection_mode, agent_token_hash, created_at, updated_at FROM environments WHERE id = ?1",
                [id],
                |row| Ok(BackupEnvironment {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    connection_mode: row.get(3)?,
                    agent_token_hash: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                }),
            ) {
                envs.push(env);
            }
        }
        envs
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, connection_mode, agent_token_hash, created_at, updated_at FROM environments",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupEnvironment {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                connection_mode: row.get(3)?,
                agent_token_hash: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    // 查询资源
    let mut resources = Vec::new();
    for env in &environments {
        let mut stmt = conn.prepare(
            "SELECT id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at FROM resources WHERE environment_id = ?1",
        )?;
        let env_resources: Vec<BackupResource> = stmt
            .query_map([&env.id], |row| {
                Ok(BackupResource {
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
            })?
            .filter_map(|r| r.ok())
            .collect();
        resources.extend(env_resources);
    }

    // 查询设置
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let settings: Vec<BackupSetting> = stmt
        .query_map([], |row| {
            Ok(BackupSetting {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let data = BackupData {
        environments,
        resources,
        settings,
    };

    let data_json = serde_json::to_string(&data)?;

    if let Some(pwd) = password {
        let salt = generate_salt()?;
        let key = derive_key(pwd, &salt)?;
        let (ciphertext, nonce) = encrypt_data(&key, &data_json)?;

        Ok(BackupFile {
            version: "1.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            hub_version: env!("CARGO_PKG_VERSION").to_string(),
            encrypted: true,
            salt: Some(BASE64.encode(&salt)),
            nonce: Some(BASE64.encode(&nonce)),
            data: Some(BASE64.encode(&ciphertext)),
        })
    } else {
        Ok(BackupFile {
            version: "1.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            hub_version: env!("CARGO_PKG_VERSION").to_string(),
            encrypted: false,
            salt: None,
            nonce: None,
            data: Some(data_json),
        })
    }
}

/// 解密备份文件
pub fn decrypt_backup(file: &BackupFile, password: &str) -> Result<BackupData> {
    if !file.encrypted {
        let data = file.data.as_ref().context("backup file missing data")?;
        return Ok(serde_json::from_str(data)?);
    }

    let salt_b64 = file.salt.as_ref().context("backup file missing salt")?;
    let nonce_b64 = file.nonce.as_ref().context("backup file missing nonce")?;
    let ciphertext_b64 = file.data.as_ref().context("backup file missing data")?;

    let salt = BASE64.decode(salt_b64)?;
    let nonce = BASE64.decode(nonce_b64)?;
    let ciphertext = BASE64.decode(ciphertext_b64)?;

    let key = derive_key(password, &salt)?;
    let plaintext = decrypt_data(&key, &nonce, &ciphertext)?;

    Ok(serde_json::from_str(&plaintext)?)
}

/// 预览备份文件（检查冲突）
pub fn preview_backup(
    db: &Database,
    file: &BackupFile,
    password: Option<&str>,
) -> Result<PreviewResult> {
    let data = if file.encrypted {
        let pwd = password.context("password required for encrypted backup")?;
        decrypt_backup(file, pwd)?
    } else {
        let data_str = file.data.as_ref().context("backup file missing data")?;
        serde_json::from_str(data_str)?
    };

    let conn = db.pool.get()?;

    // 检查环境是否已存在
    let mut environments = Vec::new();
    for env in &data.environments {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM environments WHERE id = ?1",
            [&env.id],
            |row| row.get(0),
        )?;
        environments.push(PreviewItem {
            id: env.id.clone(),
            name: env.name.clone(),
            exists,
            extra: None,
        });
    }

    // 检查资源是否已存在
    let mut resources = Vec::new();
    for resource in &data.resources {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM resources WHERE id = ?1",
            [&resource.id],
            |row| row.get(0),
        )?;
        resources.push(PreviewItem {
            id: resource.id.clone(),
            name: resource.name.clone(),
            exists,
            extra: Some(resource.protocol.clone()),
        });
    }

    Ok(PreviewResult {
        hub_version: file.hub_version.clone(),
        created_at: file.created_at.clone(),
        encrypted: file.encrypted,
        environments,
        resources,
        settings_count: data.settings.len(),
    })
}

/// 导入备份数据
pub fn import_backup(
    db: &Database,
    file: &BackupFile,
    password: Option<&str>,
    strategy: &str,
) -> Result<ImportResult> {
    let data = if file.encrypted {
        let pwd = password.context("password required for encrypted backup")?;
        decrypt_backup(file, pwd)?
    } else {
        let data_str = file.data.as_ref().context("backup file missing data")?;
        serde_json::from_str(data_str)?
    };

    let conn = db.pool.get()?;
    let mut result = ImportResult {
        environments: ImportCounts {
            created: 0,
            skipped: 0,
            updated: 0,
        },
        resources: ImportCounts {
            created: 0,
            skipped: 0,
            updated: 0,
        },
        settings: ImportCounts {
            created: 0,
            skipped: 0,
            updated: 0,
        },
        warnings: Vec::new(),
    };

    // 先导入环境
    for env in &data.environments {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM environments WHERE id = ?1",
            [&env.id],
            |row| row.get(0),
        )?;

        match (exists, strategy) {
            (true, "skip_existing") => {
                result.environments.skipped += 1;
                result.warnings.push(format!(
                    "Environment '{}' already exists, skipped",
                    env.name
                ));
            }
            (true, "overwrite") => {
                conn.execute(
                    "UPDATE environments SET name = ?1, description = ?2, connection_mode = ?3, agent_token_hash = ?4, updated_at = ?5 WHERE id = ?6",
                    rusqlite::params![
                        env.name,
                        env.description,
                        env.connection_mode,
                        env.agent_token_hash,
                        env.updated_at,
                        env.id,
                    ],
                )?;
                result.environments.updated += 1;
            }
            (true, _) => {
                result.environments.skipped += 1;
            }
            (false, _) => {
                conn.execute(
                    "INSERT INTO environments (id, name, description, connection_mode, agent_token_hash, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        env.id,
                        env.name,
                        env.description,
                        env.connection_mode,
                        env.agent_token_hash,
                        env.created_at,
                        env.updated_at,
                    ],
                )?;
                result.environments.created += 1;
            }
        }
    }

    // 再导入资源
    for resource in &data.resources {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM resources WHERE id = ?1",
            [&resource.id],
            |row| row.get(0),
        )?;

        match (exists, strategy) {
            (true, "skip_existing") => {
                result.resources.skipped += 1;
                result.warnings.push(format!(
                    "Resource '{}' already exists, skipped",
                    resource.name
                ));
            }
            (true, "overwrite") => {
                conn.execute(
                    "UPDATE resources SET environment_id = ?1, name = ?2, protocol = ?3, agent_id = ?4, config_json = ?5, status = ?6, updated_at = ?7 WHERE id = ?8",
                    rusqlite::params![
                        resource.environment_id,
                        resource.name,
                        resource.protocol,
                        resource.agent_id,
                        resource.config_json,
                        resource.status,
                        resource.updated_at,
                        resource.id,
                    ],
                )?;
                result.resources.updated += 1;
            }
            (true, _) => {
                result.resources.skipped += 1;
            }
            (false, _) => {
                conn.execute(
                    "INSERT INTO resources (id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        resource.id,
                        resource.environment_id,
                        resource.name,
                        resource.protocol,
                        resource.agent_id,
                        resource.config_json,
                        resource.status,
                        resource.created_at,
                        resource.updated_at,
                    ],
                )?;
                result.resources.created += 1;
            }
        }
    }

    // 导入设置
    for setting in &data.settings {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM settings WHERE key = ?1",
            [&setting.key],
            |row| row.get(0),
        )?;

        match (exists, strategy) {
            (true, "skip_existing") => {
                result.settings.skipped += 1;
            }
            (true, "overwrite") => {
                conn.execute(
                    "UPDATE settings SET value = ?1 WHERE key = ?2",
                    rusqlite::params![setting.value, setting.key],
                )?;
                result.settings.updated += 1;
            }
            (true, _) => {
                result.settings.skipped += 1;
            }
            (false, _) => {
                conn.execute(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                    rusqlite::params![setting.key, setting.value],
                )?;
                result.settings.created += 1;
            }
        }
    }

    Ok(result)
}

/// 生成随机盐值（16 字节）
fn generate_salt() -> Result<[u8; 16]> {
    let mut salt = [0u8; 16];
    getrandom::getrandom(&mut salt)?;
    Ok(salt)
}

/// 从密码派生加密密钥（PBKDF2-SHA256 简化实现）
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    // PBKDF2-like: 100000 轮迭代
    let mut key = [0u8; 32];
    let mut prev = [0u8; 32];

    // 第一轮：HMAC-SHA256(password, salt || 0x00000001)
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt);
    hasher.update([0u8, 0, 0, 1]);
    let first = hasher.finalize();
    prev.copy_from_slice(&first);
    key.copy_from_slice(&first);

    // 后续轮次
    for _ in 1..100_000 {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(&prev);
        let result = hasher.finalize();
        for j in 0..32 {
            key[j] ^= result[j];
        }
        prev.copy_from_slice(&result);
    }

    Ok(key)
}

/// AES-256-GCM 加密
fn encrypt_data(key: &[u8; 32], plaintext: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| anyhow::anyhow!("aes-gcm init: {}", e))?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("aes-gcm encrypt: {}", e))?;
    Ok((ciphertext, nonce_bytes.to_vec()))
}

/// AES-256-GCM 解密
fn decrypt_data(key: &[u8; 32], nonce: &[u8], ciphertext: &[u8]) -> Result<String> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| anyhow::anyhow!("aes-gcm init: {}", e))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|e| anyhow::anyhow!("aes-gcm decrypt: {}", e))?;
    Ok(String::from_utf8(plaintext)?)
}

// ---- HTTP Handlers ----

/// 从 Multipart 中依次提取 field，返回 (field_name, value) 列表
async fn extract_multipart(
    mut multipart: Multipart,
) -> std::collections::HashMap<String, String> {
    let mut fields = std::collections::HashMap::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if let Ok(text) = field.text().await {
            fields.insert(name, text);
        }
    }
    fields
}

/// POST /api/backup/export
pub async fn export_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    Json(body): Json<ExportRequest>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let env_ids = params.get("env_ids").map(|s| s.as_str());
    let backup = export_backup(&state.db, env_ids, body.password.as_deref())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: ErrorBody {
                        code: "BACKUP_EXPORT_FAILED".to_string(),
                        message: e.to_string(),
                    },
                }),
            )
        })?;

    let filename = format!(
        "rex-backup-{}.json",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let body_str = serde_json::to_string(&backup).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "BACKUP_SERIALIZE_FAILED".to_string(),
                    message: e.to_string(),
                },
            }),
        )
    })?;

    let mut resp = Response::new(Body::from(body_str));
    let headers = resp.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::try_from(format!(
            "attachment; filename=\"{}\"",
            filename
        ))
        .unwrap(),
    );
    Ok(resp)
}

#[derive(Deserialize)]
pub struct ExportRequest {
    pub password: Option<String>,
}

/// POST /api/backup/preview
pub async fn preview_handler(
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Result<Json<PreviewResult>, (StatusCode, Json<ErrorResponse>)> {
    let fields = extract_multipart(multipart).await;
    let file_content = fields.get("file").ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "MISSING_FILE".to_string(),
                    message: "backup file is required".to_string(),
                },
            }),
        )
    })?;

    let backup_file: BackupFile = serde_json::from_str(file_content).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "INVALID_BACKUP_FILE".to_string(),
                    message: e.to_string(),
                },
            }),
        )
    })?;

    let password = fields.get("password").map(|s| s.as_str());
    let result = preview_backup(&state.db, &backup_file, password).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "BACKUP_PREVIEW_FAILED".to_string(),
                    message: e.to_string(),
                },
            }),
        )
    })?;

    Ok(Json(result))
}

/// POST /api/backup/import
pub async fn import_handler(
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Result<Json<ImportResult>, (StatusCode, Json<ErrorResponse>)> {
    let fields = extract_multipart(multipart).await;
    let file_content = fields.get("file").ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "MISSING_FILE".to_string(),
                    message: "backup file is required".to_string(),
                },
            }),
        )
    })?;

    let backup_file: BackupFile = serde_json::from_str(file_content).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "INVALID_BACKUP_FILE".to_string(),
                    message: e.to_string(),
                },
            }),
        )
    })?;

    let password = fields.get("password").map(|s| s.as_str());
    let strategy = fields
        .get("strategy")
        .map(|s| s.as_str())
        .unwrap_or("skip_existing");

    let result = import_backup(&state.db, &backup_file, password, strategy).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "BACKUP_IMPORT_FAILED".to_string(),
                    message: e.to_string(),
                },
            }),
        )
    })?;

    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let plaintext = "hello world";
        let (ciphertext, nonce) = encrypt_data(&key, plaintext).unwrap();
        let decrypted = decrypt_data(&key, &nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_derive_key_deterministic() {
        let salt = [1u8; 16];
        let key1 = derive_key("password", &salt).unwrap();
        let key2 = derive_key("password", &salt).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_derive_key_different_passwords() {
        let salt = [1u8; 16];
        let key1 = derive_key("password1", &salt).unwrap();
        let key2 = derive_key("password2", &salt).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_backup_file_serialization() {
        let file = BackupFile {
            version: "1.0".to_string(),
            created_at: "2026-06-25T00:00:00Z".to_string(),
            hub_version: "0.18.0".to_string(),
            encrypted: false,
            salt: None,
            nonce: None,
            data: Some("{}".to_string()),
        };
        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("\"version\":\"1.0\""));
    }

    #[test]
    fn test_export_import_roundtrip() {
        let db = Database::new_in_memory().unwrap();
        {
            let conn = db.pool.get().unwrap();
            conn.execute(
                "INSERT INTO environments (id, name, description, connection_mode, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params!["env_1", "test-env", "desc", "direct", "2026-01-01", "2026-01-01"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                rusqlite::params!["theme", "dark"],
            )
            .unwrap();
        }

        // 导出
        let backup = export_backup(&db, None, None).unwrap();
        assert!(!backup.encrypted);
        assert_eq!(backup.version, "1.0");
        let data: BackupData = serde_json::from_str(backup.data.as_ref().unwrap()).unwrap();
        assert_eq!(data.environments.len(), 1);
        assert_eq!(data.settings.len(), 1);

        // 导入到新数据库
        let db2 = Database::new_in_memory().unwrap();
        let result = import_backup(&db2, &backup, None, "skip_existing").unwrap();
        assert_eq!(result.environments.created, 1);
        assert_eq!(result.settings.created, 1);
    }

    #[test]
    fn test_encrypted_backup_roundtrip() {
        let db = Database::new_in_memory().unwrap();
        {
            let conn = db.pool.get().unwrap();
            conn.execute(
                "INSERT INTO environments (id, name, description, connection_mode, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params!["env_1", "test-env", None::<&str>, "direct", "2026-01-01", "2026-01-01"],
            )
            .unwrap();
        }

        // 加密导出
        let backup = export_backup(&db, None, Some("mypassword")).unwrap();
        assert!(backup.encrypted);
        assert!(backup.salt.is_some());
        assert!(backup.nonce.is_some());

        // 解密导入
        let db2 = Database::new_in_memory().unwrap();
        let result = import_backup(&db2, &backup, Some("mypassword"), "skip_existing").unwrap();
        assert_eq!(result.environments.created, 1);

        // 错误密码
        let result = import_backup(&db2, &backup, Some("wrongpassword"), "skip_existing");
        assert!(result.is_err());
    }

    #[test]
    fn test_import_skip_existing() {
        let db = Database::new_in_memory().unwrap();
        {
            let conn = db.pool.get().unwrap();
            conn.execute(
                "INSERT INTO environments (id, name, description, connection_mode, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params!["env_1", "existing-env", None::<&str>, "direct", "2026-01-01", "2026-01-01"],
            )
            .unwrap();
        }

        let backup = BackupFile {
            version: "1.0".to_string(),
            created_at: "2026-01-01".to_string(),
            hub_version: "0.18.0".to_string(),
            encrypted: false,
            salt: None,
            nonce: None,
            data: Some(
                serde_json::to_string(&BackupData {
                    environments: vec![BackupEnvironment {
                        id: "env_1".to_string(),
                        name: "existing-env".to_string(),
                        description: None,
                        connection_mode: "direct".to_string(),
                        agent_token_hash: None,
                        created_at: "2026-01-01".to_string(),
                        updated_at: "2026-01-01".to_string(),
                    }],
                    resources: vec![],
                    settings: vec![],
                })
                .unwrap(),
            ),
        };

        let result = import_backup(&db, &backup, None, "skip_existing").unwrap();
        assert_eq!(result.environments.skipped, 1);
        assert_eq!(result.environments.created, 0);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_preview_backup() {
        let db = Database::new_in_memory().unwrap();
        {
            let conn = db.pool.get().unwrap();
            conn.execute(
                "INSERT INTO environments (id, name, description, connection_mode, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params!["env_1", "existing", None::<&str>, "direct", "2026-01-01", "2026-01-01"],
            )
            .unwrap();
        }

        let backup = BackupFile {
            version: "1.0".to_string(),
            created_at: "2026-01-01".to_string(),
            hub_version: "0.18.0".to_string(),
            encrypted: false,
            salt: None,
            nonce: None,
            data: Some(
                serde_json::to_string(&BackupData {
                    environments: vec![
                        BackupEnvironment {
                            id: "env_1".to_string(),
                            name: "existing".to_string(),
                            description: None,
                            connection_mode: "direct".to_string(),
                            agent_token_hash: None,
                            created_at: "2026-01-01".to_string(),
                            updated_at: "2026-01-01".to_string(),
                        },
                        BackupEnvironment {
                            id: "env_2".to_string(),
                            name: "new".to_string(),
                            description: None,
                            connection_mode: "agent".to_string(),
                            agent_token_hash: None,
                            created_at: "2026-01-01".to_string(),
                            updated_at: "2026-01-01".to_string(),
                        },
                    ],
                    resources: vec![],
                    settings: vec![BackupSetting {
                        key: "k".to_string(),
                        value: "v".to_string(),
                    }],
                })
                .unwrap(),
            ),
        };

        let preview = preview_backup(&db, &backup, None).unwrap();
        assert_eq!(preview.environments.len(), 2);
        assert!(preview.environments[0].exists);
        assert!(!preview.environments[1].exists);
        assert_eq!(preview.settings_count, 1);
    }
}
