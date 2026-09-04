//! SSH protocol implementation — 连接、认证、终端会话。

pub mod sftp;

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

impl SshSession {
    /// 建立 SSH 连接、分配 PTY、启动 shell，返回会话
    pub async fn connect(config: SshConfig) -> Result<Self> {
        let addr = format_ssh_addr(&config.host, config.port);

        // SSH 客户端配置
        let mut ssh_config = client::Config::default();
        // Default keepalive: 60s if not specified
        let keepalive = config.keepalive_interval.unwrap_or(60);
        if keepalive > 0 {
            ssh_config.keepalive_interval = Some(std::time::Duration::from_secs(keepalive as u64));
        }
        let ssh_config = Arc::new(ssh_config);

        // 建立连接
        let handler = SshHandler;
        let mut handle = client::connect(ssh_config, &addr, handler)
            .await
            .context("SSH connection failed")?;

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

        Ok(Self {
            write_half,
            events: event_rx,
        })
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
}

/// SSH 客户端事件处理器（开发阶段跳过主机密钥校验）
pub struct SshHandler;

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
}
