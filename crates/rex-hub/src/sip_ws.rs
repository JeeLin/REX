//! WebSocket SIP 桥接 — 浏览器 ↔ Hub UA₁（baresip 进程内 FFI）。
//!
//! 统一入口：/ws/sip?token=jwt&resourceId=xxx
//! Hub 从 DB 读取 SIP 资源连接信息，构造 UA₁（SipUa），经 WebSocket 与前端
//! 交换控制指令与事件。Agent 链式 UA₂（内网 SIP server）属子任务 #4，本文件
//! 仅实现直连路径；agent 模式返回明确错误，待 #4 接入。
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
use tokio::sync::mpsc;
use tokio::time::Interval;

use crate::app::AppState;
use crate::resource_conn::{load_resource_config, load_sip_conn};

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
    state: String,
}

#[derive(Debug, Serialize)]
struct RawPayload {
    raw: String,
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

/// WebSocket 连接处理主循环（直连 UA₁）
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

    // agent 链式转发属子任务 #4；此处仅直连
    if is_agent_resource(&state, &resource_id).await {
        let _ = send_ws_error(&mut ws, "SIP agent chaining arrives in 0.70.0 #4").await;
        return;
    }

    tracing::info!(
        action = "SIP_CONNECT",
        resource_id = %resource_id,
        server = %sip_cfg.server,
        transport = %sip_cfg.transport.as_str(),
        "SIP UA connection initiated"
    );

    let ua = match SipUa::real(sip_cfg) {
        Ok(ua) => Arc::new(ua),
        Err(e) => {
            let _ = send_ws_error(&mut ws, &format!("SIP UA init failed: {e}")).await;
            return;
        }
    };

    handle_sip_session(ws, ua, &resource_id).await;

    tracing::info!(action = "SIP_DISCONNECT", resource_id = %resource_id, "SIP session ended");
}

/// 真正的信令会话：注册 + 事件回推 + 控制指令下发
async fn handle_sip_session(mut ws: WebSocket, ua: Arc<SipUa>, resource_id: &str) {
    // 注册 UA
    if let Err(e) = ua.register().await {
        let _ = send_ws_error(&mut ws, &format!("SIP register failed: {e}")).await;
        return;
    }

    // 事件回推任务：baresip 事件流 → 前端帧
    let mut events = ua.events();
    let (event_tx, mut event_rx) = mpsc::channel::<ServerMsg>(64);
    let event_tx_for_cmd = event_tx.clone();
    let event_tx_for_ping = event_tx.clone();
    let event_pump = tokio::spawn(async move {
        while let Some(ev) = events.recv().await {
            if let Some(msg) = map_event(ev) {
                if event_tx.send(msg).await.is_err() {
                    break;
                }
            }
        }
    });

    let (mut ws_sink, mut ws_stream) = ws.split();

    // 前端 → UA 控制指令任务
    let ua_for_cmd = ua.clone();
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
                    if let Err(e) = dispatch_cmd(ua_for_cmd.as_ref(), cmd).await {
                        if event_tx_for_cmd
                            .send(ServerMsg::Error {
                                payload: ReasonPayload { reason: e },
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    });

    // 事件 → 前端帧任务
    let writer = tokio::spawn(async move {
        while let Some(msg) = event_rx.recv().await {
            let frame = match serde_json::to_string(&msg) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if ws_sink.send(Message::Text(frame.into())).await.is_err() {
                break;
            }
        }
    });

    // 服务端 keepalive ping（每 25 秒，防止中间件/代理超时断开）；前端忽略该帧
    let mut ping_interval = create_server_ping_interval();
    let ping_task = tokio::spawn(async move {
        loop {
            ping_interval.tick().await;
            if event_tx_for_ping.send(ServerMsg::KeepAlive).await.is_err() {
                break;
            }
        }
    });

    // 任一任务结束即结束会话（其余任务在 JoinHandle drop 时自动取消）
    tokio::select! {
        _ = event_pump => {},
        _ = cmd_task => {},
        _ = writer => {},
        _ = ping_task => {},
    }

    tracing::debug!(action = "SIP_SESSION_END", resource_id, "sip session ended");
}

/// 将 [`SipEvent`] 映射为前端 [`ServerMsg`]。返回 `None` 表示忽略该事件。
fn map_event(ev: SipEvent) -> Option<ServerMsg> {
    match ev {
        SipEvent::Registered => Some(ServerMsg::Registered),
        SipEvent::RegistrationFailed(reason) => Some(ServerMsg::RegistrationFailed {
            payload: ReasonPayload { reason },
        }),
        SipEvent::IncomingCall { call_id, from } => Some(ServerMsg::Incoming {
            payload: IncomingPayload { call_id, from },
        }),
        SipEvent::CallState { call_id, state } => Some(ServerMsg::CallState {
            payload: CallStatePayload {
                call_id,
                state: call_state_str(state),
            },
        }),
        SipEvent::Message { raw } => Some(ServerMsg::SipMessage {
            payload: RawPayload { raw },
        }),
    }
}

fn call_state_str(s: CallState) -> String {
    match s {
        CallState::Ringing => "ringing",
        CallState::Active => "active",
        CallState::Held => "held",
        CallState::Ended => "ended",
    }
    .to_string()
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

/// 判断资源是否走 Agent 隧道（环境 connection_mode == "agent"）。
async fn is_agent_resource(state: &AppState, resource_id: &str) -> bool {
    let db = state.db.clone();
    let rid = resource_id.to_string();
    tokio::task::spawn_blocking(move || {
        let resource = match db.get_resource(&rid) {
            Ok(Some(r)) => r,
            _ => return false,
        };
        db.get_environment(&resource.environment_id)
            .map(|opt| opt.map(|e| e.connection_mode == "agent").unwrap_or(false))
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
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
        let m = map_event(SipEvent::RegistrationFailed("bad".into())).unwrap();
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
}
