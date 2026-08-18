//! API 集成测试 — 测试认证和环境管理 API。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rex_hub::db::Database;
use rex_hub::{auth, crypto, AppState};
use std::sync::Arc;
use tower::util::ServiceExt;

fn test_state() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let db = Arc::new(Database::open(&db_path).unwrap());
    let auth = Arc::new(auth::AuthConfig::new(db.clone()).unwrap());
    let crypto = Arc::new(crypto::CredentialCrypto::from_data_dir(dir.path()).unwrap());

    let state = AppState {
        db,
        auth,
        crypto,
        sql_pool: Arc::new(tokio::sync::Mutex::new(
            rex_hub::sql_api::SqlConnectionPool::new(),
        )),
        redis_pool: Arc::new(tokio::sync::Mutex::new(
            rex_hub::redis_api::RedisConnectionPool::new(),
        )),
        file_pool: Arc::new(tokio::sync::Mutex::new(
            rex_hub::file_api::FileConnectionPool::new(),
        )),
        agent_tunnel: Arc::new(rex_hub::agent_ws::AgentTunnelState::new()),
        agent_binaries: Arc::new(rex_hub::update_api::AgentBinaries::new()),
        sip_capture: Arc::new(rex_hub::sip_capture::SipCaptureRegistry::new()),
        sip_recording: Arc::new(rex_hub::sip_recording::SipRecordingRegistry::new(
            dir.path().to_path_buf(),
        )),
        data_dir: dir.path().to_path_buf(),
    };
    (dir, state)
}

fn build_test_router(state: AppState) -> axum::Router {
    use rex_hub::middleware::AuthUser;

    let public_routes = axum::Router::new()
        .route("/api/auth/check", axum::routing::get(auth::check_auth))
        .route("/api/auth/login", axum::routing::post(auth::login))
        .route(
            "/api/auth/password",
            axum::routing::post(auth::set_password),
        );

    let protected_routes = axum::Router::new()
        .nest(
            "/api/environments",
            rex_hub::env_api::env_routes()
                .merge(rex_hub::resource_api::resource_routes())
                .merge(rex_hub::agent_api::env_agent_routes()),
        )
        .layer(axum::middleware::from_extractor_with_state::<
            AuthUser,
            AppState,
        >(state.clone()));

    public_routes.merge(protected_routes).with_state(state)
}

#[tokio::test]
async fn test_auth_check_requires_setup() {
    let (_dir, state) = test_state();
    let app = build_test_router(state);

    let req = Request::builder()
        .uri("/api/auth/check")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["requires_setup"], true);
}

#[tokio::test]
async fn test_set_password_then_login() {
    let (_dir, state) = test_state();
    let app = build_test_router(state);

    // 设置密码
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/password")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"password": "test123"}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["token"].is_string());

    // 登录
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"password": "test123"}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_login_wrong_password() {
    let (_dir, state) = test_state();
    let app = build_test_router(state);

    // 先设置密码
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/password")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"password": "correct"}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 用错误密码登录
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"password": "wrong"}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_route_without_token() {
    let (_dir, state) = test_state();
    let app = build_test_router(state);

    let req = Request::builder()
        .uri("/api/environments")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_and_list_environment() {
    let (_dir, state) = test_state();
    let app = build_test_router(state);

    // 先设置密码获取 token
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/password")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"password": "test123"}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token = json["token"].as_str().unwrap();

    // 创建环境
    let req = Request::builder()
        .method("POST")
        .uri("/api/environments")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({"name": "test-env", "connection_mode": "direct"}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 列出环境
    let req = Request::builder()
        .uri("/api/environments")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
    assert!(json.as_array().unwrap().len() >= 1);
}
