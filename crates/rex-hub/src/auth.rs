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
        let exp = now + chrono::Duration::days(7);
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

    pub fn login(&self, password: &str) -> AuthResult<String> {
        let hash_str = self
            .db
            .get_setting("password_hash")?
            .ok_or_else(|| RExError::Message("no password set".into()))?;
        let hash =
            argon2::PasswordHash::new(&hash_str).map_err(|e| RExError::Message(e.to_string()))?;
        use argon2::password_hash::PasswordVerifier;
        argon2::Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .map_err(|_| RExError::Message("invalid password".into()))?;
        self.generate_token()
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
            state
                .db
                .write_audit_log(&crate::models::NewAuditEntry {
                    action: "AUTH_LOGIN".into(),
                    result: "success".into(),
                    ..Default::default()
                })
                .ok();
            let expires = chrono::Utc::now() + chrono::Duration::days(7);
            Ok((
                StatusCode::OK,
                Json(serde_json::json!({
                    "token": token,
                    "expiresAt": expires.to_rfc3339()
                })),
            ))
        }
        Err(e) => {
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
                "密码错误",
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
            "密码已设置",
        ));
    }
    state.auth.set_password(&body.password).map_err(|e| {
        error_with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            &e.to_string(),
        )
    })?;
    let token = state.auth.generate_token().map_err(|e| {
        error_with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            &e.to_string(),
        )
    })?;
    let expires = chrono::Utc::now() + chrono::Duration::days(7);
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "token": token,
            "expiresAt": expires.to_rfc3339()
        })),
    ))
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
}
