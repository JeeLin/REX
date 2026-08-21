//! 单用户密码认证：argon2 哈希 + JWT token。

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use rex_common::RExError;
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::error::error_with_status;

type AuthResult<T> = std::result::Result<T, RExError>;

// --- JWT Claims ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

// --- AuthConfig ---

pub struct AuthConfig {
    db: Arc<Database>,
    jwt_secret: Vec<u8>,
}

impl AuthConfig {
    pub fn new(db: Arc<Database>) -> AuthResult<Self> {
        // JWT secret: 从 settings 读取，首次自动生成并存储
        let jwt_secret = match db.get_setting("jwt_secret")? {
            Some(secret) => {
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &secret)
                    .map_err(|e| RExError::Message(format!("invalid jwt_secret: {e}")))?
            }
            None => {
                use rand_core::RngCore;
                let mut secret = vec![0u8; 64];
                rand_core::OsRng.fill_bytes(&mut secret);
                let encoded =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &secret);
                db.set_setting("jwt_secret", &encoded)?;
                secret
            }
        };
        Ok(Self { db, jwt_secret })
    }

    pub fn generate_token(&self) -> AuthResult<String> {
        let now = chrono::Utc::now();
        let exp = now + chrono::Duration::days(30);
        let claims = Claims {
            sub: "admin".into(),
            iat: now.timestamp() as usize,
            exp: exp.timestamp() as usize,
        };
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(&self.jwt_secret),
        )
        .map_err(|e| RExError::Message(format!("token generation failed: {e}")))
    }

    pub fn verify_token(&self, token: &str) -> AuthResult<Claims> {
        let data = jsonwebtoken::decode::<Claims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(&self.jwt_secret),
            &jsonwebtoken::Validation::default(),
        )
        .map_err(|e| RExError::Message(format!("invalid token: {e}")))?;
        Ok(data.claims)
    }

    pub fn requires_setup(&self) -> AuthResult<bool> {
        let hash = self.db.get_setting("password_hash")?;
        Ok(hash.is_none())
    }

    pub fn set_password(&self, password: &str) -> AuthResult<()> {
        use argon2::password_hash::SaltString;
        use argon2::PasswordHasher;
        let salt = SaltString::generate(&mut rand_core::OsRng);
        let hash = argon2::Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| RExError::Message(format!("hash failed: {e}")))?;
        self.db
            .set_setting("password_hash", hash.to_string().as_str())?;
        Ok(())
    }

    /// 验证密码是否正确（内部复用）
    fn verify_password(&self, password: &str) -> AuthResult<()> {
        let hash_str = self
            .db
            .get_setting("password_hash")?
            .ok_or_else(|| RExError::Message("no password set".into()))?;
        let hash =
            argon2::PasswordHash::new(&hash_str).map_err(|e| RExError::Message(e.to_string()))?;
        use argon2::password_hash::PasswordVerifier;
        argon2::Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .map_err(|_| RExError::Message("invalid password".into()))
    }

    pub fn login(&self, password: &str) -> AuthResult<String> {
        self.verify_password(password)?;
        self.generate_token()
    }

    pub fn change_password(&self, current: &str, new_password: &str) -> AuthResult<()> {
        self.verify_password(current)
            .map_err(|_| RExError::Message("current password is incorrect".into()))?;
        self.set_password(new_password)
    }
}

