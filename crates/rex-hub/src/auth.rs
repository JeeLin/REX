use argon2::password_hash::PasswordHash;
use argon2::PasswordVerifier;
use axum::extract::State;
use axum::http::{header::HeaderMap, StatusCode};
use axum::Json;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::audit::write_audit_log;
use crate::helpers::{err_resp, ApiResponse, ErrorResponse};
use crate::routes::{extract_client_ip, AppState};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Claims {
    pub(crate) sub: String,
    pub(crate) exp: usize,
}

pub fn verify_token(secret: &str, token: &str) -> bool {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .is_ok()
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let ip = extract_client_ip(&headers);

    let password_hash = crate::helpers::get_or_create_password_hash(&state.db);

    let parsed_hash =
        PasswordHash::new(&password_hash).map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

    let password_valid = argon2::Argon2::default()
        .verify_password(input.password.as_bytes(), &parsed_hash)
        .is_ok();

    if !password_valid {
        write_audit_log(
            &state.db,
            "login",
            "failure",
            "登录失败：密码错误",
            None,
            None,
            None,
            None,
            Some(&ip),
        );
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: crate::helpers::ErrorBody {
                    code: "AUTH_INVALID".to_string(),
                    message: "密码错误".to_string(),
                },
            }),
        ));
    }

    let secret = &state.secret_key;
    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + 7 * 24 * 3600;

    let claims = Claims {
        sub: "admin".to_string(),
        exp,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

    write_audit_log(
        &state.db,
        "login",
        "success",
        "登录成功",
        None,
        None,
        None,
        None,
        Some(&ip),
    );

    let expires_at = format!("{exp:010}");
    Ok(Json(ApiResponse {
        data: LoginResponse { token, expires_at },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_valid_token() {
        let secret = "test-secret";
        let exp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
            + 3600;
        let claims = Claims {
            sub: "admin".to_string(),
            exp,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();
        assert!(verify_token(secret, &token));
    }

    #[test]
    fn verify_invalid_token() {
        assert!(!verify_token("secret", "invalid-token"));
    }

    #[test]
    fn verify_wrong_secret() {
        let secret = "correct-secret";
        let exp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
            + 3600;
        let claims = Claims {
            sub: "admin".to_string(),
            exp,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();
        assert!(!verify_token("wrong-secret", &token));
    }

    #[test]
    fn verify_expired_token() {
        let secret = "test-secret";
        let exp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
            - 3600; // 已过期 1 小时
        let claims = Claims {
            sub: "admin".to_string(),
            exp,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();
        assert!(!verify_token(secret, &token));
    }

    #[test]
    fn verify_empty_token() {
        assert!(!verify_token("secret", ""));
    }

    #[test]
    fn verify_token_with_empty_secret() {
        let claims = Claims {
            sub: "admin".to_string(),
            exp: 9999999999,
        };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(b"")).unwrap();
        // 使用不同 secret 验证，应失败
        assert!(!verify_token("not-empty", &token));
    }
}
