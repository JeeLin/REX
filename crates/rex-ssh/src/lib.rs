//! SSH protocol implementation — 连接、认证、终端会话。

pub mod sftp;

pub(crate) mod pool;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use bytes::Bytes;
use russh::client;
use russh::keys::{decode_secret_key, PrivateKeyWithHashAlg, PublicKey};
use russh::{Channel, ChannelMsg, ChannelWriteHalf, Pty};
use tokio::sync::mpsc;

/// SSH 连接配置
#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    /// KeepAlive 间隔（秒），0 表示禁用
    pub keepalive_interval: Option<u32>,
    /// 会话建立后自动执行的初始化脚本（多行以 `\n` 分隔，逐行发送）
    pub init_script: Option<String>,
}

/// 终端事件 — 从 SSH 会话流向 WebSocket
#[derive(Debug)]
pub enum TerminalEvent {
    /// 终端输出数据（UTF-8）
    Data(String),
    /// SSH 会话断开
    Disconnected(String),
}

/// SSH 终端会话 — 管理与远端服务器的连接和数据转发
pub struct SshSession {
    /// SSH 写半区（用于发送数据 / resize / close）
    write_half: ChannelWriteHalf<client::Msg>,
    /// SSH 事件接收（终端输出 / 断开通知）
    events: mpsc::Receiver<TerminalEvent>,
}

/// SSH Handle 类型别名（用于在 Agent 侧共享连接）
pub type SshHandle = client::Handle<SshHandler>;

