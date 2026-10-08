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
use std::io;
use std::sync::Arc;

use crate::agent_ws::SshHandlePool;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};

use rex_ssh::{SshConfig, SshSession, TerminalEvent};
use serde_json::Value;

use rex_common::agent_proto::AgentEvent;
use rex_common::resource_config::config_private_key;

use crate::agent_ws::LocalChannel;

/// 探测（[`probe_ssh`]）的单步超时：与 Hub 侧 `resource_api::PROBE_TIMEOUT` 同口径，
/// 两段相加仍在 Hub 的等待窗口内。
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// SSH 标识串读取上限（字节），防畸形对端不发换行把探测拖死。
const SSH_BANNER_MAX: usize = 512;

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
    let private_key = config_private_key(cfg);
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

// ── SSH connect failure diagnosis ────────────────────────────────
//
// rex_ssh::SshSession::connect_with_handle 把 TCP / KEX / 认证 / 通道
// 阶段的底层 russh::Error 串在 anyhow 上下文链上；anyhow 的非 alternate
// Display (`{}`) 仅显示最外层 context（例如 "SSH connection failed"），
// 正因如此 Agent 日志才只见 `error=SSH connection failed`。
//
// 这里用 `{err:#}` 展开整条链；并从链上分类出失败阶段 + 稳定错误码，既写日志
// 也回传 Hub。注意：该栈用的是 `russh`（非 ssh2/libssh2），错误码用
// `SSH_ERR_*` 映射 russh / std::io 的语义。

/// Agent 侧 SSH 建连失败的阶段。
///
/// 对标「TCP 连接 / 版本协商与握手 / 认证」三大阶段，额外加一档 `session`
///（认证成功后打开 channel / PTY / shell 失败）。握手阶段命名为 `kex`
/// 而非 `handshake`，是因为 Hub 侧 `error::classify_connect_error` 把
/// 文案中含 "handshake" 判为 TlsFailure —— SSH 版本协商与密钥交换本非
/// TLS，用 `kex` 即可精确、又不误判。
pub(crate) const STAGE_TCP: &str = "tcp";
pub(crate) const STAGE_KEX: &str = "kex";
pub(crate) const STAGE_AUTH: &str = "auth";
pub(crate) const STAGE_SESSION: &str = "session";

/// 一次 SSH 建连失败的结构化诊断。`detail` 由 anyhow 整链拼接，不含密码/密钥。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SshConnectDiag {
    pub stage: &'static str,
    pub code: &'static str,
    pub detail: String,
}

impl SshConnectDiag {
    /// 发给 Hub 前端的可展示文案。Hub 仅把该字符串原样回传给前端
    /// （终端错误行 + toast），前端已预留渲染 `code`/`message`。
    pub(crate) fn message(&self) -> String {
        format!(
            "SSH connect failed: stage={}, code={}, {}",
            self.stage, self.code, self.detail
        )
    }
}

/// 将 `SshSession::connect_with_handle` 抛出的 anyhow 错误分类为
/// (阶段, 错误码, 完整链)。纯函数，便于单元测试。
pub(crate) fn diagnose_ssh_failure(err: &anyhow::Error) -> SshConnectDiag {
    let detail = format!("{err:#}");
    let lower = detail.to_lowercase();

    // 1. 认证阶段：rex_ssh 聚合 "SSH authentication failed (...)"，或
    //    私钥解码 / 公钥认证调用链上的失败。须在 TCP 文本匹配前判定，
    //    否则 "password: Connection refused" 会被误判为 TCP 错误。
    if is_auth_failure(&lower) {
        let code = if lower.contains("failed to decode private key") {
            "SSH_ERR_PRIVATE_KEY_DECODE"
        } else {
            "SSH_ERR_AUTH_FAILED"
        };
        return SshConnectDiag {
            stage: STAGE_AUTH,
            code,
            detail,
        };
    }

    // 2. 会话阶段：认证通过后打开 channel / 请求 PTY / 请求 shell。
    if let Some(code) = session_code(&lower) {
        return SshConnectDiag {
            stage: STAGE_SESSION,
            code,
            detail,
        };
    }

    // 3. TCP 传输阶段：优先从 io::ErrorKind 判定；若为未分类错误，
    //    退而用文本关键词细化（DNS / 拒绝 / 超时 …）。
    if let Some(ioe) = find_io_error(err) {
        let code = refine_io_code(ioe.kind(), &lower);
        return SshConnectDiag {
            stage: STAGE_TCP,
            code,
            detail,
        };
    }
    if let Some(code) = tcp_text_code(&lower) {
        return SshConnectDiag {
            stage: STAGE_TCP,
            code,
            detail,
        };
    }

    // 4. 握手 / KEX 阶段：russh 版本协商、算法协商、密钥交换。
    if let Some(code) = kex_code(&lower) {
        return SshConnectDiag {
            stage: STAGE_KEX,
            code,
            detail,
        };
    }

    // 5. 回源不到具体原因但带 "SSH connection failed" 上下文 → 握手阶段。
    if lower.contains("ssh connection failed") {
        return SshConnectDiag {
            stage: STAGE_KEX,
            code: "SSH_ERR_CONNECT_FAILED",
            detail,
        };
    }

    // 6. 兜底。
    SshConnectDiag {
        stage: STAGE_TCP,
        code: "SSH_ERR_CONNECT_FAILED",
        detail,
    }
}

