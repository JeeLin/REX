//! WebSocket 终端桥接 — 浏览器 ↔ Hub ↔ SSH 服务器 / Agent。
//!
//! 统一入口：/ws/terminal?token=jwt&resourceId=xxx
//! Hub 从 DB 读取资源连接信息，自动判断直连或 Agent 隧道。
//! 前端完全不感知底层连接方式。

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use rex_ssh::{SshConfig, SshSession, TerminalEvent};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::Interval;

use crate::agent_ws::{AgentEvent, ConnectResponse};
use crate::db::{audit_log_scoped, AuditScope};
use crate::error::{connect_error_with_stage, ConnFailure, ErrorPayload, ProtoKind};
use crate::AppState;

/// 前端 → 后端的消息（连接建立后的控制消息）
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMsg {
    #[serde(rename = "terminal.data")]
    Data { data: String },
    #[serde(rename = "terminal.resize")]
    Resize { cols: u32, rows: u32 },
    #[serde(rename = "terminal.disconnect")]
    Disconnect,
    /// 客户端心跳（每30秒发送），后端忽略即可维持连接活跃
    #[serde(rename = "ping")]
    Ping,
}

/// 后端 → 前端的消息
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMsg {
    #[serde(rename = "terminal.connected")]
    Connected { payload: ConnectedPayload },
    #[serde(rename = "terminal.data")]
    Data { payload: DataPayload },
    #[serde(rename = "terminal.disconnected")]
    Disconnected { payload: DisconnectedPayload },
    #[serde(rename = "terminal.error")]
    Error { payload: ErrorPayload },
}

#[derive(Debug, Serialize)]
struct ConnectedPayload {
    #[serde(rename = "sessionId")]
    session_id: String,
}

#[derive(Debug, Serialize)]
struct DataPayload {
    data: String,
}

#[derive(Debug, Serialize)]
struct DisconnectedPayload {
    reason: String,
}

/// 带分类的资源加载失败。`String` 版本无法区分二者，会让前端对解密失败
/// 也走自动重连，形成无限重连。
#[derive(Debug)]
struct ConnError {
    message: String,
    failure: ConnFailure,
}

impl ConnError {
    fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            failure: ConnFailure::Fatal,
        }
    }

    fn transient(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            failure: ConnFailure::Transient,
        }
    }
}

/// URL 查询参数
#[derive(Deserialize)]
pub struct TerminalQuery {
    #[serde(rename = "resourceId")]
    pub resource_id: String,
}

/// 资源连接信息
struct ResourceConnInfo {
    name: String,
    environment_id: String,
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    private_key: Option<String>,
    use_agent: bool,
    agent_id: Option<String>,
    keepalive_interval: Option<u32>,
    init_script: Option<String>,
}

impl ResourceConnInfo {
    /// 审计归属：SSH 会话事件必须带上环境 / 资源 / Agent，
    /// 否则按环境或按 Agent 过滤的审计查看器查不到任何记录。
    fn audit_scope(&self, resource_id: &str) -> AuditScope {
        AuditScope {
            environment_id: Some(self.environment_id.clone()),
            resource_id: Some(resource_id.to_string()),
            agent_id: self.agent_id.clone(),
        }
    }
}

/// GET /ws/terminal?token=jwt&resourceId=xxx
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<TerminalQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, query.resource_id))
}

