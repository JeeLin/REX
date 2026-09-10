//! SSH protocol implementation — 连接、认证、终端会话。

pub mod sftp;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use bytes::Bytes;
use russh::client;
use russh::keys::{decode_secret_key, PrivateKeyWithHashAlg, PublicKey};
use russh::{Channel, ChannelMsg, ChannelWriteHalf};
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
    /// SSH ProxyJump 跳板机（逗号分隔多个跳板机，按顺序连接）
    /// 格式: `user@host:port` 或 `host:port`
    pub proxy_jump: Option<String>,
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
fn format_ssh_addr(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
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

// ── ProxyJump 解析 ──

/// 解析 ProxyJump 字符串，返回跳板机列表（纯逻辑，便于单元测试）
fn parse_proxy_jump(proxy_jump: &str) -> Vec<String> {
    proxy_jump
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 解析跳板机地址字符串为 (user, host, port)
/// 支持格式：`user@host:port`, `host:port`, `host`
fn parse_jump_host(jump: &str) -> (Option<String>, String, u16) {
    let (user, host_port) = if let Some(at_pos) = jump.find('@') {
        (Some(jump[..at_pos].to_string()), &jump[at_pos + 1..])
    } else {
        (None, jump)
    };

    // 处理 IPv6 [host]:port 格式
    if host_port.starts_with('[') {
        if let Some(bracket_end) = host_port.find(']') {
            let host = &host_port[1..bracket_end];
            let remaining = &host_port[bracket_end + 1..];
            let port = remaining
                .strip_prefix(':')
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(22);
            return (user, host.to_string(), port);
        }
    }

    // 处理 host:port 格式（最后一个冒号分割，避免 IPv6 地址误分割）
    if let Some(colon_pos) = host_port.rfind(':') {
        let host = &host_port[..colon_pos];
        let port_str = &host_port[colon_pos + 1..];
        if let Ok(port) = port_str.parse::<u16>() {
            return (user, host.to_string(), port);
        }
    }

    // 仅主机名
    (user, host_port.to_string(), 22)
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
    pub async fn connect(config: SshConfig) -> Result<Self> {
        let (_handle, session) = Self::connect_with_handle(config).await?;
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

        // 建立连接（支持 ProxyJump 跳板机链）
        let handler = SshHandler;
        let mut handle = if let Some(ref proxy_jump_str) = config.proxy_jump {
            let jumps = parse_proxy_jump(proxy_jump_str);
            if jumps.is_empty() {
                // proxy_jump 为空，直接连接
                Self::connect_direct(&ssh_config, &config.host, config.port, handler).await?
            } else {
                // 通过跳板机链连接
                Self::connect_via_jumps(
                    &ssh_config,
                    &jumps,
                    &config.host,
                    config.port,
                    handler,
                    config.password.as_deref(),
                    config.private_key.as_deref(),
                )
                .await?
            }
        } else {
            Self::connect_direct(&ssh_config, &config.host, config.port, handler).await?
        };

        // 认证
        if let Some(ref key_pem) = config.private_key {
            let private_key = decode_secret_key(key_pem, config.password.as_deref())
                .context("failed to decode private key PEM")?;
            let key_with_hash = PrivateKeyWithHashAlg::new(Arc::new(private_key), None);
            handle
                .authenticate_publickey(&config.username, key_with_hash)
                .await
                .context("SSH public key authentication failed")?;
        } else if let Some(ref password) = config.password {
            handle
                .authenticate_password(&config.username, password)
                .await
                .context("SSH password authentication failed")?;
        } else {
            handle
                .authenticate_none(&config.username)
                .await
                .context("SSH none authentication failed")?;
        }

        // 打开 session channel
        let channel: Channel<client::Msg> = handle
            .channel_open_session()
            .await
            .context("failed to open session")?;

        // 分离读写半区
        let (mut read_half, write_half) = channel.split();

        // 请求 PTY（xterm-256color，80x24 初始尺寸，前端会立即 resize）
        write_half
            .request_pty(true, "xterm-256color", 80, 24, 0, 0, &[])
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

    // ── ProxyJump 内部连接方法 ──

    /// 直接 SSH 连接（无跳板机），使用双栈 DNS 解析
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

    /// 通过跳板机链 SSH 连接
    /// 依次连接每个跳板机，打开 direct-tcpip 通道到下一个节点，最终连接到目标
    async fn connect_via_jumps(
        ssh_config: &Arc<client::Config>,
        jumps: &[String],
        target_host: &str,
        target_port: u16,
        handler: SshHandler,
        password: Option<&str>,
        private_key: Option<&str>,
    ) -> Result<client::Handle<SshHandler>> {
        // 连接第一个跳板机（直接连接）
        let (jump_user, jump_host, jump_port) = parse_jump_host(&jumps[0]);
        let jump_user = jump_user.unwrap_or_else(|| "root".to_string());

        let mut current_handle =
            Self::connect_direct(ssh_config, &jump_host, jump_port, handler.clone()).await?;

        // 认证跳板机
        Self::authenticate_handle(&mut current_handle, &jump_user, password, private_key).await?;
        tracing::info!(jump = %jumps[0], "authenticated to jump host");

        // 依次通过每个跳板机的 direct-tcpip 通道
        for jump_str in jumps.iter().skip(1) {
            let (next_user, next_host, next_port) = parse_jump_host(jump_str);
            let next_user = next_user.unwrap_or_else(|| "root".to_string());

            // 打开 direct-tcpip 通道到下一个跳板机
            let channel = current_handle
                .channel_open_direct_tcpip(&next_host, next_port as u32, "127.0.0.1", 0)
                .await
                .context("failed to open direct-tcpip channel through jump host")?;

            // 通过通道建立 SSH 连接
            let stream = channel.into_stream();
            current_handle = client::connect_stream(ssh_config.clone(), stream, handler.clone())
                .await
                .context("SSH connection through jump channel failed")?;

            // 认证下一个跳板机
            Self::authenticate_handle(&mut current_handle, &next_user, password, private_key)
                .await?;
            tracing::info!(
                jump = %jump_str,
                "authenticated to intermediate jump host"
            );
        }

        // 通过最后一个跳板机打开 direct-tcpip 通道到最终目标
        let channel = current_handle
            .channel_open_direct_tcpip(target_host, target_port as u32, "127.0.0.1", 0)
            .await
            .context("failed to open direct-tcpip channel to target through last jump host")?;

        let stream = channel.into_stream();
        let final_handle = client::connect_stream(ssh_config.clone(), stream, handler)
            .await
            .context("SSH connection to target through jump channel failed")?;

        tracing::info!(
            target = %target_host,
            port = target_port,
            "connected to target through proxy jump chain"
        );
        Ok(final_handle)
    }

    /// 对给定的 SSH Handle 进行认证（跳板机使用与目标相同的凭据）
    async fn authenticate_handle(
        handle: &mut client::Handle<SshHandler>,
        username: &str,
        password: Option<&str>,
        private_key: Option<&str>,
    ) -> Result<()> {
        // 公钥 → 密码 → None
        if let Some(key_pem) = private_key {
            let pk = decode_secret_key(key_pem, password)
                .context("failed to decode private key PEM for jump host")?;
            let key_with_hash = PrivateKeyWithHashAlg::new(Arc::new(pk), None);
            handle
                .authenticate_publickey(username, key_with_hash)
                .await
                .context("jump host public key authentication failed")?;
        } else if let Some(pwd) = password {
            handle
                .authenticate_password(username, pwd)
                .await
                .context("jump host password authentication failed")?;
        } else {
            handle
                .authenticate_none(username)
                .await
                .context("jump host none authentication failed")?;
        }
        Ok(())
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
    fn test_parse_proxy_jump() {
        assert_eq!(parse_proxy_jump("j1.example.com"), vec!["j1.example.com"]);
        assert_eq!(
            parse_proxy_jump("j1.example.com, j2.example.com"),
            vec!["j1.example.com", "j2.example.com"]
        );
        assert_eq!(parse_proxy_jump(""), Vec::<String>::new());
        assert_eq!(parse_proxy_jump(" j1 , j2 , j3 "), vec!["j1", "j2", "j3"]);
    }

    #[test]
    fn test_parse_jump_host() {
        // 简单 host
        assert_eq!(
            parse_jump_host("j1.example.com"),
            (None, "j1.example.com".to_string(), 22)
        );
        // user@host:port
        assert_eq!(
            parse_jump_host("admin@j1.example.com:2222"),
            (
                Some("admin".to_string()),
                "j1.example.com".to_string(),
                2222
            )
        );
        // host:port
        assert_eq!(
            parse_jump_host("j1.example.com:2222"),
            (None, "j1.example.com".to_string(), 2222)
        );
        // IPv6 with brackets
        assert_eq!(
            parse_jump_host("user@[::1]:22"),
            (Some("user".to_string()), "::1".to_string(), 22)
        );
        // IPv6 without port
        assert_eq!(
            parse_jump_host("[2001:db8::1]"),
            (None, "2001:db8::1".to_string(), 22)
        );
    }
}