fn is_auth_failure(lower: &str) -> bool {
    lower.contains("authentication failed")
        || lower.contains("public key authentication failed")
        || lower.contains("failed to decode private key")
}

/// 从 anyhow 错误链中取出首个 `std::io::Error`（russh::Error::IO 在链上传播）。
fn find_io_error(err: &anyhow::Error) -> Option<&io::Error> {
    err.chain()
        .find_map(|link| link.downcast_ref::<io::Error>())
}

fn io_kind_code(kind: io::ErrorKind) -> &'static str {
    match kind {
        io::ErrorKind::ConnectionRefused => "SSH_ERR_CONNECTION_REFUSED",
        io::ErrorKind::ConnectionReset => "SSH_ERR_CONNECTION_RESET",
        io::ErrorKind::ConnectionAborted => "SSH_ERR_CONNECTION_ABORTED",
        io::ErrorKind::NotConnected => "SSH_ERR_DISCONNECTED",
        io::ErrorKind::TimedOut => "SSH_ERR_TIMEOUT",
        io::ErrorKind::HostUnreachable => "SSH_ERR_HOST_UNREACHABLE",
        io::ErrorKind::NetworkUnreachable => "SSH_ERR_NETWORK_UNREACHABLE",
        io::ErrorKind::AddrInUse => "SSH_ERR_ADDR_IN_USE",
        io::ErrorKind::AddrNotAvailable => "SSH_ERR_ADDR_NOT_AVAILABLE",
        io::ErrorKind::PermissionDenied => "SSH_ERR_PERMISSION_DENIED",
        io::ErrorKind::NotFound => "SSH_ERR_NOT_FOUND",
        io::ErrorKind::InvalidInput => "SSH_ERR_INVALID_INPUT",
        _ => "SSH_ERR_IO",
    }
}

/// io::ErrorKind 未细分时，用错误文本关键词补充更有区分度的码。
fn refine_io_code(kind: io::ErrorKind, lower: &str) -> &'static str {
    let code = io_kind_code(kind);
    if code != "SSH_ERR_IO" {
        return code;
    }
    tcp_text_code(lower).unwrap_or(code)
}

fn tcp_text_code(lower: &str) -> Option<&'static str> {
    if lower.contains("dns")
        || lower.contains("no such host")
        || lower.contains("failed to resolve")
        || lower.contains("no addresses resolved")
        || lower.contains("lookup")
        || lower.contains("name or service not known")
    {
        Some("SSH_ERR_DNS_FAILURE")
    } else if lower.contains("connection refused") {
        Some("SSH_ERR_CONNECTION_REFUSED")
    } else if lower.contains("timed out") || lower.contains("timeout") || lower.contains("deadline")
    {
        Some("SSH_ERR_TIMEOUT")
    } else if lower.contains("unreachable") {
        Some("SSH_ERR_NET_UNREACHABLE")
    } else if lower.contains("address already in use") {
        Some("SSH_ERR_ADDR_IN_USE")
    } else {
        None
    }
}

fn session_code(lower: &str) -> Option<&'static str> {
    if lower.contains("failed to open session") {
        Some("SSH_ERR_SESSION_OPEN")
    } else if lower.contains("failed to request pty") {
        Some("SSH_ERR_PTY_REQUEST")
    } else if lower.contains("failed to request shell") {
        Some("SSH_ERR_SHELL_REQUEST")
    } else {
        None
    }
}

