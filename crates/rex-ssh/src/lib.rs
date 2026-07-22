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

impl SshSession {
    /// 建立 SSH 连接、分配 PTY、启动 shell，返回会话
    pub async fn connect(config: SshConfig) -> Result<Self> {
        let addr = format!("{}:{}", config.host, config.port);

        // SSH 客户端配置
        let mut ssh_config = client::Config::default();
        if let Some(interval) = config.keepalive_interval {
            if interval > 0 {
                ssh_config.keepalive_interval =
                    Some(std::time::Duration::from_secs(interval as u64));
            }
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
