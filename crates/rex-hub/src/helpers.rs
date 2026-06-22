use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use uuid::Uuid;

// ── 共享类型 ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

// ── 工具函数 ──────────────────────────────────────────────

pub fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{secs:010}")
}

pub fn gen_id(prefix: &str) -> String {
    let uuid = Uuid::new_v4();
    let bytes = uuid.as_bytes();
    let id_num = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    format!("{prefix}_{id_num:08x}")
}

// ── 错误响应构造 ──────────────────────────────────────────

fn make_error(status: StatusCode, code: &str, msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: ErrorBody {
                code: code.to_string(),
                message: msg.to_string(),
            },
        }),
    )
}

pub fn err_resp(code: &str, msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    make_error(StatusCode::INTERNAL_SERVER_ERROR, code, msg)
}

pub fn not_found(code: &str, msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    make_error(StatusCode::NOT_FOUND, code, msg)
}

pub fn bad_request(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    make_error(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg)
}

pub fn conflict(code: &str, msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    make_error(StatusCode::CONFLICT, code, msg)
}

/// 从数据库获取密码哈希，不存在时用默认密码生成
pub fn get_or_create_password_hash(db: &crate::db::Database) -> String {
    let stored: Option<String> = db
        .pool
        .get()
        .unwrap()
        .query_row(
            "SELECT value FROM settings WHERE key = 'password_hash'",
            [],
            |row| row.get(0),
        )
        .ok();

    stored.unwrap_or_else(|| {
        use argon2::password_hash::SaltString;
        use argon2::PasswordHasher;
        use rand_core::OsRng;
        let default_password =
            std::env::var("REX_DEFAULT_PASSWORD").unwrap_or_else(|_| "admin".to_string());
        let salt = SaltString::generate(&mut OsRng);
        let hash = argon2::password_hash::PasswordHasher::hash_password(
            &argon2::Argon2::default(),
            default_password.as_bytes(),
            &salt,
        )
        .unwrap()
        .to_string();
        let _ = db.pool.get().unwrap().execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('password_hash', ?1)",
            rusqlite::params![hash],
        );
        hash
    })
}