fn kex_code(lower: &str) -> Option<&'static str> {
    if lower.contains("no common") || lower.contains("algorithm") {
        Some("SSH_ERR_KEX_NO_COMMON_ALGO")
    } else if lower.contains("invalid ssh version") {
        Some("SSH_ERR_SSH_VERSION")
    } else if lower.contains("unknown algorithm") {
        Some("SSH_ERR_UNKNOWN_ALGO")
    } else if lower.contains("key exchange") {
        Some("SSH_ERR_KEX")
    } else if lower.contains("connection closed by the remote side") {
        Some("SSH_ERR_REMOTE_CLOSED")
    } else if lower.contains("strict key exchange") {
        Some("SSH_ERR_STRICT_KEX_VIOLATION")
    } else {
        None
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
    ssh_handles: SshHandlePool,
    pool_key: String,
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

    // ── out_task：终端输出（russh 事件）→ 隧道帧 ──
    // 加超时检测：若 60s 无任何事件，判定 SSH 连接已僵死并主动退出。
    let evt_tx_out = evt_tx.clone();
    let cid_out = channel_id.clone();
    let out_task = tokio::spawn(async move {
        let stall_timeout = std::time::Duration::from_secs(60);
        loop {
            match tokio::time::timeout(stall_timeout, events.recv()).await {
                Ok(Some(TerminalEvent::Data(data))) => {
                    let mut frame = Vec::with_capacity(4 + data.len());
                    frame.extend_from_slice(&ch_id_num.to_be_bytes());
                    frame.extend_from_slice(data.as_bytes());
                    if evt_tx_out.send(AgentEvent::Binary(frame)).await.is_err() {
                        tracing::debug!(action = "AGENT_SSH_OUT", channel_id = %cid_out, "evt_tx closed, stopping out_task");
                        break;
                    }
                }
                Ok(Some(TerminalEvent::Disconnected(reason))) => {
                    tracing::info!(action = "AGENT_SSH_OUT", channel_id = %cid_out, reason = %reason, "SSH disconnected");
                    let close = serde_json::to_string(&crate::agent_ws::AgentMsg::Closed {
                        payload: crate::agent_ws::ChannelPayload {
                            channel_id: cid_out.clone(),
                        },
                    })
                    .unwrap_or_default();
                    let _ = evt_tx_out.send(AgentEvent::Text(close)).await;
                    break;
                }
                Ok(None) => {
                    tracing::info!(action = "AGENT_SSH_OUT", channel_id = %cid_out, "events channel closed");
                    break;
                }
                Err(_elapsed) => {
                    // 超时无事件 — SSH 连接可能已僵死（半关闭 TCP 等）。
                    tracing::warn!(
                        action = "AGENT_SSH_OUT",
                        channel_id = %cid_out,
                        timeout_secs = stall_timeout.as_secs(),
                        "SSH event stall detected, closing session"
                    );
                    let close = serde_json::to_string(&crate::agent_ws::AgentMsg::Closed {
                        payload: crate::agent_ws::ChannelPayload {
                            channel_id: cid_out.clone(),
                        },
                    })
                    .unwrap_or_default();
                    let _ = evt_tx_out.send(AgentEvent::Text(close)).await;
                    break;
                }
            }
        }
    });
    // ── in_task：隧道输入（浏览器键入）→ russh send_data；resize 帧 → russh window_change。 ──
    // write_half 通过 Arc 共享，send_data/resize 仅需 &self，无锁竞争。
    // 用信号量限制并发写入，防止无限 spawn 导致资源耗尽或 SSH 会话乱序。
    let write_semaphore = Arc::new(tokio::sync::Semaphore::new(16));
    let in_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                maybe = data_rx.recv() => {
                    match maybe {
                        Some(data) => {
                            if data.is_empty() {
                                tracing::debug!(action = "AGENT_SSH_IN", "received close signal");
                                break;
                            }
                            let sem = write_semaphore.clone();
                            let wh = write_half.clone();
                            let bytes = bytes::Bytes::copy_from_slice(&data);
                            tokio::spawn(async move {
                                let _permit = sem.acquire().await;
                                if let Err(e) = wh.data_bytes(bytes).await {
                                    tracing::debug!(action = "AGENT_SSH_IN", error = %e, "write failed");
                                }
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
        _ = out_task => {
            tracing::debug!(action = "AGENT_SSH_SESSION", channel_id = %channel_id, "out_task finished");
        },
        _ = in_task => {
            tracing::debug!(action = "AGENT_SSH_SESSION", channel_id = %channel_id, "in_task finished");
        },
    }

    // 清理 channel 表。
    {
        let mut chs = channels.write().await;
        chs.remove(&channel_id);
    }
    tracing::info!(action = "AGENT_SSH_END", channel_id = %channel_id, "agent SSH session ended");

    // 从连接池中移除 SSH Handle（SSH 会话已断开，Handle 不可复用）
    {
        let mut handles = ssh_handles.write().await;
        if handles.remove(&pool_key).is_some() {
            tracing::info!(
                action = "AGENT_SSH_POOL_REMOVED",
                pool_key = %pool_key,
                remaining = handles.len(),
                "SSH handle removed from pool (session ended)"
            );
        }
    }
}

/// 由 `handle_connect` 的 ssh 分支调用：在 Agent 内建立 russh 会话并接管隧道帧。
pub async fn handle_connect_ssh(
    request_id: String,
    cfg: &Value,
    evt_tx: mpsc::Sender<AgentEvent>,
    channels: Arc<RwLock<HashMap<String, LocalChannel>>>,
    channel_id: String,
    ssh_handles: SshHandlePool,
) {
    let ssh_cfg = parse_ssh_config(cfg);

    let auth_method = if ssh_cfg.private_key.is_some() {
        "key"
    } else if ssh_cfg.password.is_some() {
        "password"
    } else {
        "none"
    };
    tracing::info!(
        action = "AGENT_SSH_CONNECT",
        request_id = %request_id,
        host = %ssh_cfg.host,
        port = ssh_cfg.port,
        username = %ssh_cfg.username,
        auth_method = auth_method,
        has_init_script = ssh_cfg.init_script.is_some(),
        "SSH connection initiated"
    );

    let (handle, session) = match SshSession::connect_with_handle(ssh_cfg.clone()).await {
        Ok(s) => s,
        Err(e) => {
            // 展开底层 russh/io 错误，标出失败阶段与稳定码。`%e` 仅显示
            // anyhow 最外层 context，必然丢失 “Connection refused / No common
            // Kex algorithm” 这类可诊断信息，故改用 `diagnose_ssh_failure`。
            let diag = diagnose_ssh_failure(&e);
            tracing::warn!(
                action = "AGENT_SSH_FAILED",
                request_id = %request_id,
                host = %ssh_cfg.host,
                port = ssh_cfg.port,
                username = %ssh_cfg.username,
                auth_method = auth_method,
                stage = %diag.stage,
                code = %diag.code,
                error = %diag.detail,
                "agent SSH connect failed"
            );
            let err = serde_json::to_string(&crate::agent_ws::AgentMsg::ConnectError {
                payload: crate::agent_ws::ConnectErrorPayload {
                    request_id,
                    error: diag.message(),
                },
            })
            .unwrap_or_default();
            let _ = evt_tx.send(AgentEvent::Text(err)).await;
            return;
        }
    };

    // 将 handle 存入连接池，供 SFTP 复用（键含 username，同 host:port 不同
    // 用户不共用已认证会话，与 Hub 侧 rex_ssh::pool 同口径）
    let pool_key = crate::agent_ws::ssh_pool_key_from_cfg(cfg);
    {
        let mut handles = ssh_handles.write().await;
        handles.insert(pool_key.clone(), Arc::new(tokio::sync::Mutex::new(handle)));
        tracing::info!(
            action = "AGENT_SSH_POOL_STORED",
            host = %ssh_cfg.host,
            port = ssh_cfg.port,
            pool_key = %pool_key,
            pool_size = handles.len(),
            "SSH handle stored in pool for SFTP reuse"
        );
    }

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

    run_ssh_session(
        session,
        channel_id,
        evt_tx,
        channels,
        data_rx,
        ssh_handles,
        pool_key,
    )
    .await;
}

/// 「测试连接」探测：SSH 可达性握手，无会话副作用。
///
/// 与 `handle_connect_ssh` 的区别是**不建会话**：只连 TCP 并读一帧 SSH 标识串，
/// 不分配 PTY、不 open shell、不写共享 Handle 池。因此探测既不在目标机留下 shell
/// 启动痕迹（`.bashrc` / MOTD / audit 日志），也不占 sshd 的 `MaxSessions` 配额；
/// 池键与真实会话天然不冲突，用户终端的 Handle 既不会被顶掉，也不会在探测结束时
/// 被误删。
///
/// 成功回 `Connected`（`channel_id` 从不在本地注册，Hub 收到后随即下发 `close`），
/// 失败回 `ConnectError`。`diagnose_ssh_failure` 在此不适用：未进入认证阶段，
/// 没有可稳定分类的 russh 错误码，只有 TCP 层错误。
pub async fn probe_ssh(
    request_id: String,
    channel_id: String,
    cfg: &Value,
    evt_tx: mpsc::Sender<AgentEvent>,
) {
    let host = cfg
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let port = cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;

    let fail = |error: String| send_probe_error(&evt_tx, &request_id, error);

    if host.is_empty() {
        fail("missing host".into()).await;
        return;
    }

    // IPv6 addresses need brackets: [::1]:22
    let addr = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };

    let mut stream = match tokio::time::timeout(PROBE_TIMEOUT, TcpStream::connect(&addr)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            fail(format!("TCP connect failed: {e}")).await;
            return;
        }
        Err(_) => {
            fail("connection timed out".into()).await;
            return;
        }
    };

    let banner = match tokio::time::timeout(PROBE_TIMEOUT, read_ssh_banner(&mut stream)).await {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => {
            fail(format!("SSH identification read failed: {e}")).await;
            return;
        }
        Err(_) => {
            fail("SSH identification timed out".into()).await;
            return;
        }
    };

    // 不回显标识串原文：它由对端控制（含换行即可污染 Hub 日志与前端 toast）。
    if !banner.starts_with("SSH-") {
        fail("not an SSH service".into()).await;
        return;
    }

    tracing::info!(
        action = "AGENT_SSH_PROBE",
        request_id = %request_id,
        host = %host,
        port = port,
        "SSH probe ok (no session opened)"
    );

    let ok = serde_json::to_string(&crate::agent_ws::AgentMsg::Connected {
        payload: crate::agent_ws::ConnectedPayload {
            request_id,
            channel_id,
        },
    })
    .unwrap_or_default();
    let _ = evt_tx.send(AgentEvent::Text(ok)).await;
}

/// 读 SSH 标识串首行（RFC 4253 §4.2：客户端标识串以 CRLF 结束）。
///
/// 上限 [`SSH_BANNER_MAX`] 字节，防畸形对端一直不发换行把探测拖死。
async fn read_ssh_banner(stream: &mut tokio::net::TcpStream) -> io::Result<String> {
    use tokio::io::AsyncReadExt;
    let mut banner = Vec::with_capacity(SSH_BANNER_MAX);
    let mut byte = [0u8; 1];
    while banner.len() < SSH_BANNER_MAX {
        if stream.read(&mut byte).await? == 0 {
            break;
        }
        banner.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&banner).trim().to_string())
}

/// 探测失败回帧：与 `handle_connect_ssh` 的失败路径同一形状（`connect_error`）。
async fn send_probe_error(evt_tx: &mpsc::Sender<AgentEvent>, request_id: &str, error: String) {
    let msg = serde_json::to_string(&crate::agent_ws::AgentMsg::ConnectError {
        payload: crate::agent_ws::ConnectErrorPayload {
            request_id: request_id.to_string(),
            error,
        },
    })
    .unwrap_or_default();
    let _ = evt_tx.send(AgentEvent::Text(msg)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::time::Duration;
    use tokio::io::AsyncReadExt;

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

    /// CR10 回归：池键必须含 username（与 Hub 侧 `rex_ssh::pool` 的
    /// `user@host:port` 同口径），同 host/port 不同用户拿到不同池条目。
    #[test]
    fn pool_key_contains_username_and_separates_users_on_same_host_port() {
        let alice = crate::agent_ws::ssh_pool_key_from_cfg(
            &serde_json::json!({"host":"10.0.0.1","port":22,"username":"alice"}),
        );
        assert_eq!(alice, "alice@10.0.0.1:22");

        let bob = crate::agent_ws::ssh_pool_key_from_cfg(
            &serde_json::json!({"host":"10.0.0.1","port":22,"username":"bob"}),
        );
        assert_ne!(
            alice, bob,
            "same host:port with different users must not share a pool entry"
        );
    }

    /// SFTP 入口（`agent_file::build_connector`）与终端入口共用
    /// `ssh_pool_key_from_cfg`：同一下发 config 由 `parse_ssh_config` 解析出的
    /// 用户/主机/端口与池键提取结果一致，两个入口必然落到同一条目。
    #[test]
    fn pool_key_matches_parsed_ssh_config_for_sftp_entry() {
        let cfg =
            serde_json::json!({"host":"10.0.0.5","port":2222,"username":"ops","password":"pw"});
        let ssh = parse_ssh_config(&cfg);
        assert_eq!(
            crate::agent_ws::ssh_pool_key_from_cfg(&cfg),
            format!("{}@{}:{}", ssh.username, ssh.host, ssh.port),
            "ssh and sftp entries must derive one pool key from one config"
        );

        let bare = serde_json::json!({"host":"10.0.0.5","port":2222});
        assert_eq!(
            crate::agent_ws::ssh_pool_key_from_cfg(&bare),
            "@10.0.0.5:2222"
        );
    }

    // ── 「测试连接」探测（probe_ssh）──

    /// 探测读到 `SSH-` 标识串即算成功，且不回显对端可控的标识串原文
    /// （含换行即可污染 Hub 日志与前端 toast）。
    #[tokio::test]
    async fn probe_ok_on_ssh_banner_without_echoing_it() {
        let (addr, conn) = spawn_banner_server(BANNER_SSH).await;
        let (evt_tx, mut evt_rx) = mpsc::channel::<AgentEvent>(8);
        let request_id = "req_probe".to_string();
        let channel_id = "7".to_string();

        probe_ssh(
            request_id.clone(),
            channel_id.clone(),
            &serde_json::json!({"host": "127.0.0.1", "port": addr.port()}),
            evt_tx,
        )
        .await;

        let sent = evt_rx.recv().await.expect("probe must report a verdict");
        let AgentEvent::Text(raw) = sent else {
            panic!("probe reply must be text");
        };
        let msg: serde_json::Value = serde_json::from_str(&raw).expect("probe reply must be JSON");
        assert_eq!(msg["type"], "connected");
        assert_eq!(msg["payload"]["request_id"], request_id.as_str());
        assert_eq!(msg["payload"]["channel_id"], channel_id.as_str());
        assert!(
            !raw.contains("OpenSSH_9.6p1"),
            "remote-controlled banner text must not be echoed back: {raw}"
        );

        // 探测只握手不建会话：读走标识串即断，从不发客户端标识串（RFC 4253
        // 要求会话建立时由客户端先发 `SSH-2.0-...`），更不请求 PTY / shell。
        let mut server_side = conn.await.expect("banner server task must not panic");
        let mut buf = [0u8; 64];
        let tail = tokio::time::timeout(Duration::from_millis(200), server_side.read(&mut buf))
            .await
            .map(|r| r.unwrap_or(0))
            .unwrap_or(0);
        assert_eq!(
            tail, 0,
            "probe must not send a client identification string or any session request"
        );
    }

    /// 非 SSH 服务：标识串不以 `SSH-` 开头即判失败，回 `connect_error`。
    #[tokio::test]
    async fn probe_fails_on_non_ssh_service() {
        let (addr, _conn) = spawn_banner_server(BANNER_FTP).await;
        let (evt_tx, mut evt_rx) = mpsc::channel::<AgentEvent>(8);

        probe_ssh(
            "req_probe".into(),
            "7".into(),
            &serde_json::json!({"host": "127.0.0.1", "port": addr.port()}),
            evt_tx,
        )
        .await;

        let Some(AgentEvent::Text(raw)) = evt_rx.recv().await else {
            panic!("probe must report a verdict");
        };
        let msg: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(msg["type"], "connect_error");
        assert_eq!(msg["payload"]["request_id"], "req_probe");
        assert_eq!(msg["payload"]["error"], "not an SSH service");
    }

    /// TCP 拒绝连接：不进 SSH 阶段即失败，文案不泄漏任何凭据。
    #[tokio::test]
    async fn probe_fails_when_tcp_refused() {
        // 绑定后立即 drop：端口基本确定处于「无人监听」状态
        let port = {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap().port()
        };
        let (evt_tx, mut evt_rx) = mpsc::channel::<AgentEvent>(8);

        probe_ssh(
            "req_probe".into(),
            "7".into(),
            &serde_json::json!({"host": "127.0.0.1", "port": port, "password": "s3cr3t"}),
            evt_tx,
        )
        .await;

        let Some(AgentEvent::Text(raw)) = evt_rx.recv().await else {
            panic!("probe must report a verdict");
        };
        let msg: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(msg["type"], "connect_error");
        let error = msg["payload"]["error"].as_str().unwrap();
        assert!(error.starts_with("TCP connect failed"), "{error}");
        assert!(!error.contains("s3cr3t"), "{error}");
    }

    const BANNER_SSH: &str = "SSH-2.0-OpenSSH_9.6p1 Debian-1\r\n";
    const BANNER_FTP: &str = "220 ProFTPD 1.3.5 Server ready\r\n";

    /// 监听一次、写完标识串首行后返回「已 accept 的连接」，供探测侧读完后
    /// 断言它没有再发任何东西（不回客户端标识串、不请求 PTY / shell）。
    async fn spawn_banner_server(
        banner: &'static str,
    ) -> (
        std::net::SocketAddr,
        tokio::task::JoinHandle<tokio::net::TcpStream>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            let (mut stream, _) = listener.accept().await.expect("accept banner connection");
            let _ = stream.write_all(banner.as_bytes()).await;
            stream
        });
        (addr, handle)
    }

    // ── SSH failure diagnosis (Bug fix: surface underlying error + stage) ──

    /// 模拟 rex_ssh 建连路径把底层错误串在 anyhow 上下文链上。
    /// `anyhow!(io_err).context(...)` 让 `find_io_error` 能 `downcast_ref::<io::Error>`
    /// 拿到 `ErrorKind`，同时 `format!("{e:#}")` 展开整条链 —— 即是
    /// `handle_connect_ssh` 见到的形态。
    fn io_chain(kind: io::ErrorKind, msg: &str) -> anyhow::Error {
        anyhow::anyhow!(io::Error::new(kind, msg)).context("SSH connection failed")
    }

    #[test]
    fn diag_tcp_refused_is_distinct_from_auth() {
        let e = io_chain(
            io::ErrorKind::ConnectionRefused,
            "Connection refused (os error 111)",
        );
        let d = diagnose_ssh_failure(&e);
        assert_eq!(d.stage, STAGE_TCP);
        assert_eq!(d.code, "SSH_ERR_CONNECTION_REFUSED");
        assert!(d.detail.contains("Connection refused (os error 111)"));
        assert!(d.message().contains("stage=tcp"));
    }

    #[test]
    fn diag_tcp_timeout_maps_to_distinct_code() {
        let e = io_chain(
            io::ErrorKind::TimedOut,
            "Connection timed out (os error 110)",
        );
        let d = diagnose_ssh_failure(&e);
        assert_eq!(d.stage, STAGE_TCP);
        assert_eq!(d.code, "SSH_ERR_TIMEOUT");
    }

    #[test]
    fn diag_dns_failure_is_tcp_stage() {
        // rex_ssh resolve_dual_stack 用 lookup_host → io::Error，用 context 包装。
        let e = anyhow::anyhow!(io::Error::new(
            io::ErrorKind::Other,
            "failed to lookup address information: Name or service not known"
        ))
        .context("DNS resolution failed");
        let d = diagnose_ssh_failure(&e);
        assert_eq!(d.stage, STAGE_TCP);
        assert_eq!(d.code, "SSH_ERR_DNS_FAILURE");
        assert!(d.detail.contains("DNS resolution failed"));
    }

    #[test]
    fn diag_no_addresses_resolved_is_distinct_from_refused() {
        let e = anyhow::anyhow!("no addresses resolved for 172.20.100.11");
        let d = diagnose_ssh_failure(&e);
        assert_eq!(d.stage, STAGE_TCP);
        assert_eq!(d.code, "SSH_ERR_DNS_FAILURE");
        assert_ne!(d.code, "SSH_ERR_CONNECTION_REFUSED");
    }

    #[test]
    fn diag_kex_no_common_algo_is_handshake_stage() {
        let e = anyhow::anyhow!(
            "No common Kex algorithm - ours: [\"diffie-hellman-group14-sha256\"], theirs: [\"ecdh-sha2-nistp256\"]"
        )
        .context("SSH connection failed");
        let d = diagnose_ssh_failure(&e);
        assert_eq!(d.stage, STAGE_KEX);
        assert_eq!(d.code, "SSH_ERR_KEX_NO_COMMON_ALGO");
        assert!(d.detail.contains("No common Kex algorithm"));
        assert!(d.message().contains("stage=kex"));
    }

    #[test]
    fn diag_invalid_ssh_version_is_handshake_stage() {
        let e = anyhow::anyhow!("invalid SSH version string").context("SSH connection failed");
        let d = diagnose_ssh_failure(&e);
        assert_eq!(d.stage, STAGE_KEX);
        assert_eq!(d.code, "SSH_ERR_SSH_VERSION");
    }

    #[test]
    fn diag_auth_failure_is_distinct_stage_and_keeps_chain_text() {
        // rex_ssh `auth_failed` 聚合文案，包含方法名但不含密钥/密码。
        let e = anyhow::anyhow!(
            "SSH authentication failed (password: partial_success=false, remaining methods: MethodSet([PublicKey]))"
        );
        let d = diagnose_ssh_failure(&e);
        assert_eq!(d.stage, STAGE_AUTH);
        assert_eq!(d.code, "SSH_ERR_AUTH_FAILED");
        // 文案保留 "authentication failed"，使 Hub classify_connect_error 仍判为 Fatal / AUTH_FAILED。
        assert!(d.message().contains("authentication failed"));
        assert!(d.message().contains("stage=auth"));
    }

    #[test]
    fn diag_private_key_decode_is_auth_stage() {
        let e = anyhow::anyhow!("failed to decode private key PEM")
            .context("SSH public key authentication failed");
        let d = diagnose_ssh_failure(&e);
        assert_eq!(d.stage, STAGE_AUTH);
        assert_eq!(d.code, "SSH_ERR_PRIVATE_KEY_DECODE");
    }

    #[test]
    fn diag_remote_closed_during_connect_is_handshake() {
        // russh::Error::HUP（远端在 handshake/TCP 阶段关闭）。
        let e = anyhow::anyhow!("Connection closed by the remote side")
            .context("SSH connection failed");
        let d = diagnose_ssh_failure(&e);
        assert_eq!(d.stage, STAGE_KEX);
        assert_eq!(d.code, "SSH_ERR_REMOTE_CLOSED");
    }

    #[test]
    fn diag_session_open_failure_is_session_stage() {
        let e = anyhow::anyhow!("Channel closed").context("failed to open session");
        let d = diagnose_ssh_failure(&e);
        assert_eq!(d.stage, STAGE_SESSION);
        assert_eq!(d.code, "SSH_ERR_SESSION_OPEN");
    }

    #[test]
    fn diag_distinct_stages_are_not_collapsed() {
        let tcp = diagnose_ssh_failure(&io_chain(
            io::ErrorKind::ConnectionRefused,
            "Connection refused (os error 111)",
        ));
        let auth = diagnose_ssh_failure(&anyhow::anyhow!(
            "SSH authentication failed (password: partial_success=false)"
        ));
        let kex = diagnose_ssh_failure(
            &anyhow::anyhow!("No common Kex algorithm - ours: [a], theirs: [b]")
                .context("SSH connection failed"),
        );
        assert_ne!(tcp.code, auth.code);
        assert_ne!(auth.code, kex.code);
        assert_ne!(tcp.code, kex.code);
        // 面向前端/Hub：三种失败的 message 互不相同。
        assert_ne!(tcp.message(), auth.message());
        assert_ne!(auth.message(), kex.message());
    }

    #[test]
    fn diag_message_format_is_stable_and_excludes_secrets() {
        let planted_secret = "s3cr3t-pw-1234";
        // 现实中 rex_ssh 的 auth 文案只含方法名与 partial_success，不含密码。
        let e = anyhow::anyhow!(
            "SSH authentication failed (password: partial_success=false, remaining methods: MethodSet([PublicKey]))"
        );
        let d = diagnose_ssh_failure(&e);
        let msg = d.message();
        assert!(msg.starts_with("SSH connect failed: stage="));
        assert!(msg.contains("code=SSH_ERR_AUTH_FAILED"));
        // 绝不泄露任何秘密。
        assert!(!msg.contains(planted_secret));
        assert!(!msg.contains("s3cr3t"));
    }
}
