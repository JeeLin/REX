//! WebSocket SIP 桥接 — 浏览器 ↔ Hub UA₁（baresip 进程内 FFI）或 Agent UA₂ 链式。
//!
//! 统一入口：/ws/sip?token=jwt&resourceId=xxx
//! Hub 从 DB 读取 SIP 资源连接信息：
//! - 直连资源：Hub 本地构造 UA₁（SipUa），经本 WebSocket 与前端交换控制/事件；
//! - agent 资源：Hub 不跑 UA，仅做中继——把前端控制帧封成 `SipControl` JSON 经
//!   隧道 binary 帧 `[4B channelId][json]` 发给 Agent 的 UA₂；Agent UA₂ 的 `SipEvent`
//!   经隧道回 Hub 转前端（`handle_agent_sip`）。Agent（UA₂）是最终 SIP 终端。
//!
//! 前端完全不感知底层 UA 是 Hub 本地还是 Agent 链式。

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use rex_sip::{CallState, SipEvent, SipUa, SipUaTrait};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use tokio::time::Interval;

use crate::app::AppState;
use crate::resource_conn::{load_resource_config, load_sip_conn};

/// 统一出站帧：事件为 Text（JSON），媒体为 Binary（原始 S16LE PCM）。
enum Outbound {
    Text(String),
    Binary(Vec<u8>),
}

/// 前端 → 后端的控制消息
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMsg {
    #[serde(rename = "sip.dial")]
    Dial { destination: String },
    #[serde(rename = "sip.answer")]
    Answer {
        #[serde(rename = "callId")]
        call_id: String,
    },
    #[serde(rename = "sip.hangup")]
    Hangup {
        #[serde(rename = "callId")]
        call_id: String,
    },
    #[serde(rename = "sip.hold")]
    Hold {
        #[serde(rename = "callId")]
        call_id: String,
    },
    #[serde(rename = "sip.unhold")]
    Unhold {
        #[serde(rename = "callId")]
        call_id: String,
    },
    #[serde(rename = "sip.dtmf")]
    Dtmf {
        #[serde(rename = "callId")]
        call_id: String,
        digit: char,
    },
    /// 客户端心跳（每 30 秒发送），后端忽略即可维持连接活跃
    #[serde(rename = "ping")]
    Ping,
}

/// 后端 → 前端的事件消息
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMsg {
    #[serde(rename = "sip.registered")]
    Registered,
    #[serde(rename = "sip.registration_failed")]
    RegistrationFailed { payload: ReasonPayload },
    #[serde(rename = "sip.incoming")]
    Incoming { payload: IncomingPayload },
    #[serde(rename = "sip.call_state")]
    CallState { payload: CallStatePayload },
    #[serde(rename = "sip.sip_message")]
    SipMessage { payload: RawPayload },
    #[serde(rename = "sip.quality")]
    Quality { payload: QualityPayload },
    #[serde(rename = "sip.error")]
    Error { payload: ReasonPayload },
    /// 服务端心跳（前端忽略）
    #[serde(rename = "sip.ping")]
    KeepAlive,
}

#[derive(Debug, Serialize)]
struct ReasonPayload {
    reason: String,
}

#[derive(Debug, Serialize)]
struct IncomingPayload {
    #[serde(rename = "callId")]
    call_id: String,
    from: String,
}

#[derive(Debug, Serialize)]
struct CallStatePayload {
    #[serde(rename = "callId")]
    call_id: String,
    state: CallState,
}

#[derive(Debug, Serialize)]
struct RawPayload {
    raw: String,
}

#[derive(Debug, Serialize)]
struct QualityPayload {
    /// 丢帧率 0..1。
    loss: f32,
    /// 抖动（ms）。
    jitter: f32,
    /// 端到端延迟代理（ms）。
    rtt: f32,
}

/// URL 查询参数
#[derive(Deserialize)]
pub struct SipQuery {
    #[serde(rename = "resourceId")]
    pub resource_id: String,
}

/// GET /ws/sip?token=jwt&resourceId=xxx
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<SipQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, query.resource_id))
}

/// WebSocket 连接处理主循环（直连 UA₁ / Agent UA₂ 链式）
async fn handle_socket(mut ws: WebSocket, state: AppState, resource_id: String) {
    // 从 DB 读取并解密 SIP 资源配置
    let sip_cfg = match load_resource_config(&state, &resource_id)
        .map_err(|e| e.to_string())
        .and_then(|info| load_sip_conn(&info).map_err(|e| e.to_string()))
    {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::warn!(action = "SIP_LOAD", resource_id = %resource_id, error = %e, "SIP resource load failed");
            let _ = send_ws_error(&mut ws, &e).await;
            return;
        }
    };

    // agent 资源：Hub 不跑 UA，仅做 JSON 控制/事件中继，转发到 Agent 的 UA₂。
    // `find_online_agent` 在 connection_mode != "agent" 时即返回 None，故无需
    // 再用 `is_agent_resource` 单独判定（两者查的是同一份 resource/env 数据）。
    if let Some(agent_id) = find_online_agent(&state, &resource_id).await {
        handle_agent_sip(ws, &state, &resource_id, &sip_cfg, &agent_id).await;
        return;
    }

    tracing::info!(
        action = "SIP_CONNECT",
        resource_id = %resource_id,
        server = %sip_cfg.server,
        transport = %sip_cfg.transport.as_str(),
        "SIP UA connection initiated"
    );

    let ua = match SipUa::real(sip_cfg).await {
        Ok(ua) => Arc::new(ua),
        Err(e) => {
            let _ = send_ws_error(&mut ws, &format!("SIP UA init failed: {e}")).await;
            return;
        }
    };

    handle_sip_session(ws, ua, &resource_id, &state).await;

    tracing::info!(action = "SIP_DISCONNECT", resource_id = %resource_id, "SIP session ended");
}

