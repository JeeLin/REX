//! SFTP 文件连接器 — 基于 russh-sftp 实现 FileConnector。

use anyhow::{Context, Result};
use async_trait::async_trait;
use rex_common::file_transfer::{FileConnector, FileEntry, ProgressCallback, UploadResult};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use tokio::io::AsyncWriteExt;

/// SFTP 连接器
pub struct SftpConnector {
    session: SftpSession,
}

impl SftpConnector {
    /// 从 SSH channel 建立 SFTP 连接
    pub async fn connect(channel: russh::Channel<russh::client::Msg>) -> Result<Self> {
        let session = SftpSession::new(channel.into_stream())
            .await
            .context("failed to create SFTP session")?;
        Ok(Self { session })
    }

    /// 从已有的 SSH Handle 打开新 session channel 建立 SFTP 连接
    /// 用于复用已有的 SSH 连接（避免并发会话限制）
    pub async fn connect_from_handle(
        handle: &russh::client::Handle<crate::SshHandler>,
        host: &str,
    ) -> Result<Self> {
        tracing::info!(action = "SFTP_CONNECT", host = %host, "SFTP: opening session channel from existing SSH handle");
        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| {
                tracing::error!(action = "SFTP_CONNECT", host = %host, error = %e, "SFTP: channel_open_session failed from existing handle");
                crate::session_open_error("pooled connection", e)
            })?;
        tracing::info!(action = "SFTP_CONNECT", host = %host, "SFTP: session channel opened from existing handle, creating SFTP session");
        Self::connect(channel).await
    }

    /// 从 SSH 配置直接建立 SFTP 连接
    ///
    /// 优先复用连接池中已有的连接（同 `user@host:port`），复用失败则降级为新建连接。
    pub async fn connect_with_config(config: crate::SshConfig) -> Result<Self> {
        tracing::info!(action = "SFTP_CONNECT", host = %config.host, port = config.port, username = %config.username, has_password = config.password.is_some(), has_key = config.private_key.is_some(), "SFTP: opening session channel");
        let channel = open_session_channel(&config).await?;
        tracing::info!(action = "SFTP_CONNECT", host = %config.host, "SFTP: session channel opened, creating SFTP session");
        Self::connect(channel).await
    }
}

/// 池连接 / 新连接打开 session channel 的超时
const OPEN_SESSION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// 打开一个 session channel：优先复用连接池中的连接，失败则降级为新建连接。
///
/// 1. 池命中 → 在同一条 TCP 连接上打开新 channel（不触发服务端并发会话限制）；
/// 2. 池打开失败 → 按 [`crate::pool::should_evict`] 决定是否剔除该连接，随后降级；
/// 3. 降级 → 新建连接、认证、打开 channel，失败返回统一的 MaxSessions 指引文案。
pub(crate) async fn open_session_channel(
    config: &crate::SshConfig,
) -> Result<russh::Channel<russh::client::Msg>> {
    let key = crate::pool::pool_key(config);
    if let Some(cell) = crate::pool::get(&key).await {
        tracing::info!(action = "SFTP_CONNECT", key = %key, "SFTP: reusing pooled SSH connection");
        let opened = tokio::time::timeout(OPEN_SESSION_TIMEOUT, async {
            let handle = cell.lock().await;
            handle.channel_open_session().await
        })
        .await;
        match opened {
            Ok(Ok(channel)) => return Ok(channel),
            Ok(Err(e)) => {
                tracing::warn!(action = "SFTP_CONNECT", key = %key, error = %e, "SFTP: pooled channel open failed, falling back to a new connection");
                if crate::pool::should_evict(&e) {
                    crate::pool::evict(&key, &cell).await;
                }
            }
            Err(_) => {
                tracing::warn!(action = "SFTP_CONNECT", key = %key, "SFTP: pooled channel open timed out, falling back to a new connection");
                crate::pool::evict(&key, &cell).await;
            }
        }
    }

    // 降级：新建连接
    let handle = fresh_handle(config).await?;
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| {
            tracing::error!(action = "SFTP_CONNECT", host = %config.host, port = config.port, error = %e, "SFTP: channel_open_session failed on new connection");
            crate::session_open_error("new connection", e)
        })?;
    crate::pool::register(&crate::pool::pool_key(config), handle).await;
    Ok(channel)
}

