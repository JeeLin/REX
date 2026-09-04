//! Agent 侧 SSH 协议执行层（v0.70.6 子任务 #3）。
//!
//! 此前 agent 对 ssh 资源只做「裸 TCP 管道」（`handle_connect` 的 generic 分支）：
//! agent 向目标发起 TCP 后双向转发原始字节，Hub 同样做裸字节桥接，于是浏览器
//! 看到的是服务端横幅 `SSH-2.0-...` 而非交互式 shell（协议从未被终结）。
//!
//! 本模块让 **Agent 在私网内运行 russh 终结 SSH 协议**（握手/认证/PTY），把已经
//! 协商好的终端 I/O 以「`[4B channelId][data]` 二进制帧」经既有单 WS 隧道回传 Hub，
//! 前端拿到真正的 shell。传输层复用 M82 已验证的单 WS + channel_id 多路复用范式，
//! 不新建通道。

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use rex_ssh::{SshConfig, SshSession, TerminalEvent};
use serde_json::Value;

use crate::agent_ws::{AgentEvent, LocalChannel};

/// 从 connect config 解析 SSH 配置（对应 Hub 侧 `handle_agent_terminal` 下发的字段约定）。
pub fn parse_ssh_config(cfg: &Value) -> SshConfig {
    let host = cfg
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let port = cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;
    let username = cfg
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let password = cfg
        .get("password")
        .and_then(|v| v.as_str())
        .map(String::from);
    let private_key = cfg
        .get("privateKey")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| {
            cfg.get("private_key")
                .and_then(|v| v.as_str())
                .map(String::from)
        });
    let keepalive_interval = cfg
        .get("keepalive_interval")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let init_script = cfg
        .get("initScript")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(String::from)
        .or_else(|| {
            cfg.get("init_script")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(String::from)
        });

    SshConfig {
        host,
        port,
        username,
        password,
        private_key,
        keepalive_interval,
        init_script,
    }
}

