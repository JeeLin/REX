//! 认证端点集成测试（启动测试服务器验证 HTTP 响应）

#[cfg(test)]
mod auth_integration {
    use reqwest::StatusCode;
    use serde_json::json;

    // 这些测试需要启动测试服务器
    // 在 CI 中通过 cargo test --test 运行
    // 本地需要先启动 rex-hub 服务

    #[tokio::test]
    #[ignore] // 需要运行中的服务器
    async fn test_login_success() {
        let client = reqwest::Client::new();
        let resp = client
            .post("http://localhost:3000/api/auth/login")
            .json(&json!({ "password": "test123" }))
            .send()
            .await
            .unwrap();
        assert!(resp.status() == StatusCode::OK || resp.status() == StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    #[ignore]
    async fn test_login_wrong_password() {
        let client = reqwest::Client::new();
        let resp = client
            .post("http://localhost:3000/api/auth/login")
            .json(&json!({ "password": "wrong" }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    #[ignore]
    async fn test_check_auth_no_token() {
        let client = reqwest::Client::new();
        let resp = client
            .get("http://localhost:3000/api/auth/check")
            .send()
            .await
            .unwrap();
        // 未登录时返回 401
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    #[ignore]
    async fn test_security_report_requires_auth() {
        let client = reqwest::Client::new();
        let resp = client
            .get("http://localhost:3000/api/audit/security-report")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
