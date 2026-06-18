use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use russh::*;
use russh_keys::key::PrivateKeyWithHashAlg;
use russh_keys::*;
use tokio::sync::mpsc;

use crate::auth::AuthMethod;

// ── SSH 事件 ─────────────────────────────────────────────

/// SSH 数据事件，由 recv() 返回
#[derive(Debug)]
pub struct SshEvent {
    /// 终端输出数据
    pub data: Vec<u8>,
    /// 连接已关闭
    pub closed: bool,
    /// 错误信息
    pub error: Option<String>,
}

// ── Client Handler ───────────────────────────────────────

struct ClientHandler {
    data_tx: mpsc::UnboundedSender<Vec<u8>>,
    closed_tx: mpsc::UnboundedSender<()>,
}

#[async_trait]
impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // 跳过 known_hosts 校验（与 M3a 设计一致）
        Ok(true)
    }

    async fn data(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let _ = self.data_tx.send(data.to_vec());
        Ok(())
    }

    async fn channel_close(
        &mut self,
        _channel: ChannelId,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let _ = self.closed_tx.send(());
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        _channel: ChannelId,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let _ = self.closed_tx.send(());
        Ok(())
    }

    async fn disconnected(
        &mut self,
        _reason: client::DisconnectReason<Self::Error>,
    ) -> Result<(), Self::Error> {
        let _ = self.closed_tx.send(());
        Ok(())
    }
}

// ── SshClient ────────────────────────────────────────────

/// SSH 客户端封装
pub struct SshClient {
    handle: client::Handle<ClientHandler>,
    channel: Option<Channel<client::Msg>>,
    data_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    closed_rx: mpsc::UnboundedReceiver<()>,
}

impl SshClient {
    /// 连接到 SSH 服务器并认证
    pub async fn connect(host: &str, port: u16, username: &str, auth: AuthMethod) -> Result<Self> {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        let (closed_tx, closed_rx) = mpsc::unbounded_channel();

        let handler = ClientHandler { data_tx, closed_tx };
        let config = Arc::new(client::Config::default());

        let addr = format!("{}:{}", host, port);
        let mut handle = client::connect(config, addr.as_str(), handler)
            .await
            .with_context(|| format!("failed to connect to SSH server at {}", addr))?;

        // 认证
        match auth {
            AuthMethod::Password(password) => {
                let result = handle
                    .authenticate_password(username, password)
                    .await
                    .context("password authentication failed")?;
                if !result.success() {
                    anyhow::bail!("password authentication rejected by server");
                }
            }
            AuthMethod::Key {
                private_key_path,
                passphrase,
            } => {
                let key_pair = load_secret_key(&private_key_path, passphrase.as_deref())
                    .with_context(|| {
                        format!("failed to load private key from {}", private_key_path)
                    })?;
                let result = handle
                    .authenticate_publickey(
                        username,
                        PrivateKeyWithHashAlg::new(Arc::new(key_pair), None)
                            .context("failed to create PrivateKeyWithHashAlg")?,
                    )
                    .await
                    .context("public key authentication failed")?;
                if !result.success() {
                    anyhow::bail!("public key authentication rejected by server");
                }
            }
        }

        // 打开 session channel
        let channel = handle
            .channel_open_session()
            .await
            .context("failed to open SSH session channel")?;

        tracing::info!(host = %host, port = port, username = %username, "SSH connected and authenticated");

        Ok(Self {
            handle,
            channel: Some(channel),
            data_rx,
            closed_rx,
        })
    }

    /// 请求 PTY
    pub async fn request_pty(&mut self, cols: u32, rows: u32) -> Result<()> {
        let channel = self.channel.as_ref().context("SSH channel not open")?;
        channel
            .request_pty(false, "xterm-256color", cols, rows, 0, 0, &[])
            .await
            .context("failed to request PTY")?;
        Ok(())
    }

    /// 请求 shell
    pub async fn request_shell(&mut self) -> Result<()> {
        let channel = self.channel.as_ref().context("SSH channel not open")?;
        channel
            .request_shell(true)
            .await
            .context("failed to request shell")?;
        Ok(())
    }

    /// 发送终端输入数据
    pub async fn send_data(&mut self, data: &[u8]) -> Result<()> {
        let channel = self.channel.as_ref().context("SSH channel not open")?;
        // beta.7 的 data() 接受 R: AsyncRead + Unpin，&[u8] 实现了 AsyncRead
        channel.data(data).await.context("failed to send data")?;
        Ok(())
    }

    /// 调整终端窗口大小
    pub async fn window_change(&mut self, cols: u32, rows: u32) -> Result<()> {
        let channel = self.channel.as_ref().context("SSH channel not open")?;
        channel
            .window_change(cols, rows, cols * 8, rows * 16)
            .await
            .context("failed to change window size")?;
        Ok(())
    }

    /// 接收 SSH 数据事件
    pub async fn recv(&mut self) -> SshEvent {
        tokio::select! {
            data = self.data_rx.recv() => {
                match data {
                    Some(d) => SshEvent {
                        data: d,
                        closed: false,
                        error: None,
                    },
                    None => SshEvent {
                        data: vec![],
                        closed: true,
                        error: Some("channel closed".to_string()),
                    },
                }
            }
            _ = self.closed_rx.recv() => SshEvent {
                data: vec![],
                closed: true,
                error: None,
            },
        }
    }

    /// 关闭连接
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(channel) = self.channel.take() {
            let _ = channel.close().await;
        }
        self.handle
            .disconnect(Disconnect::ByApplication, "", "English")
            .await
            .context("failed to disconnect")?;
        tracing::info!("SSH connection closed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_event_struct() {
        let event = SshEvent {
            data: vec![1, 2, 3],
            closed: false,
            error: None,
        };
        assert!(!event.data.is_empty());
        assert!(!event.closed);
        assert!(event.error.is_none());
    }

    #[test]
    fn ssh_event_closed() {
        let event = SshEvent {
            data: vec![],
            closed: true,
            error: Some("test".to_string()),
        };
        assert!(event.closed);
        assert_eq!(event.error.unwrap(), "test");
    }
}