/// WebSocket 连接处理主循环
async fn handle_socket(mut ws: WebSocket, state: AppState, resource_id: String) {
    let session_id = format!("sess_{}", &uuid::Uuid::new_v4().to_string()[..8]);

    // 从 DB 读取资源连接信息（含解密密码）
    let conn_info = match load_resource_conn(&state, &resource_id).await {
        Ok(info) => info,
        Err(e) => {
            // 终止性错误（解密失败 / 配置非法 / 资源不存在）标 fatal：前端据此
            // 停止重连。若沿用可重试语义，用户会看到终端无限重连且每次都回到
            // 第一次的状态，却始终拿不到真正的原因。
            tracing::warn!(
                action = "SSH_RESOURCE_LOAD",
                resource_id = %resource_id,
                retryable = e.failure.retryable(),
                error = %e.message,
                "SSH resource load failed"
            );
            let _ = send_ws_error_classified(&mut ws, &e.message, e.failure).await;
            return;
        }
    };

    let scope = conn_info.audit_scope(&resource_id);
    tracing::info!(
        action = "SSH_CONNECT",
        resource_id = %resource_id,
        host = %conn_info.host,
        port = conn_info.port,
        username = %conn_info.username,
        use_agent = conn_info.use_agent,
        "SSH connection initiated"
    );
    audit_log_scoped(
        &state.db,
        "SSH_CONNECT",
        "success",
        Some(conn_info.name.clone()),
        scope.clone(),
    );

    if conn_info.use_agent {
        handle_agent_terminal(ws, &state, &conn_info, &resource_id, &session_id).await;
    } else {
        handle_direct_terminal(ws, &state, &conn_info, &resource_id, &session_id).await;
    }

    tracing::info!(
        action = "SSH_DISCONNECT",
        resource_id = %resource_id,
        name = %conn_info.name,
        "SSH session ended"
    );
    audit_log_scoped(
        &state.db,
        "SSH_DISCONNECT",
        "success",
        Some(conn_info.name),
        scope,
    );
}

