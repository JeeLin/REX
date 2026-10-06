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
        mongo_pool: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        data_dir: dir.path().to_path_buf(),
        coordinator: Arc::new(rex_hub::transfer_coordinator::TransferCoordinator::new()),
        sync_coordinator: Arc::new(rex_hub::sync_coordinator::SyncCoordinator::new()),
        transfer_bcast: tokio::sync::broadcast::channel(128).0,
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
        .nest("/api/audit-log", rex_hub::audit_api::audit_routes())
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
    assert!(!json.as_array().unwrap().is_empty());
}

/// Regression for the empty audit/log viewer: SSH sessions connected fine but the
/// in-app audit list showed nothing. Two independent causes, both covered here.
///
/// 1. `/api/audit-log/stats` returned 500 whenever the filtered set was empty
///    (SQL `SUM()` over no rows is NULL, read as i64 → InvalidColumnType). The audit
///    page issues list+stats via `Promise.all`, so that 500 rejected the pair and the
///    catch handler blanked the already-fetched entries — an empty table with no error.
///    Zero matches must answer 200 with zeroed counters.
/// 2. SSH session audit rows were written without environment/resource/agent
///    attribution, so any viewer scoped by environment (audit page env chips) or by
///    agent (agent log panel, `?agent_id=`) could never match an SSH event.
#[tokio::test]
async fn test_audit_log_viewer_returns_scoped_ssh_events_and_zeroed_stats() {
    let (_dir, state) = test_state();
    let app = build_test_router(state.clone());

    // set_password returns the token used by the bearer-protected routes.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/password")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"password": "test123"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let token: String = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();
    let auth = format!("Bearer {token}");

    // Empty filtered set: stats must answer 200 + zeros, list must answer 200 + [].
    for (uri, expect_total_zero) in [
        ("/api/audit-log/stats?environment_id=env-absent", true),
        ("/api/audit-log?environment_id=env-absent", false),
    ] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("authorization", auth.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "{uri} must not 500 (it would blank the viewer)"
        );
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        if expect_total_zero {
            assert_eq!(json["total"], 0, "{uri}");
            assert_eq!(json["success_count"], 0, "{uri}");
            assert_eq!(json["failure_count"], 0, "{uri}");
        } else {
            assert_eq!(json.as_array().expect("array").len(), 0, "{uri}");
        }
    }

    // A scoped SSH event must be reachable through both the env chip and the agent
    // log panel, which query the exact shapes the frontend sends.
    state
        .db
        .write_audit_log(&rex_hub::models::NewAuditEntry {
            action: "SSH_CONNECT".into(),
            target: Some("prod-web".into()),
            environment_id: Some("env-1".into()),
            resource_id: Some("res-1".into()),
            agent_id: Some("agent-1".into()),
            result: "success".into(),
            ..Default::default()
        })
        .unwrap();

    let fetch = |app: axum::Router, uri: &'static str| {
        let auth = auth.clone();
        async move {
            let resp = app
                .oneshot(
                    Request::builder()
                        .uri(uri)
                        .header("authorization", auth)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK, "{uri}");
            let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap();
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()
        }
    };

    // Audit page, environment chip selected.
    let json = fetch(app.clone(), "/api/audit-log?environment_id=env-1&limit=50").await;
    let rows = json.as_array().expect("array");
    assert_eq!(
        rows.len(),
        1,
        "env-scoped audit query must return the SSH event"
    );
    assert_eq!(rows[0]["action"], "SSH_CONNECT");
    assert_eq!(rows[0]["resource_id"], "res-1");

    // Agent log panel: /api/audit-log?agent_id=<id>&limit=100.
    let json = fetch(app.clone(), "/api/audit-log?agent_id=agent-1&limit=100").await;
    let rows = json.as_array().expect("array");
    assert_eq!(
        rows.len(),
        1,
        "agent-scoped audit query must return the SSH event"
    );
    assert_eq!(rows[0]["action"], "SSH_CONNECT");
    assert_eq!(rows[0]["environment_id"], "env-1");

    // Stats on the same scope must count the event instead of erroring.
    let json = fetch(app.clone(), "/api/audit-log/stats?environment_id=env-1").await;
    assert_eq!(json["total"], 1);
    assert_eq!(json["success_count"], 1);
    assert_eq!(json["failure_count"], 0);
}