/// 将初始化脚本按行拆分，跳过空行并去除行尾空白。
/// 纯逻辑，便于单元测试。
fn split_init_script(script: &str) -> Vec<String> {
    script
        .lines()
        .map(|l| l.trim_end().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// 拼装 SSH 连接地址；IPv6 需加方括号（已有方括号不再重复添加）。
/// 纯逻辑，便于单元测试。
pub(crate) fn format_ssh_addr(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

/// 显式检查认证结果。
///
/// russh 认证失败返回 `Ok(AuthResult::Failure)` 而非 `Err`，不检查会把
/// 「认证被拒」伪装成后续开 channel 时的 `Disconnected`，误导排查方向。
/// 纯逻辑，便于单元测试。
pub(crate) fn ensure_auth_success(result: client::AuthResult) -> Result<()> {
    match result {
        client::AuthResult::Success => Ok(()),
        client::AuthResult::Failure {
            remaining_methods,
            partial_success,
        } => Err(anyhow::anyhow!(
            "SSH authentication failed (partial_success={partial_success}, remaining methods: {remaining_methods:?})"
        )),
    }
}

/// 对已建立的 SSH 连接按配置完成认证（公钥 → 密码 → none）并校验认证结果。
/// 终端会话与 SFTP 新建连接共用，避免三认证分支重复。
pub(crate) async fn authenticate(
    handle: &mut client::Handle<SshHandler>,
    config: &SshConfig,
) -> Result<()> {
    if let Some(ref key_pem) = config.private_key {
        let private_key = decode_secret_key(key_pem, config.password.as_deref())
            .context("failed to decode private key PEM")?;
        let key_with_hash = PrivateKeyWithHashAlg::new(Arc::new(private_key), None);
        let result = handle
            .authenticate_publickey(&config.username, key_with_hash)
            .await
            .context("SSH public key authentication failed")?;
        ensure_auth_success(result)?;
    } else if let Some(ref password) = config.password {
        let result = handle
            .authenticate_password(&config.username, password)
            .await
            .context("SSH password authentication failed")?;
        ensure_auth_success(result)?;
    } else {
        let result = handle
            .authenticate_none(&config.username)
            .await
            .context("SSH none authentication failed")?;
        ensure_auth_success(result)?;
    }
    Ok(())
}

/// 统一 session channel 打开失败的错误文案（含 MaxSessions 排查指引）。
/// `origin` 标注连接来源，便于区分池复用路径与新建连接路径。纯逻辑，便于单元测试。
pub(crate) fn session_open_error(origin: &str, e: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(
        "failed to open session ({origin}): {e}. The SSH server may not support concurrent sessions. Try: (1) set MaxSessions ≥2 in sshd_config, or (2) disconnect the terminal first and retry SFTP."
    )
}

// ── 地址类型检测 ──

/// 地址类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddrType {
    /// IPv4 地址（如 192.168.1.1）
    Ipv4,
    /// IPv6 地址（如 ::1, 2001:db8::1）
    Ipv6,
    /// 主机名（如 example.com）
    Hostname,
}

/// 检测地址类型（纯逻辑，便于单元测试）
fn classify_addr(host: &str) -> AddrType {
    let h = host
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(host);
    if h.parse::<std::net::Ipv4Addr>().is_ok() {
        AddrType::Ipv4
    } else if h.parse::<std::net::Ipv6Addr>().is_ok() {
        AddrType::Ipv6
    } else {
        AddrType::Hostname
    }
}

// ── 终端模式 ──

/// 返回 PTY 请求的标准终端模式。
///
/// 空模式 (`&[]`) 会导致 SSH 服务器不设置任何终端属性，vim/nano 等全屏编辑器
/// 可能无法正常渲染（屏幕闪烁、光标错位、输入丢失等）。传入标准模式确保
/// 服务器端 terminal driver 以合理的默认值运行。
fn default_terminal_modes() -> Vec<(Pty, u32)> {
    vec![
        // 输入标志
        (Pty::ICRNL, 1), // 将 CR (0x0D) 转换为 NL (0x0A)
        (Pty::IXON, 1),  // 启用 XON/XOFF 流控
        (Pty::IUTF8, 1), // UTF-8 输入模式
        // 本地标志
        (Pty::ISIG, 1),    // 启用信号（Ctrl+C → SIGINT, Ctrl+Z → SIGTSTP）
        (Pty::ICANON, 1),  // 规范模式（行缓冲，Backspace/Delete 正常工作）
        (Pty::ECHO, 1),    // 回显输入
        (Pty::ECHOE, 1),   // 退格时删除前一个字符（视觉上）
        (Pty::ECHOK, 1),   // 删除行时回显换行
        (Pty::IEXTEN, 1),  // 启用扩展输入（Ctrl+V, Ctrl+O 等）
        (Pty::ECHOCTL, 1), // 控制字符可见（如 ^C 显示为 ^C）
        (Pty::ECHOKE, 1),  // 删除行时视觉删除
        // 输出标志
        (Pty::OPOST, 1), // 启用输出处理
        (Pty::ONLCR, 1), // 将 NL 转换为 CR-NL（终端输出换行正确）
        // 特殊字符（保持默认值，确保 vim 快捷键正常）
        (Pty::VINTR, 3),    // Ctrl+C → 中断信号
        (Pty::VQUIT, 28),   // Ctrl+\ → 退出信号
        (Pty::VERASE, 127), // Backspace → 删除
        (Pty::VKILL, 21),   // Ctrl+U → 删除行
        (Pty::VEOF, 4),     // Ctrl+D → EOF
        (Pty::VSTART, 17),  // Ctrl+Q → 恢复输出（XON）
        (Pty::VSTOP, 19),   // Ctrl+S → 暂停输出（XOFF）
        (Pty::VSUSP, 26),   // Ctrl+Z → 挂起信号
    ]
}

// ── 双栈 DNS 解析 ──

/// 双栈 DNS 解析：优先 IPv4，失败后尝试 IPv6
/// 对于已经是 IP 地址的输入直接返回。
async fn resolve_dual_stack(host: &str, port: u16) -> Result<Vec<SocketAddr>> {
    match classify_addr(host) {
        AddrType::Ipv4 => {
            let addr: SocketAddr = format!("{host}:{port}")
                .parse()
                .context("invalid IPv4 address")?;
            Ok(vec![addr])
        }
        AddrType::Ipv6 => {
            let h = host
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .unwrap_or(host);
            let addr: SocketAddr = format!("{h}:{port}")
                .parse()
                .context("invalid IPv6 address")?;
            Ok(vec![addr])
        }
        AddrType::Hostname => {
            let addrs = try_resolve_with_preference(host, port).await?;
            Ok(addrs)
        }
    }
}

/// 尝试按偏好顺序解析主机名（IPv4 优先）
async fn try_resolve_with_preference(host: &str, port: u16) -> Result<Vec<SocketAddr>> {
    let lookup_str = format!("{host}:{port}");
    let all_addrs: Vec<SocketAddr> = tokio::net::lookup_host(&lookup_str)
        .await
        .context("DNS resolution failed")?
        .collect();

    if all_addrs.is_empty() {
        anyhow::bail!("no addresses resolved for {host}");
    }

    // 按 IPv4 优先排序
    let mut ipv4_addrs: Vec<SocketAddr> =
        all_addrs.iter().filter(|a| a.is_ipv4()).copied().collect();
    let mut ipv6_addrs: Vec<SocketAddr> =
        all_addrs.iter().filter(|a| a.is_ipv6()).copied().collect();

    ipv4_addrs.append(&mut ipv6_addrs);
    Ok(ipv4_addrs)
}

impl SshSession {
    /// 建立 SSH 连接、分配 PTY、启动 shell，返回会话
    ///
    /// 连接会注册进进程内连接池，供后续 SFTP 在同一条 TCP 连接上打开
    /// session channel（避免服务端并发会话限制）。
    pub async fn connect(config: SshConfig) -> Result<Self> {
        let pool_key = pool::pool_key(&config);
        let (handle, session) = Self::connect_with_handle(config).await?;
        if let Some(key) = pool_key {
            pool::register(&key, handle).await;
        }
        Ok(session)
    }

    /// 建立 SSH 连接、分配 PTY、启动 shell，返回 (Handle, 会话)
    /// Handle 可用于在同一连接上打开额外的 channel（如 SFTP）
    pub async fn connect_with_handle(
        config: SshConfig,
    ) -> Result<(client::Handle<SshHandler>, Self)> {
        // SSH 客户端配置
        let mut ssh_config = client::Config::default();
        let keepalive = config.keepalive_interval.unwrap_or(60);
        if keepalive > 0 {
            ssh_config.keepalive_interval = Some(std::time::Duration::from_secs(keepalive as u64));
        }
        let ssh_config = Arc::new(ssh_config);

        // 建立连接
        let handler = SshHandler;
        let mut handle =
            Self::connect_direct(&ssh_config, &config.host, config.port, handler).await?;

        // 认证
        authenticate(&mut handle, &config).await?;

        // 打开 session channel
        let channel: Channel<client::Msg> = handle
            .channel_open_session()
            .await
            .context("failed to open session")?;

        // 分离读写半区
        let (mut read_half, write_half) = channel.split();

        // 请求 PTY（xterm-256color，80x24 初始尺寸，前端会立即 resize）
        // 传入标准终端模式以确保 vim/nano 等全屏编辑器正常工作。
        // 空模式 (&[]) 会导致服务器不设置任何终端属性，vim 等程序可能无法正常渲染。
        write_half
            .request_pty(
                true,
                "xterm-256color",
                80,
                24,
                0,
                0,
                &default_terminal_modes(),
            )
            .await
            .context("failed to request PTY")?;

        // 请求 shell
        write_half
            .request_shell(true)
            .await
            .context("failed to request shell")?;

        // 会话建立后执行初始化脚本（逐行发送，失败仅记录不阻断）
        if let Some(ref script) = config.init_script {
            for line in split_init_script(script) {
                if let Err(e) = write_half.data(format!("{}\n", line).as_bytes()).await {
                    tracing::warn!("init_script line failed: {e}");
                    break;
                }
            }
        }

        // 事件通道：SSH 读取 → WebSocket 写入
        let (event_tx, event_rx) = mpsc::channel::<TerminalEvent>(512);

        // 后台任务：SSH read_half → event_tx（终端输出）
        tokio::spawn(async move {
            loop {
                match read_half.wait().await {
                    Some(ChannelMsg::Data { data }) => {
                        let s = String::from_utf8_lossy(&data).into_owned();
                        if event_tx.send(TerminalEvent::Data(s)).await.is_err() {
                            break;
                        }
                    }
                    Some(ChannelMsg::ExtendedData { data, .. }) => {
                        let s = String::from_utf8_lossy(&data).into_owned();
                        if event_tx.send(TerminalEvent::Data(s)).await.is_err() {
                            break;
                        }
                    }
                    Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) => {
                        let _ = event_tx
                            .send(TerminalEvent::Disconnected("session closed".into()))
                            .await;
                        break;
                    }
                    None => {
                        let _ = event_tx
                            .send(TerminalEvent::Disconnected("channel closed".into()))
                            .await;
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok((
            handle,
            Self {
                write_half,
                events: event_rx,
            },
        ))
    }

    /// 向 SSH 发送终端输入
    pub async fn send_data(&self, data: Bytes) -> Result<()> {
        self.write_half
            .data_bytes(data)
            .await
            .context("failed to send data")?;
        Ok(())
    }

    /// 发送终端 resize 事件
    pub async fn resize(&self, cols: u32, rows: u32) -> Result<()> {
        self.write_half
            .window_change(cols, rows, 0, 0)
            .await
            .context("failed to resize")?;
        Ok(())
    }

    /// 断开 SSH 连接
    pub async fn disconnect(&self) -> Result<()> {
        let _ = self.write_half.eof().await;
        Ok(())
    }

    /// 拆分会话为写半区和事件接收器，用于并发读写（避免 Mutex 死锁）
    pub fn split(self) -> (ChannelWriteHalf<client::Msg>, mpsc::Receiver<TerminalEvent>) {
        (self.write_half, self.events)
    }

    /// 接收下一个终端事件（阻塞直到有数据或断开）
    pub async fn recv(&mut self) -> Option<TerminalEvent> {
        self.events.recv().await
    }

    /// 直接 SSH 连接，使用双栈 DNS 解析
    async fn connect_direct(
        ssh_config: &Arc<client::Config>,
        host: &str,
        port: u16,
        handler: SshHandler,
    ) -> Result<client::Handle<SshHandler>> {
        let addrs = resolve_dual_stack(host, port).await?;
        let mut last_err = None;
        for addr in &addrs {
            let formatted = format_ssh_addr(&addr.ip().to_string(), addr.port());
            match client::connect(ssh_config.clone(), &formatted, handler.clone()).await {
                Ok(handle) => return Ok(handle),
                Err(e) => {
                    tracing::debug!(
                        addr = %formatted,
                        error = %e,
                        "direct SSH connect failed, trying next address"
                    );
                    last_err = Some(e);
                }
            }
        }
        match last_err {
            Some(e) => Err(anyhow::anyhow!(e)),
            None => anyhow::bail!("no addresses to connect to {host}"),
        }
        .context("SSH connection failed")
    }
}

/// SSH 客户端事件处理器（开发阶段跳过主机密钥校验）
pub struct SshHandler;

impl Clone for SshHandler {
    fn clone(&self) -> Self {
        SshHandler
    }
}

impl client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        // DEV ONLY: 跳过主机密钥校验（生产环境应校验 known_hosts）
        tracing::warn!("SSH host key verification disabled (dev mode) — MITM risk");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_init_script() {
        // 单行
        assert_eq!(split_init_script("echo hi"), vec!["echo hi"]);
        // 多行，跳过空行
        assert_eq!(
            split_init_script("cd /data/logs\n\necho ready\n"),
            vec!["cd /data/logs", "echo ready"]
        );
        // 去除行尾空白
        assert_eq!(split_init_script("ls   \n"), vec!["ls"]);
        // 全空
        assert!(split_init_script("\n\n").is_empty());
        // 空字符串即无命令
        assert!(split_init_script("").is_empty());
    }

    #[test]
    fn test_format_ssh_addr_ipv4() {
        assert_eq!(format_ssh_addr("192.168.1.1", 22), "192.168.1.1:22");
    }

    #[test]
    fn test_format_ssh_addr_ipv6_gets_brackets() {
        assert_eq!(format_ssh_addr("::1", 22), "[::1]:22");
        assert_eq!(format_ssh_addr("2001:db8::1", 2222), "[2001:db8::1]:2222");
    }

    #[test]
    fn test_format_ssh_addr_ipv6_existing_brackets_untouched() {
        assert_eq!(format_ssh_addr("[::1]", 22), "[::1]:22");
    }

    #[test]
    fn test_format_ssh_addr_hostname() {
        assert_eq!(format_ssh_addr("example.com", 22), "example.com:22");
    }

    #[test]
    fn test_classify_addr() {
        assert_eq!(classify_addr("192.168.1.1"), AddrType::Ipv4);
        assert_eq!(classify_addr("::1"), AddrType::Ipv6);
        assert_eq!(classify_addr("2001:db8::1"), AddrType::Ipv6);
        assert_eq!(classify_addr("[::1]"), AddrType::Ipv6);
        assert_eq!(classify_addr("example.com"), AddrType::Hostname);
        assert_eq!(classify_addr("localhost"), AddrType::Hostname);
    }

    #[test]
    fn test_ensure_auth_success_accepts_success() {
        assert!(ensure_auth_success(client::AuthResult::Success).is_ok());
    }

    #[test]
    fn test_ensure_auth_success_reports_rejection() {
        let err = ensure_auth_success(client::AuthResult::Failure {
            remaining_methods: (&[russh::MethodKind::Password][..]).into(),
            partial_success: false,
        })
        .expect_err("rejected auth must fail");
        let msg = err.to_string();
        assert!(msg.contains("authentication failed"), "{msg}");
        assert!(!msg.contains("Disconnected"), "{msg}");
    }

    #[test]
    fn test_session_open_error_carries_origin_and_maxsessions_hint() {
        let msg = session_open_error("new connection", "Disconnected").to_string();
        assert!(
            msg.starts_with("failed to open session (new connection): Disconnected"),
            "{msg}"
        );
        assert!(msg.contains("MaxSessions"), "{msg}");

        let pooled = session_open_error("pooled connection", "boom").to_string();
        assert!(
            pooled.starts_with("failed to open session (pooled connection): boom"),
            "{pooled}"
        );
    }
}