/// 从 DB 读取资源连接信息
/// host/port/username 来自 Resource 顶层字段；password/privateKey 来自 config_json（加密存储）
async fn load_resource_conn(
    state: &AppState,
    resource_id: &str,
) -> Result<ResourceConnInfo, ConnError> {
    let db = state.db.clone();
    let rid = resource_id.to_string();
    let crypto = state.crypto.clone();

    tokio::task::spawn_blocking(move || {
        // 获取资源
        tracing::debug!(action = "SSH_RESOURCE_LOAD", resource_id = %rid, "loading resource connection info");
        let resource = db
            .get_resource(&rid)
            .map_err(|e| {
                tracing::error!(action = "SSH_RESOURCE_LOAD", resource_id = %rid, error = %e, "database query failed");
                ConnError::transient(format!("db error: {e}"))
            })?
            .ok_or_else(|| {
                tracing::warn!(action = "SSH_RESOURCE_LOAD", resource_id = %rid, "resource not found in database");
                ConnError::fatal(format!("resource not found: {rid}"))
            })?;

        tracing::debug!(
            action = "SSH_RESOURCE_LOAD",
            resource_id = %rid,
            name = %resource.name,
            protocol = %resource.protocol,
            host = %resource.host,
            port = ?resource.port,
            username = %resource.username,
            has_config_json = %crate::resource_conn::has_config_json(&resource.config_json),
            "resource loaded"
        );

        // 从 Resource 顶层字段获取连接信息
        let host = resource.host.clone();
        if host.is_empty() {
            return Err(ConnError::fatal(format!("resource {rid}: host is empty, please fill in host in resource settings")));
        }
        let port = resource.port.unwrap_or(22);
        // 与 SFTP/其它协议共用同一兜底口径，保证连接池键 user@host:port 一致
        let username = crate::resource_conn::normalize_username(&resource.username);

        // 从 config_json 解密敏感字段（password、privateKey/private_key、initScript）
        // 未知键静默忽略，不报错也不告警。
        let (password, private_key, init_script) =
            if crate::resource_conn::has_config_json(&resource.config_json) {
            let config_str = crypto
                    .decrypt(&resource.config_json)
                    .map_err(|e| {
                        tracing::error!(action = "SSH_CONFIG_DECRYPT", resource_id = %rid, resource_name = %resource.name, error = %e, "config_json decryption failed");
                        ConnError::fatal(format!("{} ({e})", crate::error::CREDENTIAL_DECRYPT_MSG))
                    })?;

            let config: serde_json::Value = serde_json::from_str(&config_str).map_err(|e| {
                tracing::error!(action = "SSH_CONFIG_PARSE", resource_id = %rid, resource_name = %resource.name, error = %e, "config_json parse failed");
                ConnError::fatal(format!("invalid config json: {e}"))
            })?;

            let pw = config
                .get("password")
                .and_then(|v| v.as_str())
                .map(String::from);
            let pk = rex_common::resource_config::config_private_key(&config);
            let init_script = config
                .get("initScript")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(String::from);

            tracing::debug!(
                action = "SSH_CONFIG_LOADED",
                resource_id = %rid,
                has_password = pw.is_some(),
                has_private_key = pk.is_some(),
                "sensitive config loaded"
            );

            (pw, pk, init_script)
        } else {
            tracing::debug!(action = "SSH_CONFIG_PARSE", resource_id = %rid, resource_name = %resource.name, "no config_json — using defaults");
            (None, None, None)
        };

        let auth_method = if private_key.is_some() {
            "key"
        } else if password.is_some() {
            "password"
        } else {
            "none"
        };
        tracing::info!(
            action = "SSH_CONFIG_LOADED",
            resource_id = %rid,
            host = %host,
            port = port,
            username = %username,
            auth_method = auth_method,
            "SSH connection parameters resolved"
        );

        // 获取环境信息
        let env = db
            .get_environment(&resource.environment_id)
            .map_err(|e| {
                tracing::error!(action = "SSH_ENV_LOAD", resource_id = %rid, resource_name = %resource.name, env_id = %resource.environment_id, error = %e, "failed to load environment");
                ConnError::transient(format!("db error: {e}"))
            })?
            .ok_or_else(|| {
                tracing::warn!(action = "SSH_ENV_NOT_FOUND", resource_id = %rid, resource_name = %resource.name, env_id = %resource.environment_id, "environment not found");
                ConnError::fatal(format!("environment not found: {}", resource.environment_id))
            })?;

        let use_agent = env.connection_mode == "agent";
        tracing::debug!(
            action = "SSH_ENV_LOADED",
            resource_id = %rid,
            env_id = %resource.environment_id,
            connection_mode = %env.connection_mode,
            use_agent = use_agent,
            "environment loaded"
        );

        let agent_id = if use_agent {
            let agents = db
                .list_agents_by_env(&resource.environment_id)
                .unwrap_or_default();
            let online = agents.iter().find(|a| a.status == "online");
            tracing::debug!(
                action = "SSH_AGENT_LOOKUP",
                resource_id = %rid,
                total_agents = agents.len(),
                online_agent = online.map(|a| a.id.as_str()).unwrap_or("none"),
                "agent lookup"
            );
            online.map(|a| a.id.clone())
        } else {
            None
        };

        if use_agent && agent_id.is_none() {
            tracing::warn!(
                action = "SSH_NO_AGENT",
                resource_id = %rid,
                env_id = %resource.environment_id,
                "no online agent available — agent connection will fail"
            );
        }

        Ok(ResourceConnInfo {
            name: resource.name.clone(),
            environment_id: resource.environment_id.clone(),
            host,
            port,
            username,
            password,
            private_key,
            use_agent,
            agent_id,
            keepalive_interval: None,
            init_script,
        })
    })
    .await
    .map_err(|e| ConnError::transient(format!("task join error: {e}")))?
}

// ═══════════════════════════════════════
// 直连模式
// ═══════════════════════════════════════

