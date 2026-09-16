//! Agent HTTP server — 提供本地 Web 访问能力。
//!
//! Agent 启动时创建 HTTP server，提供静态文件服务和 API 端点。
//! 用户可以通过浏览器直连 Agent 进行管理。
//!
//! API 代理：所有 /api/* 请求通过 HTTP 转发到 Hub，实现 Agent 本地访问。

use crate::agent_ws::{AgentEvent, AgentMsg, ApiPendingMap};
use axum::extract::{Request, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use include_dir::{include_dir, Dir};
use rex_common::embedded_static::EmbeddedStatic;
use std::sync::Arc;
use tokio::sync::mpsc;

/// 嵌入的前端 dist 目录
static DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../packages/rex-console-web/dist");

/// Agent HTTP server 状态
struct AgentState {
    api_tx: mpsc::Sender<AgentEvent>,
    api_pending: ApiPendingMap,
}

/// 健康检查端点
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "mode": "agent",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// API 代理：通过 WebSocket 隧道转发 /api/* 请求到 Hub
async fn proxy_api(
    State(state): State<Arc<AgentState>>,
    req: Request,
) -> Result<Response, axum::http::StatusCode> {
    // 提取请求信息
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| q.to_string());
    let method = req.method().to_string();
    let headers = req.headers().clone();

    // 读取请求体
    let body = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?
        .to_vec();

    // 生成唯一 request_id
    let request_id = uuid::Uuid::new_v4().to_string();

    // 复制关键头
    let mut header_map = std::collections::HashMap::new();
    for (key, value) in headers.iter() {
        if key == "content-type" || key == "authorization" || key == "cookie" {
            if let Ok(v) = value.to_str() {
                header_map.insert(key.as_str().to_string(), v.to_string());
            }
        }
    }

    // 创建 oneshot channel 等待响应
    let (tx, rx) = tokio::sync::oneshot::channel();

    // 注册挂起请求
    {
        let mut pending = state.api_pending.write().await;
        pending.insert(request_id.clone(), tx);
    }

    // 构造 ApiRequest 消息
    let api_msg = AgentMsg::ApiRequest {
        payload: crate::agent_ws::ApiRequestPayload {
            request_id: request_id.clone(),
            method,
            path,
            query,
            headers: if header_map.is_empty() {
                None
            } else {
                Some(header_map)
            },
            body,
        },
    };

    // 序列化并发送
    let msg_json = serde_json::to_string(&api_msg).map_err(|_| {
        // 清理挂起请求
        let api_pending = state.api_pending.clone();
        let rid = request_id.clone();
        tokio::spawn(async move {
            api_pending.write().await.remove(&rid);
        });
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if state.api_tx.send(AgentEvent::Text(msg_json)).await.is_err() {
        // WS 已断开
        state.api_pending.write().await.remove(&request_id);
        return Err(axum::http::StatusCode::BAD_GATEWAY);
    }

    // 等待响应（超时 30 秒）
    let api_response = match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(_)) => {
            // oneshot sender dropped
            state.api_pending.write().await.remove(&request_id);
            return Err(axum::http::StatusCode::BAD_GATEWAY);
        }
        Err(_) => {
            // timeout
            state.api_pending.write().await.remove(&request_id);
            return Err(axum::http::StatusCode::GATEWAY_TIMEOUT);
        }
    };

    // 构建 HTTP 响应
    let status = axum::http::StatusCode::from_u16(api_response.status)
        .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);

    let mut response = Response::new(axum::body::Body::from(api_response.body));
    *response.status_mut() = status;

    // 复制响应头
    for (key, value) in &api_response.headers {
        if let (Ok(name), Ok(val)) = (
            axum::http::header::HeaderName::from_bytes(key.as_bytes()),
            axum::http::HeaderValue::from_str(value),
        ) {
            response.headers_mut().insert(name, val);
        }
    }

    Ok(response)
}

/// 启动 Agent HTTP server
pub async fn start_http_server(
    port: u16,
    _hub_url: String,
    api_tx: mpsc::Sender<AgentEvent>,
    api_pending: ApiPendingMap,
) -> anyhow::Result<()> {
    let embedded = EmbeddedStatic::new("/", &DIST);

    let state = Arc::new(AgentState {
        api_tx,
        api_pending,
    });

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/*path", axum::routing::any(proxy_api))
        .fallback_service(axum::routing::any_service(embedded))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!(addr = %addr, "starting agent HTTP server");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
