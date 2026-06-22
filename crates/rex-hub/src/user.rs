use argon2::password_hash::PasswordVerifier;
use argon2::password_hash::{PasswordHash, SaltString};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::helpers::{bad_request, err_resp, ApiResponse, ErrorResponse};
use crate::routes::AppState;

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUsernameRequest {
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// GET /api/user/profile
pub async fn get_profile(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<UserProfile>>, (StatusCode, Json<ErrorResponse>)> {
    let username: Option<String> = state
        .db
        .pool
        .get()
        .unwrap()
        .query_row(
            "SELECT value FROM settings WHERE key = 'username'",
            [],
            |row| row.get(0),
        )
        .ok();

    Ok(Json(ApiResponse {
        data: UserProfile {
            username: username.unwrap_or_else(|| "admin".to_string()),
        },
    }))
}

/// PUT /api/user/profile
pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    Json(input): Json<UpdateUsernameRequest>,
) -> Result<Json<ApiResponse<UserProfile>>, (StatusCode, Json<ErrorResponse>)> {
    if input.username.trim().is_empty() {
        return Err(bad_request("用户名不能为空"));
    }

    if input.username.len() > 64 {
        return Err(bad_request("用户名不能超过 64 个字符"));
    }

    state
        .db
        .pool
        .get()
        .unwrap()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('username', ?1)",
            rusqlite::params![input.username],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "保存用户名失败"))?;

    Ok(Json(ApiResponse {
        data: UserProfile {
            username: input.username,
        },
    }))
}

/// PUT /api/user/password
pub async fn change_password(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ErrorResponse>)> {
    if input.new_password.len() < 6 {
        return Err(bad_request("新密码长度不能少于 6 个字符"));
    }

    let password_hash = crate::helpers::get_or_create_password_hash(&state.db);

    // 验证当前密码
    let parsed_hash =
        PasswordHash::new(&password_hash).map_err(|_| err_resp("INTERNAL_ERROR", "内部错误"))?;

    let current_valid = argon2::Argon2::default()
        .verify_password(input.current_password.as_bytes(), &parsed_hash)
        .is_ok();

    if !current_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: crate::helpers::ErrorBody {
                    code: "AUTH_INVALID".to_string(),
                    message: "当前密码错误".to_string(),
                },
            }),
        ));
    }

    // 生成新密码哈希
    let new_salt = SaltString::generate(&mut OsRng);
    let new_hash = argon2::password_hash::PasswordHasher::hash_password(
        &argon2::Argon2::default(),
        input.new_password.as_bytes(),
        &new_salt,
    )
    .map_err(|_| err_resp("INTERNAL_ERROR", "密码哈希生成失败"))?
    .to_string();

    state
        .db
        .pool
        .get()
        .unwrap()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('password_hash', ?1)",
            rusqlite::params![new_hash],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "保存密码失败"))?;

    Ok(Json(ApiResponse { data: () }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Arc<crate::db::Database> {
        Arc::new(crate::db::Database::new_in_memory().unwrap())
    }

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            db: test_db(),
            secret_key: "test".to_string(),
            connections: Arc::new(crate::ws::new_connections()),
            sessions: Arc::new(crate::terminal::SessionManager::new(900)),
            transfer: None,
            update_cache: tokio::sync::RwLock::new(crate::routes::UpdateCache::new()),
            data_dir: std::path::PathBuf::from("./data"),
        })
    }

    #[tokio::test]
    async fn get_profile_returns_default_username() {
        let state = test_state();
        let result = get_profile(State(state)).await.unwrap();
        assert_eq!(result.0.data.username, "admin");
    }

    #[tokio::test]
    async fn update_profile_saves_username() {
        let state = test_state();
        let result = update_profile(
            State(state.clone()),
            Json(UpdateUsernameRequest {
                username: "test_user".to_string(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(result.0.data.username, "test_user");

        let result = get_profile(State(state)).await.unwrap();
        assert_eq!(result.0.data.username, "test_user");
    }

    #[tokio::test]
    async fn update_profile_rejects_empty_username() {
        let state = test_state();
        let result = update_profile(
            State(state),
            Json(UpdateUsernameRequest {
                username: "".to_string(),
            }),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn change_password_wrong_current_password() {
        let state = test_state();
        let result = change_password(
            State(state),
            Json(ChangePasswordRequest {
                current_password: "wrong".to_string(),
                new_password: "newpass123".to_string(),
            }),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn change_password_rejects_short_password() {
        let state = test_state();
        let result = change_password(
            State(state),
            Json(ChangePasswordRequest {
                current_password: "admin".to_string(),
                new_password: "123".to_string(),
            }),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn change_password_success() {
        let state = test_state();
        let result = change_password(
            State(state),
            Json(ChangePasswordRequest {
                current_password: "admin".to_string(),
                new_password: "newpass123".to_string(),
            }),
        )
        .await;
        assert!(result.is_ok());
    }
}