/// 新建一条 SSH 连接并完成认证（不打开 channel）
async fn fresh_handle(
    config: &crate::SshConfig,
) -> Result<russh::client::Handle<crate::SshHandler>> {
    use russh::client;
    use std::sync::Arc;

    let ssh_config = Arc::new(client::Config::default());
    let handler = crate::SshHandler;
    let addr = crate::format_ssh_addr(&config.host, config.port);
    tracing::info!(action = "SFTP_CONNECT", host = %config.host, port = config.port, "SFTP: opening new SSH connection");
    let mut handle = client::connect(ssh_config, &addr, handler)
        .await
        .map_err(|e| {
            tracing::error!(action = "SFTP_CONNECT", host = %config.host, port = config.port, error = %e, "SFTP: SSH connection failed");
            anyhow::anyhow!("SSH connection failed for SFTP: {e}")
        })?;
    tracing::info!(action = "SFTP_CONNECT", host = %config.host, "SFTP: SSH connected, authenticating");

    crate::authenticate(&mut handle, config).await?;

    tracing::info!(action = "SFTP_CONNECT", host = %config.host, "SFTP: auth ok");
    Ok(handle)
}

#[async_trait]
impl FileConnector for SftpConnector {
    async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>> {
        let dir = self
            .session
            .read_dir(path)
            .await
            .context("failed to list directory")?;

        let mut entries = Vec::new();
        for entry in dir {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let meta = entry.metadata();
            let full_path = if path.ends_with('/') {
                format!("{path}{name}")
            } else {
                format!("{path}/{name}")
            };
            let modified = meta.modified().ok().and_then(|t| {
                let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                chrono::DateTime::from_timestamp(dur.as_secs() as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            });
            entries.push(FileEntry {
                name,
                path: full_path,
                is_dir: meta.is_dir(),
                size: meta.len(),
                modified,
                permissions: None,
                storage_class: None,
                acl: None,
            });
        }
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        Ok(entries)
    }

    async fn stat(&mut self, path: &str) -> Result<FileEntry> {
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        // 简化：尝试读取路径，成功则为文件
        match self.session.canonicalize(path).await {
            Ok(resolved) => Ok(FileEntry {
                name,
                path: resolved,
                is_dir: false,
                size: 0,
                modified: None,
                permissions: None,
                storage_class: None,
                acl: None,
            }),
            Err(_) => Ok(FileEntry {
                name,
                path: path.to_string(),
                is_dir: false,
                size: 0,
                modified: None,
                permissions: None,
                storage_class: None,
                acl: None,
            }),
        }
    }

    async fn upload(
        &mut self,
        remote_path: &str,
        data: Vec<u8>,
        offset: u64,
        progress: Option<&ProgressCallback>,
    ) -> Result<UploadResult> {
        let total = data.len() as u64;

        // Clamp offset to data length to prevent panic
        let offset = offset.min(total);

        // If offset > 0, open existing file for append; otherwise create new
        let mut file = if offset > 0 {
            self.session
                .open_with_flags(remote_path, OpenFlags::WRITE | OpenFlags::APPEND)
                .await
                .with_context(|| format!("failed to open {remote_path} for resume"))?
        } else {
            self.session
                .create(remote_path)
                .await
                .with_context(|| format!("failed to create {remote_path}"))?
        };

        let chunk_size = 64 * 1024;
        let mut written = offset;
        // Skip already-uploaded data
        let start = offset as usize;
        for chunk in data[start..].chunks(chunk_size) {
            file.write_all(chunk)
                .await
                .context("failed to write chunk")?;
            written += chunk.len() as u64;
            if let Some(ref cb) = progress {
                cb(written, total);
            }
        }
        file.flush().await.context("failed to flush")?;
        Ok(UploadResult::default())
    }

    async fn download(&mut self, path: &str) -> Result<Vec<u8>> {
        self.session
            .read(path)
            .await
            .with_context(|| format!("failed to read {path}"))
    }

    async fn download_range(
        &mut self,
        path: &str,
        offset: u64,
        limit: Option<u64>,
    ) -> Result<Vec<u8>> {
        let all_data = self
            .session
            .read(path)
            .await
            .with_context(|| format!("failed to read {path}"))?;
        let start = (offset as usize).min(all_data.len());
        let end = match limit {
            Some(len) => ((offset + len) as usize).min(all_data.len()),
            None => all_data.len(),
        };
        Ok(all_data[start..end].to_vec())
    }

    async fn delete(&mut self, path: &str) -> Result<()> {
        if self.session.remove_file(path).await.is_err() {
            self.session
                .remove_dir(path)
                .await
                .with_context(|| format!("failed to delete {path}"))?;
        }
        Ok(())
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        self.session
            .rename(from, to)
            .await
            .with_context(|| format!("failed to rename {from} -> {to}"))?;
        Ok(())
    }

    async fn mkdir(&mut self, path: &str) -> Result<()> {
        self.session
            .create_dir(path)
            .await
            .with_context(|| format!("failed to mkdir {path}"))?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.session.close().await.ok();
        Ok(())
    }

    async fn read_for_edit(&mut self, path: &str) -> Result<Vec<u8>> {
        let data = self
            .session
            .read(path)
            .await
            .with_context(|| format!("failed to read {path}"))?;
        if data.len() > 5 * 1024 * 1024 {
            anyhow::bail!("File too large for editing (>5MB)");
        }
        Ok(data)
    }

    async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> Result<()> {
        let mut file = self
            .session
            .create(path)
            .await
            .with_context(|| format!("failed to create {path} for save"))?;
        file.write_all(&data)
            .await
            .context("failed to write data")?;
        file.flush().await.context("failed to flush")?;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::{Duration, Instant};

    use russh::keys::decode_secret_key;
    use russh::server::{self, Auth, ChannelOpenHandle, Server as _, Session};
    use russh::{Channel, ChannelId, ChannelOpenFailure, Disconnect, Pty};

    use super::*;
    use crate::test_support::HOST_KEY_PEM;
    use crate::{SshConfig, SshSession};

    /// 保证每个 harness 的 pool key（含 username）全局唯一，隔离并行测试
    static NEXT_HARNESS_ID: AtomicUsize = AtomicUsize::new(0);

    struct ServerState {
        conns: AtomicUsize,
        max_sessions: usize,
        accept_auth: bool,
        server_handles: StdMutex<Vec<server::Handle>>,
    }

    struct TestServer {
        state: Arc<ServerState>,
    }

    impl server::Server for TestServer {
        type Handler = TestHandler;

        fn new_client(&mut self, _peer: Option<SocketAddr>) -> TestHandler {
            self.state.conns.fetch_add(1, Ordering::SeqCst);
            TestHandler {
                state: self.state.clone(),
                sessions: 0,
            }
        }
    }

    struct TestHandler {
        state: Arc<ServerState>,
        /// 本条连接上已接受的 session channel 数（模拟 sshd MaxSessions）
        sessions: usize,
    }

    impl TestHandler {
        fn auth(&self) -> Auth {
            if self.state.accept_auth {
                Auth::Accept
            } else {
                Auth::reject()
            }
        }
    }

    impl server::Handler for TestHandler {
        type Error = russh::Error;

        async fn auth_none(&mut self, _user: &str) -> Result<Auth, Self::Error> {
            Ok(self.auth())
        }

        async fn auth_password(
            &mut self,
            _user: &str,
            _password: &str,
        ) -> Result<Auth, Self::Error> {
            Ok(self.auth())
        }

        async fn channel_open_session(
            &mut self,
            _channel: Channel<server::Msg>,
            reply: ChannelOpenHandle,
            _session: &mut Session,
        ) -> Result<(), Self::Error> {
            if self.sessions >= self.state.max_sessions {
                reply
                    .reject(ChannelOpenFailure::AdministrativelyProhibited)
                    .await;
            } else {
                self.sessions += 1;
                reply.accept().await;
            }
            Ok(())
        }

        async fn pty_request(
            &mut self,
            channel: ChannelId,
            _term: &str,
            _col_width: u32,
            _row_height: u32,
            _pix_width: u32,
            _pix_height: u32,
            _modes: &[(Pty, u32)],
            session: &mut Session,
        ) -> Result<(), Self::Error> {
            let _ = session.channel_success(channel);
            Ok(())
        }

        async fn shell_request(
            &mut self,
            channel: ChannelId,
            session: &mut Session,
        ) -> Result<(), Self::Error> {
            let _ = session.channel_success(channel);
            Ok(())
        }
    }

    /// 回环测试 SSH server：可配置 MaxSessions 与认证是否放行
    struct Harness {
        addr: SocketAddr,
        state: Arc<ServerState>,
        username: String,
    }

    impl Harness {
        async fn start(max_sessions: usize, accept_auth: bool) -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind loopback sshd");
            let addr = listener.local_addr().expect("local addr");
            let state = Arc::new(ServerState {
                conns: AtomicUsize::new(0),
                max_sessions,
                accept_auth,
                server_handles: StdMutex::new(Vec::new()),
            });

            let server_config = server::Config {
                auth_rejection_time: Duration::from_millis(10),
                keys: vec![
                    decode_secret_key(&format!("{HOST_KEY_PEM}\n"), None).expect("decode host key")
                ],
                ..Default::default()
            };
            let server_config = Arc::new(server_config);

            let accept_state = state.clone();
            tokio::spawn(async move {
                let mut srv = TestServer {
                    state: accept_state.clone(),
                };
                loop {
                    let Ok((socket, _)) = listener.accept().await else {
                        break;
                    };
                    let handler = srv.new_client(socket.peer_addr().ok());
                    let config = server_config.clone();
                    let state = accept_state.clone();
                    tokio::spawn(async move {
                        if let Ok(running) = server::run_stream(config, socket, handler).await {
                            state
                                .server_handles
                                .lock()
                                .expect("lock handles")
                                .push(running.handle());
                            let _ = running.await;
                        }
                    });
                }
            });

            let id = NEXT_HARNESS_ID.fetch_add(1, Ordering::SeqCst);
            Harness {
                addr,
                state,
                username: format!("tester{id}"),
            }
        }

        fn conns(&self) -> usize {
            self.state.conns.load(Ordering::SeqCst)
        }

        fn config(&self) -> SshConfig {
            SshConfig {
                host: "127.0.0.1".to_string(),
                port: self.addr.port(),
                username: self.username.clone(),
                password: Some("secret".to_string()),
                private_key: None,
                keepalive_interval: Some(0),
                init_script: None,
            }
        }

        /// 主动断开所有已建立的服务端连接（模拟服务端掐线）
        async fn disconnect_all(&self) {
            let handles =
                std::mem::take(&mut *self.state.server_handles.lock().expect("lock handles"));
            for handle in handles {
                let _ = handle
                    .disconnect(
                        Disconnect::ByApplication,
                        "test shutdown".to_string(),
                        String::new(),
                    )
                    .await;
            }
        }
    }

    /// 方向 a：SFTP 在终端已建立的连接上开 channel，不新建第二条连接
    #[tokio::test]
    async fn sftp_reuses_terminal_connection_from_pool() {
        let harness = Harness::start(usize::MAX, true).await;
        let cfg = harness.config();
        let _terminal = SshSession::connect(cfg.clone())
            .await
            .expect("terminal connect");
        assert_eq!(harness.conns(), 1);

        let key = crate::pool::pool_key(&cfg);
        assert!(
            crate::pool::get(&key).await.is_some(),
            "terminal handle must be registered into the pool"
        );

        let channel = open_session_channel(&cfg)
            .await
            .expect("sftp channel over pooled connection");
        assert_eq!(
            harness.conns(),
            1,
            "SFTP must reuse the terminal's TCP connection"
        );
        drop(channel);
    }

    /// 方向 b：池连接已被服务端掐断 → 剔除死句柄并降级为新连接
    #[tokio::test]
    async fn sftp_falls_back_when_pooled_handle_is_dead() {
        let harness = Harness::start(usize::MAX, true).await;
        let cfg = harness.config();
        let _terminal = SshSession::connect(cfg.clone())
            .await
            .expect("terminal connect");
        assert_eq!(harness.conns(), 1);
        let key = crate::pool::pool_key(&cfg);

        harness.disconnect_all().await;

        // 等待客户端感知断开：pool::get 对已关闭连接返回 None 并剔除
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if crate::pool::get(&key).await.is_none() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "pooled handle never reported closed"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let channel = open_session_channel(&cfg)
            .await
            .expect("should degrade to a fresh connection");
        assert_eq!(
            harness.conns(),
            2,
            "dead pooled handle must trigger a new SSH connection"
        );
        assert!(
            crate::pool::get(&key).await.is_some(),
            "fresh connection should be re-registered"
        );
        drop(channel);
    }

    /// MaxSessions=1 拒绝第二条 channel：连接仍存活 → 保留池句柄并降级新连接
    #[tokio::test]
    async fn sftp_degrades_after_channel_open_rejection() {
        let harness = Harness::start(1, true).await;
        let cfg = harness.config();
        let _terminal = SshSession::connect(cfg.clone())
            .await
            .expect("terminal connect");
        assert_eq!(harness.conns(), 1);

        let channel = open_session_channel(&cfg)
            .await
            .expect("should degrade after channel open rejection");
        assert_eq!(
            harness.conns(),
            2,
            "rejection on the pooled connection must fall back to a new connection"
        );
        drop(channel);
    }

    /// 认证被拒必须报 authentication failed，而不是误导性的 Disconnected
    #[tokio::test]
    async fn sftp_reports_authentication_failure_clearly() {
        let harness = Harness::start(usize::MAX, false).await;
        let cfg = harness.config();
        let err = match SftpConnector::connect_with_config(cfg).await {
            Ok(_) => panic!("rejected authentication must fail"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("authentication failed"),
            "unexpected error: {msg}"
        );
        assert!(
            !msg.contains("Disconnected"),
            "must not misreport as Disconnected: {msg}"
        );
    }
}