/// Agent 链式 SIP：Hub 不跑 UA，仅做前端 ↔ Agent UA₂ 的 JSON 中继。
///
/// 流程：
/// 1. 解析在线 Agent（取资源所属环境的 online agent）；
/// 2. 经既有 `/ws/agent` 隧道发 `connect`（protocol="sip"，config=解密后的 SIP 配置），
///    Agent 侧起真实 UA₂（最终 SIP 终端）；
/// 3. 把前端控制帧封成 `SipControl` JSON，加 `[4B channelId]` 前缀经隧道发给 Agent；
/// 4. Agent 回传的 `SipEvent` 二进制帧解包后经 `map_event` 转前端 `ServerMsg`。
async fn handle_agent_sip(
    mut ws: WebSocket,
    state: &AppState,
    resource_id: &str,
    cfg: &rex_sip::SipConfig,
    agent_id: &str,
) {
    let agent_conn = {
        let conns = state.agent_tunnel.connections.read().await;
        conns.get(agent_id).cloned()
    };
    let agent_conn = match agent_conn {
        Some(c) => c,
        None => {
            let _ = send_ws_error(&mut ws, "agent not connected").await;
            return;
        }
    };

    // 发 connect 到 Agent（protocol=sip）。
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
            "protocol": "sip",
            "config": {
                "server": cfg.server,
                "port": cfg.port,
                "username": cfg.username,
                "password": cfg.password,
                "displayName": cfg.display_name,
                "transport": cfg.transport.as_str(),
            }
        }
    });
    if agent_conn
        .sender
        .send(crate::agent_ws::AgentEvent::Text(connect_msg.to_string()))
        .await
        .is_err()
    {
        let _ = send_ws_error(&mut ws, "failed to contact agent").await;
        return;
    }

    // 等待 Agent 回 channel_id（UA₂ 已就绪）。
    let channel_id = match tokio::time::timeout(std::time::Duration::from_secs(10), resp_rx).await {
        Ok(Ok(crate::agent_ws::ConnectResponse {
            channel_id: Some(id),
            ..
        })) => id,
        Ok(Ok(crate::agent_ws::ConnectResponse { error: Some(e), .. })) => {
            let _ = send_ws_error(&mut ws, &e).await;
            return;
        }
        Ok(Ok(_)) => {
            let _ = send_ws_error(&mut ws, "agent returned unexpected response").await;
            return;
        }
        Ok(Err(_)) => {
            let _ = send_ws_error(&mut ws, "agent connection failed (channel closed)").await;
            return;
        }
        Err(_) => {
            let _ = send_ws_error(&mut ws, "agent connection timeout").await;
            return;
        }
    };

    tracing::info!(action = "SIP_AGENT_CONNECTED", agent_id = %agent_id, channel_id = %channel_id, resource_id = %resource_id, "agent SIP UA2 tunnel established");

    // 通知前端连接成功（直发，未进统一出站通道）。
    let (mut ws_sink, mut ws_stream) = ws.split();
    if ws_sink
        .send(Message::Text(
            serde_json::to_string(&ServerMsg::Registered)
                .unwrap()
                .into(),
        ))
        .await
        .is_err()
    {
        return;
    }
    // 统一出站通道：Agent 回传的信令/媒体帧经此发往浏览器（单一 writer 独占 ws_sink）。
    let (out_tx, mut out_rx) = mpsc::channel::<Outbound>(128);
    // 注意：Registered 帧仅表示隧道就绪，真实注册结果由 Agent UA₂ 经 SipEvent 回推。

    // 注册 tunnel data channel（接收 Agent 回传的 SipEvent 二进制帧）。
    let (data_tx, mut data_rx) = mpsc::channel::<Vec<u8>>(512);
    {
        let mut tunnel_data = state.agent_tunnel.tunnel_data.write().await;
        tunnel_data.insert(channel_id.clone(), data_tx);
    }
    let ch_id_num = channel_id.parse::<u32>().unwrap_or(0);

    // 前端控制帧 → 隧道 SipControl 帧。
    let agent_for_send = agent_conn.clone();
    let channel_id_clone = channel_id.clone();
    let frontend_to_agent = tokio::spawn(async move {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) {
                        if let Some(ctrl) = map_control(client_msg) {
                            if let Ok(payload) = serde_json::to_vec(&ctrl) {
                                let mut frame = Vec::with_capacity(4 + payload.len());
                                frame.extend_from_slice(&ch_id_num.to_be_bytes());
                                frame.extend_from_slice(&payload);
                                let _ = agent_for_send
                                    .sender
                                    .send(crate::agent_ws::AgentEvent::Bytes(frame))
                                    .await;
                            }
                        }
                    }
                }
                // 浏览器上行二进制帧（音频 PCM / 视频像素）→ 隧道媒体帧发给 Agent UA₂。
                // 首字节 kind 在浏览器侧已按音频/视频区分，原样透传 kind（1=音频 / 2=视频）。
                Ok(Message::Binary(bytes)) => {
                    let (kind, payload) = crate::sip_media::unwrap_tunnel_frame(&bytes);
                    let media_kind = if kind == crate::sip_media::KIND_VIDEO {
                        crate::sip_media::KIND_VIDEO
                    } else {
                        crate::sip_media::KIND_MEDIA
                    };
                    let media = crate::sip_media::wrap_tunnel_frame(media_kind, payload);
                    let mut frame = Vec::with_capacity(4 + media.len());
                    frame.extend_from_slice(&ch_id_num.to_be_bytes());
                    frame.extend_from_slice(&media);
                    let _ = agent_for_send
                        .sender
                        .send(crate::agent_ws::AgentEvent::Bytes(frame))
                        .await;
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
        // 前端断开后通知 Agent 关闭 channel。
        let _ = agent_for_send
            .sender
            .send(crate::agent_ws::AgentEvent::Text(
                serde_json::to_string(&serde_json::json!({
                    "type": "close",
                    "payload": { "channelId": channel_id_clone }
                }))
                .unwrap(),
            ))
            .await;
    });

    // Agent 回传帧 → 前端（经统一出站通道）。隧道 payload 首字节为 kind：
    // kind=0 信令（SipEvent JSON），kind=1 媒体（PCM 二进制帧）。
    let event_channel_id = channel_id.clone();
    let out_tx_for_event = out_tx.clone();
    let agent_cdr_db = state.db.clone();
    let agent_cdr_rid = resource_id.to_string();
    let agent_capture = state.sip_capture.clone();
    let agent_cdr_db_recording = state.sip_recording.clone();
    let event_tx = tokio::spawn(async move {
        let mut trackers: std::collections::HashMap<String, CdrTracker> = Default::default();
        while let Some(data) = data_rx.recv().await {
            // `data` 来自 `tunnel_data`：Agent 经 `/ws/agent` 隧道发来的帧已由
            // `agent_ws.rs` 读取循环剥去 4 字节 channelId 前缀，此处 payload = `[1B kind][rest]`。
            let (kind, rest) = crate::sip_media::unwrap_tunnel_frame(&data);
            match kind {
                crate::sip_media::KIND_MEDIA => {
                    // 媒体帧（Agent UA₂ 抽出的下行 PCM）→ 原样推浏览器播放。
                    // 录音（子任务 #2）：下行 PCM 同时写入录音缓冲（按当前激活通话分文件落盘）。
                    crate::sip_recording::append_tunnel_media(&agent_cdr_db_recording, rest);
                    if out_tx_for_event
                        .send(Outbound::Binary(rest.to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                crate::sip_media::KIND_VIDEO => {
                    // 视频帧（Agent UA₂ 抽出的下行像素）→ 原样推浏览器渲染（子任务 #1）。
                    if out_tx_for_event
                        .send(Outbound::Binary(rest.to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                _ => {
                    // 信令帧：SipEvent JSON → 前端 ServerMsg（兼持久化 CDR，子任务 #4）。
                    let ev = match decode_tunnel_sip_event(rest) {
                        Some(e) => e,
                        None => {
                            tracing::warn!(action = "SIP_AGENT_EVENT_PARSE", channel_id = %event_channel_id, "invalid SipEvent frame");
                            continue;
                        }
                    };
                    // 抓包（子任务 #3）：Agent UA₂ 入站 SipEvent 经隧道回 Hub 后记录
                    // （UA₁ 真实 SIP 字节由 rex_sip::capture 的 baresip 钩子捕获，此处仅 UA₂）。
                    if agent_capture.is_active(&agent_cdr_rid) {
                        if let Ok(json) = serde_json::to_string(&ev) {
                            agent_capture.record_ua2(&agent_cdr_rid, "ua2_in", &json);
                        }
                    }
                    match &ev {
                        rex_sip::SipEvent::IncomingCall { call_id, from } => {
                            trackers.insert(
                                call_id.clone(),
                                CdrTracker::new(
                                    &agent_cdr_rid,
                                    call_id,
                                    from,
                                    "in",
                                    agent_cdr_db_recording.clone(),
                                ),
                            );
                        }
                        rex_sip::SipEvent::CallState { call_id, state } => {
                            if let Some(tr) = trackers.get_mut(call_id) {
                                tr.apply(*state, &agent_cdr_db);
                            }
                        }
                        _ => {}
                    }
                    if let Some(msg) = map_event(ev) {
                        if let Ok(s) = serde_json::to_string(&msg) {
                            if out_tx_for_event.send(Outbound::Text(s)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        }
    });

    // 单一 writer：消费统一出站通道，独占 ws_sink。
    let writer = tokio::spawn(async move {
        while let Some(out) = out_rx.recv().await {
            let msg = match out {
                Outbound::Text(s) => Message::Text(s.into()),
                Outbound::Binary(b) => Message::Binary(b.into()),
            };
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 服务端 keepalive ping（每 25 秒）。
    let mut ping_interval = create_server_ping_interval();
    let agent_ping = agent_conn.clone();
    let ping_task = tokio::spawn(async move {
        loop {
            ping_interval.tick().await;
            if agent_ping
                .sender
                .send(crate::agent_ws::AgentEvent::Text(
                    serde_json::to_string(&serde_json::json!({"type": "ping", "payload": {}}))
                        .unwrap(),
                ))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    tokio::select! {
        _ = frontend_to_agent => {},
        _ = event_tx => {},
        _ = writer => {},
        _ = ping_task => {},
    }

    // 清理 tunnel。
    {
        let mut tunnel_data = state.agent_tunnel.tunnel_data.write().await;
        tunnel_data.remove(&channel_id);
    }
    {
        let mut channels = state.agent_tunnel.channels.write().await;
        channels.remove(&channel_id);
    }

    tracing::info!(action = "SIP_AGENT_DISCONNECT", channel_id = %channel_id, resource_id = %resource_id, "agent SIP UA2 session ended");
}

/// 将前端 [`ClientMsg`] 控制帧映射为隧道 [`SipControl`]（前端↔Agent 线格式一致）。
fn map_control(cmd: ClientMsg) -> Option<rex_sip::SipControl> {
    match cmd {
        ClientMsg::Dial { destination } => Some(rex_sip::SipControl::Dial { destination }),
        ClientMsg::Answer { call_id } => Some(rex_sip::SipControl::Answer { call_id }),
        ClientMsg::Hangup { call_id } => Some(rex_sip::SipControl::Hangup { call_id }),
        ClientMsg::Hold { call_id } => Some(rex_sip::SipControl::Hold { call_id }),
        ClientMsg::Unhold { call_id } => Some(rex_sip::SipControl::Unhold { call_id }),
        ClientMsg::Dtmf { call_id, digit } => Some(rex_sip::SipControl::Dtmf { call_id, digit }),
        // 前端心跳不映射为任何 UA 控制。
        ClientMsg::Ping => None,
    }
}

/// 解析资源所属环境的在线 Agent（取 status == "online" 的 agent）。
async fn find_online_agent(state: &AppState, resource_id: &str) -> Option<String> {
    let db = state.db.clone();
    let rid = resource_id.to_string();
    tokio::task::spawn_blocking(move || {
        let resource = db.get_resource(&rid).ok().flatten()?;
        let env = db
            .get_environment(&resource.environment_id)
            .ok()
            .flatten()?;
        if env.connection_mode != "agent" {
            return None;
        }
        let agents = db
            .list_agents_by_env(&resource.environment_id)
            .unwrap_or_default();
        agents
            .iter()
            .find(|a| a.status == "online")
            .map(|a| a.id.clone())
    })
    .await
    .ok()
    .flatten()
}

/// 真正的信令会话：注册 + 事件回推 + 控制指令下发 + 实时音频媒体通道（M82b）
async fn handle_sip_session(
    mut ws: WebSocket,
    ua: Arc<SipUa>,
    resource_id: &str,
    state: &AppState,
) {
    // 注册 UA
    if let Err(e) = ua.register().await {
        let _ = send_ws_error(&mut ws, &format!("SIP register failed: {e}")).await;
        return;
    }

    // 事件回推任务：baresip 事件流 → 前端帧 + CDR 持久化（子任务 #4）。
    let mut events = ua.events();
    let (event_tx, mut event_rx) = mpsc::channel::<ServerMsg>(64);
    let db_for_cdr = state.db.clone();
    let rid = resource_id.to_string();
    let rec_for_cdr = state.sip_recording.clone();
    let event_pump = tokio::spawn(async move {
        let db = db_for_cdr;
        let mut trackers: std::collections::HashMap<String, CdrTracker> = Default::default();
        while let Some(ev) = events.recv().await {
            // UA₁ 真实 SIP 信令字节已由 rex_sip::capture 的 baresip 钩子全局捕获（子任务 #3），
            // 此处不再做中继层 JSON 记录。
            match &ev {
                SipEvent::IncomingCall { call_id, from } => {
                    trackers.insert(
                        call_id.clone(),
                        CdrTracker::new(&rid, call_id, from, "in", rec_for_cdr.clone()),
                    );
                }
                SipEvent::CallState { call_id, state } => {
                    if let Some(tr) = trackers.get_mut(call_id) {
                        tr.apply(*state, &db);
                    }
                }
                _ => {}
            }
            if let Some(msg) = map_event(ev) {
                if event_tx.send(msg).await.is_err() {
                    break;
                }
            }
        }
    });

    let (mut ws_sink, mut ws_stream) = ws.split();

    // 统一出站通道：事件（Text）与媒体（Binary）都经此发往浏览器，由单一 writer 任务
    // 独占 ws_sink（SplitSink 不 Clone，故不能多任务各持一份）。
    let (out_tx, mut out_rx) = mpsc::channel::<Outbound>(128);

    // --- 实时音频媒体通道（M82b）：baresip 抽出的 RX PCM → on_rtp 回调 → PCM 帧 →
    //     经 out_tx 交 writer 以 `Message::Binary` 推浏览器。回调在 baresip 泵线程内同步
    //     调用，发送失败（浏览器已断）即丢弃该帧。
    let out_tx_for_rtp = out_tx.clone();
    // 录音（子任务 #2）：下行 PCM 同时写入录音缓冲（按当前激活通话分文件落盘）。
    let rec_for_rtp = state.sip_recording.clone();
    ua.on_rtp(Box::new(move |pcm: &[i16]| {
        let frame = crate::sip_media::encode_pcm_frame(pcm);
        // 录音在本次借用 frame 后再 move 进 out_tx，避免 use-after-move。
        crate::sip_recording::append_tunnel_media(&rec_for_rtp, &frame);
        let _ = out_tx_for_rtp.try_send(Outbound::Binary(frame));
    }));

    // 浏览器实时视频（0.70.2 子任务 #1）：下行对端像素帧 → 视频隧道帧（KIND_VIDEO）→ 浏览器。
    let out_tx_for_video = out_tx.clone();
    ua.on_video(Box::new(move |vf: &rex_sip::video_bridge::VideoFrame| {
        if let Ok(bytes) = crate::sip_media::encode_video_frame(
            crate::sip_media::VideoPixFmt::Rgba,
            vf.width,
            vf.height,
            &vf.rgba,
        ) {
            let frame = crate::sip_media::wrap_tunnel_frame(crate::sip_media::KIND_VIDEO, &bytes);
            let _ = out_tx_for_video.try_send(Outbound::Binary(frame));
        }
    }));

    // 前端 → UA 控制指令任务（兼收媒体二进制帧）
    let ua_for_cmd = ua.clone();
    let out_tx_for_cmd = out_tx.clone();
    let cmd_task = tokio::spawn(async move {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let cmd = match serde_json::from_str::<ClientMsg>(&text) {
                        Ok(c) => c,
                        Err(e) => {
                            tracing::debug!(action = "SIP_BAD_MSG", error = %e, "invalid client msg");
                            continue;
                        }
                    };
                    // UA₁ 真实 SIP 信令字节(含拨号/应答等触发的 request)已由 rex_sip::capture 的
                    // baresip 钩子全局捕获（子任务 #3），此处不再做中继层 JSON 记录。
                    if let Err(e) = dispatch_cmd(ua_for_cmd.as_ref(), cmd).await {
                        if out_tx_for_cmd
                            .send(Outbound::Text(
                                serde_json::to_string(&ServerMsg::Error {
                                    payload: ReasonPayload { reason: e },
                                })
                                .unwrap_or_default(),
                            ))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                // 浏览器上行二进制帧：首字节 kind 区分音频（PCM）/视频（像素）。
                Ok(Message::Binary(bytes)) => {
                    let (kind, payload) = crate::sip_media::unwrap_tunnel_frame(&bytes);
                    if kind == crate::sip_media::KIND_VIDEO {
                        // 视频上行：解码像素帧 → 喂回 baresip 发送链路（0.70.2 子任务 #1）。
                        if let Ok((_, w, h, rgba)) = crate::sip_media::decode_video_frame(payload) {
                            let frame = rex_sip::video_bridge::VideoFrame {
                                width: w,
                                height: h,
                                rgba,
                            };
                            if let Err(e) = ua_for_cmd.send_video(frame).await {
                                tracing::debug!(action = "SIP_VIDEO_UP", error = %e, "send_video failed");
                            }
                        }
                    } else {
                        // 音频上行：解码 S16LE PCM → 喂回 baresip 发送链路（M82b）。
                        let pcm = crate::sip_media::decode_media_frame(payload);
                        if !pcm.is_empty() {
                            if let Err(e) = ua_for_cmd.send_audio(pcm).await {
                                tracing::debug!(action = "SIP_MEDIA_UP", error = %e, "send_audio failed");
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    });

    // 事件 → 前端帧任务（经统一出站通道）
    let out_tx_for_event = out_tx.clone();
    let event_relay = tokio::spawn(async move {
        while let Some(msg) = event_rx.recv().await {
            let frame = match serde_json::to_string(&msg) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if out_tx_for_event.send(Outbound::Text(frame)).await.is_err() {
                break;
            }
        }
    });

    // 单一 writer：消费统一出站通道，独占 ws_sink。
    let writer = tokio::spawn(async move {
        while let Some(out) = out_rx.recv().await {
            let msg = match out {
                Outbound::Text(s) => Message::Text(s.into()),
                Outbound::Binary(b) => Message::Binary(b.into()),
            };
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 服务端 keepalive ping（每 25 秒，防止中间件/代理超时断开）；前端忽略该帧
    let mut ping_interval = create_server_ping_interval();
    let out_tx_for_ping = out_tx.clone();
    let ping_task = tokio::spawn(async move {
        loop {
            ping_interval.tick().await;
            if out_tx_for_ping
                .send(Outbound::Text(
                    serde_json::to_string(&ServerMsg::KeepAlive).unwrap_or_default(),
                ))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // 实时质量指标采样（子任务 #5）：每秒从 UA 取一次质量快照，经 `sip.quality` 事件推浏览器。
    let ua_for_quality = ua.clone();
    let out_tx_for_quality = out_tx.clone();
    let quality_task = tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tick.tick().await;
            let q = ua_for_quality.quality();
            if out_tx_for_quality
                .send(Outbound::Text(
                    serde_json::to_string(&ServerMsg::Quality {
                        payload: QualityPayload {
                            loss: q.loss,
                            jitter: q.jitter,
                            rtt: q.rtt,
                        },
                    })
                    .unwrap_or_default(),
                ))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // 任一任务结束即结束会话（其余任务在 JoinHandle drop 时自动取消）
    tokio::select! {
        _ = event_pump => {},
        _ = event_relay => {},
        _ = cmd_task => {},
        _ = writer => {},
        _ = ping_task => {},
        _ = quality_task => {},
    }

    tracing::debug!(action = "SIP_SESSION_END", resource_id, "sip session ended");
}

/// CDR 持久化挂钩：把通话状态变更写入 SQLite（子任务 #4）。
///
/// 简单状态机：拨号/来电 → 创建 CDR（ringing）；active → 标记接通并开始计时；
/// ended/missed → 落止时间与时长。所有写入走 spawn_blocking 避免阻塞事件泵。
/// CDR id 以 call_id 为基础，保证同通电话多次状态变更幂等 upsert。
struct CdrTracker {
    resource_id: String,
    call_id: String,
    peer: String,
    direction: String,
    created_at: Option<String>,
    active_at: Option<String>,
    /// 录音落盘后的 CDR 主键（`cdr:{call_id}`），用于落盘文件名与 URL 回填。
    cdr_id: String,
    /// 录音注册表（子任务 #2）：通话结束时把下行缓冲落盘并回填 `recording_url`。
    recording: std::sync::Arc<crate::sip_recording::SipRecordingRegistry>,
}

impl CdrTracker {
    fn new(
        resource_id: &str,
        call_id: &str,
        peer: &str,
        direction: &str,
        recording: std::sync::Arc<crate::sip_recording::SipRecordingRegistry>,
    ) -> Self {
        CdrTracker {
            resource_id: resource_id.to_string(),
            call_id: call_id.to_string(),
            peer: peer.to_string(),
            direction: direction.to_string(),
            created_at: None,
            active_at: None,
            cdr_id: format!("cdr:{call_id}"),
            recording,
        }
    }

    fn id(&self) -> String {
        format!("cdr:{}", self.call_id)
    }

    fn now_iso() -> String {
        // 用 chrono 生成 RFC3339；若不可用退回空串（DB 列有默认值）。
        use chrono::Utc;
        Utc::now().to_rfc3339()
    }

    fn apply(&mut self, state: CallState, db: &crate::db::Database) {
        match state {
            CallState::Ringing => {
                if self.created_at.is_none() {
                    self.created_at = Some(Self::now_iso());
                }
            }
            CallState::Active => {
                if self.active_at.is_none() {
                    self.active_at = Some(Self::now_iso());
                }
                // 录音（子任务 #2）：通话接通即开始承接下行媒体帧到本 call 缓冲。
                self.recording.begin_call(&self.call_id);
            }
            CallState::Held => {}
            CallState::Ended => {}
        }
        let start = self.created_at.clone().unwrap_or_else(Self::now_iso);
        let (end, duration) = match state {
            CallState::Ended | CallState::Held => {
                let end = Self::now_iso();
                let dur = self.active_at.as_ref().map(|a| {
                    let t0 = chrono::DateTime::parse_from_rfc3339(a)
                        .map(|d| d.timestamp())
                        .unwrap_or(0);
                    let t1 = chrono::DateTime::parse_from_rfc3339(&end)
                        .map(|d| d.timestamp())
                        .unwrap_or(0);
                    (t1 - t0).max(0)
                });
                (Some(end), dur.unwrap_or(0))
            }
            _ => (None, 0),
        };
        // 录音（子任务 #2）：通话结束把下行缓冲落盘，回填 `recording_url`。
        let recording_url = if matches!(state, CallState::Ended | CallState::Held) {
            self.recording
                .finalize_call(&self.cdr_id)
                .unwrap_or_default()
        } else {
            String::new()
        };
        let cdr = crate::models::NewCdr {
            id: self.id(),
            resource_id: self.resource_id.clone(),
            peer: self.peer.clone(),
            call_id: self.call_id.clone(),
            start_time: start,
            end_time: end,
            duration_sec: duration,
            direction: self.direction.clone(),
            state: match state {
                CallState::Ringing => "ringing",
                CallState::Active => "active",
                CallState::Held => "held",
                CallState::Ended => "ended",
            }
            .to_string(),
            recording_url,
            pcap_url: String::new(),
        };
        let _ = db.upsert_cdr(&cdr);
    }
}

/// 将 [`SipEvent`] 映射为前端 [`ServerMsg`]。返回 `None` 表示忽略该事件。
fn map_event(ev: SipEvent) -> Option<ServerMsg> {
    match ev {
        SipEvent::Registered => Some(ServerMsg::Registered),
        SipEvent::RegistrationFailed { reason } => Some(ServerMsg::RegistrationFailed {
            payload: ReasonPayload { reason },
        }),
        SipEvent::IncomingCall { call_id, from } => Some(ServerMsg::Incoming {
            payload: IncomingPayload { call_id, from },
        }),
        SipEvent::CallState { call_id, state } => Some(ServerMsg::CallState {
            payload: CallStatePayload { call_id, state },
        }),
        SipEvent::Message { raw } => Some(ServerMsg::SipMessage {
            payload: RawPayload { raw },
        }),
    }
}

/// 解析经隧道回 Hub 的 SipEvent 帧。
///
/// `data` 来自 `tunnel_data`：Agent 的 UA₂ 经 `send_sip_event` 所发 `[4B channelId][SipEvent JSON]`
/// 二进制帧，已由 `agent_ws.rs` 的 `/ws/agent` 读取循环剥去 4 字节 channelId 前缀，此处已是纯
/// `SipEvent` JSON，直接反序列化即可（不要再跳过 4 字节 —— 见回归 #7）。
fn decode_tunnel_sip_event(data: &[u8]) -> Option<rex_sip::SipEvent> {
    serde_json::from_slice(data).ok()
}

/// 将前端控制消息分发到 UA。返回 `Err` 表示需要向用户报告的错误。
async fn dispatch_cmd<U: SipUaTrait + ?Sized>(ua: &U, cmd: ClientMsg) -> Result<(), String> {
    match cmd {
        ClientMsg::Dial { destination } => ua
            .dial(&destination)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string()),
        ClientMsg::Answer { call_id } => ua.answer(&call_id).await.map_err(|e| e.to_string()),
        ClientMsg::Hangup { call_id } => ua.hangup(&call_id).await.map_err(|e| e.to_string()),
        ClientMsg::Hold { call_id } => ua.hold(&call_id).await.map_err(|e| e.to_string()),
        ClientMsg::Unhold { call_id } => ua.unhold(&call_id).await.map_err(|e| e.to_string()),
        ClientMsg::Dtmf { call_id, digit } => {
            ua.dtmf(&call_id, digit).await.map_err(|e| e.to_string())
        }
        ClientMsg::Ping => Ok(()),
    }
}

async fn send_ws_error(ws: &mut WebSocket, msg: &str) -> Result<(), axum::Error> {
    ws.send(Message::Text(
        serde_json::to_string(&ServerMsg::Error {
            payload: ReasonPayload {
                reason: msg.to_string(),
            },
        })
        .unwrap_or_default()
        .into(),
    ))
    .await
}

/// 创建服务端 keepalive ping 定时器（每 25 秒触发一次）。
fn create_server_ping_interval() -> Interval {
    let mut interval = tokio::time::interval(Duration::from_secs(25));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    interval
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_sip::{MockAction, MockSipUa};

    fn cfg() -> rex_sip::SipConfig {
        rex_sip::SipConfig {
            server: "sip.example.com".into(),
            port: 5060,
            username: "1000".into(),
            password: Some("secret".into()),
            display_name: None,
            transport: rex_sip::SipTransport::Udp,
        }
    }

    #[test]
    fn map_event_incoming_to_server_msg() {
        let m = map_event(SipEvent::IncomingCall {
            call_id: "call-1".into(),
            from: "2000@example.com".into(),
        })
        .unwrap();
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("sip.incoming"));
        assert!(s.contains("call-1"));
        assert!(s.contains("2000@example.com"));
    }

    #[test]
    fn map_event_call_state_active() {
        let m = map_event(SipEvent::CallState {
            call_id: "call-1".into(),
            state: CallState::Active,
        })
        .unwrap();
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("sip.call_state"));
        assert!(s.contains("\"state\":\"active\""));
    }

    #[test]
    fn map_event_registered_and_failed() {
        assert!(matches!(
            map_event(SipEvent::Registered),
            Some(ServerMsg::Registered)
        ));
        let m = map_event(SipEvent::RegistrationFailed {
            reason: "bad".into(),
        })
        .unwrap();
        assert!(serde_json::to_string(&m)
            .unwrap()
            .contains("registration_failed"));
    }

    #[tokio::test]
    async fn dispatch_dial_triggers_ua() {
        let ua = MockSipUa::new(cfg(), vec![]);
        dispatch_cmd(
            &ua,
            ClientMsg::Dial {
                destination: "2000".into(),
            },
        )
        .await
        .unwrap();
        let acts = ua.actions.lock().unwrap();
        assert_eq!(acts[0], rex_sip::MockAction::Dial("2000".into()));
    }

    #[tokio::test]
    async fn dispatch_hold_hangup_dtmf() {
        let ua = MockSipUa::new(cfg(), vec![]);
        dispatch_cmd(
            &ua,
            ClientMsg::Hold {
                call_id: "c1".into(),
            },
        )
        .await
        .unwrap();
        dispatch_cmd(
            &ua,
            ClientMsg::Dtmf {
                call_id: "c1".into(),
                digit: '5',
            },
        )
        .await
        .unwrap();
        dispatch_cmd(
            &ua,
            ClientMsg::Hangup {
                call_id: "c1".into(),
            },
        )
        .await
        .unwrap();
        let acts = ua.actions.lock().unwrap();
        assert_eq!(acts[0], rex_sip::MockAction::Hold("c1".into()));
        assert_eq!(acts[1], rex_sip::MockAction::Dtmf("c1".into(), '5'));
        assert_eq!(acts[2], rex_sip::MockAction::Hangup("c1".into()));
    }

    #[test]
    fn map_control_dial_to_sip_control() {
        let ctrl = map_control(ClientMsg::Dial {
            destination: "2000".into(),
        })
        .unwrap();
        let s = serde_json::to_string(&ctrl).unwrap();
        assert!(s.contains("dial"));
        assert!(s.contains("2000"));
        // 隧道线格式：type=snake_case → "dial"。
        assert!(s.contains("\"type\":\"dial\""));
    }

    #[test]
    fn map_control_answer_hangup_hold_unhold_dtmf() {
        assert!(matches!(
            map_control(ClientMsg::Answer { call_id: "c1".into() }).unwrap(),
            rex_sip::SipControl::Answer { call_id } if call_id == "c1"
        ));
        assert!(matches!(
            map_control(ClientMsg::Hangup { call_id: "c1".into() }).unwrap(),
            rex_sip::SipControl::Hangup { call_id } if call_id == "c1"
        ));
        assert!(matches!(
            map_control(ClientMsg::Hold { call_id: "c1".into() }).unwrap(),
            rex_sip::SipControl::Hold { call_id } if call_id == "c1"
        ));
        assert!(matches!(
            map_control(ClientMsg::Unhold { call_id: "c1".into() }).unwrap(),
            rex_sip::SipControl::Unhold { call_id } if call_id == "c1"
        ));
        assert!(matches!(
            map_control(ClientMsg::Dtmf {
                call_id: "c1".into(),
                digit: '9'
            })
            .unwrap(),
            rex_sip::SipControl::Dtmf { call_id, digit } if call_id == "c1" && digit == '9'
        ));
    }

    #[test]
    fn map_control_ping_is_none() {
        assert!(map_control(ClientMsg::Ping).is_none());
    }

    #[test]
    fn map_event_registration_failed_reason_field() {
        // 验证 #4 修复后的 RegistrationFailed { reason } 序列化结构正确。
        let m = map_event(SipEvent::RegistrationFailed {
            reason: "auth failed".into(),
        })
        .unwrap();
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("registration_failed"));
        assert!(s.contains("auth failed"));
    }

    #[test]
    fn quality_event_serializes_metric_fields() {
        // 子任务 #5：sip.quality 事件携带 loss/jitter/rtt 字段，命名与前端解码对齐。
        let m = ServerMsg::Quality {
            payload: QualityPayload {
                loss: 0.25,
                jitter: 12.5,
                rtt: 80.0,
            },
        };
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("sip.quality"));
        assert!(s.contains("\"loss\":0.25"));
        assert!(s.contains("\"jitter\":12.5"));
        assert!(s.contains("\"rtt\":80"));
    }

    // --- 子任务 #7：前后端联调（契约锁定 + Agent 链路回归）---
    //
    // CI 无法托管真 SIP server，联调以「合约测试 + 手动验证」为准（见里程碑文档 #7）。
    // 下列测试锁定两条链路的关键契约：
    //   (a) Hub UA₁ 直连：SipEvent → 前端 ServerMsg 的 JSON 字段（callId camelCase / state snake_case）；
    //   (b) Agent UA₂ 链式：Agent 经隧道回 Hub 的 `[4B channelId][SipEvent JSON]` 二进制帧，在
    //       被 `/ws/agent` 读取循环剥去 4 字节前缀后，必须能被 `decode_tunnel_sip_event` 正确还原
    //       —— 这是 #7 回归 bug（之前错误地再跳过 4 字节导致 JSON 头部被截、事件全丢）的对锁。

    // 模拟 Agent 经隧道发回 Hub 的二进制帧：前 4 字节为 channelId（u32 大端）。
    fn agent_tunnel_frame(channel_id: u32, ev: &rex_sip::SipEvent) -> Vec<u8> {
        let mut frame = channel_id.to_be_bytes().to_vec();
        let json = serde_json::to_vec(ev).unwrap();
        frame.extend_from_slice(&json);
        frame
    }

    #[test]
    fn agent_tunnel_frame_round_trips_through_hub_decapsulation() {
        // Agent 端 send_sip_event 构造的帧：IncomingCall。
        let ev = SipEvent::IncomingCall {
            call_id: "call-1".into(),
            from: "2000@example.com".into(),
        };
        let frame = agent_tunnel_frame(2, &ev);

        // Hub `/ws/agent` 读取循环剥去前 4 字节 channelId（见 agent_ws.rs:347-361）。
        assert!(frame.len() >= 4);
        let decapsulated = frame[4..].to_vec();

        // Hub SIP 事件任务用 decode_tunnel_sip_event 反序列化（不要再跳过 4 字节）。
        let got = decode_tunnel_sip_event(&decapsulated).expect("decoded SipEvent");
        assert_eq!(
            got,
            SipEvent::IncomingCall {
                call_id: "call-1".into(),
                from: "2000@example.com".into()
            }
        );
    }

    #[test]
    fn agent_tunnel_frame_not_double_stripped() {
        // 回归 #7：旧代码 `serde_json::from_slice(&data[4..])` 在 decapsulated 上再跳 4 字节，
        // 截掉 `{"ty` 导致解析失败。这里断言新代码直接反序列化成功、旧代码必失败。
        let ev = SipEvent::CallState {
            call_id: "call-1".into(),
            state: CallState::Active,
        };
        let decapsulated = agent_tunnel_frame(7, &ev)[4..].to_vec();

        // 新代码：成功。
        assert!(decode_tunnel_sip_event(&decapsulated).is_some());

        // 旧代码（双重剥离）：必失败 —— 证明 bug 已被修复。
        let old_decode: Option<rex_sip::SipEvent> = serde_json::from_slice(&decapsulated[4..]).ok();
        assert!(
            old_decode.is_none(),
            "旧代码的双重剥离应失败，否则回归未真正修复"
        );
    }

    #[test]
    fn agent_event_maps_to_frontend_contract() {
        // Agent 链路的 SipEvent 经 tunnel 回 Hub 后，必须映射成前端 decodeEvent 期望的 JSON：
        // `sip.incoming` payload 含 camelCase `callId`；`sip.call_state` payload 含 snake_case state。
        let incoming = decode_tunnel_sip_event(
            &agent_tunnel_frame(
                3,
                &SipEvent::IncomingCall {
                    call_id: "call-9".into(),
                    from: "3000@example.com".into(),
                },
            )[4..],
        )
        .unwrap();
        let incoming_json = serde_json::to_string(&map_event(incoming).unwrap()).unwrap();
        assert!(incoming_json.contains("sip.incoming"));
        assert!(incoming_json.contains("\"callId\":\"call-9\""));
        assert!(incoming_json.contains("\"from\":\"3000@example.com\""));

        let call_state = decode_tunnel_sip_event(
            &agent_tunnel_frame(
                3,
                &SipEvent::CallState {
                    call_id: "call-9".into(),
                    state: CallState::Active,
                },
            )[4..],
        )
        .unwrap();
        let cs_json = serde_json::to_string(&map_event(call_state).unwrap()).unwrap();
        assert!(cs_json.contains("sip.call_state"));
        assert!(cs_json.contains("\"callId\":\"call-9\""));
        assert!(cs_json.contains("\"state\":\"active\""));
    }

    // --- 子任务 #2：媒体通道契约（M82b 下行/上行 PCM 帧经隧道 kind 封装）---
    //
    // 直连 UA₁：浏览器↔Hub 之间媒体帧即原始 S16LE PCM 二进制（无 kind 字节，方向隐含）。
    // Agent UA₂ 链式：Hub↔Agent 隧道上媒体帧带 1 字节 kind（KIND_MEDIA=1），与信令帧（=0）
    // 区分，再叠加 4 字节 channelId 前缀。下列测试对锁这两条媒体路径的帧契约。

    #[test]
    fn direct_media_pcm_binary_carries_samples_no_kind() {
        // 直连路径：前端解码后的上行 PCM 二进制帧不带 kind 字节，长度严格为样本数*2。
        let pcm: Vec<i16> = vec![-32000, 0, 1234, 7];
        let up = crate::sip_media::encode_pcm_frame(&pcm);
        assert_eq!(up.len(), pcm.len() * 2);
        // 下行（UA₁ 抽出的远端 PCM）原样推浏览器，解码后样本一致。
        let down = crate::sip_media::decode_media_frame(&up);
        assert_eq!(down, pcm);
    }

    #[test]
    fn agent_downlink_media_frame_preserves_pcm_through_tunnel() {
        // Agent UA₂ 抽出下行 PCM → 经隧道发 `[4B channelId][1B kind=1][pcm]` 给 Hub。
        // Hub 读取循环剥去 4 字节 channelId，剩余 `[1B kind][pcm]`，由 event_tx 按 kind 分发。
        let ch_id: u32 = 11;
        let pcm: Vec<i16> = vec![-100, 256, -512, 1];
        let pcm_bytes = crate::sip_media::encode_pcm_frame(&pcm);
        // Agent 侧封装：kind + pcm。
        let tunnel_inner =
            crate::sip_media::wrap_tunnel_frame(crate::sip_media::KIND_MEDIA, &pcm_bytes);
        // 叠加 channelId 前缀（agent_ws.rs 读取循环前会剥掉）。
        let mut frame = ch_id.to_be_bytes().to_vec();
        frame.extend_from_slice(&tunnel_inner);

        // Hub 侧：剥 channelId。
        let decapped = frame[4..].to_vec();
        let (kind, rest) = crate::sip_media::unwrap_tunnel_frame(&decapped);
        assert_eq!(kind, crate::sip_media::KIND_MEDIA);
        // 媒体帧原样转 Binary 推浏览器 → 浏览器解码得到原始 PCM。
        let got = crate::sip_media::decode_media_frame(rest);
        assert_eq!(got, pcm);
    }

    #[test]
    fn agent_uplink_browser_pcm_wraps_to_media_tunnel_frame() {
        // 浏览器上行麦克风 PCM（Binary 帧）→ Hub frontend_to_agent 包裹为 tunnel 媒体帧
        // `[4B channelId][1B kind=1][pcm]` 发给 Agent UA₂。
        let ch_id: u32 = 5;
        let pcm: Vec<i16> = vec![42, -42, 8000, -8000];
        let up = crate::sip_media::encode_pcm_frame(&pcm);
        // frontend_to_agent 逻辑：wrap kind + 加 ch_id 前缀。
        let media = crate::sip_media::wrap_tunnel_frame(crate::sip_media::KIND_MEDIA, &up);
        let mut frame = ch_id.to_be_bytes().to_vec();
        frame.extend_from_slice(&media);
        // Agent 侧剥 channelId 后，按 kind 识别为媒体帧 → decode 喂回 UA₂ 发送链路。
        let decapped = frame[4..].to_vec();
        let (kind, rest) = crate::sip_media::unwrap_tunnel_frame(&decapped);
        assert_eq!(kind, crate::sip_media::KIND_MEDIA);
        let got = crate::sip_media::decode_media_frame(rest);
        assert_eq!(got, pcm);
    }

    #[test]
    fn agent_uplink_browser_video_wraps_to_video_tunnel_frame() {
        // 浏览器上行视频像素帧（Binary 帧，kind=2）→ Hub frontend_to_agent 透传 kind
        // 包裹为 tunnel 视频帧 `[4B channelId][1B kind=2][pixels]` 发给 Agent UA₂；
        // Agent 侧按 kind 识别为视频帧 → decode 喂回 UA₂ 发送链路（0.70.2 子任务 #1）。
        let ch_id: u32 = 7;
        let vf = rex_sip::video_bridge::VideoFrame {
            width: 2,
            height: 2,
            rgba: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        };
        let pix = crate::sip_media::encode_video_frame(
            crate::sip_media::VideoPixFmt::Rgba,
            vf.width,
            vf.height,
            &vf.rgba,
        )
        .unwrap();
        // 浏览器侧上行帧：wrap kind=2 + 像素。Hub 剥出 kind 后原样透传 kind 给隧道帧。
        let media = crate::sip_media::wrap_tunnel_frame(crate::sip_media::KIND_VIDEO, &pix);
        let mut frame = ch_id.to_be_bytes().to_vec();
        frame.extend_from_slice(&media);
        let decapped = frame[4..].to_vec();
        let (kind, rest) = crate::sip_media::unwrap_tunnel_frame(&decapped);
        assert_eq!(kind, crate::sip_media::KIND_VIDEO);
        // downlink Hub→browser 同样按 kind=2 原样推 Binary（rest 已是视频像素帧）。
        let (dw, w, h, rgba) = crate::sip_media::decode_video_frame(rest).unwrap();
        assert_eq!(dw, crate::sip_media::VideoPixFmt::Rgba);
        assert_eq!(w, 2);
        assert_eq!(h, 2);
        assert_eq!(rgba, vf.rgba);
    }

    #[test]
    fn video_and_audio_uplink_kinds_kept_distinct() {
        // 浏览器同时上行音频（kind=1）与视频（kind=2），Hub 透传时 kind 互不串台。
        let audio_pcm = vec![1i16, -2];
        let audio_frame = crate::sip_media::wrap_tunnel_frame(
            crate::sip_media::KIND_MEDIA,
            &crate::sip_media::encode_pcm_frame(&audio_pcm),
        );
        let video_rgba = vec![9u8; 4 * 4]; // 2x2 RGBA
        let video_frame = crate::sip_media::wrap_tunnel_frame(
            crate::sip_media::KIND_VIDEO,
            &crate::sip_media::encode_video_frame(
                crate::sip_media::VideoPixFmt::Rgba,
                2,
                2,
                &video_rgba,
            )
            .unwrap(),
        );
        let (ak, ap) = crate::sip_media::unwrap_tunnel_frame(&audio_frame);
        let (vk, vp) = crate::sip_media::unwrap_tunnel_frame(&video_frame);
        assert_eq!(ak, crate::sip_media::KIND_MEDIA);
        assert_eq!(vk, crate::sip_media::KIND_VIDEO);
        // 互不得出对方数据。
        assert_ne!(ap, vp);
        assert_eq!(crate::sip_media::decode_media_frame(ap), audio_pcm);
        assert_eq!(
            crate::sip_media::decode_video_frame(vp).unwrap().3,
            video_rgba
        );
    }

    #[test]
    fn tunnel_kind_signal_vs_media_distinguished() {
        // 同一隧道上信令帧（kind=0）与媒体帧（kind=1）必须按 kind 走不同分发分支。
        let ch_id: u32 = 1;
        // 信令帧：SipEvent JSON → kind=0。
        let sig = crate::sip_media::wrap_tunnel_frame(
            crate::sip_media::KIND_SIGNAL,
            &serde_json::to_vec(&rex_sip::SipEvent::Registered).unwrap(),
        );
        let mut sig_frame = ch_id.to_be_bytes().to_vec();
        sig_frame.extend_from_slice(&sig);
        let sig_decap = sig_frame[4..].to_vec();
        let (sig_kind, sig_rest) = crate::sip_media::unwrap_tunnel_frame(&sig_decap);
        assert_eq!(sig_kind, crate::sip_media::KIND_SIGNAL);
        assert!(decode_tunnel_sip_event(sig_rest).is_some());

        // 媒体帧：随机 PCM → kind=1。
        let pcm = vec![1i16, 2, 3];
        let med = crate::sip_media::wrap_tunnel_frame(
            crate::sip_media::KIND_MEDIA,
            &crate::sip_media::encode_pcm_frame(&pcm),
        );
        let mut med_frame = ch_id.to_be_bytes().to_vec();
        med_frame.extend_from_slice(&med);
        let med_decap = med_frame[4..].to_vec();
        let (med_kind, med_rest) = crate::sip_media::unwrap_tunnel_frame(&med_decap);
        assert_eq!(med_kind, crate::sip_media::KIND_MEDIA);
        assert_eq!(crate::sip_media::decode_media_frame(med_rest), pcm);
    }

    #[test]
    fn full_agent_chain_signaling_sequence() {
        // 模拟 Agent 链式一通信令的完整事件流（前端期望看到的序列）：
        // registered → incoming → call_state:Active → call_state:Ended。
        let events = vec![
            SipEvent::Registered,
            SipEvent::IncomingCall {
                call_id: "call-1".into(),
                from: "2000@example.com".into(),
            },
            SipEvent::CallState {
                call_id: "call-1".into(),
                state: CallState::Active,
            },
            SipEvent::CallState {
                call_id: "call-1".into(),
                state: CallState::Ended,
            },
        ];
        let mut saw_registered = false;
        let mut saw_incoming = false;
        let mut saw_active = false;
        let mut saw_ended = false;
        for ev in &events {
            let decapsulated = agent_tunnel_frame(1, ev)[4..].to_vec();
            let parsed = decode_tunnel_sip_event(&decapsulated).expect("round-trip");
            let json = serde_json::to_string(&map_event(parsed).unwrap()).unwrap();
            if json.contains("sip.registered") {
                saw_registered = true;
            }
            if json.contains("sip.incoming") {
                assert!(json.contains("\"callId\":\"call-1\""));
                saw_incoming = true;
            }
            if json.contains("sip.call_state") {
                if json.contains("\"state\":\"active\"") {
                    saw_active = true;
                }
                if json.contains("\"state\":\"ended\"") {
                    saw_ended = true;
                }
            }
        }
        assert!(saw_registered && saw_incoming && saw_active && saw_ended);
    }

    // --- 子任务 #5：端到端媒体管线联调契约（M82b 实时双向音频）---
    //
    // 真音频端点（baresip server + 麦克风/扬声器）无法在 CI 托管，故联调以「跨语言线格式
    // 契约锁定 + 管道路径单测」为准，手动端到端验证记录在里程碑报告 step6。下列测试覆盖
    // 两条链路完整媒体帧路径：
    //   (a) Hub 直连：on_rtp(PCM) → encode → Binary 帧（浏览器侧 decode 还原）；
    //       浏览器上行 Binary(PCM) → decode → send_audio（经 MockSipUa 记录）。
    //   (b) Agent 链式：下行 on_rtp(PCM) → tunnel kind=1 帧 → Hub decap → 浏览器 decode；
    //       上行浏览器 PCM → tunnel kind=1 → Agent dispatch_media → send_audio。

    #[test]
    fn hub_direct_downlink_pcm_round_trips_to_browser() {
        // 模拟 on_rtp 回调产出的远端 PCM 帧，经 encode 推浏览器，前端 decode 还原。
        let remote_pcm: Vec<i16> = (0..160).map(|i| (i as i16) * 11 - 500).collect();
        let down = crate::sip_media::encode_pcm_frame(&remote_pcm);
        // 浏览器侧（sipMedia.decodeMediaFrame）等价解码，必须无损还原。
        assert_eq!(crate::sip_media::decode_media_frame(&down), remote_pcm);
    }

    #[tokio::test]
    async fn hub_direct_uplink_pcm_reaches_ua_send_audio() {
        // 浏览器上行麦克风帧（Binary PCM）→ decode → ua.send_audio，Mock 记录帧样本数。
        let ua = MockSipUa::new(cfg(), vec![]);
        let mic_pcm: Vec<i16> = vec![100, -200, 300, -400, 500];
        let up = crate::sip_media::encode_pcm_frame(&mic_pcm);
        // 等价于 handle_sip_session cmd_task 中上行分支：decode → send_audio。
        let decoded = crate::sip_media::decode_media_frame(&up);
        ua.send_audio(decoded).await.unwrap();
        let acts = ua.actions.lock().unwrap();
        assert_eq!(*acts, vec![MockAction::SendAudio(mic_pcm.len())]);
    }

    #[test]
    fn agent_downlink_pcm_tunnel_to_browser_round_trip() {
        // Agent UA₂ on_rtp(PCM) → 隧道帧（ch_id + kind=1 + pcm）→ Hub decap → 浏览器 decode。
        let ch_id: u32 = 42;
        let remote_pcm: Vec<i16> = vec![-7, 13, -21, 34];
        let inner = crate::sip_media::wrap_tunnel_frame(
            crate::sip_media::KIND_MEDIA,
            &crate::sip_media::encode_pcm_frame(&remote_pcm),
        );
        let mut frame = ch_id.to_be_bytes().to_vec();
        frame.extend_from_slice(&inner);

        // Hub event_tx 分支：剥 ch_id，按 kind=1 原样推浏览器。
        let decapped = frame[4..].to_vec();
        let (kind, rest) = crate::sip_media::unwrap_tunnel_frame(&decapped);
        assert_eq!(kind, crate::sip_media::KIND_MEDIA);
        // 浏览器侧解码应无损还原。
        assert_eq!(crate::sip_media::decode_media_frame(rest), remote_pcm);
    }

    #[tokio::test]
    async fn agent_uplink_browser_pcm_reaches_ua2_send_audio() {
        // 浏览器上行 PCM → Agent 隧道 kind=1 帧 → dispatch_sip_tunnel_frame → send_audio。
        // 此处用 rex-hub 侧等价路径（`unwrap` + `decode_media_frame` + `MockSipUa::send_audio`）
        // 复刻 agent_ws.rs `dispatch_sip_tunnel_frame` 的媒体分支契约。
        let ua = MockSipUa::new(cfg(), vec![]);
        let mic_pcm: Vec<i16> = vec![1, -2, 3, -4, 5, 6];
        let frame = crate::sip_media::wrap_tunnel_frame(
            crate::sip_media::KIND_MEDIA,
            &crate::sip_media::encode_pcm_frame(&mic_pcm),
        );
        let (kind, payload) = crate::sip_media::unwrap_tunnel_frame(&frame);
        assert_eq!(kind, crate::sip_media::KIND_MEDIA);
        let pcm = crate::sip_media::decode_media_frame(payload);
        assert!(!pcm.is_empty());
        ua.send_audio(pcm).await.unwrap();
        let acts = ua.actions.lock().unwrap();
        assert_eq!(*acts, vec![MockAction::SendAudio(mic_pcm.len())]);
    }

    // --- 子任务 #4：CDR 状态机持久化（状态机驱动 call_state 事件 → 行正确）---

    use crate::db::Database;
    use tempfile::tempdir;

    #[test]
    fn cdr_tracker_creates_and_finalizes_record() {
        let dir = tempdir().unwrap();
        let db = Database::open(&dir.path().join("cdr-test.db")).unwrap();
        let mut tr = CdrTracker::new(
            "res-1",
            "call-1",
            "sip:bob@x",
            "out",
            Arc::new(crate::sip_recording::SipRecordingRegistry::new(
                tempdir().unwrap().path().to_path_buf(),
            )),
        );

        // 拨号 → ringing，落 start_time。
        tr.apply(CallState::Ringing, &db);
        let created = db.get_cdr("cdr:call-1").unwrap().expect("CDR 已创建");
        assert_eq!(created.peer, "sip:bob@x");
        assert_eq!(created.direction, "out");
        assert_eq!(created.state, "ringing");

        // 接通 → active（不落 end_time）。
        tr.apply(CallState::Active, &db);
        let active = db.get_cdr("cdr:call-1").unwrap().unwrap();
        assert_eq!(active.state, "active");
        assert!(active.end_time.is_none());

        // 挂断 → ended，落 end_time + duration。
        tr.apply(CallState::Ended, &db);
        let ended = db.get_cdr("cdr:call-1").unwrap().unwrap();
        assert_eq!(ended.state, "ended");
        assert!(ended.end_time.is_some());
        assert!(ended.duration_sec >= 0);
    }

    #[test]
    fn cdr_tracker_idempotent_on_same_call() {
        let dir = tempdir().unwrap();
        let db = Database::open(&dir.path().join("cdr-test2.db")).unwrap();
        let mut tr = CdrTracker::new(
            "res-1",
            "call-9",
            "sip:carol@y",
            "in",
            Arc::new(crate::sip_recording::SipRecordingRegistry::new(
                tempdir().unwrap().path().to_path_buf(),
            )),
        );
        tr.apply(CallState::Ringing, &db);
        tr.apply(CallState::Active, &db);
        tr.apply(CallState::Ended, &db);
        // 同 call_id 只应有一行。
        let all = db.query_cdr(&crate::models::CdrFilter::default()).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].peer, "sip:carol@y");
        assert_eq!(all[0].direction, "in");
    }
}