// --- Request / Response types ---

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Deserialize)]
pub struct PasswordRequest {
    pub password: String,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

// --- Handlers ---

/// GET /api/auth/check
pub async fn check_auth(State(state): State<crate::AppState>) -> Json<serde_json::Value> {
    let requires_setup = state.auth.requires_setup().unwrap_or(true);
    Json(serde_json::json!({ "requires_setup": requires_setup }))
}

/// POST /api/auth/login
pub async fn login(
    State(state): State<crate::AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<crate::error::ErrorBody>)> {
    match state.auth.login(&body.password) {
        Ok(token) => {
            tracing::info!(action = "AUTH_LOGIN", result = "success", "user logged in");
            state
                .db
                .write_audit_log(&crate::models::NewAuditEntry {
                    action: "AUTH_LOGIN".into(),
                    result: "success".into(),
                    ..Default::default()
                })
                .ok();
            let expires = chrono::Utc::now() + chrono::Duration::days(30);
            Ok((
                StatusCode::OK,
                Json(serde_json::json!({
                    "token": token,
                    "expiresAt": expires.to_rfc3339()
                })),
            ))
        }
        Err(e) => {
            tracing::warn!(action = "AUTH_LOGIN", result = "failure", error = %e, "login failed");
            state
                .db
                .write_audit_log(&crate::models::NewAuditEntry {
                    action: "AUTH_LOGIN".into(),
                    result: "failure".into(),
                    detail: Some(e.to_string()),
                    ..Default::default()
                })
                .ok();
            Err(error_with_status(
                StatusCode::UNAUTHORIZED,
                "AUTH_INVALID",
                "invalid password",
            ))
        }
    }
}

/// POST /api/auth/password（首次设置密码）
pub async fn set_password(
    State(state): State<crate::AppState>,
    Json(body): Json<PasswordRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<crate::error::ErrorBody>)> {
    if !state.auth.requires_setup().map_err(|e| {
        error_with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            &e.to_string(),
        )
    })? {
        return Err(error_with_status(
            StatusCode::CONFLICT,
            "PASSWORD_ALREADY_SET",
            "password already set",
        ));
    }
    state.auth.set_password(&body.password).map_err(|e| {
        error_with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            &e.to_string(),
        )
    })?;
    tracing::info!(action = "AUTH_PASSWORD_SET", "password set successfully");
    let token = state.auth.generate_token().map_err(|e| {
        error_with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            &e.to_string(),
        )
    })?;
    let expires = chrono::Utc::now() + chrono::Duration::days(30);
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "token": token,
            "expiresAt": expires.to_rfc3339()
        })),
    ))
}

/// POST /api/auth/change-password
pub async fn change_password(
    State(state): State<crate::AppState>,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<crate::error::ErrorBody>)> {
    state
        .auth
        .change_password(&body.current_password, &body.new_password)
        .map_err(|e| {
            let msg = e.to_string();
            let code = if msg.contains("incorrect") {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            error_with_status(code, "PASSWORD_CHANGE_FAILED", &msg)
        })?;
    tracing::info!(
        action = "AUTH_PASSWORD_CHANGED",
        "password changed successfully"
    );
    state
        .db
        .write_audit_log(&crate::models::NewAuditEntry {
            action: "AUTH_PASSWORD_CHANGED".into(),
            result: "success".into(),
            ..Default::default()
        })
        .ok();
    Ok((StatusCode::OK, Json(serde_json::json!({ "ok": true }))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use tempfile::tempdir;

    fn test_auth() -> (tempfile::TempDir, AuthConfig) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let auth = AuthConfig::new(db).unwrap();
        (dir, auth)
    }

    #[test]
    fn test_jwt_roundtrip() {
        let (_dir, auth) = test_auth();
        let token = auth.generate_token().unwrap();
        let claims = auth.verify_token(&token).unwrap();
        assert_eq!(claims.sub, "admin");
    }

    #[test]
    fn test_password_set_and_login() {
        let (_dir, auth) = test_auth();
        assert!(auth.requires_setup().unwrap());
        auth.set_password("test123").unwrap();
        assert!(!auth.requires_setup().unwrap());
        let token = auth.login("test123").unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_login_wrong_password() {
        let (_dir, auth) = test_auth();
        auth.set_password("correct").unwrap();
        assert!(auth.login("wrong").is_err());
    }

    #[test]
    fn test_token_expiry() {
        let (_dir, auth) = test_auth();
        let token = auth.generate_token().unwrap();
        let claims = auth.verify_token(&token).unwrap();
        // Token should expire ~30 days from now
        let now = chrono::Utc::now().timestamp() as usize;
        assert!(claims.exp > now + 29 * 86400);
        assert!(claims.exp <= now + 31 * 86400);
    }

    #[test]
    fn test_change_password() {
        let (_dir, auth) = test_auth();
        auth.set_password("old123").unwrap();
        auth.change_password("old123", "new456").unwrap();
        assert!(auth.login("new456").is_ok());
        assert!(auth.login("old123").is_err());
    }

    #[test]
    fn test_change_password_wrong_current() {
        let (_dir, auth) = test_auth();
        auth.set_password("correct").unwrap();
        assert!(auth.change_password("wrong", "new123").is_err());
    }

    #[test]
    fn test_requires_setup_initial() {
        let (_dir, auth) = test_auth();
        assert!(auth.requires_setup().unwrap());
    }

    #[test]
    fn test_invalid_token() {
        let (_dir, auth) = test_auth();
        assert!(auth.verify_token("invalid-token").is_err());
    }
}