/// 驱动一次 Agent 侧 SSH 会话：russh 终结协议，终端 I/O 经隧道帧上送 Hub。
///
/// `data_rx` 来自隧道（Hub 经 `[4B channelId][data]` 下发浏览器键入的明文字节）；
/// 本函数把 russh 收到的终端输出封装成同样的隧道帧，由 `evt_tx` 写回 WS。
pub async fn run_ssh_session(
    session: SshSession,
    channel_id: String,
    evt_tx: mpsc::Sender<AgentEvent>,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
    mut data_rx: mpsc::Receiver<Vec<u8>>,
) {
    let ch_id_num = channel_id.parse::<u32>().unwrap_or(0);

    // 拆分会话为独立的写半区和事件接收器，避免 Mutex 死锁。
    // 此前 out_task 持有 Arc<Mutex<SshSession>> 调用 recv()（&mut self），
    // 导致 in_task 无法获取锁调用 send_data()（&self），造成死锁。
    let (write_half, mut events) = session.split();
    let write_half = Arc::new(write_half);

    // SSH resize 控制通道：Hub 经隧道下发 resize 帧 → 本通道 → russh window_change。
    let (resize_tx, mut resize_rx) = mpsc::unbounded_channel::<(u32, u32)>();
    {
        let mut chs = channels.write().await;
        if let Some(ch) = chs.get_mut(&channel_id) {
            ch.resize_tx = Some(resize_tx);
        }
    }

    // 终端输出（russh 事件）→ 隧道帧（带 channelId 前缀）。
    // events 独立拥有事件接收器，无需锁。
    let evt_tx_out = evt_tx.clone();
    let cid_out = channel_id.clone();
    let out_task = tokio::spawn(async move {
        loop {
            match events.recv().await {
                Some(TerminalEvent::Data(data)) => {
                    let mut frame = Vec::with_capacity(4 + data.len());
                    frame.extend_from_slice(&ch_id_num.to_be_bytes());
                    frame.extend_from_slice(data.as_bytes());
                    if evt_tx_out.send(AgentEvent::Binary(frame)).await.is_err() {
                        break;
                    }
                }
                Some(TerminalEvent::Disconnected(_reason)) => {
                    let close = serde_json::to_string(&crate::agent_ws::AgentMsg::Closed {
                        payload: crate::agent_ws::ChannelPayload {
                            channel_id: cid_out.clone(),
                        },
                    })
                    .unwrap_or_default();
                    let _ = evt_tx_out.send(AgentEvent::Text(close)).await;
                    break;
                }
                None => break,
            }
        }
    });

    // 隧道输入（浏览器键入）→ russh send_data；resize 帧 → russh window_change。
    // write_half 通过 Arc 共享，send_data/resize 仅需 &self，无锁竞争。
    let in_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                maybe = data_rx.recv() => {
                    match maybe {
                        Some(data) => {
                            if data.is_empty() {
                                break; // 关闭信号
                            }
                            let wh = write_half.clone();
                            let bytes = bytes::Bytes::copy_from_slice(&data);
                            tokio::spawn(async move {
                                let _ = wh.data_bytes(bytes).await;
                            });
                        }
                        None => break,
                    }
                }
                resize = resize_rx.recv() => {
                    match resize {
                        Some((cols, rows)) => {
                            let wh = write_half.clone();
                            tokio::spawn(async move {
                                let _ = wh.window_change(cols, rows, 0, 0).await;
                            });
                        }
                        None => break,
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = out_task => {},
        _ = in_task => {},
    }

    // 清理 channel 表。
    {
        let mut chs = channels.write().await;
        chs.remove(&channel_id);
    }
    tracing::info!(action = "AGENT_SSH_END", channel_id = %channel_id, "agent SSH session ended");
}

/// 由 `handle_connect` 的 ssh 分支调用：在 Agent 内建立 russh 会话并接管隧道帧。
pub async fn handle_connect_ssh(
    request_id: String,
    cfg: &Value,
    evt_tx: mpsc::Sender<AgentEvent>,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
    channel_id: String,
) {
    let ssh_cfg = parse_ssh_config(cfg);

    let session = match SshSession::connect(ssh_cfg).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                action = "AGENT_SSH_FAILED",
                request_id = %request_id,
                error = %e,
                "agent SSH connect failed"
            );
            let err = serde_json::to_string(&crate::agent_ws::AgentMsg::ConnectError {
                payload: crate::agent_ws::ConnectErrorPayload {
                    request_id,
                    error: format!("SSH connection failed: {e}"),
                },
            })
            .unwrap_or_default();
            let _ = evt_tx.send(AgentEvent::Text(err)).await;
            return;
        }
    };

    tracing::info!(
        action = "AGENT_SSH_CONNECTED",
        request_id = %request_id,
        channel_id = %channel_id,
        "agent SSH session established (russh terminated)"
    );

    // 通知 Hub 连接成功（协议已在 Agent 终结，后续回传的是终端流）。
    let ok = serde_json::to_string(&crate::agent_ws::AgentMsg::Connected {
        payload: crate::agent_ws::ConnectedPayload {
            request_id,
            channel_id: channel_id.clone(),
        },
    })
    .unwrap_or_default();
    let _ = evt_tx.send(AgentEvent::Text(ok)).await;

    // 注册 channel（接收 Hub 经隧道下发的键入字节）。
    let (data_tx, data_rx) = mpsc::channel::<Vec<u8>>(512);
    {
        let mut chs = channels.write().await;
        chs.insert(
            channel_id.clone(),
            LocalChannel {
                channel_id: channel_id.clone(),
                data_tx,
                resize_tx: None,
            },
        );
    }

    run_ssh_session(session, channel_id, evt_tx, channels, data_rx).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 锁定 Agent 侧 SSH 配置解析与 Hub 下发字段的契约
    /// （`terminal_ws::handle_agent_terminal` 的 connect payload 用
    /// `privateKey` / `username` / `host` / `port` / `password`）。
    #[test]
    fn parse_ssh_config_reads_hub_contract_fields() {
        let cfg = serde_json::json!({
            "host": "10.0.0.5",
            "port": 2222,
            "username": "ops",
            "password": "secret",
            "privateKey": "PEM-KEY",
            "initScript": "cd /data\n"
        });
        let ssh = parse_ssh_config(&cfg);
        assert_eq!(ssh.host, "10.0.0.5");
        assert_eq!(ssh.port, 2222);
        assert_eq!(ssh.username, "ops");
        assert_eq!(ssh.password.as_deref(), Some("secret"));
        assert_eq!(ssh.private_key.as_deref(), Some("PEM-KEY"));
        assert_eq!(ssh.init_script.as_deref(), Some("cd /data\n"));
    }

    #[test]
    fn parse_ssh_config_defaults_and_optional_private_key() {
        let cfg = serde_json::json!({"host":"h","username":"u"});
        let ssh = parse_ssh_config(&cfg);
        assert_eq!(ssh.port, 22);
        assert!(ssh.password.is_none());
        assert!(ssh.private_key.is_none());
    }
}
