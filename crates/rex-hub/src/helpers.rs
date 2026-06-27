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
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_iso_returns_iso8601_string() {
        let ts = now_iso();
        assert!(!ts.is_empty());
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
        assert!(ts.len() >= 20);
    }

    #[test]
    fn gen_id_has_prefix() {
        let id = gen_id("res");
        assert!(id.starts_with("res_"));
        assert_eq!(id.len(), 12); // "res_" + 8 hex chars
    }

    #[test]
    fn gen_id_unique() {
        let id1 = gen_id("test");
        let id2 = gen_id("test");
        assert_ne!(id1, id2);
    }

    #[test]
    fn err_resp_returns_500() {
        let (status, _json) = err_resp("TEST_ERROR", "test message");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn not_found_returns_404() {
        let (status, _json) = not_found("NOT_FOUND", "not found message");
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn bad_request_returns_400() {
        let (status, json) = bad_request("bad request message");
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json.error.code, "VALIDATION_ERROR");
        assert_eq!(json.error.message, "bad request message");
    }

    #[test]
    fn conflict_returns_409() {
        let (status, _json) = conflict("CONFLICT", "conflict message");
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[test]
    fn error_response_serializes() {
        let err = ErrorResponse {
            error: ErrorBody {
                code: "TEST".to_string(),
                message: "test".to_string(),
            },
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("TEST"));
    }
}