async fn handle_direct_terminal(
    mut ws: WebSocket,
    state: &AppState,
    conn: &ResourceConnInfo,
    resource_id: &str,
    session_id: &str,
) {
    tracing::info!(
        action = "SSH_DIRECT_CONNECT",
        session_id = %session_id,
        resource_name = %conn.name,
        host = %conn.host,
        port = conn.port,
        username = %conn.username,
        has_password = conn.password.is_some(),
        has_private_key = conn.private_key.is_some(),
        "SSH direct connection attempting"
    );

    let config = SshConfig {
        host: conn.host.clone(),
        port: conn.port,
        username: conn.username.clone(),
        password: conn.password.clone(),
        private_key: conn.private_key.clone(),
        keepalive_interval: conn.keepalive_interval,
        init_script: conn.init_script.clone(),
    };

    let session = match SshSession::connect(config).await {
        Ok(s) => {
            tracing::info!(action = "SSH_DIRECT_CONNECTED", session_id = %session_id, host = %conn.host, "SSH direct connection established");
            s
        }
        Err(e) => {
            tracing::error!(
                action = "SSH_DIRECT_FAILED",
                session_id = %session_id,
                host = %conn.host,
                port = conn.port,
                username = %conn.username,
                error = %e,
                "SSH direct connection failed"
            );
            audit_log_scoped(
                &state.db,
                "SSH_CONNECT",
                "failure",
                Some(conn.name.clone()),
                conn.audit_scope(resource_id),
            );
            let _ = send_ws_error_classified(
                &mut ws,
                &format!("SSH connection failed: {e}"),
                classify_ssh_connect_failure(&e),
            )
            .await;
            return;
        }
    };

    let _ = ws
        .send(Message::Text(
            serde_json::to_string(&ServerMsg::Connected {
                payload: ConnectedPayload {
                    session_id: session_id.to_string(),
                },
            })
            .unwrap()
            .into(),
        ))
        .await;

    let session = Arc::new(Mutex::new(session));
    let (mut ws_sink, mut ws_stream) = ws.split();
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<ClientMsg>(64);
    let (data_tx, mut data_rx) = mpsc::channel::<String>(512);

    let cmd_tx_for_ping = cmd_tx.clone();
    let mut ws_read_task = tokio::spawn(async move {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) {
                        let is_disconnect = matches!(client_msg, ClientMsg::Disconnect);
                        if cmd_tx.send(client_msg).await.is_err() {
                            break;
                        }
                        if is_disconnect {
                            break;
                        }
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    });

    let session_for_ssh = session.clone();
    let mut ssh_task = tokio::spawn(async move {
        loop {
            let mut session = session_for_ssh.lock().await;
            tokio::select! {
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(ClientMsg::Data { data }) => {
                            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&data) {
                                let _ = session.send_data(bytes::Bytes::from(bytes)).await;
                            }
                        }
                        Some(ClientMsg::Resize { cols, rows }) => {
                            let _ = session.resize(cols, rows).await;
                        }
                        Some(ClientMsg::Disconnect) | None => {
                            // 收到 `terminal.disconnect` 帧 → 主动断开 SSH，
                            // 释放服务端会话；随后 ws_read_task 退出，select! 解除。
                            let _ = session.disconnect().await;
                            break;
                        }
                        Some(ClientMsg::Ping) => {
                            // 心跳收到，无需响应（前端只是发 ping 维持连接活跃）
                        }
                    }
                }
                event = session.recv() => {
                    drop(session);
                    match event {
                        Some(TerminalEvent::Data(data)) => {
                            let encoded = base64::engine::general_purpose::STANDARD.encode(data.as_bytes());
                            if data_tx.send(encoded).await.is_err() {
                                break;
                            }
                        }
                        Some(TerminalEvent::Disconnected(reason)) => {
                            let msg = ServerMsg::Disconnected {
                                payload: DisconnectedPayload { reason },
                            };
                            let _ = data_tx.send(serde_json::to_string(&msg).unwrap_or_default()).await;
                            break;
                        }
                        None => break,
                    }
                    continue;
                }
            }
            drop(session);
        }
    });

    let mut ws_write_task = tokio::spawn(async move {
        while let Some(data) = data_rx.recv().await {
            let msg = if data.starts_with('{') {
                Message::Text(data.into())
            } else {
                let wrapped = ServerMsg::Data {
                    payload: DataPayload { data },
                };
                Message::Text(serde_json::to_string(&wrapped).unwrap().into())
            };
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 服务端 keepalive ping（每 25 秒发送 ping，防止中间件/代理超时断开）
    let mut ping_interval = create_server_ping_interval();
    let mut ws_ping_task = tokio::spawn(async move {
        loop {
            ping_interval.tick().await;
            // 通过 cmd_tx_for_ping 发送 ping（不直接持有 ws_sink）
            if cmd_tx_for_ping.send(ClientMsg::Ping).await.is_err() {
                break;
            }
        }
    });

    // 任一子任务退出（前端 WS 关闭 / 断开 / session 断开）→ 取消其它任务并
    // 主动断开 SSH。过去仅 `tokio::select!` 等待其中一者结束就原样返回，
    // 另外三个任务仍可能滞留 —— 若前端因某种原因未及时触发 WS close
    //（如 tab 被 KeepAlive 缓存后又因路由/异常脱离）会话就会泄漏，
    // 服务端继续重连且永远不退出。
    tokio::select! {
        _ = &mut ws_read_task => {},
        _ = &mut ssh_task => {},
        _ = &mut ws_write_task => {},
        _ = &mut ws_ping_task => {},
    }
    ws_read_task.abort();
    ssh_task.abort();
    ws_write_task.abort();
    ws_ping_task.abort();
    // 主动断开 SSH channel（直接模式 session 在本函数作用域持有 Arc）。
    if let Err(e) = session.lock().await.disconnect().await {
        tracing::debug!(action = "SSH_SESSION_END", session_id, error = %e, "ssh disconnect on session end");
    }

    tracing::debug!(
        action = "SSH_SESSION_END",
        session_id,
        "terminal session ended"
    );
}

// ═══════════════════════════════════════
// Agent 隧道模式
// ═══════════════════════════════════════

async fn handle_agent_terminal(
    mut ws: WebSocket,
    state: &AppState,
    conn: &ResourceConnInfo,
    resource_id: &str,
    session_id: &str,
) {
    tracing::info!(
        action = "SSH_AGENT_CONNECT",
        session_id = %session_id,
        resource_id = %resource_id,
        host = %conn.host,
        port = conn.port,
        username = %conn.username,
        "SSH agent connection attempting"
    );

    let agent_id = match conn.agent_id.as_ref() {
        Some(id) => id.clone(),
        None => {
            tracing::error!(
                action = "SSH_AGENT_NO_ONLINE",
                session_id = %session_id,
                resource_id = %resource_id,
                "no online agent available for this environment"
            );
            let _ = send_ws_error(&mut ws, "no online agent for this environment").await;
            return;
        }
    };

    tracing::debug!(action = "SSH_AGENT_SELECTED", session_id = %session_id, agent_id = %agent_id, "agent selected");

    let agent_conn = {
        let conns = state.agent_tunnel.connections.read().await;
        conns.get(&agent_id).cloned()
    };

    let agent_conn = match agent_conn {
        Some(c) => c,
        None => {
            tracing::error!(
                action = "SSH_AGENT_NOT_FOUND",
                session_id = %session_id,
                agent_id = %agent_id,
                "agent WebSocket connection not found — agent may have disconnected"
            );
            let _ = send_ws_error(&mut ws, "agent not connected").await;
            return;
        }
    };

    // 发送 connect 到 Agent
    let request_id = format!("req_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let (resp_tx, resp_rx) = oneshot::channel();

    {
        let mut pending = state.agent_tunnel.pending_requests.write().await;
        pending.insert(request_id.clone(), resp_tx);
    }

    let connect_msg = serde_json::json!({
        "type": "connect",
        "payload": {
            "request_id": request_id,
            "resource_id": resource_id,
            "protocol": "ssh",
            "config": {
                "host": conn.host,
                "port": conn.port,
                "username": conn.username,
                "password": conn.password,
                "privateKey": conn.private_key,
            }
        }
    });
    tracing::debug!(action = "SSH_AGENT_CONFIG", session_id = %session_id, resource_id = %resource_id, host = %conn.host, port = conn.port, username = %conn.username, has_password = conn.password.is_some(), has_private_key = conn.private_key.is_some(), has_init_script = conn.init_script.is_some(), "SSH config forwarded to agent");

    tracing::debug!(
        action = "SSH_AGENT_SEND_FAILED",
        session_id = %session_id,
        agent_id = %agent_id,
        request_id = %request_id,
        host = %conn.host,
        port = conn.port,
        "sending connect request to agent"
    );

    if agent_conn
        .sender
        .send(AgentEvent::Text(connect_msg.to_string()))
        .await
        .is_err()
    {
        tracing::error!(
            action = "SSH_AGENT_SEND_FAILED",
            session_id = %session_id,
            agent_id = %agent_id,
            request_id = %request_id,
            "failed to send connect request to agent — channel may be closed"
        );
        let _ = send_ws_error(&mut ws, "failed to contact agent").await;
        return;
    }

    // 等待 Agent 响应
    let channel_id = match tokio::time::timeout(std::time::Duration::from_secs(10), resp_rx).await {
        Ok(Ok(ConnectResponse {
            channel_id: Some(id),
            ..
        })) => {
            tracing::info!(
                action = "SSH_AGENT_CONNECTED",
                session_id = %session_id,
                agent_id = %agent_id,
                channel_id = %id,
                "agent SSH connection established"
            );
            id
        }
        Ok(Ok(ConnectResponse { error: Some(e), .. })) => {
            tracing::error!(
                action = "SSH_AGENT_ERROR",
                session_id = %session_id,
                agent_id = %agent_id,
                request_id = %request_id,
                error = %e,
                "agent reported connection error"
            );
            let _ = send_ws_error_classified(&mut ws, &e, classify_ssh_connect_failure(&e)).await;
            return;
        }
        Ok(Ok(_)) => {
            tracing::error!(
                action = "SSH_AGENT_ERROR",
                session_id = %session_id,
                agent_id = %agent_id,
                request_id = %request_id,
                "agent returned unexpected response (no channel_id, no error)"
            );
            let _ = send_ws_error(&mut ws, "agent returned unexpected response").await;
            return;
        }
        Ok(Err(_)) => {
            tracing::error!(
                action = "SSH_AGENT_ERROR",
                session_id = %session_id,
                agent_id = %agent_id,
                request_id = %request_id,
                "agent response channel closed unexpectedly"
            );
            let _ = send_ws_error(&mut ws, "agent connection failed (channel closed)").await;
            return;
        }
        Err(_) => {
            tracing::error!(
                action = "SSH_AGENT_TIMEOUT",
                session_id = %session_id,
                agent_id = %agent_id,
                request_id = %request_id,
                "agent connection timed out after 10s"
            );
            audit_log_scoped(
                &state.db,
                "SSH_AGENT_TIMEOUT",
                "failure",
                Some(agent_id.to_string()),
                conn.audit_scope(resource_id),
            );
            let _ = send_ws_error(&mut ws, "agent connection timeout").await;
            return;
        }
    };

    tracing::info!(action = "SSH_AGENT_CONNECTED", channel_id = %channel_id, session_id = %session_id, "agent terminal connected");

    // 通知前端连接成功
    let _ = ws
        .send(Message::Text(
            serde_json::to_string(&ServerMsg::Connected {
                payload: ConnectedPayload {
                    session_id: session_id.to_string(),
                },
            })
            .unwrap()
            .into(),
        ))
        .await;

    // 注册 tunnel data channel
    let (data_tx, mut data_rx) = mpsc::channel::<Vec<u8>>(512);
    {
        let mut tunnel_data = state.agent_tunnel.tunnel_data.write().await;
        tunnel_data.insert(channel_id.clone(), data_tx);
    }

    // 桥接：前端 ↔ Agent
    let (mut ws_sink, mut ws_stream) = ws.split();
    let ch_id_num = channel_id.parse::<u32>().unwrap_or(0);

    let agent_for_send = agent_conn.clone();
    let channel_id_clone = channel_id.clone();
    let mut frontend_to_agent = tokio::spawn(async move {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) {
                        match client_msg {
                            ClientMsg::Data { data } => {
                                if let Ok(decoded) =
                                    base64::engine::general_purpose::STANDARD.decode(&data)
                                {
                                    let mut frame = Vec::with_capacity(4 + decoded.len());
                                    frame.extend_from_slice(&ch_id_num.to_be_bytes());
                                    frame.extend_from_slice(&decoded);
                                    let _ =
                                        agent_for_send.sender.send(AgentEvent::Bytes(frame)).await;
                                }
                            }
                            ClientMsg::Resize { cols, rows } => {
                                let resize_msg = serde_json::json!({
                                    "type": "resize",
                                    "payload": { "channelId": channel_id_clone, "cols": cols, "rows": rows }
                                });
                                let _ = agent_for_send
                                    .sender
                                    .send(AgentEvent::Text(resize_msg.to_string()))
                                    .await;
                            }
                            ClientMsg::Disconnect => break,
                            ClientMsg::Ping => {
                                // 心跳收到，无需响应
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    });

    let mut agent_to_frontend = tokio::spawn(async move {
        while let Some(data) = data_rx.recv().await {
            let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
            let msg = ServerMsg::Data {
                payload: DataPayload { data: encoded },
            };
            if ws_sink
                .send(Message::Text(serde_json::to_string(&msg).unwrap().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // 服务端 keepalive ping（每 25 秒发送 ping，防止中间件/代理超时断开）
    let mut ping_interval = create_server_ping_interval();
    let agent_for_ping = agent_conn.clone();
    let mut agent_ping_task = tokio::spawn(async move {
        loop {
            ping_interval.tick().await;
            let ping_msg = serde_json::json!({
                "type": "ping",
                "payload": {}
            });
            if agent_for_ping
                .sender
                .send(AgentEvent::Text(ping_msg.to_string()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // 任一子任务退出（前端 WS 关闭 / Agent 通道断开）→ 取消其它子任务并
    // 通知 Agent 关闭 channel。否则 agent 侧 russh session 与 out/in task
    // 会滞留，表现为「关了 tab 后台仍在重连」。见 Bug 2 回报。
    tokio::select! {
        _ = &mut frontend_to_agent => {},
        _ = &mut agent_to_frontend => {},
        _ = &mut agent_ping_task => {},
    }
    frontend_to_agent.abort();
    agent_to_frontend.abort();
    agent_ping_task.abort();

    // 显式告知 Agent 关闭该 channel：Agent 侧 `run_ssh_session` 于是退出，
    // russh handle 从池中移除，SSH 连接在 Agent 内被动断开。
    // 帧形状与 `resource_api` 的测试连接收尾一致（`channel_id`）。
    let close_msg = serde_json::json!({
        "type": "close",
        "payload": { "channel_id": channel_id.clone() }
    })
    .to_string();
    if agent_conn
        .sender
        .send(AgentEvent::Text(close_msg))
        .await
        .is_err()
    {
        tracing::debug!(
            action = "SSH_AGENT_CLOSE",
            session_id = %session_id,
            channel_id = %channel_id,
            "agent connection already gone when sending close frame"
        );
    }

    // 清理
    {
        let mut tunnel_data = state.agent_tunnel.tunnel_data.write().await;
        tunnel_data.remove(&channel_id);
    }
    {
        let mut channels = state.agent_tunnel.channels.write().await;
        channels.remove(&channel_id);
    }

    tracing::debug!(
        action = "SSH_SESSION_END",
        session_id,
        "agent terminal session ended"
    );
}

/// 发错误信令到前端。
///
/// 默认按「可重试 / transient」分类：保持原有行为（前端自动重连）。
/// 仅凭据 / 配置级别的错误应走 [`send_ws_error_classified`] 并传 `Fatal`。
async fn send_ws_error(ws: &mut WebSocket, msg: &str) -> Result<(), axum::Error> {
    send_ws_error_classified(ws, msg, ConnFailure::Transient).await
}

/// 发分类错误信令。`Fatal` 错误带 `retryable=false`，前端在收到后停止
/// 自动重连并把 `message` 呈现给用户（解密失败 / 认证失败 / 资源缺失等）。
///
/// 分类与码合成统一走 `crate::error::connect_error_with_stage`；SSH 不带协议
/// 前缀，故产出与迁移前逐字一致（可重试 → 裸根因码，终止性 → 裸终止性码）。
async fn send_ws_error_classified(
    ws: &mut WebSocket,
    msg: &str,
    failure: ConnFailure,
) -> Result<(), axum::Error> {
    use crate::error::send_ws_json;
    let fatal = !failure.retryable();
    send_ws_json(
        ws,
        &ServerMsg::Error {
            payload: connect_error_with_stage(msg, ProtoKind::Ssh, fatal),
        },
    )
    .await
}

/// SSH 建连失败 → 可重试性。
///
/// 认证失败（密码/私钥错误）属于凭据问题：重连仍用同一份凭据，必然被拒 →
/// 终止性。网络层失败（超时 / 拒绝 / DNS / TLS）可能因网络恢复而自愈 →
/// 可重试。判定复用 `crate::error::classify_connect_error` 的 `AuthFailed`。
///
/// 此处 `e` 应为 `rex_ssh` 建连抛出的 `anyhow::Error`，其 Display 仅显示
/// 最外层 context；`rex_ssh` 自身已把底层 russh 错误串在链上，所以传入
/// `e.to_string()` 可能丢失二级原因，`classify_ssh_connect_failure` 只能做
/// 启发式 —— 详见 `connect_direct` 的 `.context("SSH connection failed")`。
fn classify_ssh_connect_failure(err: impl std::fmt::Display) -> ConnFailure {
    match crate::error::classify_connect_error(&err.to_string()) {
        crate::error::ErrorCode::AuthFailed => ConnFailure::Fatal,
        _ => ConnFailure::Transient,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_error_frame_keeps_legacy_wire_shape() {
        // 无协议前缀时必须原样输出既有裸码，避免改动前端判定。
        let fatal = connect_error_with_stage("resource not found: r1", ProtoKind::Ssh, true);
        assert_eq!(fatal.code, "RESOURCE_NOT_FOUND");
        assert_eq!(fatal.message, "resource not found: r1");
        assert!(!fatal.retryable);

        let transient =
            connect_error_with_stage("db error: connection refused", ProtoKind::Ssh, false);
        assert_eq!(transient.code, "CONNECTION_REFUSED");
        assert!(transient.retryable);
    }

    #[test]
    fn classify_ssh_auth_failure_is_fatal() {
        // russh 聚合认证失败文案 → 终止性。
        assert_eq!(
            classify_ssh_connect_failure(
                "SSH authentication failed (password: partial_success=false)"
            ),
            ConnFailure::Fatal
        );
    }

    #[test]
    fn classify_ssh_transport_failure_is_transient() {
        assert_eq!(
            classify_ssh_connect_failure(
                "SSH connection failed: Connection refused (os error 111)"
            ),
            ConnFailure::Transient
        );
        assert_eq!(
            classify_ssh_connect_failure("SSH connection failed: DNS resolution failed"),
            ConnFailure::Transient
        );
        assert_eq!(
            classify_ssh_connect_failure("SSH connection failed: TLS handshake failed"),
            ConnFailure::Transient
        );
    }

    #[test]
    fn conn_error_default_is_transient_for_db_errors() {
        let e = ConnError::transient("db error: connection refused");
        assert!(e.failure.retryable());
        assert_eq!(
            crate::error::classify_connect_error(&e.message),
            crate::error::ErrorCode::ConnectionRefused
        );
    }

    #[test]
    fn decrypt_failure_is_fatal_and_maps_to_decrypt_code() {
        // 文案 → 终止性码必须真的走 `fatal_error_code` 的解密分支：
        // 旧断言只查 `ConnError::fatal(..)` 的 `retryable`，而该构造函数无条件
        // 设 Fatal，与文案无关（恒真）。这里按生产路径合成 wire 码，
        // 文案一旦不含 "decryption failed" 就会退化到其它分支而红。
        let message = format!("{} (aead::Error)", crate::error::CREDENTIAL_DECRYPT_MSG);
        let e = ConnError::fatal(message);
        let fatal = !e.failure.retryable();
        let payload = connect_error_with_stage(&e.message, ProtoKind::Ssh, fatal);

        assert!(fatal);
        assert_eq!(payload.code, "SSH_CONFIG_DECRYPT_FAILED");
        assert_eq!(payload.message, e.message);
        assert!(!payload.retryable);
    }
}

/// 创建服务端 keepalive ping 定时器（每 25 秒发送一次 ping）
fn create_server_ping_interval() -> Interval {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(25));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    interval
}
